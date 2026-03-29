import { writable } from "svelte/store";

const SAVED_KEY = "revault_saved_bytes";
const OPS_KEY = "revault_operations_count";
const HEIC_KEY = "revault_heic_count";
const ORIG_KEY = "revault_original_bytes";
const COMP_KEY = "revault_compressed_bytes";

export interface SavingsState {
  totalSavedBytes: number;
  filesProcessed: number;
  operationsCount: number;
  heicCount: number;
  totalOriginalBytes: number;
  totalCompressedBytes: number;
}

function safeParse(key: string): number {
  try {
    const val = localStorage.getItem(key);
    if (val === null) return 0;
    const n = Number(val);
    return Number.isFinite(n) ? n : 0;
  } catch {
    return 0;
  }
}

function persist(key: string, value: number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    // localStorage unavailable or quota exceeded — state still updated in-memory
  }
}

function createSavingsStore() {
  const { subscribe, update } = writable<SavingsState>({
    totalSavedBytes: safeParse(SAVED_KEY),
    filesProcessed: safeParse("revault_files_processed"),
    operationsCount: safeParse(OPS_KEY),
    heicCount: safeParse(HEIC_KEY),
    totalOriginalBytes: safeParse(ORIG_KEY),
    totalCompressedBytes: safeParse(COMP_KEY),
  });

  return {
    subscribe,
    add(bytes: number) {
      if (bytes <= 0) return;
      update((s) => {
        const next = { ...s, totalSavedBytes: s.totalSavedBytes + bytes };
        persist(SAVED_KEY, next.totalSavedBytes);
        return next;
      });
    },
    incrementOps(count: number) {
      if (count <= 0) return;
      update((s) => {
        const next = { ...s, operationsCount: s.operationsCount + count, filesProcessed: s.filesProcessed + count };
        persist(OPS_KEY, next.operationsCount);
        persist("revault_files_processed", next.filesProcessed);
        return next;
      });
    },
    incrementHeic(count: number) {
      if (count <= 0) return;
      update((s) => {
        const next = { ...s, heicCount: s.heicCount + count };
        persist(HEIC_KEY, next.heicCount);
        return next;
      });
    },
    addOriginalBytes(bytes: number) {
      if (bytes <= 0) return;
      update((s) => {
        const next = { ...s, totalOriginalBytes: s.totalOriginalBytes + bytes };
        persist(ORIG_KEY, next.totalOriginalBytes);
        return next;
      });
    },
    addCompressedBytes(bytes: number) {
      if (bytes <= 0) return;
      update((s) => {
        const next = { ...s, totalCompressedBytes: s.totalCompressedBytes + bytes };
        persist(COMP_KEY, next.totalCompressedBytes);
        return next;
      });
    },
  };
}

export const savings = createSavingsStore();
