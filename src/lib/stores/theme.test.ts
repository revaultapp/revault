import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

describe("theme store", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    vi.resetModules();
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
});
