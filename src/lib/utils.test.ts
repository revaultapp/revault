import { describe, it, expect, beforeEach, vi } from "vitest";
import { get, writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import {
  addUniqueByPath,
  formatBytes,
  moveByPath,
  persisted,
  persistedWithGlobalDefault,
  removeByPath,
  runWithConcurrency,
  withListener,
} from "./utils";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const mockListen = listen as ReturnType<typeof vi.fn>;

describe("formatBytes", () => {
  it("formats zero", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats sub-KB values as bytes", () => {
    expect(formatBytes(500)).toBe("500 B");
  });

  it("formats KB", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("formats MB", () => {
    expect(formatBytes(1024 * 1024 * 2.5)).toBe("2.5 MB");
  });

  it("formats GB", () => {
    expect(formatBytes(1024 * 1024 * 1024 * 1.25)).toBe("1.25 GB");
  });

  it("formats negative values by prefixing the positive format", () => {
    expect(formatBytes(-500)).toBe("-500 B");
    expect(formatBytes(-2048)).toBe("-2.0 KB");
  });
});

describe("runWithConcurrency", () => {
  beforeEach(() => {
    Object.defineProperty(navigator, "hardwareConcurrency", {
      value: 6,
      configurable: true,
    });
  });

  it("resolves immediately for an empty list without calling the task", async () => {
    let called = false;
    await runWithConcurrency([], async () => {
      called = true;
    });
    expect(called).toBe(false);
  });

  it("never runs more concurrent tasks than the computed limit", async () => {
    const items = Array.from({ length: 8 }, (_, i) => i);
    let active = 0;
    let maxActive = 0;
    await runWithConcurrency(items, async () => {
      active++;
      maxActive = Math.max(maxActive, active);
      await new Promise((r) => setTimeout(r, 5));
      active--;
    });
    // concurrency = min(max(2, hardwareConcurrency - 2), items.length) = min(4, 8) = 4
    expect(maxActive).toBe(4);
  });

  it("invokes tasks in input order even when earlier items resolve slower", async () => {
    // concurrency = min(max(2, hardwareConcurrency - 2), items.length) = min(4, 4) = 4,
    // so all 4 items get their own worker slot with no requeueing to complicate timing.
    const items = [0, 1, 2, 3];
    const invoked: number[] = [];
    const completed: number[] = [];
    await runWithConcurrency(items, async (item) => {
      invoked.push(item);
      await new Promise((r) => setTimeout(r, (items.length - item) * 5));
      completed.push(item);
    });
    expect(invoked).toEqual([0, 1, 2, 3]);
    expect(completed).toEqual([3, 2, 1, 0]);
  });
});

describe("persisted", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("uses the initial value when nothing is persisted", () => {
    const store = persisted("test_persisted_default", 5);
    expect(get(store)).toBe(5);
  });

  it("reads the persisted value instead of the initial value", () => {
    localStorage.setItem("test_persisted_existing", JSON.stringify(42));
    const store = persisted("test_persisted_existing", 0);
    expect(get(store)).toBe(42);
  });

  it("writes updates to localStorage as JSON", () => {
    const store = persisted("test_persisted_write", { a: 1 });
    store.set({ a: 2 });
    expect(localStorage.getItem("test_persisted_write")).toBe(JSON.stringify({ a: 2 }));
  });

  it("round-trips across separate store instances for the same key", () => {
    const first = persisted("test_persisted_roundtrip", "a");
    first.set("b");
    const second = persisted("test_persisted_roundtrip", "z");
    expect(get(second)).toBe("b");
  });

  it("falls back to the initial value on corrupt (non-JSON) stored data instead of throwing", () => {
    localStorage.setItem("test_persisted_corrupt", "{not json");
    const store = persisted("test_persisted_corrupt", "safe");
    expect(get(store)).toBe("safe");
  });

  it("falls back to the initial value when validate rejects the stored value", () => {
    localStorage.setItem("test_persisted_stale", JSON.stringify("Medium"));
    const isPreset = (v: unknown) => v === "Smallest" || v === "Balanced";
    const store = persisted("test_persisted_stale", "Balanced", isPreset);
    expect(get(store)).toBe("Balanced");
  });

  it("keeps the stored value when validate accepts it", () => {
    localStorage.setItem("test_persisted_valid", JSON.stringify("Smallest"));
    const isPreset = (v: unknown) => v === "Smallest" || v === "Balanced";
    const store = persisted("test_persisted_valid", "Balanced", isPreset);
    expect(get(store)).toBe("Smallest");
  });
});

