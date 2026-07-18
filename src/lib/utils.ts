import { writable } from "svelte/store";
import type { Writable } from "svelte/store";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

export function persisted<T>(key: string, initial: T): Writable<T> {
  const stored =
    typeof localStorage !== "undefined" ? localStorage.getItem(key) : null;
  const store = writable<T>(stored !== null ? (JSON.parse(stored) as T) : initial);
  store.subscribe((v) => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(key, JSON.stringify(v));
    }
  });
  return store;
}

export async function browseOutputDir(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  return typeof dir === "string" ? dir : null;
}


export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 0) return `-${formatBytes(-bytes)}`;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export async function runWithConcurrency<T>(
  items: T[],
  task: (item: T) => Promise<void>,
): Promise<void> {
  if (items.length === 0) return;
  const concurrency = Math.min(
    Math.max(2, (navigator.hardwareConcurrency || 4) - 2),
    items.length,
  );
  let nextIndex = 0;
  async function worker() {
    while (nextIndex < items.length) {
      await task(items[nextIndex++]);
    }
  }
  await new Promise<void>((r) => setTimeout(r, 0));
  await Promise.all(Array.from({ length: concurrency }, () => worker()));
}


/**
 * Dedupe-and-append for path-keyed list stores. The same shape was
 * re-implemented in 8+ stores (pdf ×3, compress, convert, resize, privacy,
 * video); new code should use these instead of hand-rolling.
 */
export function addUniqueByPath<T extends { path: string }>(
  curr: T[],
  paths: string[],
  make: (path: string, name: string) => T,
): T[] {
  const existing = new Set(curr.map((f) => f.path));
  const fresh = paths.filter((p) => {
    if (existing.has(p)) return false;
    existing.add(p);
    return true;
  });
  return [...curr, ...fresh.map((p) => make(p, p.split(/[\\/]/).pop() ?? p))];
}

export function removeByPath<T extends { path: string }>(curr: T[], path: string): T[] {
  return curr.filter((f) => f.path !== path);
}

/** Swap an item one position up/down; no-op at the boundaries or if absent. */
export function moveByPath<T extends { path: string }>(
  curr: T[],
  path: string,
  direction: -1 | 1,
): T[] {
  const idx = curr.findIndex((f) => f.path === path);
  const target = idx + direction;
  if (idx === -1 || target < 0 || target >= curr.length) return curr;
  const next = [...curr];
  [next[idx], next[target]] = [next[target], next[idx]];
  return next;
}

/**
 * Runs `work` with a Tauri event listener attached, guaranteeing the listener
 * is removed on every exit path — including a rejection from `listen()`
 * itself. Replaces the listen-before-try pattern that could leave stores
 * stuck in a busy state if listen() rejected outside the caller's try/catch.
 */
export async function withListener<T, P>(
  event: string,
  onEvent: (payload: P) => void,
  work: () => Promise<T>,
): Promise<T> {
  let unlisten: (() => void) | null = null;
  try {
    unlisten = await listen<P>(event, (e) => onEvent(e.payload));
    return await work();
  } finally {
    unlisten?.();
  }
}
