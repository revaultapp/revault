import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
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

export interface PreviewResult {
  input_path: string;
  original_size: number;
  compressed_size: number;
  may_increase: boolean;
  error: string | null;
}

export interface PreviewResponse {
  total_original_bytes: number;
  sample_results: PreviewResult[];
}

export interface SavingsEstimate {
  estimatedSavedBytes: number;
  sampleRatio: number;
  filesMayIncrease: number;
  sampleSize: number;
  totalOriginalBytes: number;
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
export const isEstimating = writable(false);
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

/**
 * Estimate compression savings by:
 * 1. Reading sizes of ALL files (for accurate total)
 * 2. Compressing a sample (5 largest) to get compression ratio
 * 3. Applying ratio to total for accurate estimate
 */
export async function estimateSavings(
  allFiles: CompressFile[],
  preset: QualityPreset,
  fmt: OutputFormat | null,
): Promise<SavingsEstimate | null> {
  if (allFiles.length === 0) return null;

  isEstimating.set(true);
  try {
    const allPaths = allFiles.map((f) => f.path);

    const response = await invoke<PreviewResponse>("preview_compress", {
      allPaths,
      qualityPreset: preset,
      format: fmt,
    });

    const { total_original_bytes, sample_results } = response;

    let sampleOriginal = 0;
    let sampleCompressed = 0;
    let filesMayIncrease = 0;
    let validResults = 0;

    for (const r of sample_results) {
      if (r.error) continue;
      sampleOriginal += r.original_size;
      sampleCompressed += r.compressed_size;
      if (r.may_increase) filesMayIncrease++;
      validResults++;
    }

    if (validResults === 0) return null;

    const sampleRatio = sampleCompressed / sampleOriginal;
    const estimatedCompressed = Math.round(total_original_bytes * sampleRatio);
    const estimatedSavedBytes = total_original_bytes - estimatedCompressed;

    return {
      estimatedSavedBytes,
      sampleRatio,
      filesMayIncrease,
      sampleSize: validResults,
      totalOriginalBytes: total_original_bytes,
    };
  } catch {
    return null;
  } finally {
    isEstimating.set(false);
  }
}