describe("persistedWithGlobalDefault", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("seeds from a non-null global default over an existing persisted value", () => {
    localStorage.setItem("test_pwgd_seed", JSON.stringify("HighQuality"));
    const global = writable<string | null>("Smallest");
    const store = persistedWithGlobalDefault("test_pwgd_seed", "Balanced", global);
    expect(get(store)).toBe("Smallest");
  });

  it("falls back to the persisted value when the global default is null", () => {
    localStorage.setItem("test_pwgd_persisted", JSON.stringify("HighQuality"));
    const global = writable<string | null>(null);
    const store = persistedWithGlobalDefault("test_pwgd_persisted", "Balanced", global);
    expect(get(store)).toBe("HighQuality");
  });

  it("falls back to the fallback when the global is null and nothing is persisted", () => {
    const global = writable<string | null>(null);
    const store = persistedWithGlobalDefault("test_pwgd_fallback", "Balanced", global);
    expect(get(store)).toBe("Balanced");
  });

  it("propagates a live non-null global change to the store and persists it", () => {
    const global = writable<string | null>(null);
    const store = persistedWithGlobalDefault("test_pwgd_live", "Balanced", global);
    global.set("HighQuality");
    expect(get(store)).toBe("HighQuality");
    expect(localStorage.getItem("test_pwgd_live")).toBe(JSON.stringify("HighQuality"));
  });

  it("treats a live null global change as a no-op (remember-last)", () => {
    const global = writable<string | null>("Smallest");
    const store = persistedWithGlobalDefault("test_pwgd_null_noop", "Balanced", global);
    store.set("HighQuality");
    global.set(null);
    expect(get(store)).toBe("HighQuality");
  });

  it("persists post-seed writes normally and never writes back to the global", () => {
    const global = writable<string | null>("Smallest");
    const store = persistedWithGlobalDefault("test_pwgd_writeback", "Balanced", global);
    store.set("HighQuality");
    expect(localStorage.getItem("test_pwgd_writeback")).toBe(JSON.stringify("HighQuality"));
    expect(get(global)).toBe("Smallest");
  });

  it("applies validate to the persisted fallback path under a null global", () => {
    localStorage.setItem("test_pwgd_validate", JSON.stringify("Medium"));
    const global = writable<string | null>(null);
    const isPreset = (v: unknown) => v === "Smallest" || v === "Balanced" || v === "HighQuality";
    const store = persistedWithGlobalDefault("test_pwgd_validate", "Balanced", global, isPreset);
    expect(get(store)).toBe("Balanced");
  });

  it("falls back on corrupt persisted data under a null global instead of throwing", () => {
    localStorage.setItem("test_pwgd_corrupt", "{not json");
    const global = writable<string | null>(null);
    const store = persistedWithGlobalDefault("test_pwgd_corrupt", "Balanced", global);
    expect(get(store)).toBe("Balanced");
  });
});

describe("addUniqueByPath / removeByPath / moveByPath", () => {
  const make = (path: string, name: string) => ({ path, name });

  it("appends only unseen paths, preserving order, and dedupes within one call", () => {
    const curr = [{ path: "/a", name: "a" }];
    const next = addUniqueByPath(curr, ["/a", "/b", "/b", "/c"], make);
    expect(next.map((f) => f.path)).toEqual(["/a", "/b", "/c"]);
  });

  it("derives the name from the last path segment (both separators)", () => {
    const next = addUniqueByPath([], ["/x/y/photo.jpg", "C:\\dir\\doc.pdf"], make);
    expect(next.map((f) => f.name)).toEqual(["photo.jpg", "doc.pdf"]);
  });

  it("removeByPath removes the match and no-ops on unknown paths", () => {
    const curr = [{ path: "/a", name: "a" }, { path: "/b", name: "b" }];
    expect(removeByPath(curr, "/a").map((f) => f.path)).toEqual(["/b"]);
    expect(removeByPath(curr, "/nope")).toHaveLength(2);
  });

  it("moveByPath swaps neighbours and no-ops at the boundaries", () => {
    const curr = [{ path: "/a", name: "a" }, { path: "/b", name: "b" }, { path: "/c", name: "c" }];
    expect(moveByPath(curr, "/b", -1).map((f) => f.path)).toEqual(["/b", "/a", "/c"]);
    expect(moveByPath(curr, "/a", -1).map((f) => f.path)).toEqual(["/a", "/b", "/c"]);
    expect(moveByPath(curr, "/c", 1).map((f) => f.path)).toEqual(["/a", "/b", "/c"]);
    expect(moveByPath(curr, "/missing", 1)).toBe(curr);
  });
});

describe("withListener", () => {
  it("attaches, forwards payloads, and unlistens after work resolves", async () => {
    const unlisten = vi.fn();
    let handler: ((e: { payload: unknown }) => void) | undefined;
    mockListen.mockImplementationOnce(async (_evt: string, cb: (e: { payload: unknown }) => void) => {
      handler = cb;
      return unlisten;
    });
    const seen: unknown[] = [];
    const result = await withListener("test-evt", (p) => seen.push(p), async () => {
      handler?.({ payload: 42 });
      return "done";
    });
    expect(result).toBe("done");
    expect(seen).toEqual([42]);
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("unlistens even when work rejects", async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementationOnce(async () => unlisten);
    await expect(
      withListener("test-evt", () => {}, async () => {
        throw new Error("boom");
      }),
    ).rejects.toThrow("boom");
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("propagates a listen() rejection without invoking work", async () => {
    mockListen.mockRejectedValueOnce(new Error("ipc down"));
    const work = vi.fn();
    await expect(withListener("test-evt", () => {}, work)).rejects.toThrow("ipc down");
    expect(work).not.toHaveBeenCalled();
  });
});
