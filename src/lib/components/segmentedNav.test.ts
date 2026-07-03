import { describe, it, expect } from "vitest";
import { nextSegmentIndex } from "./segmentedNav";

describe("nextSegmentIndex", () => {
  it("ArrowRight from middle moves to next", () => {
    expect(nextSegmentIndex(1, "ArrowRight", 4)).toBe(2);
  });

  it("ArrowDown from middle moves to next", () => {
    expect(nextSegmentIndex(1, "ArrowDown", 4)).toBe(2);
  });

  it("ArrowRight from last wraps to first", () => {
    expect(nextSegmentIndex(3, "ArrowRight", 4)).toBe(0);
  });

  it("ArrowLeft from first wraps to last", () => {
    expect(nextSegmentIndex(0, "ArrowLeft", 4)).toBe(3);
  });

  it("ArrowUp from first wraps to last", () => {
    expect(nextSegmentIndex(0, "ArrowUp", 4)).toBe(3);
  });

  it("Home jumps to first", () => {
    expect(nextSegmentIndex(2, "Home", 4)).toBe(0);
  });

  it("End jumps to last", () => {
    expect(nextSegmentIndex(2, "End", 4)).toBe(3);
  });

  it("unhandled key returns null", () => {
    expect(nextSegmentIndex(1, "a", 4)).toBeNull();
  });
});
