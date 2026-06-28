import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { createSavingsStore } from "./savings";

const SAVED_KEY = "revault_saved_bytes";
const OPS_KEY = "revault_operations_count";
const FILES_KEY = "revault_files_processed";
const HEIC_KEY = "revault_heic_count";
const ORIG_KEY = "revault_original_bytes";
const COMP_KEY = "revault_compressed_bytes";

describe("savings store", () => {
  let store: ReturnType<typeof createSavingsStore>;

  beforeEach(() => {
    localStorage.clear();
    store = createSavingsStore();
  });

  describe("add", () => {
    it("increases totalSavedBytes and persists", () => {
      store.add(500);
      expect(get(store).totalSavedBytes).toBe(500);
      expect(localStorage.getItem(SAVED_KEY)).toBe("500");

      store.add(300);
      expect(get(store).totalSavedBytes).toBe(800);
      expect(localStorage.getItem(SAVED_KEY)).toBe("800");
    });

    it("ignores zero and negative", () => {
      store.add(0);
      store.add(-1);
      expect(get(store).totalSavedBytes).toBe(0);
      expect(localStorage.getItem(SAVED_KEY)).toBeNull();
    });
  });

  describe("incrementOps", () => {
    it("increases operationsCount and filesProcessed, persists both", () => {
      store.incrementOps(3);
      const s = get(store);
      expect(s.operationsCount).toBe(3);
      expect(s.filesProcessed).toBe(3);
      expect(localStorage.getItem(OPS_KEY)).toBe("3");
      expect(localStorage.getItem(FILES_KEY)).toBe("3");
    });

    it("ignores zero and negative", () => {
      store.incrementOps(0);
      store.incrementOps(-5);
      const s = get(store);
      expect(s.operationsCount).toBe(0);
      expect(s.filesProcessed).toBe(0);
    });
  });

  describe("incrementHeic", () => {
    it("increases heicCount and persists", () => {
      store.incrementHeic(2);
      expect(get(store).heicCount).toBe(2);
      expect(localStorage.getItem(HEIC_KEY)).toBe("2");
    });

    it("ignores zero and negative", () => {
      store.incrementHeic(0);
      store.incrementHeic(-1);
      expect(get(store).heicCount).toBe(0);
    });
  });

  describe("addOriginalBytes / addCompressedBytes", () => {
    it("accumulates original bytes", () => {
      store.addOriginalBytes(1024);
      expect(get(store).totalOriginalBytes).toBe(1024);
      expect(localStorage.getItem(ORIG_KEY)).toBe("1024");
    });

    it("accumulates compressed bytes", () => {
      store.addCompressedBytes(512);
      expect(get(store).totalCompressedBytes).toBe(512);
      expect(localStorage.getItem(COMP_KEY)).toBe("512");
    });

    it("ignores zero and negative for both", () => {
      store.addOriginalBytes(0);
      store.addOriginalBytes(-100);
      store.addCompressedBytes(0);
      store.addCompressedBytes(-1);
      expect(get(store).totalOriginalBytes).toBe(0);
      expect(get(store).totalCompressedBytes).toBe(0);
    });
  });

  describe("initial state from localStorage", () => {
    it("reads all persisted values on creation", () => {
      localStorage.setItem(SAVED_KEY, "9000");
      localStorage.setItem(OPS_KEY, "42");
      localStorage.setItem(FILES_KEY, "42");
      localStorage.setItem(HEIC_KEY, "7");
      localStorage.setItem(ORIG_KEY, "8000");
      localStorage.setItem(COMP_KEY, "4000");

      const fresh = createSavingsStore();
      const s = get(fresh);
      expect(s.totalSavedBytes).toBe(9000);
      expect(s.operationsCount).toBe(42);
      expect(s.filesProcessed).toBe(42);
      expect(s.heicCount).toBe(7);
      expect(s.totalOriginalBytes).toBe(8000);
      expect(s.totalCompressedBytes).toBe(4000);
    });

    it("defaults corrupt localStorage values to 0", () => {
      localStorage.setItem(SAVED_KEY, "not-a-number");
      const fresh = createSavingsStore();
      expect(get(fresh).totalSavedBytes).toBe(0);
    });
  });
});
