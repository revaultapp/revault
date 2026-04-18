import { writable, derived, get } from "svelte/store";
import type { Readable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { activity } from "./activity";
import { savings } from "./savings";
import { persisted } from "$lib/utils";

// ── FFmpeg availability ───────────────────────────────────────────────────────

export type FfmpegStatus = "checking" | "needs_download" | "downloading" | "ready";

export interface FfmpegDownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export const ffmpegStatus = writable<FfmpegStatus>("checking");
export const ffmpegDownloadProgress = writable<FfmpegDownloadProgress>({
  downloaded: 0,
  total: 0,
  percent: 0,
});

export async function checkFfmpeg(): Promise<void> {
  ffmpegStatus.set("checking");
  try {
    const available = await invoke<boolean>("check_ffmpeg");
    ffmpegStatus.set(available ? "ready" : "needs_download");
  } catch {
    ffmpegStatus.set("needs_download");
  }
}

export async function downloadFfmpeg(): Promise<void> {
  ffmpegStatus.set("downloading");
  ffmpegDownloadProgress.set({ downloaded: 0, total: 0, percent: 0 });

  const unlisten = await listen<FfmpegDownloadProgress>(
    "ffmpeg-download-progress",
    (event) => {
      ffmpegDownloadProgress.set(event.payload);
    }
  );

  try {
    await invoke("download_ffmpeg");
    ffmpegStatus.set("ready");
  } catch (err) {
    ffmpegStatus.set("needs_download");
    throw err;
  } finally {
    unlisten();
  }
}

// ── Types ──────────────────────────────────────────────────────────────────────

export type VideoPreset = "Smallest" | "Balanced" | "HighQuality";

// Matches Rust PrivacyMode enum (serde lowercase)
export type PrivacyMode = "off" | "smart" | "gps_only" | "full";

export interface VideoCompressionPreview {
  inputPath: string;
  durationSec: number;
  originalSizeBytes: number;
  estimatedSizeBytes: number;
  estimatedSavingsPct: number;
  confidence: number;
  method: string;
}

export type VideoPreviewState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; preview: VideoCompressionPreview; cacheKey: string }
  | { status: "error"; message: string };

export type VideoFileStatus = "idle" | "compressing" | "done" | "error" | "cancelled";

export interface VideoFile {
  path: string;
  name: string;
  status: VideoFileStatus;
  originalSize: number;
  compressedSize?: number;
  outputPath?: string;
  error?: string;
  progress: number;
  fps: number;
  speed: number;
}

export interface VideoProgress {
  input_path: string;
  percent: number;
  fps: number;
  size_kb: number;
  speed: number;
}

export interface VideoCompressionResult {
  input_path: string;
  output_path: string;
  original_size: number;
  compressed_size: number;
  error: string | null;
}

// ── Stores ─────────────────────────────────────────────────────────────────────

export const videoFiles = writable<VideoFile[]>([]);
export const videoPreset = persisted<VideoPreset>("video_preset", "Balanced");
export const videoOutputDir = persisted<string | null>("video_output_dir", null);
export const videoPrivacyMode = persisted<PrivacyMode>("video_privacy_mode", "smart");
export const isCompressing = writable(false);

export const videoPreviews = writable<Map<string, VideoPreviewState>>(new Map());

export const videoSummary = derived(videoFiles, ($files) => {
  const done = $files.filter((f) => f.status === "done");
  const failed = $files.filter((f) => f.status === "error");
  const pending = $files.filter(
    (f) => f.status === "idle" || f.status === "compressing"
  );
  const savedBytes = Math.max(
    0,
    done.reduce((acc, f) => acc + (f.originalSize - (f.compressedSize ?? f.originalSize)), 0),
  );
  return {
    done: done.length,
    failed: failed.length,
    pending: pending.length,
    savedBytes,
  };
});

export const videoPreviewSummary: Readable<{
  filesReady: number;
  filesTotal: number;
  totalOriginal: number;
  totalEstimated: number;
  totalSaved: number;
  savingsPct: number;
}> = derived([videoFiles, videoPreviews], ([$files, $previews]) => {
  const filesTotal = $files.length;
  let filesReady = 0;
  let totalOriginal = 0;
  let totalEstimated = 0;

  for (const file of $files) {
    const state = $previews.get(file.path);
    if (state?.status === "ready") {
      filesReady++;
      totalOriginal += state.preview.originalSizeBytes;
      totalEstimated += state.preview.estimatedSizeBytes;
    }
  }

  const totalSaved = Math.max(0, totalOriginal - totalEstimated);
  const savingsPct =
    filesReady === 0 ? 0 : Math.min(100, Math.max(0, (totalSaved / totalOriginal) * 100));

  return { filesReady, filesTotal, totalOriginal, totalEstimated, totalSaved, savingsPct };
});

// ── Actions ────────────────────────────────────────────────────────────────────

