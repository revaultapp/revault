import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { persisted } from "$lib/utils";

export interface DuplicateFile {
  path: string;
  size: number;
  modified: number;
}

export interface DuplicateGroup {
  hash: string;
  distance: number;
  max_distance: number;
  files: DuplicateFile[];
}

export interface FindDuplicatesResult {
  groups: DuplicateGroup[];
  total_scanned: number;
  errors: string[];
}

export interface ScanProgress {
  request_id?: number;
  current: number;
  total: number;
  phase: string;
}

export const scanMode = persisted<"exact" | "similar">("dedupe-scan-mode", "exact");

export const duplicateGroups = writable<DuplicateGroup[]>([]);
export const isScanning = writable(false);
export const totalFound = writable(0);
export const scanError = writable<string | null>(null);
export const scanProgress = writable<ScanProgress | null>(null);

let scanRequestId = 0;

export function setMode(m: "exact" | "similar") {
  scanMode.set(m);
  clearResults();
}

export async function scanForDuplicates(paths: string[], recursive = true) {
  const requestId = ++scanRequestId;
  isScanning.set(true);
  duplicateGroups.set([]);
  totalFound.set(0);
  scanError.set(null);
  scanProgress.set(null);

  // Listen for progress events from Rust — scoped to this call
  let unlisten: (() => void) | null = await listen<ScanProgress>("dedupe-progress", (event) => {
    if (requestId !== scanRequestId || event.payload.request_id !== requestId) return;
    scanProgress.set(event.payload);
  });

  try {
    const result = await invoke<FindDuplicatesResult>("find_duplicates", { paths, recursive, mode: get(scanMode), requestId });
    if (requestId !== scanRequestId) return;
    const total = result.groups.reduce((acc, g) => acc + g.files.length - 1, 0);
    duplicateGroups.set(result.groups);
    totalFound.set(total);
  } catch (err) {
    if (requestId !== scanRequestId) return;
    scanError.set(String(err));
  } finally {
    if (requestId === scanRequestId) {
      isScanning.set(false);
      scanProgress.set(null);
    }
    unlisten?.();
  }
}

export async function cancelScan() {
  if (!get(isScanning)) return;
  scanRequestId++;
  isScanning.set(false);
  scanProgress.set(null);
  try {
    await invoke("cancel_dedupe_scan");
  } catch (err) {
    scanError.set(String(err));
  }
}

export function clearResults() {
  scanRequestId++;
  isScanning.set(false);
  duplicateGroups.set([]);
  totalFound.set(0);
  scanError.set(null);
  scanProgress.set(null);
}
