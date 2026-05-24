import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("convert store", () => {
  beforeEach(() => {
    vi.resetModules();
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it("does not override a user-selected format when HEIC files are added", async () => {
    mockInvoke.mockResolvedValueOnce([1000]);

    const { addFiles, targetFormat, hasHeicFiles, heicBannerDismissed } = await import("./convert");
    targetFormat.set("Webp");
    heicBannerDismissed.set(true);

    await addFiles(["/photos/IMG_0001.HEIC"]);

    expect(get(targetFormat)).toBe("Webp");
    expect(get(hasHeicFiles)).toBe(true);
    expect(get(heicBannerDismissed)).toBe(false);
  });
});
