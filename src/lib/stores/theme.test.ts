import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

function mockMatchMedia(initialMatches: boolean) {
  const listeners = new Set<(e: { matches: boolean }) => void>();
  const mql = {
    matches: initialMatches,
    media: "(prefers-color-scheme: dark)",
    addEventListener: vi.fn((_event: string, cb: (e: { matches: boolean }) => void) => {
      listeners.add(cb);
    }),
    removeEventListener: vi.fn((_event: string, cb: (e: { matches: boolean }) => void) => {
      listeners.delete(cb);
    }),
    dispatchEvent: () => false,
  };
  window.matchMedia = vi.fn().mockReturnValue(mql);
  return {
    mql,
    fireChange(matches: boolean) {
      mql.matches = matches;
      listeners.forEach((cb) => cb({ matches }));
    },
  };
}

describe("theme store", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    vi.resetModules();
    mockMatchMedia(false);
  });

  it('default is "dark" when localStorage empty', async () => {
    const { theme } = await import("./theme");
    expect(get(theme)).toBe("dark");
  });

  it("persists to localStorage on change", async () => {
    const { theme } = await import("./theme");
    theme.set("light");
    expect(localStorage.getItem("theme")).toBe("light");
  });

  it("reads stored value on init", async () => {
    localStorage.setItem("theme", "light");
    const { theme } = await import("./theme");
    expect(get(theme)).toBe("light");
  });

  it('persists "system" as the raw value, not the resolved theme', async () => {
    const { theme } = await import("./theme");
    theme.set("system");
    expect(localStorage.getItem("theme")).toBe("system");
  });

  it("system mode resolves to dark when the OS prefers dark", async () => {
    mockMatchMedia(true);
    const { theme } = await import("./theme");
    theme.set("system");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("system mode resolves to light when the OS prefers light", async () => {
    mockMatchMedia(false);
    const { theme } = await import("./theme");
    theme.set("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("re-applies live when the OS preference changes while in system mode", async () => {
    const { fireChange } = mockMatchMedia(false);
    const { theme } = await import("./theme");
    theme.set("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);

    fireChange(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");

    fireChange(false);
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("stops reacting to OS changes after leaving system mode", async () => {
    const { mql, fireChange } = mockMatchMedia(false);
    const { theme } = await import("./theme");
    theme.set("system");
    theme.set("light");

    expect(mql.removeEventListener).toHaveBeenCalled();

    fireChange(true);
    // Still light — the change listener was detached on leaving system mode.
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});
