import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import {
  duplicateGroups,
  isScanning,
  scanError,
  scanProgress,
  totalFound,
  clearResults,
  scanForDuplicates,
} from "./dedupe";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

const mockInvoke = vi.mocked(invoke);

describe("dedupe store", () => {
  beforeEach(() => {
    clearResults();
    isScanning.set(false);
    mockInvoke.mockReset();
  });

  it("clearResults resets all state", () => {
    duplicateGroups.set([{ hash: "abc", distance: 0, max_distance: 0, files: [{ path: "a", size: 1, modified: 0 }] }]);
    totalFound.set(5);
    scanError.set("oops");
    scanProgress.set({ current: 3, total: 10, phase: "hashing" });

    clearResults();

    expect(get(duplicateGroups)).toEqual([]);
    expect(get(totalFound)).toBe(0);
    expect(get(scanError)).toBeNull();
    expect(get(scanProgress)).toBeNull();
  });

  it("scanForDuplicates on error sets scanError and clears isScanning", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("scan failed"));

    await scanForDuplicates(["/some/folder"]);

    expect(get(scanError)).toBe("Error: scan failed");
    expect(get(isScanning)).toBe(false);
  });

  it("totalFound counts files minus one per group", async () => {
    mockInvoke.mockResolvedValueOnce({
      groups: [
        {
          hash: "abc",
          distance: 0,
          max_distance: 0,
          files: [{ path: "a", size: 1, modified: 0 }, { path: "b", size: 1, modified: 0 }, { path: "c", size: 1, modified: 0 }],
        },
      ],
      total_scanned: 3,
      errors: [],
    });

    await scanForDuplicates(["/some/folder"]);

    // 3 files in group - 1 = 2 duplicates
    expect(get(totalFound)).toBe(2);
    expect(get(duplicateGroups)).toHaveLength(1);
  });

  it("ignores stale scan results after clearResults", async () => {
    let resolveScan: (value: unknown) => void = () => {};
    mockInvoke.mockReturnValueOnce(new Promise((resolve) => {
      resolveScan = resolve;
    }));

    const scanPromise = scanForDuplicates(["/some/folder"]);
    clearResults();
    resolveScan({
      groups: [
        {
          hash: "stale",
          distance: 0,
          max_distance: 0,
          files: [{ path: "a", size: 1, modified: 0 }, { path: "b", size: 1, modified: 0 }],
        },
      ],
      total_scanned: 2,
      errors: [],
    });
    await scanPromise;

    expect(get(duplicateGroups)).toEqual([]);
    expect(get(totalFound)).toBe(0);
    expect(get(isScanning)).toBe(false);
  });
});
