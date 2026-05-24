import { writable, derived } from "svelte/store";
import type { Readable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { BaseFile } from "$lib/types";
import { persisted } from "$lib/utils";
import { defaultOutputDir } from "./settings";

export type FileStatus = "pending" | "resizing" | "done" | "error";
export type ResizeMode = "Fit" | "Exact";

export interface ResizeFile extends BaseFile {
  size: number;
  status: FileStatus;
  outputPath?: string;
  outputWidth?: number;
  outputHeight?: number;
  originalWidth?: number;
  originalHeight?: number;
  outputSize?: number;
  error?: string;
}

export const files = writable<ResizeFile[]>([]);
export const isResizing = writable(false);
export const outputDir = persisted<string | null>("resize-output-dir", null);
export const resolvedOutputDir = derived(
  [outputDir, defaultOutputDir],
  ([$out, $def]) => $out ?? $def,
);
export const resizeMode = writable<ResizeMode>("Fit");
export const width = writable(1920);
export const height = writable(1080);

export function willUpscale(
  file: Pick<ResizeFile, "originalWidth" | "originalHeight">,
  targetWidth: number,
  targetHeight: number,
  mode: ResizeMode,
): boolean {
  if (file.originalWidth === undefined || file.originalHeight === undefined) return false;
  if (mode === "Exact") {
    return targetWidth > file.originalWidth || targetHeight > file.originalHeight;
  }
  return targetWidth > file.originalWidth && targetHeight > file.originalHeight;
}

export const summary = derived(files, ($files) => ({
  done: $files.filter((f) => f.status === "done").length,
  failed: $files.filter((f) => f.status === "error").length,
  pending: $files.filter((f) => f.status === "pending" || f.status === "resizing").length,
}));

export const upscaleWarning: Readable<boolean> = derived(
  [files, width, height, resizeMode],
  ([$files, $width, $height, $resizeMode]) =>
    $files.some((f) => willUpscale(f, $width, $height, $resizeMode)),
);

export const upscaleCount: Readable<number> = derived(
  [files, width, height, resizeMode],
  ([$files, $width, $height, $resizeMode]) =>
    $files.filter((f) => willUpscale(f, $width, $height, $resizeMode)).length,
);

export async function addFiles(paths: string[]) {
  let newPaths: string[] = [];
  files.update((current) => {
    const existing = new Set(current.map((f) => f.path));
    const newFiles: ResizeFile[] = paths
      .filter((p) => !existing.has(p))
      .map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        size: 0,
        status: "pending" as const,
      }));
    newPaths = newFiles.map((f) => f.path);
    return [...current, ...newFiles];
  });
  if (newPaths.length === 0) return;
  try {
    const sizes = await invoke<number[]>("get_file_sizes", { paths: newPaths });
    files.update((current) =>
      current.map((f) => {
        const idx = newPaths.indexOf(f.path);
        return idx >= 0 && sizes[idx] > 0 ? { ...f, size: sizes[idx] } : f;
      })
    );
  } catch {
    // sizes stay at 0, not critical
  }
  await Promise.allSettled(
    newPaths.map(async (path) => {
      try {
        const [w, h] = await invoke<[number, number]>("get_image_dimensions", { path });
        files.update((current) =>
          current.map((f) =>
            f.path === path ? { ...f, originalWidth: w, originalHeight: h } : f,
          ),
        );
      } catch {
        // dimensions stay undefined — warning simply won't show
      }
    }),
  );
}

export function removeFile(path: string) {
  files.update((current) => current.filter((f) => f.path !== path));
}

export function clearFiles() {
  files.set([]);
  isResizing.set(false);
}
