import { writable, derived } from "svelte/store";

export type FileStatus = "pending" | "compressing" | "done" | "error";
export type OutputFormat = "Jpeg" | "Png" | "Webp";

export interface CompressFile {
  path: string;
  name: string;
  size: number;
  status: FileStatus;
  compressedSize?: number;
  outputPath?: string;
  error?: string;
}

export const files = writable<CompressFile[]>([]);
export const quality = writable(80);
export const format = writable<OutputFormat | null>(null);
export const isCompressing = writable(false);

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
