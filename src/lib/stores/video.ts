import { writable, derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { activity } from "./activity";
import { savings } from "./savings";

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

// ── Helpers ────────────────────────────────────────────────────────────────────

function persisted<T>(key: string, initial: T) {
  const stored =
    typeof localStorage !== "undefined" ? localStorage.getItem(key) : null;
  const value: T = stored !== null ? (JSON.parse(stored) as T) : initial;
  const store = writable<T>(value);
  store.subscribe((v) => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(key, JSON.stringify(v));
    }
  });
  return store;
}

// ── Stores ─────────────────────────────────────────────────────────────────────

export const videoFiles = writable<VideoFile[]>([]);
export const videoPreset = persisted<VideoPreset>("video_preset", "Balanced");
export const videoOutputDir = persisted<string | null>("video_output_dir", null);
export const isCompressing = writable(false);

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

export async function revealVideoOutput(outputPath: string): Promise<void> {
  await invoke("reveal_video_output", { path: outputPath });
}
