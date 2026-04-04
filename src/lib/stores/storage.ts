import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface ImageInfo {
  path: string;
  relative_path: string;
  size: number;
  extension: string;
}

export interface ScanResult {
  images: ImageInfo[];
  total_size: number;
  skipped: number;
}

export interface ExtensionGroup {
  extension: string;
  count: number;
  totalSize: number;
  percentage: number;
}

type ScanState = "idle" | "scanning" | "done" | "error";

interface StorageState {
  scanState: ScanState;
  scanResult: ScanResult | null;
  errorMessage: string | null;
  folderPath: string | null;
}

function createStorageStore() {
  const { subscribe, set, update } = writable<StorageState>({
    scanState: "idle",
    scanResult: null,
    errorMessage: null,
    folderPath: null,
  });

  let currentScanId = 0;

  return {
    subscribe,

    async scanFolder(): Promise<void> {
      const dir = await open({ directory: true, multiple: false });
      if (!dir || typeof dir !== "string") return;

      const thisScanId = ++currentScanId;
      update((s) => ({ ...s, scanState: "scanning", errorMessage: null, folderPath: dir }));

      try {
        const result = await invoke<ScanResult>("scan_folder", {
          path: dir,
          recursive: true,
        });
        if (thisScanId !== currentScanId) return;
        update((s) => ({ ...s, scanState: "done", scanResult: result }));
      } catch (err) {
        if (thisScanId !== currentScanId) return;
        update((s) => ({
          ...s,
          scanState: "error",
          errorMessage: String(err),
        }));
      }
    },

    reset(): void {
      set({ scanState: "idle", scanResult: null, errorMessage: null, folderPath: null });
    },
  };
}

export const storage = createStorageStore();

export const breakdown = derived(storage, ($storage): ExtensionGroup[] => {
  if (!$storage.scanResult) return [];

  const map = new Map<string, { count: number; totalSize: number }>();

  for (const img of $storage.scanResult.images) {
    const existing = map.get(img.extension) ?? { count: 0, totalSize: 0 };
    map.set(img.extension, {
      count: existing.count + 1,
      totalSize: existing.totalSize + img.size,
    });
  }

  const total = $storage.scanResult.total_size;
  if (total === 0) return [];

  return Array.from(map.entries())
    .map(([extension, data]) => ({
      extension: extension.toUpperCase(),
      count: data.count,
      totalSize: data.totalSize,
      percentage: Math.round((data.totalSize / total) * 100),
    }))
    .sort((a, b) => b.totalSize - a.totalSize);
});
