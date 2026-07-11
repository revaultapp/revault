import { writable, derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { BaseFile } from "$lib/types";
import { persisted } from "$lib/utils";
import { defaultOutputDir } from "./settings";

export type PdfStatus = "pending" | "processing" | "done" | "error";

export interface PdfFile extends BaseFile {
  status: PdfStatus;
  originalSize?: number;
  outputSize?: number;
  outputPath?: string;
  error?: string;
}

interface PdfResult {
  input_path: string;
  output_path: string;
  original_size: number;
  output_size: number;
  error: string | null;
}

export const files = writable<PdfFile[]>([]);
export const isProcessing = writable(false);
export const outputDir = persisted<string | null>("revault-pdf-outputDir", null);
export const stripMetadata = persisted<boolean>("revault-pdf-strip", true);
export const compressStreams = persisted<boolean>("revault-pdf-compress", true);
export const compressImages = persisted<boolean>("pdf-compress-images", false);

export const resolvedOutputDir = derived(
  [outputDir, defaultOutputDir],
  ([$out, $def]) => $out ?? $def,
);

export const summary = derived(files, ($f) => {
  const done = $f.filter((f) => f.status === "done");
  return {
    done: done.length,
    failed: $f.filter((f) => f.status === "error").length,
    pending: $f.filter((f) => f.status === "pending" || f.status === "processing").length,
    savedBytes: Math.max(0, done.reduce((acc, f) => acc + ((f.originalSize ?? 0) - (f.outputSize ?? f.originalSize ?? 0)), 0)),
  };
});

export function addFiles(paths: string[]) {
  files.update((curr) => {
    const existing = new Set(curr.map((f) => f.path));
    const newPaths = paths.filter((p) => {
      if (existing.has(p)) return false;
      existing.add(p);
      return true;
    });
    return [
      ...curr,
      ...newPaths.map((p) => ({
        path: p,
        name: p.split(/[\\/]/).pop() ?? p,
        status: "pending" as const,
      })),
    ];
  });
}

export function removeFile(path: string) {
  files.update((curr) => curr.filter((f) => f.path !== path));
}

export function clearFiles() {
  files.set([]);
  isProcessing.set(false);
}

export async function revealPdfOutput(path: string): Promise<void> {
  await invoke("reveal_pdf_output", { path });
}

// --- Merge: N ordered PDFs -> 1 output ---

export interface MergeFile {
  path: string;
  name: string;
}

export interface MergeResultInfo {
  outputPath: string;
  outputSize: number;
  pageCount: number;
}

export const mergeFiles = writable<MergeFile[]>([]);
export const isMerging = writable(false);
export const mergeResult = writable<MergeResultInfo | null>(null);
export const mergeError = writable<string | null>(null);

export function addMergeFiles(paths: string[]) {
  mergeFiles.update((curr) => {
    const existing = new Set(curr.map((f) => f.path));
    const newPaths = paths.filter((p) => {
      if (existing.has(p)) return false;
      existing.add(p);
      return true;
    });
    return [
      ...curr,
      ...newPaths.map((p) => ({ path: p, name: p.split(/[\\/]/).pop() ?? p })),
    ];
  });
}

export function removeMergeFile(path: string) {
  mergeFiles.update((curr) => curr.filter((f) => f.path !== path));
}

export function moveMergeFile(path: string, direction: -1 | 1) {
  mergeFiles.update((curr) => {
    const idx = curr.findIndex((f) => f.path === path);
    const target = idx + direction;
    if (idx === -1 || target < 0 || target >= curr.length) return curr;
    const next = [...curr];
    [next[idx], next[target]] = [next[target], next[idx]];
    return next;
  });
}

export function clearMerge() {
  mergeFiles.set([]);
  mergeResult.set(null);
  mergeError.set(null);
  isMerging.set(false);
}

export async function mergePdfs(outDir: string | null) {
  const paths = get(mergeFiles).map((f) => f.path);
  if (paths.length < 2) return;
  isMerging.set(true);
  mergeError.set(null);
  try {
    const result = await invoke<{ output_path: string; output_size: number; page_count: number }>(
      "merge_pdfs",
      { paths, outputDir: outDir },
    );
    mergeResult.set({
      outputPath: result.output_path,
      outputSize: result.output_size,
      pageCount: result.page_count,
    });
  } catch (e) {
    mergeError.set(String(e));
  } finally {
    isMerging.set(false);
  }
}

// --- Split: 1 PDF -> range extract or one file per page ---

export type SplitKind = "range" | "each";

export interface SplitFile {
  path: string;
  name: string;
}

export const splitFile = writable<SplitFile | null>(null);
export const isSplitting = writable(false);
export const splitResults = writable<string[]>([]);
export const splitError = writable<string | null>(null);

export function setSplitFile(path: string) {
  splitFile.set({ path, name: path.split(/[\\/]/).pop() ?? path });
  splitResults.set([]);
  splitError.set(null);
}

export function clearSplit() {
  splitFile.set(null);
  splitResults.set([]);
  splitError.set(null);
  isSplitting.set(false);
}

export async function splitPdf(
  mode: SplitKind,
  start: number | undefined,
  end: number | undefined,
  outDir: string | null,
) {
  const input = get(splitFile);
  if (!input) return;
  isSplitting.set(true);
  splitError.set(null);
  try {
    const paths = await invoke<string[]>("split_pdf", {
      input: input.path,
      mode,
      start,
      end,
      outputDir: outDir,
    });
    splitResults.set(paths);
  } catch (e) {
    splitError.set(String(e));
  } finally {
    isSplitting.set(false);
  }
}

export async function processPdfs(
  outDir: string | null,
  strip: boolean,
  compress: boolean,
  compressImgs: boolean,
) {
  isProcessing.set(true);
  files.update((curr) => curr.map((f) => ({ ...f, status: "processing" as const })));
  try {
    const paths = get(files).map((f) => f.path);
    const results = await invoke<PdfResult[]>("process_pdfs", {
      paths,
      outputDir: outDir,
      stripMetadata: strip,
      compressStreams: compress,
      compressImages: compressImgs,
    });
    const byPath = new Map(results.map((r) => [r.input_path, r]));
    files.update((curr) =>
      curr.map((f) => {
        const r = byPath.get(f.path);
        if (!r) return f;
        if (r.error) return { ...f, status: "error" as const, error: r.error };
        return {
          ...f,
          status: "done" as const,
          originalSize: r.original_size,
          outputSize: r.output_size,
          outputPath: r.output_path,
        };
      }),
    );
  } catch (e) {
    files.update((curr) =>
      curr.map((f) => ({ ...f, status: "error" as const, error: String(e) })),
    );
  } finally {
    isProcessing.set(false);
  }
}
