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

export const duplicateGroups = writable<DuplicateGroup[]>([]);
export const isScanning = writable(false);
export const totalFound = writable(0);

export async function scanForDuplicates(paths: string[]) {
  isScanning.set(true);
  duplicateGroups.set([]);
  totalFound.set(0);
  try {
    const groups = await invoke<DuplicateGroup[]>("find_duplicates", { paths });
    const total = groups.reduce((acc, g) => acc + g.files.length - 1, 0);
    duplicateGroups.set(groups);
    totalFound.set(total);
  } finally {
    isScanning.set(false);
  }
}

export function clearResults() {
  duplicateGroups.set([]);
  totalFound.set(0);
}
