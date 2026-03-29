import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export interface DuplicateFile {
  path: string;
  size: number;
  modified: number;
}

export interface DuplicateGroup {
  hash: string;
  distance: number;
  files: DuplicateFile[];
}

export interface FindDuplicatesResult {
  groups: DuplicateGroup[];
  total_scanned: number;
  errors: string[];
}

export const duplicateGroups = writable<DuplicateGroup[]>([]);
export const isScanning = writable(false);
export const totalFound = writable(0);
export const scanError = writable<string | null>(null);

export async function scanForDuplicates(paths: string[]) {
  isScanning.set(true);
  duplicateGroups.set([]);
  totalFound.set(0);
  scanError.set(null);
  try {
    const result = await invoke<FindDuplicatesResult>("find_duplicates", { paths });
    const total = result.groups.reduce((acc, g) => acc + g.files.length - 1, 0);
    duplicateGroups.set(result.groups);
    totalFound.set(total);
  } catch (err) {
    scanError.set(String(err));
  } finally {
    isScanning.set(false);
  }
}

export function clearResults() {
  duplicateGroups.set([]);
  totalFound.set(0);
}
