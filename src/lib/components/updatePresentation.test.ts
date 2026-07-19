import { describe, expect, it } from "vitest";
import { progressPercent, shouldShowUpdateDialog } from "./updatePresentation";

describe("update presentation", () => {
  it("shows only actionable update flows with a discovered update", () => {
    expect(shouldShowUpdateDialog("available", true, false)).toBe(true);
    expect(shouldShowUpdateDialog("downloading", false, true)).toBe(true);
    expect(shouldShowUpdateDialog("readyToRestart", false, true)).toBe(true);
    expect(shouldShowUpdateDialog("error", false, true)).toBe(true);
    expect(shouldShowUpdateDialog("error", false, false)).toBe(false);
    expect(shouldShowUpdateDialog("upToDate", false, false)).toBe(false);
  });

  it("clamps determinate download progress and handles an unknown total", () => {
    expect(progressPercent({ downloaded: 25, total: 100 })).toBe(25);
    expect(progressPercent({ downloaded: 140, total: 100 })).toBe(100);
    expect(progressPercent({ downloaded: -10, total: 100 })).toBe(0);
    expect(progressPercent({ downloaded: 20, total: 0 })).toBeNull();
  });
});
