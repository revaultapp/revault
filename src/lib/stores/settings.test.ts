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

  describe("global processing defaults", () => {
    it("defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy all default to null", async () => {
      const { defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } = await import("./settings");
      expect(get(defaultImagePreset)).toBeNull();
      expect(get(defaultVideoPreset)).toBeNull();
      expect(get(defaultVideoPrivacy)).toBeNull();
    });

    it("persist their values to localStorage on change", async () => {
      const { defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } = await import("./settings");
      defaultImagePreset.set("Smallest");
      defaultVideoPreset.set("HighQuality");
      defaultVideoPrivacy.set("full");
      expect(localStorage.getItem("settings-default-image-preset")).toBe(JSON.stringify("Smallest"));
      expect(localStorage.getItem("settings-default-video-preset")).toBe(JSON.stringify("HighQuality"));
      expect(localStorage.getItem("settings-default-video-privacy")).toBe(JSON.stringify("full"));
    });

    it("rehydrate from localStorage on reimport, including back to null", async () => {
      localStorage.setItem("settings-default-image-preset", JSON.stringify("Balanced"));
      const { defaultImagePreset } = await import("./settings");
      expect(get(defaultImagePreset)).toBe("Balanced");

      defaultImagePreset.set(null);
      expect(localStorage.getItem("settings-default-image-preset")).toBe("null");
    });

    it("rehydrate stale enum values (e.g. a renamed preset variant) as null", async () => {
      localStorage.setItem("settings-default-image-preset", JSON.stringify("Medium"));
      localStorage.setItem("settings-default-video-preset", JSON.stringify("Ultra"));
      localStorage.setItem("settings-default-video-privacy", JSON.stringify("all"));
      const { defaultImagePreset, defaultVideoPreset, defaultVideoPrivacy } = await import("./settings");
      expect(get(defaultImagePreset)).toBeNull();
      expect(get(defaultVideoPreset)).toBeNull();
      expect(get(defaultVideoPrivacy)).toBeNull();
    });
  });
});
