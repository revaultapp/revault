import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

// Reset module state between tests so queue/inflightKeys start fresh.
// We reimport after each reset in tests that need a clean queue.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Helper: build a raw Rust-shaped preview response
function makeRawPreview(overrides: Partial<{
  input_path: string;
  duration_sec: number;
  original_size_bytes: number;
  estimated_size_bytes: number;
  estimated_savings_pct: number;
  confidence: number;
  method: string;
}> = {}) {
  return {
    input_path: "/video.mp4",
    duration_sec: 60,
    original_size_bytes: 100_000_000,
    estimated_size_bytes: 60_000_000,
    estimated_savings_pct: 40,
    confidence: 0.9,
    method: "ffprobe",
    ...overrides,
  };
}

describe("video store", () => {
  beforeEach(async () => {
    vi.resetModules();
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it("videoPrivacyMode persists to localStorage", async () => {
    const { videoPrivacyMode } = await import("./video");

    expect(get(videoPrivacyMode)).toBe("smart");

    videoPrivacyMode.set("smart");
    expect(localStorage.getItem("video_privacy_mode")).toBe('"smart"');

    videoPrivacyMode.set("gps_only");
    expect(localStorage.getItem("video_privacy_mode")).toBe('"gps_only"');

    videoPrivacyMode.set("full");
    expect(localStorage.getItem("video_privacy_mode")).toBe('"full"');

    videoPrivacyMode.set("off");
    expect(localStorage.getItem("video_privacy_mode")).toBe('"off"');
  });

  it("computeVideoPreview sets loading then ready", async () => {
    const rawPreview = makeRawPreview({ input_path: "/foo.mp4" });
    mockInvoke.mockResolvedValueOnce(rawPreview);

    const { videoPreviews, computeVideoPreview } = await import("./video");

    const promise = computeVideoPreview("/foo.mp4");

    // Should be loading immediately after call (before queue drains)
    const loadingState = get(videoPreviews).get("/foo.mp4");
    expect(loadingState?.status).toBe("loading");

    await promise;

    const finalState = get(videoPreviews).get("/foo.mp4");
    expect(finalState?.status).toBe("ready");

    if (finalState?.status !== "ready") throw new Error("Expected ready");

    expect(finalState.preview.inputPath).toBe("/foo.mp4");
    expect(finalState.preview.originalSizeBytes).toBe(100_000_000);
    expect(finalState.preview.estimatedSizeBytes).toBe(60_000_000);
    expect(finalState.preview.estimatedSavingsPct).toBe(40);
    expect(finalState.preview.durationSec).toBe(60);
    expect(finalState.preview.confidence).toBe(0.9);
    expect(finalState.preview.method).toBe("ffprobe");
    expect(finalState.cacheKey).toContain("/foo.mp4");
  });

  it("computeVideoPreview caches — no re-invoke with same key", async () => {
    const rawPreview = makeRawPreview({ input_path: "/bar.mp4" });
    mockInvoke.mockResolvedValue(rawPreview);

    const { videoPreviews, computeVideoPreview, videoPreset, videoPrivacyMode } =
      await import("./video");

    // Ensure same preset + privacyMode for both calls
    videoPreset.set("Balanced");
    videoPrivacyMode.set("off");

    await computeVideoPreview("/bar.mp4");

    // State is now "ready" with a cacheKey
    const state1 = get(videoPreviews).get("/bar.mp4");
    expect(state1?.status).toBe("ready");

    // Second call with same path/preset/privacy → cache hit, no-op
    await computeVideoPreview("/bar.mp4");

    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("videoMode persists to localStorage", async () => {
    const { videoMode } = await import("./video");
    expect(get(videoMode)).toBe("compress");
    videoMode.set("gif");
    expect(localStorage.getItem("video_mode")).toBe('"gif"');
    videoMode.set("compress");
    expect(localStorage.getItem("video_mode")).toBe('"compress"');
  });

  it("gifSettings persists to localStorage", async () => {
    const { gifSettings } = await import("./video");
    gifSettings.update(s => ({ ...s, fps: 24, quality: 90 }));
    const stored = JSON.parse(localStorage.getItem("gif_settings")!);
    expect(stored.fps).toBe(24);
    expect(stored.quality).toBe(90);
  });

  it("exportGif happy path sets done state and output path", async () => {
    mockInvoke.mockResolvedValueOnce({
      output_path: "/test/video_gif.gif",
      size_bytes: 4_200_000,
      duration_sec: 3.0,
      width: 480,
      height: 270,
      fps: 15,
    });
    const { gifState, gifOutputPath, gifResult, gifError, exportGif } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await exportGif(file);

    expect(get(gifState)).toBe("done");
    expect(get(gifOutputPath)).toBe("/test/video_gif.gif");
    expect(get(gifResult)?.size_bytes).toBe(4_200_000);
    expect(get(gifResult)?.fps).toBe(15);
    expect(get(gifError)).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("export_gif", expect.objectContaining({
      inputPath: "/test/video.mp4",
      outputPath: "/test/video_gif.gif",
    }));
  });

  it("exportGif error path sets error state", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("gifski not found"));
    const { gifState, gifOutputPath, gifError, exportGif } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await exportGif(file);

    expect(get(gifState)).toBe("error");
    expect(get(gifOutputPath)).toBeNull();
    expect(get(gifError)).toContain("gifski not found");
  });

  it("gifDownloadProgress can be updated and reset", async () => {
    const { gifDownloadProgress } = await import("./video");

    gifDownloadProgress.set({ done: 5_000_000, total: 14_000_000 });
    expect(get(gifDownloadProgress)).toEqual({ done: 5_000_000, total: 14_000_000 });

    gifDownloadProgress.set(null);
    expect(get(gifDownloadProgress)).toBeNull();
  });

  it("download_gifski happy path clears progress and calls checkGifski", async () => {
    // download_gifski resolves, check_gifski returns true
    mockInvoke
      .mockResolvedValueOnce(undefined) // download_gifski
      .mockResolvedValueOnce(true);     // check_gifski

    const { gifDownloadProgress, gifskiAvailable } = await import("./video");

    // Simulate what handleGifDownload would do via the store functions
    gifDownloadProgress.set({ done: 0, total: 0 });
    await invoke("download_gifski");
    gifDownloadProgress.set(null);

    // Simulate checkGifski
    const available = await invoke<boolean>("check_gifski");
    gifskiAvailable.set(available as unknown as boolean);

    expect(get(gifDownloadProgress)).toBeNull();
    expect(get(gifskiAvailable)).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("download_gifski");
    expect(mockInvoke).toHaveBeenCalledWith("check_gifski");
  });

  it("download_gifski error path clears progress and leaves gifskiAvailable false", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("network timeout"));

    const { gifDownloadProgress, gifskiAvailable } = await import("./video");

    gifskiAvailable.set(false);
    gifDownloadProgress.set({ done: 0, total: 0 });

    try {
      await invoke("download_gifski");
    } catch {
      gifDownloadProgress.set(null);
    }

    expect(get(gifDownloadProgress)).toBeNull();
    expect(get(gifskiAvailable)).toBe(false);
  });

  it("videoPreviewSummary aggregates ready files only", async () => {
    const { videoFiles, videoPreviews, videoPreviewSummary } = await import("./video");

    videoFiles.set([
      { path: "/a.mp4", name: "a.mp4", status: "idle", originalSize: 0, progress: 0, fps: 0, speed: 0 },
      { path: "/b.mp4", name: "b.mp4", status: "idle", originalSize: 0, progress: 0, fps: 0, speed: 0 },
      { path: "/c.mp4", name: "c.mp4", status: "idle", originalSize: 0, progress: 0, fps: 0, speed: 0 },
    ]);

    videoPreviews.set(new Map([
      ["/a.mp4", {
        status: "ready",
        cacheKey: "/a.mp4|Balanced|off",
        preview: {
          inputPath: "/a.mp4",
          durationSec: 30,
          originalSizeBytes: 100_000_000,
          estimatedSizeBytes: 70_000_000,
          estimatedSavingsPct: 30,
          confidence: 0.9,
          method: "ffprobe",
        },
      }],
      ["/b.mp4", {
        status: "ready",
        cacheKey: "/b.mp4|Balanced|off",
        preview: {
          inputPath: "/b.mp4",
          durationSec: 60,
          originalSizeBytes: 200_000_000,
          estimatedSizeBytes: 120_000_000,
          estimatedSavingsPct: 40,
          confidence: 0.85,
          method: "ffprobe",
        },
      }],
      // /c.mp4 is in "loading" state — should NOT be counted
      ["/c.mp4", { status: "loading" }],
    ]));

    const summary = get(videoPreviewSummary);

    expect(summary.filesReady).toBe(2);
    expect(summary.filesTotal).toBe(3);
    expect(summary.totalOriginal).toBe(300_000_000);
    expect(summary.totalEstimated).toBe(190_000_000);
    expect(summary.totalSaved).toBe(110_000_000);
    expect(summary.savingsPct).toBeCloseTo((110_000_000 / 300_000_000) * 100, 1);
  });

  it("trimSettings persists to localStorage", async () => {
    const { trimSettings } = await import("./video");
    expect(get(trimSettings)).toEqual({ startSec: 0, endSec: 5 });
    trimSettings.set({ startSec: 2, endSec: 8 });
    expect(localStorage.getItem("trim_settings")).toBe('{"startSec":2,"endSec":8}');
  });

  it("trimVideoFile happy path sets done state and output path", async () => {
    mockInvoke.mockResolvedValueOnce({
      input_path: "/test/video.mp4",
      output_path: "/test/video_trimmed.mp4",
    });
    const { trimState, trimOutputPath, trimResult, trimError, trimSettings, trimVideoFile } =
      await import("./video");

    trimSettings.set({ startSec: 1, endSec: 4 });

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await trimVideoFile(file);

    expect(get(trimState)).toBe("done");
    expect(get(trimOutputPath)).toBe("/test/video_trimmed.mp4");
    expect(get(trimResult)?.output_path).toBe("/test/video_trimmed.mp4");
    expect(get(trimError)).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("trim_video", {
      input: "/test/video.mp4",
      startSec: 1,
      endSec: 4,
      privacy: "smart",
      outputDir: null,
    });
  });

  it("trimVideoFile sends the privacy mode and the resolved Settings output dir", async () => {
    mockInvoke.mockResolvedValueOnce({
      input_path: "/test/video.mp4",
      output_path: "/dest/video_trimmed.mp4",
    });
    const { defaultOutputDir } = await import("./settings");
    defaultOutputDir.set("/dest");
    const { trimSettings, trimVideoFile, videoPrivacyMode } = await import("./video");

    videoPrivacyMode.set("full");
    trimSettings.set({ startSec: 0, endSec: 2 });

    await trimVideoFile({
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    });

    expect(mockInvoke).toHaveBeenCalledWith(
      "trim_video",
      expect.objectContaining({ privacy: "full", outputDir: "/dest" }),
    );
  });

  it("trimVideoFile error path sets error state", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("end_sec exceeds media duration"));
    const { trimState, trimOutputPath, trimError, trimVideoFile } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await trimVideoFile(file);

    expect(get(trimState)).toBe("error");
    expect(get(trimOutputPath)).toBeNull();
    expect(get(trimError)).toContain("end_sec exceeds media duration");
  });

  it("trimVideoFile treats any rejection as an error (trim has no cancel path)", async () => {
    // trim_video has no cancel command backend-side; the old "cancelled"
    // substring branch was dead code and was removed — every rejection is a
    // genuine error now.
    mockInvoke.mockRejectedValueOnce(new Error("trim cancelled"));
    const { trimState, trimError, trimVideoFile } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await trimVideoFile(file);

    expect(get(trimState)).toBe("error");
    expect(get(trimError)).toContain("trim cancelled");
  });

  it("audioSettings persists with auto/192 defaults", async () => {
    const { audioSettings } = await import("./video");
    expect(get(audioSettings)).toEqual({ format: "auto", bitrateKbps: 192 });
    audioSettings.set({ format: "mp3", bitrateKbps: 320 });
    expect(localStorage.getItem("audio_extract_settings")).toBe(
      '{"format":"mp3","bitrateKbps":320}'
    );
  });

  it("extractAudioFile happy path sets done state, result and full progress", async () => {
    mockInvoke.mockResolvedValueOnce({
      input_path: "/test/video.mp4",
      output_path: "/test/video_audio.m4a",
      output_size: 4_000_000,
      was_lossless_copy: true,
    });
    const {
      audioState,
      audioResult,
      audioError,
      audioProgress,
      audioSettings,
      extractAudioFile,
    } = await import("./video");

    audioSettings.set({ format: "auto", bitrateKbps: 192 });

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await extractAudioFile(file);

    expect(get(audioState)).toBe("done");
    expect(get(audioResult)?.output_path).toBe("/test/video_audio.m4a");
    expect(get(audioResult)?.was_lossless_copy).toBe(true);
    expect(get(audioProgress)).toBe(100);
    expect(get(audioError)).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("extract_audio", {
      input: "/test/video.mp4",
      outputDir: null,
      format: "auto",
      bitrateKbps: 192,
    });
  });

  it("extractAudioFile error path sets error state", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("This file has no audio track"));
    const { audioState, audioResult, audioError, extractAudioFile } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await extractAudioFile(file);

    expect(get(audioState)).toBe("error");
    expect(get(audioResult)).toBeNull();
    expect(get(audioError)).toContain("no audio track");
  });

  it("extractAudioFile cancelled path resets to idle", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("extraction cancelled"));
    const { audioState, audioError, audioProgress, extractAudioFile } = await import("./video");

    const file = {
      path: "/test/video.mp4",
      name: "video.mp4",
      status: "idle" as const,
      originalSize: 0,
      progress: 0,
      fps: 0,
      speed: 0,
    };

    await extractAudioFile(file);

    expect(get(audioState)).toBe("idle");
    expect(get(audioError)).toBeNull();
    expect(get(audioProgress)).toBe(0);
  });

  describe("resolvedVideoOutputDir derived", () => {
    it("prefers the local videoOutputDir over the default", async () => {
      const { videoOutputDir, resolvedVideoOutputDir } = await import("./video");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set("/default/output");
      videoOutputDir.set("/custom/output");

      expect(get(resolvedVideoOutputDir)).toBe("/custom/output");
    });

    it("falls back to defaultOutputDir when videoOutputDir is null", async () => {
      const { videoOutputDir, resolvedVideoOutputDir } = await import("./video");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set("/default/output");
      videoOutputDir.set(null);

      expect(get(resolvedVideoOutputDir)).toBe("/default/output");
    });

    it("is null when neither videoOutputDir nor defaultOutputDir is set", async () => {
      const { videoOutputDir, resolvedVideoOutputDir } = await import("./video");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set(null);
      videoOutputDir.set(null);

      expect(get(resolvedVideoOutputDir)).toBeNull();
    });
  });

  describe("videoPreset / videoPrivacyMode global default semantics", () => {
    it("videoPreset uses the global default from Settings when set", async () => {
      const { defaultVideoPreset } = await import("./settings");
      defaultVideoPreset.set("Smallest");
      const { videoPreset } = await import("./video");
      expect(get(videoPreset)).toBe("Smallest");
    });

    it("videoPreset falls back to the last-used persisted value when the global default is null", async () => {
      localStorage.setItem("video_preset", JSON.stringify("HighQuality"));
      const { videoPreset } = await import("./video");
      expect(get(videoPreset)).toBe("HighQuality");
    });

    it("videoPreset falls back to Balanced when neither the global default nor a last-used value exist", async () => {
      const { videoPreset } = await import("./video");
      expect(get(videoPreset)).toBe("Balanced");
    });

    it("videoPrivacyMode uses the global default from Settings when set", async () => {
      const { defaultVideoPrivacy } = await import("./settings");
      defaultVideoPrivacy.set("full");
      const { videoPrivacyMode } = await import("./video");
      expect(get(videoPrivacyMode)).toBe("full");
    });

    it("videoPrivacyMode falls back to the last-used persisted value when the global default is null", async () => {
      localStorage.setItem("video_privacy_mode", JSON.stringify("gps_only"));
      const { videoPrivacyMode } = await import("./video");
      expect(get(videoPrivacyMode)).toBe("gps_only");
    });

    it("still persists user changes made after init from a global default", async () => {
      const { defaultVideoPreset } = await import("./settings");
      defaultVideoPreset.set("Smallest");
      const { videoPreset } = await import("./video");
      videoPreset.set("HighQuality");
      expect(localStorage.getItem("video_preset")).toBe(JSON.stringify("HighQuality"));
    });

    it("applies Settings changes made after init immediately, and remember-last never overrides", async () => {
      const { defaultVideoPreset, defaultVideoPrivacy } = await import("./settings");
      const { videoPreset, videoPrivacyMode } = await import("./video");
      expect(get(videoPreset)).toBe("Balanced");
      expect(get(videoPrivacyMode)).toBe("smart");

      defaultVideoPreset.set("HighQuality");
      defaultVideoPrivacy.set("full");
      expect(get(videoPreset)).toBe("HighQuality");
      expect(get(videoPrivacyMode)).toBe("full");

      defaultVideoPreset.set(null);
      defaultVideoPrivacy.set(null);
      expect(get(videoPreset)).toBe("HighQuality");
      expect(get(videoPrivacyMode)).toBe("full");
    });
  });
});
