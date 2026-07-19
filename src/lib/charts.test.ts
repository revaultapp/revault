import { describe, expect, it } from "vitest";

import { nextChartIndex, normalizeChartIndex } from "./charts";

describe("normalizeChartIndex", () => {
  it("preserves valid indices and clamps integer indices to the series", () => {
    expect(normalizeChartIndex(5, 12)).toBe(5);
    expect(normalizeChartIndex(-1, 12)).toBe(0);
    expect(normalizeChartIndex(12, 12)).toBe(11);
  });

  it("rejects invalid series lengths and non-integer indices", () => {
    expect(normalizeChartIndex(0, 0)).toBeNull();
    expect(normalizeChartIndex(0, -1)).toBeNull();
    expect(normalizeChartIndex(NaN, 12)).toBeNull();
    expect(normalizeChartIndex(1.5, 12)).toBeNull();
  });
});

describe("nextChartIndex", () => {
  it("moves right and wraps from the last point", () => {
    expect(nextChartIndex(5, "ArrowRight", 12)).toBe(6);
    expect(nextChartIndex(11, "ArrowRight", 12)).toBe(0);
  });

  it("moves left and wraps from the first point", () => {
    expect(nextChartIndex(5, "ArrowLeft", 12)).toBe(4);
    expect(nextChartIndex(0, "ArrowLeft", 12)).toBe(11);
  });

  it("jumps to the first and last points", () => {
    expect(nextChartIndex(7, "Home", 12)).toBe(0);
    expect(nextChartIndex(3, "End", 12)).toBe(11);
  });

  it("ignores unsupported keys", () => {
    expect(nextChartIndex(5, "ArrowDown", 12)).toBeNull();
    expect(nextChartIndex(5, "Enter", 12)).toBeNull();
  });

  it("rejects invalid series and non-integer active indices", () => {
    expect(nextChartIndex(0, "ArrowRight", 0)).toBeNull();
    expect(nextChartIndex(0, "ArrowRight", -1)).toBeNull();
    expect(nextChartIndex(NaN, "ArrowRight", 12)).toBeNull();
    expect(nextChartIndex(1.5, "ArrowRight", 12)).toBeNull();
  });

  it("clamps integer active indices before navigating", () => {
    expect(nextChartIndex(-1, "ArrowRight", 12)).toBe(1);
    expect(nextChartIndex(12, "ArrowLeft", 12)).toBe(10);
  });
});
