import { writable, derived } from "svelte/store";
import type { BaseFile } from "$lib/types";

export type MetaCategory = "GPS" | "Device" | "DateTime" | "Author" | "Technical";

export interface PrivacyFile extends BaseFile {
  status: "pending" | "scanning" | "scanned" | "stripping" | "done" | "error";
  gps?: string;
  device?: string;
  datetime?: string;
  author?: string;
  technical?: string;
  hasMetadata?: boolean;
  outputPath?: string;
  originalSize?: number;
  strippedSize?: number;
  error?: string;
}

export const files = writable<PrivacyFile[]>([]);
export const selectedCategories = writable<Set<MetaCategory>>(
  new Set(["GPS", "Device", "DateTime", "Author", "Technical"])
);
export const isProcessing = writable(false);

export const summary = derived(files, ($f) => ({
  scanned: $f.filter((f) => f.status === "scanned" || f.status === "done").length,
  stripped: $f.filter((f) => f.status === "done").length,
  failed: $f.filter((f) => f.status === "error").length,
}));

export function addFiles(paths: string[]) {
  files.update((curr) => {
    const existing = new Set(curr.map((f) => f.path));
    return [
      ...curr,
      ...paths
        .filter((p) => !existing.has(p))
        .map((p) => ({
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
