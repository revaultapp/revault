import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

describe("savings store", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
  });

  it("loads numeric values from localStorage", async () => {
    localStorage.setItem("revault_saved_bytes", "100");
    localStorage.setItem("revault_operations_count", "2");
    localStorage.setItem("revault_heic_count", "1");
    localStorage.setItem("revault_original_bytes", "500");
    localStorage.setItem("revault_compressed_bytes", "300");

    const { savings } = await import("./savings");

    expect(get(savings)).toMatchObject({
      totalSavedBytes: 100,
      filesProcessed: 0,
      operationsCount: 2,
      heicCount: 1,
      totalOriginalBytes: 500,
      totalCompressedBytes: 300,
    });
  });

  it("adds savings and persists positive updates", async () => {
    const { savings } = await import("./savings");

    savings.add(120);
    savings.incrementOps(3);
    savings.incrementHeic(2);
    savings.addOriginalBytes(500);
    savings.addCompressedBytes(380);

    expect(get(savings)).toEqual({
      totalSavedBytes: 120,
      filesProcessed: 3,
      operationsCount: 3,
      heicCount: 2,
      totalOriginalBytes: 500,
      totalCompressedBytes: 380,
    });
    expect(localStorage.getItem("revault_saved_bytes")).toBe("120");
    expect(localStorage.getItem("revault_operations_count")).toBe("3");
    expect(localStorage.getItem("revault_files_processed")).toBe("3");
    expect(localStorage.getItem("revault_heic_count")).toBe("2");
  });

  it("ignores non-positive updates", async () => {
    const { savings } = await import("./savings");

    savings.add(0);
    savings.incrementOps(-1);
    savings.incrementHeic(0);
    savings.addOriginalBytes(-1);
    savings.addCompressedBytes(0);

    expect(get(savings)).toEqual({
      totalSavedBytes: 0,
      filesProcessed: 0,
      operationsCount: 0,
      heicCount: 0,
      totalOriginalBytes: 0,
      totalCompressedBytes: 0,
    });
  });
});
