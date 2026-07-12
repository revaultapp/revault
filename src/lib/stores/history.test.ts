import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { get } from "svelte/store";

const STORAGE_KEY = "revault_history_v1";

async function freshHistory() {
  vi.resetModules();
  return await import("./history");
}

describe("history store", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 5, 15)); // 2026-06-15
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("monthKey formats YYYY-MM with zero-padded month", async () => {
    const { monthKey } = await freshHistory();
    expect(monthKey(new Date(2026, 0, 5))).toBe("2026-01");
    expect(monthKey(new Date(2026, 10, 5))).toBe("2026-11");
  });

  it("recordSavings accumulates per kind and orig/comp/ops", async () => {
    const { history, monthKey } = await freshHistory();
    history.recordSavings("img", 1000, 400);
    history.recordSavings("img", 500, 200);
    const key = monthKey(new Date());
    const bucket = get(history).months[key];
    expect(bucket.img).toBe(900);
    expect(bucket.orig).toBe(1500);
    expect(bucket.comp).toBe(600);
    expect(bucket.ops).toBe(2);
  });

  it("recordSavings ignores non-finite and negative inputs", async () => {
    const { history } = await freshHistory();
    history.recordSavings("img", NaN, 100);
    history.recordSavings("img", -5, 100);
    history.recordSavings("img", 100, -5);
    expect(Object.keys(get(history).months).length).toBe(0);
  });

  it("recordProtected ignores zero and negative, accumulates otherwise", async () => {
    const { history, monthKey } = await freshHistory();
    history.recordProtected(0);
    history.recordProtected(-3);
    expect(Object.keys(get(history).months).length).toBe(0);
    history.recordProtected(4);
    const key = monthKey(new Date());
    expect(get(history).months[key].protected).toBe(4);
  });

  it("creates a new bucket on month rollover", async () => {
    const { history, monthKey } = await freshHistory();
    history.recordSavings("img", 1000, 400);
    const juneKey = monthKey(new Date());
    vi.setSystemTime(new Date(2026, 6, 1)); // 2026-07-01
    history.recordSavings("img", 200, 100);
    const s = get(history);
    expect(Object.keys(s.months).sort()).toEqual([juneKey, "2026-07"].sort());
    expect(s.months[juneKey].img).toBe(600);
    expect(s.months["2026-07"].img).toBe(100);
  });

  it("prunes to the 13 most recent months", async () => {
    const { history } = await freshHistory();
    for (let i = 0; i < 15; i++) {
      vi.setSystemTime(new Date(2025, i, 15)); // 2025-01 .. 2026-03
      history.recordSavings("img", 100, 50);
    }
    const keys = Object.keys(get(history).months).sort();
    expect(keys.length).toBe(13);
    expect(keys[0]).toBe("2025-03");
    expect(keys[keys.length - 1]).toBe("2026-03");
  });

  it("persists and reloads across fresh module instances", async () => {
    const first = await freshHistory();
    first.history.recordSavings("img", 1000, 400);
    const second = await freshHistory();
    const key = second.monthKey(new Date());
    expect(get(second.history).months[key].img).toBe(600);
  });

  it("falls back to fresh state on corrupt JSON", async () => {
    localStorage.setItem(STORAGE_KEY, "{not-json");
    const { history } = await freshHistory();
    expect(get(history)).toEqual({ v: 1, months: {}, lastScan: null });
  });

  it("falls back to fresh state on version mismatch", async () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ v: 2, months: {}, lastScan: null }));
    const { history } = await freshHistory();
    const s = get(history);
    expect(s.v).toBe(1);
    expect(s.months).toEqual({});
  });

  it("momDeltas.saved is null with only a single month of data, series still has 12 entries", async () => {
    const { history, momDeltas, monthlySeries } = await freshHistory();
    history.recordSavings("img", 1000, 400);
    const deltas = get(momDeltas);
    expect(deltas.saved).toBeNull();
    expect(deltas.compression).toBeNull();
    const series = get(monthlySeries);
    expect(series.length).toBe(12);
    expect(series[11].img).toBe(600);
    expect(series[11].total).toBe(600);
    expect(series[0].total).toBe(0);
  });

  it("momDeltas.saved is null when previous month total is zero even if its bucket exists", async () => {
    const { history, momDeltas } = await freshHistory();
    history.recordProtected(2); // creates a June bucket with all savings fields at 0
    vi.setSystemTime(new Date(2026, 6, 15)); // 2026-07-15
    history.recordSavings("img", 1000, 400);
    expect(get(momDeltas).saved).toBeNull();
  });

  it("setLastScan stores and persists the scan snapshot", async () => {
    const { history } = await freshHistory();
    const scan = { ts: 123, total: 5000, types: [["jpg", 3000, 2], ["png", 2000, 1]] as [string, number, number][] };
    history.setLastScan(scan);
    expect(get(history).lastScan).toEqual(scan);
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
    expect(raw.lastScan.total).toBe(5000);
  });
});
