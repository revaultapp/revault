import { writable, derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { activity } from "./activity";
import { savings } from "./savings";
import type { BaseFile } from "$lib/types";
import { addUniqueByPath, moveByPath, persisted, removeByPath, withListener } from "$lib/utils";
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
// One-time migration: this key originally shipped without the `revault-pdf-`
// prefix its siblings use. Copy any legacy value before binding the new key.
try {
  const legacy = localStorage.getItem("pdf-compress-images");
  if (legacy !== null && localStorage.getItem("revault-pdf-compress-images") === null) {
    localStorage.setItem("revault-pdf-compress-images", legacy);
  }
} catch {
  // localStorage unavailable — nothing to migrate
}
export const compressImages = persisted<boolean>("revault-pdf-compress-images", false);

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
  files.update((curr) =>
    addUniqueByPath(curr, paths, (path, name) => ({ path, name, status: "pending" as const })),
  );
}

export function removeFile(path: string) {
  files.update((curr) => removeByPath(curr, path));
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
  mergeFiles.update((curr) => addUniqueByPath(curr, paths, (path, name) => ({ path, name })));
}

export function removeMergeFile(path: string) {
  mergeFiles.update((curr) => removeByPath(curr, path));
}

export function moveMergeFile(path: string, direction: -1 | 1) {
  mergeFiles.update((curr) => moveByPath(curr, path, direction));
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

// --- Images → PDF: N ordered images -> 1 PDF (scan-to-PDF) ---

export type PdfPageSize = "fit" | "a4" | "letter";
export type PdfPageMargin = "none" | "small" | "big";

export interface ImageToPdfFile {
  path: string;
  name: string;
}

export interface ImagesToPdfResultInfo {
  outputPath: string;
  outputSize: number;
  pageCount: number;
}

export const imageFiles = writable<ImageToPdfFile[]>([]);
export const isBuildingPdf = writable(false);
export const imagesResult = writable<ImagesToPdfResultInfo | null>(null);
export const imagesError = writable<string | null>(null);
export const pageSize = persisted<PdfPageSize>("revault-pdf-i2p-pagesize", "a4");
export const pageMargin = persisted<PdfPageMargin>("revault-pdf-i2p-margin", "small");

export function addImageFiles(paths: string[]) {
  imageFiles.update((curr) => addUniqueByPath(curr, paths, (path, name) => ({ path, name })));
}

export function removeImageFile(path: string) {
  imageFiles.update((curr) => removeByPath(curr, path));
}

export function moveImageFile(path: string, direction: -1 | 1) {
  imageFiles.update((curr) => moveByPath(curr, path, direction));
}

export function clearImages() {
  imageFiles.set([]);
  imagesResult.set(null);
  imagesError.set(null);
  isBuildingPdf.set(false);
}

export async function imagesToPdf(outDir: string | null) {
  const paths = get(imageFiles).map((f) => f.path);
  if (paths.length < 1) return;
  isBuildingPdf.set(true);
  imagesError.set(null);
  try {
    const result = await invoke<{ output_path: string; output_size: number; page_count: number }>(
      "images_to_pdf",
      { paths, outputDir: outDir, pageSize: get(pageSize), margin: get(pageMargin) },
    );
    imagesResult.set({
      outputPath: result.output_path,
      outputSize: result.output_size,
      pageCount: result.page_count,
    });
  } catch (e) {
    imagesError.set(String(e));
  } finally {
    isBuildingPdf.set(false);
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

// --- PDF → Images: 1 PDF -> N rasterized page images ---

export type PdfPagesMode = "all" | "range";
export type PdfRasterFormat = "jpg" | "png";
export type PdfRasterDpi = 150 | 300;

export interface PdfToImagesFile {
  path: string;
  name: string;
}

export const p2iFile = writable<PdfToImagesFile | null>(null);
export const isRasterizing = writable(false);
export const p2iResults = writable<string[]>([]);
export const p2iError = writable<string | null>(null);
export const p2iProgress = writable<{ current: number; total: number } | null>(null);
export const p2iFormat = persisted<PdfRasterFormat>("revault-pdf-p2i-format", "jpg");
export const p2iDpi = persisted<PdfRasterDpi>("revault-pdf-p2i-dpi", 150);

export function setP2iFile(path: string) {
  p2iFile.set({ path, name: path.split(/[\\/]/).pop() ?? path });
  p2iResults.set([]);
  p2iError.set(null);
  p2iProgress.set(null);
}

export function clearP2i() {
  p2iFile.set(null);
  p2iResults.set([]);
  p2iError.set(null);
  p2iProgress.set(null);
  isRasterizing.set(false);
}

export async function pdfToImages(
  pagesMode: PdfPagesMode,
  start: number | undefined,
  end: number | undefined,
  outDir: string | null,
) {
  const input = get(p2iFile);
  if (!input) return;
  isRasterizing.set(true);
  p2iError.set(null);
  p2iResults.set([]);
  p2iProgress.set(null);

  try {
    const paths = await withListener<string[], { current: number; total: number }>(
      "pdf-rasterize-progress",
      (p) => p2iProgress.set(p),
      () =>
        invoke<string[]>("pdf_to_images", {
          input: input.path,
          pagesMode,
          start,
          end,
          dpi: get(p2iDpi),
          format: get(p2iFormat),
          outputDir: outDir,
        }),
    );
    p2iResults.set(paths);
    // Rasterizing is additive (the source PDF is kept), so no byte-level
    // savings are recorded — only the operation count, matching audio extract.
    activity.add({ type: "convert", fileCount: paths.length, savedBytes: 0 });
    savings.incrementOps(1);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("cancelled")) {
      p2iProgress.set(null);
    } else {
      p2iError.set(msg);
    }
  } finally {
    isRasterizing.set(false);
  }
}

export async function cancelPdfToImages() {
  try {
    await invoke("cancel_pdf_to_images");
  } catch {
    // best-effort cancel — the process may have already finished
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
