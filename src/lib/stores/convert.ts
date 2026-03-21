import { writable, derived } from "svelte/store";
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
export const activeProfile = writable<"Web" | "Email" | "Archive" | "Share" | "Custom">("Custom");
export const selectedPlatforms = writable<string[]>([]);

export const summary = derived(files, ($files) => {
  const done = $files.filter((f) => f.status === "done");
  return {
    done: done.length,
    failed: $files.filter((f) => f.status === "error").length,
    pending: $files.filter((f) => f.status === "pending" || f.status === "converting").length,
    savedBytes: done.reduce((acc, f) => acc + ((f.size ?? 0) - (f.outputSize ?? (f.size ?? 0))), 0),
  };
});

export function addFiles(paths: string[]) {
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
    return [...current, ...newFiles];
  });
}

export function removeFile(path: string) {
  files.update((current) => current.filter((f) => f.path !== path));
}

export function clearFiles() {
  files.set([]);
  isConverting.set(false);
}
