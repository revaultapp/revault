import { writable, derived } from "svelte/store";

export type SavingsKind = "img" | "vid" | "pdf";

interface MonthBucket {
  img: number;
  vid: number;
  pdf: number;
  orig: number;
  comp: number;
  ops: number;
  protected: number;
}

interface LastScan {
  ts: number;
  total: number;
  types: [string, number, number][];
}

interface HistoryV1 {
  v: 1;
  months: Record<string, MonthBucket>;
  lastScan: LastScan | null;
}

const STORAGE_KEY = "revault_history_v1";
const MAX_MONTHS = 13;
const BUCKET_KEYS = ["img", "vid", "pdf", "orig", "comp", "ops", "protected"] as const;

export function monthKey(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  return `${y}-${m}`;
}

function emptyBucket(): MonthBucket {
  return { img: 0, vid: 0, pdf: 0, orig: 0, comp: 0, ops: 0, protected: 0 };
}

function freshState(): HistoryV1 {
  return { v: 1, months: {}, lastScan: null };
}

function isValidBucket(b: unknown): b is MonthBucket {
  if (!b || typeof b !== "object") return false;
  return BUCKET_KEYS.every((k) => typeof (b as Record<string, unknown>)[k] === "number");
}

function isValidLastScan(s: unknown): s is LastScan {
  if (!s || typeof s !== "object") return false;
  const scan = s as Record<string, unknown>;
  return (
    typeof scan.ts === "number" &&
    typeof scan.total === "number" &&
    Array.isArray(scan.types)
  );
}

function load(): HistoryV1 {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return freshState();
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || parsed.v !== 1 || typeof parsed.months !== "object" || parsed.months === null) {
      return freshState();
    }
    const months: Record<string, MonthBucket> = {};
    for (const [key, value] of Object.entries(parsed.months as Record<string, unknown>)) {
      if (isValidBucket(value)) months[key] = value;
    }
    const lastScan = isValidLastScan(parsed.lastScan) ? parsed.lastScan : null;
    return { v: 1, months, lastScan };
  } catch {
    return freshState();
  }
}

function persist(state: HistoryV1) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // localStorage unavailable or quota exceeded — state still updated in-memory
  }
}

function pruneMonths(months: Record<string, MonthBucket>): Record<string, MonthBucket> {
  const keys = Object.keys(months).sort();
  if (keys.length <= MAX_MONTHS) return months;
  const keep = keys.slice(keys.length - MAX_MONTHS);
  const next: Record<string, MonthBucket> = {};
  for (const k of keep) next[k] = months[k];
  return next;
}

function createHistoryStore() {
  const { subscribe, update, set } = writable<HistoryV1>(load());

  function mutate(fn: (s: HistoryV1) => HistoryV1) {
    update((s) => {
      const next = fn(s);
      const pruned = { ...next, months: pruneMonths(next.months) };
      persist(pruned);
      return pruned;
    });
  }

  return {
    subscribe,
    recordSavings(kind: SavingsKind, originalBytes: number, compressedBytes: number) {
      if (!Number.isFinite(originalBytes) || !Number.isFinite(compressedBytes)) return;
      if (originalBytes < 0 || compressedBytes < 0) return;
      mutate((s) => {
        const key = monthKey(new Date());
        const bucket = { ...(s.months[key] ?? emptyBucket()) };
        bucket[kind] += Math.max(0, originalBytes - compressedBytes);
        bucket.orig += originalBytes;
        bucket.comp += compressedBytes;
        bucket.ops += 1;
        return { ...s, months: { ...s.months, [key]: bucket } };
      });
    },
    recordProtected(count: number) {
      if (!Number.isFinite(count) || count <= 0) return;
      mutate((s) => {
        const key = monthKey(new Date());
        const bucket = { ...(s.months[key] ?? emptyBucket()) };
        bucket.protected += count;
        return { ...s, months: { ...s.months, [key]: bucket } };
      });
    },
    setLastScan(scan: LastScan) {
      mutate((s) => ({ ...s, lastScan: scan }));
    },
    reset() {
      const fresh = freshState();
      persist(fresh);
      set(fresh);
    },
  };
}

export const history = createHistoryStore();
export { createHistoryStore };

export const monthlySeries = derived(history, ($h) => {
  const now = new Date();
  const result: { key: string; date: Date; img: number; vid: number; pdf: number; total: number }[] = [];
  for (let i = 11; i >= 0; i--) {
    const d = new Date(now.getFullYear(), now.getMonth() - i, 1);
    const key = monthKey(d);
    const b = $h.months[key];
    const img = b?.img ?? 0;
    const vid = b?.vid ?? 0;
    const pdf = b?.pdf ?? 0;
    result.push({ key, date: d, img, vid, pdf, total: img + vid + pdf });
  }
  return result;
});

function compressionRatio(b: MonthBucket | undefined): number | null {
  if (!b || b.orig <= 0) return null;
  return (1 - b.comp / b.orig) * 100;
}

export const momDeltas = derived(history, ($h) => {
  const now = new Date();
  const curKey = monthKey(now);
  const prevKey = monthKey(new Date(now.getFullYear(), now.getMonth() - 1, 1));
  const cur = $h.months[curKey];
  const prev = $h.months[prevKey];

  let saved: { pct: number; up: boolean } | null = null;
  const curTotal = (cur?.img ?? 0) + (cur?.vid ?? 0) + (cur?.pdf ?? 0);
  const prevTotal = (prev?.img ?? 0) + (prev?.vid ?? 0) + (prev?.pdf ?? 0);
  if (prev && prevTotal > 0) {
    const pct = ((curTotal - prevTotal) / prevTotal) * 100;
    saved = { pct: Math.abs(pct), up: pct >= 0 };
  }

  let compression: { pct: number; up: boolean } | null = null;
  const curRatio = compressionRatio(cur);
  const prevRatio = compressionRatio(prev);
  if (curRatio !== null && prevRatio !== null) {
    const delta = curRatio - prevRatio;
    compression = { pct: Math.abs(delta), up: delta >= 0 };
  }

  return { saved, compression };
});

export interface CategoryShare {
  kind: SavingsKind;
  sum: number;
  share: number;
  delta: { pct: number; up: boolean } | null;
}

export const categoryShares = derived([history, monthlySeries], ([$h, $series]) => {
  const kinds: SavingsKind[] = ["img", "vid", "pdf"];
  const sums: Record<SavingsKind, number> = { img: 0, vid: 0, pdf: 0 };
  for (const m of $series) {
    sums.img += m.img;
    sums.vid += m.vid;
    sums.pdf += m.pdf;
  }
  const grand = sums.img + sums.vid + sums.pdf;

  const now = new Date();
  const curKey = monthKey(now);
  const prevKey = monthKey(new Date(now.getFullYear(), now.getMonth() - 1, 1));
  const cur = $h.months[curKey];
  const prev = $h.months[prevKey];

  return kinds.map((kind): CategoryShare => {
    const sum = sums[kind];
    const share = grand > 0 ? (sum / grand) * 100 : 0;
    let delta: { pct: number; up: boolean } | null = null;
    const curVal = cur?.[kind] ?? 0;
    const prevVal = prev?.[kind] ?? 0;
    if (prev && prevVal > 0) {
      const pct = ((curVal - prevVal) / prevVal) * 100;
      delta = { pct: Math.abs(pct), up: pct >= 0 };
    }
    return { kind, sum, share, delta };
  });
});

export const protectedTotal = derived(history, ($h) =>
  Object.values($h.months).reduce((acc, b) => acc + b.protected, 0)
);
