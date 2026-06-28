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

export const resolvedOutputDir = derived(
  [outputDir, defaultOutputDir],
  ([$out, $def]) => $out ?? $def,
);

export const summary = derived(files, ($f) => ({
  done: $f.filter((f) => f.status === "done").length,
  failed: $f.filter((f) => f.status === "error").length,
  pending: $f.filter((f) => f.status === "pending" || f.status === "processing").length,
}));

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

export async function processPdfs(
  outDir: string | null,
  strip: boolean,
  compress: boolean,
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
