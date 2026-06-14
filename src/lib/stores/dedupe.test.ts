import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  duplicateGroups,
  isScanning,
  scanError,
  scanProgress,
  totalFound,
  cancelScan,
  clearResults,
  scanForDuplicates,
} from "./dedupe";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);
let progressHandler: ((event: { payload: { request_id?: number; current: number; total: number; phase: string } }) => void) | undefined;

describe("dedupe store", () => {
  beforeEach(() => {
    clearResults();
    isScanning.set(false);
    mockInvoke.mockReset();
    progressHandler = undefined;
    mockListen.mockReset();
    mockListen.mockImplementation(async (_event, handler) => {
      progressHandler = handler as typeof progressHandler;
      return () => {};
    });
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

  it("passes a requestId and ignores progress for other requests", async () => {
    let resolveScan: (value: unknown) => void = () => {};
    mockInvoke.mockReturnValueOnce(new Promise((resolve) => {
      resolveScan = resolve;
    }));

    const scanPromise = scanForDuplicates(["/some/folder"]);
    await Promise.resolve();

    const findArgs = mockInvoke.mock.calls[0][1] as { requestId: number };
    const requestId = findArgs.requestId;
    expect(mockInvoke).toHaveBeenCalledWith("find_duplicates", {
      paths: ["/some/folder"],
      recursive: true,
      mode: "exact",
      requestId,
    });

    progressHandler?.({ payload: { request_id: requestId + 1, current: 9, total: 10, phase: "hashing" } });
    expect(get(scanProgress)).toBeNull();

    progressHandler?.({ payload: { request_id: requestId, current: 3, total: 10, phase: "hashing" } });
    expect(get(scanProgress)).toEqual({ request_id: requestId, current: 3, total: 10, phase: "hashing" });

    resolveScan({ groups: [], total_scanned: 0, errors: [] });
    await scanPromise;
  });

  it("cancelScan invokes Rust cancellation and prevents stale scan results", async () => {
    let resolveScan: (value: unknown) => void = () => {};
    mockInvoke.mockImplementation((command) => {
      if (command === "find_duplicates") {
        return new Promise((resolve) => { resolveScan = resolve; });
      }
      return Promise.resolve(null);
    });

    const scanPromise = scanForDuplicates(["/some/folder"]);
    await Promise.resolve();

    await cancelScan();

    expect(mockInvoke).toHaveBeenCalledWith("cancel_dedupe_scan");
    expect(get(isScanning)).toBe(false);

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
