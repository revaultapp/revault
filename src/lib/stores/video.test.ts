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

  it("videoStripPrivacy persists to localStorage", async () => {
    const { videoStripPrivacy } = await import("./video");

    expect(get(videoStripPrivacy)).toBe(true);

    videoStripPrivacy.set(false);
    expect(localStorage.getItem("video_strip_privacy")).toBe("false");

    videoStripPrivacy.set(true);
    expect(localStorage.getItem("video_strip_privacy")).toBe("true");
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

    const { videoPreviews, computeVideoPreview, videoPreset, videoStripPrivacy } =
      await import("./video");

    // Ensure same preset + stripPrivacy for both calls
    videoPreset.set("Balanced");
    videoStripPrivacy.set(true);

    await computeVideoPreview("/bar.mp4");

    // State is now "ready" with a cacheKey
    const state1 = get(videoPreviews).get("/bar.mp4");
    expect(state1?.status).toBe("ready");

    // Second call with same path/preset/privacy → cache hit, no-op
    await computeVideoPreview("/bar.mp4");

    expect(mockInvoke).toHaveBeenCalledTimes(1);
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
        cacheKey: "/a.mp4|Balanced|true",
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
        cacheKey: "/b.mp4|Balanced|true",
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
});