export async function addVideoFiles(paths: string[]): Promise<void> {
  videoFiles.update((current) => {
    const existing = new Set(current.map((f) => f.path));
    const newFiles: VideoFile[] = paths
      .filter((p) => !existing.has(p))
      .map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        status: "idle" as const,
        originalSize: 0,
        progress: 0,
        fps: 0,
        speed: 0,
      }));
    return [...current, ...newFiles];
  });

  // Fetch real file sizes in batch and backfill
  try {
    const sizes = await invoke<number[]>("get_file_sizes", { paths });
    videoFiles.update((all) =>
      all.map((f) => {
        const idx = paths.indexOf(f.path);
        if (idx === -1) return f;
        return { ...f, originalSize: sizes[idx] ?? 0 };
      })
    );
  } catch {
    // non-fatal — originalSize stays 0 and will be updated from result
  }
}

export function removeVideoFile(path: string): void {
  videoFiles.update((current) => current.filter((f) => f.path !== path));
}

export function clearVideoFiles(): void {
  videoFiles.set([]);
  isCompressing.set(false);
  clearVideoPreviews();
}

export function resetVideoFilesToIdle(): void {
  videoFiles.update((files) =>
    files.map((f) => ({
      ...f,
      status: "idle" as const,
      progress: 0,
      fps: 0,
      speed: 0,
      compressedSize: undefined,
      outputPath: undefined,
      error: undefined,
    }))
  );
}

export async function compressVideoFile(
  file: VideoFile,
  preset: VideoPreset,
  outputDir: string | null
): Promise<void> {
  videoFiles.update((all) =>
    all.map((f) =>
      f.path === file.path
        ? { ...f, status: "compressing" as const, progress: 0 }
        : f
    )
  );

  // NOTE: This listener relies on compression being sequential (one file at a time).
  // If parallelism is ever added, each file needs its own typed event channel to
  // avoid all listeners firing for every event.
  const unlisten = await listen<VideoProgress>(
    "video-compress-progress",
    (event) => {
      const p = event.payload;
      if (p.input_path !== file.path) return;
      videoFiles.update((all) =>
        all.map((f) =>
          f.path === p.input_path
            ? { ...f, progress: p.percent, fps: p.fps, speed: p.speed }
            : f
        )
      );
    }
  );

  try {
    const result = await invoke<VideoCompressionResult>("compress_video", {
      input: file.path,
      preset,
      outputDir: outputDir ?? null,
      privacy: get(videoPrivacyMode),
    });

    const isWarning = result.error?.includes("Output was larger") ?? false;
    const succeeded = !result.error || isWarning;

    videoFiles.update((all) =>
      all.map((f) =>
        f.path === result.input_path
          ? {
              ...f,
              status: (result.error && !isWarning ? "error" : "done") as VideoFileStatus,
              compressedSize: result.compressed_size,
              outputPath: result.output_path,
              originalSize: result.original_size,
              error: isWarning ? undefined : (result.error ?? undefined),
              progress: succeeded ? 100 : f.progress,
            }
          : f
      )
    );

    if (succeeded) {
      const savedBytes = result.original_size - result.compressed_size;
      activity.add({ type: "video", fileCount: 1, savedBytes: Math.max(0, savedBytes) });
      if (savedBytes > 0) {
        savings.add(savedBytes);
        savings.incrementOps(1);
        savings.addOriginalBytes(result.original_size);
        savings.addCompressedBytes(result.compressed_size);
      }
    }
  } catch (err) {
    let msg = String(err);
    if (msg.includes("os error 2") || msg.includes("No such file")) {
      msg = `File not found: ${file.path}`;
    }
    const isCancelled = msg.includes("cancelled");
    videoFiles.update((all) =>
      all.map((f) =>
        f.path === file.path
          ? {
              ...f,
              status: isCancelled ? ("cancelled" as const) : ("error" as const),
              error: isCancelled ? undefined : msg,
            }
          : f
      )
    );
  } finally {
    unlisten();
  }
}

export async function cancelCompression(): Promise<void> {
  await invoke("cancel_video_compress");
}

// ── GIF export ─────────────────────────────────────────────────────────────────

export type VideoMode = "compress" | "gif";

export type GifSettings = {
  startSec: number;
  endSec: number;
  fps: 10 | 15 | 24;
  width: 320 | 480 | 640 | 720 | 1080;
  quality: number;
};

export const videoMode = persisted<VideoMode>("video_mode", "compress");

export const gifSettings = persisted<GifSettings>("gif_settings", {
  startSec: 0,
  endSec: 3,
  fps: 15,
  width: 480,
  quality: 80,
});

export interface GifResult {
  output_path: string;
  size_bytes: number;
  duration_sec: number;
  width: number;
  height: number;
  fps: number;
}

export const gifState = writable<"idle" | "generating" | "done" | "error">("idle");
export const gifOutputPath = writable<string | null>(null);
export const gifResult = writable<GifResult | null>(null);
export const gifError = writable<string | null>(null);
export const gifskiAvailable = writable<boolean | null>(null);
export const gifDownloadProgress = writable<{ done: number; total: number } | null>(null);
export const gifSizeEstimate = writable<number | null>(null);

export async function checkGifski(): Promise<void> {
  try {
    const available = await invoke<boolean>("check_gifski");
    gifskiAvailable.set(available);
  } catch {
    gifskiAvailable.set(false);
  }
}

