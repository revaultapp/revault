import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { history } from "./history";

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
        try {
          history.setLastScan({
            ts: Date.now(),
            total: result.total_size,
            types: groupByExtension(result).map((g) => [g.extension, g.totalSize, g.count]),
          });
        } catch {
          // history recording must never break scanning
        }
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

export function groupByExtension(result: ScanResult): ExtensionGroup[] {
  const map = new Map<string, { count: number; totalSize: number }>();

  for (const img of result.images) {
    const existing = map.get(img.extension) ?? { count: 0, totalSize: 0 };
    map.set(img.extension, {
      count: existing.count + 1,
      totalSize: existing.totalSize + img.size,
    });
  }

  const total = result.total_size;
  if (total === 0) return [];

  return Array.from(map.entries())
    .map(([extension, data]) => ({
      extension: extension.toUpperCase(),
      count: data.count,
      totalSize: data.totalSize,
      percentage: Math.round((data.totalSize / total) * 100),
    }))
    .sort((a, b) => b.totalSize - a.totalSize);
}

export const breakdown = derived(storage, ($storage): ExtensionGroup[] => {
  if (!$storage.scanResult) return [];
  return groupByExtension($storage.scanResult);
});
