import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: vi.fn() }));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("resize store — upscale warning", () => {
  beforeEach(() => {
    vi.resetModules();
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it("upscaleWarning is false when no files", async () => {
    const { upscaleWarning } = await import("./resize");
    expect(get(upscaleWarning)).toBe(false);
  });

  it("upscaleWarning is true when target exceeds original dimensions", async () => {
    // get_file_sizes → [1000], get_image_dimensions → [800, 600]
    mockInvoke
      .mockResolvedValueOnce([1000])
      .mockResolvedValueOnce([800, 600]);

    const { addFiles, width, height, upscaleWarning } = await import("./resize");
    width.set(1920);
    height.set(1080);

    await addFiles(["/img/photo.jpg"]);

    expect(get(upscaleWarning)).toBe(true);
  });

  it("upscaleWarning is false in Fit mode when only one target dimension exceeds original", async () => {
    mockInvoke
      .mockResolvedValueOnce([1000])
      .mockResolvedValueOnce([3000, 1000]);

    const { addFiles, width, height, resizeMode, upscaleWarning, upscaleCount } = await import("./resize");
    width.set(1920);
    height.set(1080);
    resizeMode.set("Fit");

    await addFiles(["/img/panorama.jpg"]);

    expect(get(upscaleWarning)).toBe(false);
    expect(get(upscaleCount)).toBe(0);
  });

  it("upscaleWarning is true in Exact mode when one target dimension exceeds original", async () => {
    mockInvoke
      .mockResolvedValueOnce([1000])
      .mockResolvedValueOnce([3000, 1000]);

    const { addFiles, width, height, resizeMode, upscaleWarning, upscaleCount } = await import("./resize");
    width.set(1920);
    height.set(1080);
    resizeMode.set("Exact");

    await addFiles(["/img/panorama.jpg"]);

    expect(get(upscaleWarning)).toBe(true);
    expect(get(upscaleCount)).toBe(1);
  });

  it("upscaleWarning is false when target is smaller than original", async () => {
    // get_file_sizes → [1000], get_image_dimensions → [3840, 2160]
    mockInvoke
      .mockResolvedValueOnce([1000])
      .mockResolvedValueOnce([3840, 2160]);

    const { addFiles, width, height, upscaleWarning } = await import("./resize");
    width.set(1920);
    height.set(1080);

    await addFiles(["/img/photo4k.jpg"]);

    expect(get(upscaleWarning)).toBe(false);
  });
});

describe("resize store — cross-screen output-dir contract", () => {
  beforeEach(() => {
    vi.resetModules();
    mockInvoke.mockReset();
    localStorage.clear();
  });

  // Note: the quality preset has no resize-store wiring on purpose — the page
  // shares compress's qualityPreset store (like ConvertPage), covered in
  // compress.test.ts.
  it("per-tool outputDir never writes back to the Settings default", async () => {
    const { defaultOutputDir } = await import("./settings");
    const { outputDir } = await import("./resize");
    outputDir.set("/tool");
    expect(get(defaultOutputDir)).toBeNull();
    defaultOutputDir.set("/global");
    defaultOutputDir.set(null);
    expect(get(outputDir)).toBe("/tool");
  });
});