function clampGifEnd(s: GifSettings): number {
  return Math.min(s.endSec, s.startSec + 15);
}

export async function refreshGifSizeEstimate(): Promise<void> {
  const settings = get(gifSettings);
  const clampedEnd = clampGifEnd(settings);
  try {
    const bytes = await invoke<number>("estimate_gif_size", {
      options: {
        start_sec: settings.startSec,
        end_sec: clampedEnd,
        fps: settings.fps,
        width: settings.width,
        quality: settings.quality,
      },
    });
    gifSizeEstimate.set(bytes);
  } catch {
    gifSizeEstimate.set(null);
  }
}

export async function exportGif(file: VideoFile): Promise<void> {
  const settings = get(gifSettings);
  const clampedEnd = clampGifEnd(settings);
  const inputPath = file.path;
  const baseName = (file.name || inputPath.split(/[\\/]/).pop() || "video")
    .replace(/\.[^/.]+$/, "");
  const filename = `${baseName}_gif.gif`;
  const customDir = get(videoOutputDir);
  const inputParent = inputPath.substring(
    0,
    Math.max(inputPath.lastIndexOf("/"), inputPath.lastIndexOf("\\"))
  );
  const outputDir = customDir ?? inputParent;
  const sep = outputDir.includes("\\") && !outputDir.includes("/") ? "\\" : "/";
  const outputPath = `${outputDir}${outputDir.endsWith(sep) ? "" : sep}${filename}`;

  gifState.set("generating");
  gifError.set(null);
  gifOutputPath.set(null);
  gifResult.set(null);
  gifSizeEstimate.set(null);

  try {
    const result = await invoke<GifResult>("export_gif", {
      inputPath,
      outputPath,
      options: {
        start_sec: settings.startSec,
        end_sec: clampedEnd,
        fps: settings.fps,
        width: settings.width,
        quality: settings.quality,
      },
    });
    gifResult.set(result);
    gifOutputPath.set(result.output_path);
    gifState.set("done");
  } catch (e) {
    gifState.set("error");
    gifError.set(String(e));
  }
}

export async function revealVideoOutput(outputPath: string): Promise<void> {
  await invoke("reveal_video_output", { path: outputPath });
}

// ── Preview cache key tracking (module-scoped, not reactive) ──────────────────

// Maps path → the cacheKey of the in-flight request. Used to discard stale results.
const inflightKeys = new Map<string, string>();

// FIFO serial queue — new work is chained onto the tail.
let queue: Promise<void> = Promise.resolve();

// Raw shape returned by Rust (snake_case — no rename_all on VideoCompressionPreview)
interface RawVideoCompressionPreview {
  input_path: string;
  duration_sec: number;
  original_size_bytes: number;
  estimated_size_bytes: number;
  estimated_savings_pct: number;
  confidence: number;
  method: string;
}

export function clearVideoPreviews(): void {
  videoPreviews.set(new Map());
  inflightKeys.clear();
}

export function computeVideoPreview(path: string): Promise<void> {
  const preset = get(videoPreset);
  const privacyMode = get(videoPrivacyMode);
  const cacheKey = `${path}|${preset}|${privacyMode}`;

  // Already ready with same key → no-op
  const currentState = get(videoPreviews).get(path);
  if (currentState?.status === "ready" && currentState.cacheKey === cacheKey) {
    return Promise.resolve();
  }

  // Mark loading immediately (outside queue so UI responds fast)
  videoPreviews.update((m) => {
    const next = new Map(m);
    next.set(path, { status: "loading" });
    return next;
  });

  inflightKeys.set(path, cacheKey);

  const doWork = async () => {
    // Check if superseded before doing any work
    if (inflightKeys.get(path) !== cacheKey) return;

    try {
      const raw = await invoke<RawVideoCompressionPreview>("preview_video_compression", {
        input: path,
        preset,
        privacy: privacyMode,
      });

      // Discard if a newer request has been queued for this path
      if (inflightKeys.get(path) !== cacheKey) return;

      const preview: VideoCompressionPreview = {
        inputPath: raw.input_path,
        durationSec: raw.duration_sec,
        originalSizeBytes: raw.original_size_bytes,
        estimatedSizeBytes: raw.estimated_size_bytes,
        estimatedSavingsPct: raw.estimated_savings_pct,
        confidence: raw.confidence,
        method: raw.method,
      };

      videoPreviews.update((m) => {
        const next = new Map(m);
        next.set(path, { status: "ready", preview, cacheKey });
        return next;
      });
    } catch (err) {
      if (inflightKeys.get(path) !== cacheKey) return;

      const message = err instanceof Error ? err.message : String(err);

      videoPreviews.update((m) => {
        const next = new Map(m);
        next.set(path, { status: "error", message });
        return next;
      });
    } finally {
      if (inflightKeys.get(path) === cacheKey) {
        inflightKeys.delete(path);
      }
    }
  };

  // Chain onto the FIFO queue. Return a promise that resolves when THIS work item
  // completes (not just when it's been enqueued).
  const workPromise = queue.then(doWork);
  queue = workPromise;
  return workPromise;
}
