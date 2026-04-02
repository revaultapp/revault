import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { BaseFile } from "$lib/types";

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
  error?: string;
}

export const files = writable<ResizeFile[]>([]);
export const isResizing = writable(false);
export const outputDir = writable<string | null>(null);
export const resizeMode = writable<ResizeMode>("Fit");
export const width = writable(1920);
export const height = writable(1080);

export const summary = derived(files, ($files) => ({
  done: $files.filter((f) => f.status === "done").length,
  failed: $files.filter((f) => f.status === "error").length,
  pending: $files.filter((f) => f.status === "pending" || f.status === "resizing").length,
}));

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
}

export function removeFile(path: string) {
  files.update((current) => current.filter((f) => f.path !== path));
}

export function clearFiles() {
  files.set([]);
  isResizing.set(false);
}
