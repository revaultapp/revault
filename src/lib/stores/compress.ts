import { writable, derived } from "svelte/store";
import type { BaseFile } from "$lib/types";

export type FileStatus = "pending" | "compressing" | "done" | "error";
export type OutputFormat = "Jpeg" | "Png" | "Webp" | "Avif";
export type QualityPreset = "Smallest" | "Balanced" | "HighQuality";

export interface CompressFile extends BaseFile {
  size: number;
  status: FileStatus;
  compressedSize?: number;
  outputPath?: string;
  alreadyOptimal?: boolean;
  error?: string;
}

function persisted<T>(key: string, initial: T) {
  const stored = localStorage.getItem(key);
  const value: T = stored !== null ? (JSON.parse(stored) as T) : initial;
  const store = writable<T>(value);
  store.subscribe((v) => localStorage.setItem(key, JSON.stringify(v)));
  return store;
}

export const files = writable<CompressFile[]>([]);
export const qualityPreset = persisted<QualityPreset>("compress_quality_preset", "Balanced");
export const format = persisted<OutputFormat | null>("compress_format", null);
export const outputDir = writable<string | null>(null);
export const isCompressing = writable(false);
export const stripGps = persisted<boolean>("compress_strip_gps", false);

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
