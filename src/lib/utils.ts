import { get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

function persistedStore<T>(key: string, value: T): Writable<T> {
  const store = writable<T>(value);
  store.subscribe((v) => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(key, JSON.stringify(v));
    }
  });
  return store;
}

function readPersisted<T>(key: string, fallback: T, validate?: (v: unknown) => boolean): T {
  const stored =
    typeof localStorage !== "undefined" ? localStorage.getItem(key) : null;
  if (stored === null) return fallback;
  try {
    const parsed: unknown = JSON.parse(stored);
    return validate === undefined || validate(parsed) ? (parsed as T) : fallback;
  } catch {
    // Corrupt localStorage must never blank-screen the app: persisted() runs
    // at module eval, before any error boundary could exist.
    return fallback;
  }
}

/**
 * `validate` rejects stale persisted values (e.g. an enum variant that no
 * longer exists after a refactor) so they fall back to `initial` instead of
 * reaching the IPC boundary and dying as a serde error.
 */
export function persisted<T>(
  key: string,
  initial: T,
  validate?: (v: unknown) => boolean,
): Writable<T> {
  return persistedStore<T>(key, readPersisted(key, initial, validate));
}

/**
 * Like `persisted`, but a global default (e.g. from Settings) both seeds the
 * store and tracks it live: whenever `globalDefault` emits a non-null value,
 * the store is set to it immediately — no app restart. Null (= "remember
 * last use") never touches the store: at init it falls back to the persisted
 * value or `fallback`, and later null emissions are no-ops. Tool-page writes
 * persist to `key` as normal and never write back to the global.
 *
 * Assumes `globalDefault` hydrates synchronously at module eval (true for
 * localStorage-backed stores) — the skip-first below relies on nothing
 * changing between get() and subscribe(). The subscription is deliberately
 * never torn down: both sides are module-level singletons that live for the
 * app's lifetime, so there is no teardown point. Don't add bookkeeping.
 */
export function persistedWithGlobalDefault<T>(
  key: string,
  fallback: T,
  globalDefault: Readable<T | null>,
  validate?: (v: unknown) => boolean,
): Writable<T> {
  const initialGlobal = get(globalDefault);
  const store = persistedStore<T>(
    key,
    initialGlobal !== null ? initialGlobal : readPersisted(key, fallback, validate),
  );
  // subscribe() fires synchronously with the current value, which duplicates
  // the get() above — skip it to avoid a redundant set/localStorage rewrite.
  let first = true;
  globalDefault.subscribe((v) => {
    if (first) {
      first = false;
      return;
    }
    if (v !== null && get(store) !== v) store.set(v);
  });
  return store;
}

export async function browseOutputDir(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  return typeof dir === "string" ? dir : null;
}


export function formatBytes(bytes: number, locale?: string): string {
  if (bytes === 0) return "0 B";
  if (bytes < 0) return `-${formatBytes(-bytes, locale)}`;
  const number = (value: number, digits: number) =>
    locale
      ? new Intl.NumberFormat(locale, {
          minimumFractionDigits: digits,
          maximumFractionDigits: digits,
        }).format(value)
      : value.toFixed(digits);
  if (bytes < 1024) return `${locale ? new Intl.NumberFormat(locale).format(bytes) : bytes} B`;
  if (bytes < 1024 * 1024) return `${number(bytes / 1024, 1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${number(bytes / (1024 * 1024), 1)} MB`;
  return `${number(bytes / (1024 * 1024 * 1024), 2)} GB`;
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
