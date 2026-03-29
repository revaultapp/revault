import { writable, derived } from "svelte/store";
import type { BaseFile } from "$lib/types";

export type FileStatus = "pending" | "compressing" | "done" | "error";
export type OutputFormat = "Jpeg" | "Png" | "Webp" | "Avif";

export interface CompressFile extends BaseFile {
  size: number;
  status: FileStatus;
  compressedSize?: number;
  outputPath?: string;
  alreadyOptimal?: boolean;
  error?: string;
}

export type CompressMode = "quality" | "target";
export type CompressionProfile = "Web" | "Email" | "Archive" | "Share" | "Custom";

export const files = writable<CompressFile[]>([]);
export const quality = writable(80);
export const format = writable<OutputFormat | null>(null);
export const outputDir = writable<string | null>(null);
export const isCompressing = writable(false);
export const compressMode = writable<CompressMode>("quality");
export const targetSize = writable<number>(500);
export const targetUnit = writable<"KB" | "MB">("KB");
export const activeProfile = writable<CompressionProfile>("Custom");
export const stripGps = writable(false);

export const summary = derived(files, ($files) => {
  const done = $files.filter((f) => f.status === "done");
  const failed = $files.filter((f) => f.status === "error");
  const pending = $files.filter(
    (f) => f.status === "pending" || f.status === "compressing",
  );
  const savedBytes = done.reduce(
    (acc, f) => acc + (f.size - (f.compressedSize ?? f.size)),
    0,
  );
  return { done: done.length, failed: failed.length, pending: pending.length, savedBytes };
});

export function addFiles(paths: string[]) {
  files.update((current) => {
    const existing = new Set(current.map((f) => f.path));
    const newFiles: CompressFile[] = paths
      .filter((p) => !existing.has(p))
      .map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        size: 0,
        status: "pending" as const,
      }));
    return [...current, ...newFiles];
  });
}

export function removeFile(path: string) {
  files.update((current) => current.filter((f) => f.path !== path));
}

export function clearFiles() {
  files.set([]);
  isCompressing.set(false);
}
