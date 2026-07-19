import { describe, expect, it } from "vitest";

import {
  groupDonutDisplaySegments,
  nextChartIndex,
  normalizeChartIndex,
} from "./charts";

describe("groupDonutDisplaySegments", () => {
  it("returns no display segments for empty input", () => {
    expect(groupDonutDisplaySegments([], "Other")).toEqual([]);
  });

  it("normalizes a single segment", () => {
    expect(groupDonutDisplaySegments([{ label: " PNG ", bytes: 20, count: 2 }], "Other")).toEqual([
      { key: "png", label: "PNG", bytes: 20, count: 2, sourceLabels: [" PNG "] },
    ]);
  });

  it("sorts exactly five segments without creating Other", () => {
    const segments = [
      { label: "PNG", bytes: 20, count: 2 },
      { label: "JPG", bytes: 50, count: 5 },
      { label: "HEIC", bytes: 10, count: 1 },
      { label: "WEBP", bytes: 15, count: 3 },
      { label: "AVIF", bytes: 20, count: 1 },
    ];

    expect(groupDonutDisplaySegments(segments, "Other", 5).map(({ key }) => key)).toEqual([
      "jpg",
      "avif",
      "png",
      "webp",
      "heic",
    ]);
  });

  it("keeps the top four of six and combines the tail exactly", () => {
    const segments = [
      { label: "PNG", bytes: 20, count: 2 },
      { label: "JPG", bytes: 50, count: 5 },
      { label: "HEIC", bytes: 10, count: 1 },
      { label: "WEBP", bytes: 15, count: 3 },
      { label: "AVIF", bytes: 5, count: 1 },
      { label: "BMP", bytes: 2, count: 4 },
    ];

    expect(groupDonutDisplaySegments(segments, "Other", 5)).toEqual([
      { key: "jpg", label: "JPG", bytes: 50, count: 5, sourceLabels: ["JPG"] },
      { key: "png", label: "PNG", bytes: 20, count: 2, sourceLabels: ["PNG"] },
      { key: "webp", label: "WEBP", bytes: 15, count: 3, sourceLabels: ["WEBP"] },
      { key: "heic", label: "HEIC", bytes: 10, count: 1, sourceLabels: ["HEIC"] },
      {
        key: "__other__",
        label: "Other",
        bytes: 7,
        count: 5,
        sourceLabels: ["AVIF", "BMP"],
      },
    ]);
  });

  it("merges labels case-insensitively under the first normalized label", () => {
    expect(
      groupDonutDisplaySegments(
        [
          { label: "PNG", bytes: 20, count: 2 },
          { label: "png", bytes: 5, count: 1 },
          { label: "Png", bytes: 2, count: 3 },
        ],
        "Other",
      ),
    ).toEqual([
      {
        key: "png",
        label: "PNG",
        bytes: 27,
        count: 6,
        sourceLabels: ["PNG", "png", "Png"],
      },
    ]);
  });

  it("omits empty labels and invalid bytes, and clamps invalid counts to zero", () => {
    const segments = [
      { label: "", bytes: 10, count: 1 },
      { label: "   ", bytes: 10, count: 1 },
      { label: "NaN", bytes: NaN, count: 1 },
      { label: "Infinity", bytes: Infinity, count: 1 },
      { label: "Negative", bytes: -1, count: 1 },
      { label: "PNG", bytes: 20, count: NaN },
      { label: "png", bytes: 5, count: -3 },
      { label: "JPG", bytes: 10, count: Infinity },
    ];

    expect(groupDonutDisplaySegments(segments, "Other")).toEqual([
      { key: "png", label: "PNG", bytes: 25, count: 0, sourceLabels: ["PNG", "png"] },
      { key: "jpg", label: "JPG", bytes: 10, count: 0, sourceLabels: ["JPG"] },
    ]);
  });

  it.each([0, -1, 1.5, NaN, Infinity])(
    "falls back to five visible segments for invalid maxVisible %s",
    (maxVisible) => {
      const segments = [
        { label: "A", bytes: 6, count: 1 },
        { label: "B", bytes: 5, count: 1 },
        { label: "C", bytes: 4, count: 1 },
        { label: "D", bytes: 3, count: 1 },
        { label: "E", bytes: 2, count: 1 },
        { label: "F", bytes: 1, count: 1 },
      ];

      expect(groupDonutDisplaySegments(segments, "Other", maxVisible).map(({ key }) => key)).toEqual([
        "a",
        "b",
        "c",
        "d",
        "__other__",
      ]);
    },
  );

  it("does not mutate the input or its segments", () => {
    const segments = [
      { label: "PNG", bytes: 20, count: 2 },
      { label: "JPG", bytes: 50, count: 5 },
    ];
    const snapshot = structuredClone(segments);

    groupDonutDisplaySegments(segments, "Other");

    expect(segments).toEqual(snapshot);
  });
});

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

  it("maps vertical arrows to next and previous points with wrapping", () => {
    expect(nextChartIndex(11, "ArrowDown", 12)).toBe(0);
    expect(nextChartIndex(0, "ArrowUp", 12)).toBe(11);
  });

  it("jumps to the first and last points", () => {
    expect(nextChartIndex(7, "Home", 12)).toBe(0);
    expect(nextChartIndex(3, "End", 12)).toBe(11);
  });

  it("ignores unsupported keys", () => {
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
