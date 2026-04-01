import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { BaseFile } from "$lib/types";

export type FileStatus = "pending" | "converting" | "done" | "error";
export type TargetFormat = "Jpeg" | "Png" | "Webp" | "Avif";

export interface ConvertFile extends BaseFile {
  size: number;
  status: FileStatus;
  sourceFormat: string;
  outputPath?: string;
  outputSize?: number;
  error?: string;
}

export const files = writable<ConvertFile[]>([]);
export const targetFormat = writable<TargetFormat>("Jpeg");
export const outputDir = writable<string | null>(null);
export const isConverting = writable(false);
export const selectedPlatforms = writable<string[]>([]);

export const summary = derived(files, ($files) => {
  const done = $files.filter((f) => f.status === "done");
  return {
    done: done.length,
    failed: $files.filter((f) => f.status === "error").length,
    pending: $files.filter((f) => f.status === "pending" || f.status === "converting").length,
    savedBytes: Math.max(0, done.reduce((acc, f) => acc + ((f.size ?? 0) - (f.outputSize ?? (f.size ?? 0))), 0)),
  };
});

export async function addFiles(paths: string[]) {
  let newPaths: string[] = [];
  files.update((current) => {
    const existing = new Set(current.map((f) => f.path));
    const newFiles: ConvertFile[] = paths
      .filter((p) => !existing.has(p))
      .map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        size: 0,
        status: "pending" as const,
        sourceFormat: p.split(".").pop()?.toUpperCase() ?? "?",
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
  isConverting.set(false);
}
