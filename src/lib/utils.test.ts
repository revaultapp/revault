import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { formatBytes, runWithConcurrency, persisted } from "./utils";

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
});
