import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: vi.fn() }));

describe("settings store", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
  });

  it("defaultOutputDir defaults to null", async () => {
    const { defaultOutputDir } = await import("./settings");
    expect(get(defaultOutputDir)).toBeNull();
  });

  it("defaultOutputDir persists value to localStorage", async () => {
    const { defaultOutputDir } = await import("./settings");
    defaultOutputDir.set("/Users/mike/Desktop");
    expect(localStorage.getItem("settings-default-output-dir")).toBe(
      JSON.stringify("/Users/mike/Desktop"),
    );
  });

  it("defaultOutputDir rehydrates from localStorage on reimport", async () => {
    localStorage.setItem(
      "settings-default-output-dir",
      JSON.stringify("/Users/mike/Documents"),
    );
    const { defaultOutputDir } = await import("./settings");
    expect(get(defaultOutputDir)).toBe("/Users/mike/Documents");
  });
});
