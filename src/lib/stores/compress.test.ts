import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import {
  files,
  qualityPreset,
  format,
  summary,
  addFiles,
  removeFile,
  clearFiles,
} from "./compress";

// Mock @tauri-apps/api/core for all tests
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("compress store", () => {
  beforeEach(() => {
    // Reset stores before each test
    files.set([]);
    qualityPreset.set("Balanced");
    format.set(null);
  });

  describe("addFiles", () => {
    it("adds new files to the store", () => {
      addFiles(["/path/to/photo1.jpg", "/path/to/photo2.png"]);

      const current = get(files);
      expect(current).toHaveLength(2);
      expect(current[0].path).toBe("/path/to/photo1.jpg");
      expect(current[0].name).toBe("photo1.jpg");
      expect(current[0].status).toBe("pending");
      expect(current[1].path).toBe("/path/to/photo2.png");
      expect(current[1].name).toBe("photo2.png");
    });

    it("skips duplicate file paths", () => {
      addFiles(["/path/to/photo1.jpg"]);
      addFiles(["/path/to/photo1.jpg", "/path/to/photo2.png"]);

      const current = get(files);
      expect(current).toHaveLength(2); // Still 2, not 3
      expect(current[0].path).toBe("/path/to/photo1.jpg");
      expect(current[1].path).toBe("/path/to/photo2.png");
    });

    it("extracts filename from path with forward slashes", () => {
      addFiles(["/Users/photos/vacation/sunset.jpg"]);

      const current = get(files);
      expect(current[0].name).toBe("sunset.jpg");
    });

    it("extracts filename from path with backslashes", () => {
      addFiles(["C:\\Users\\Photos\\vacation\\sunset.jpg"]);

      const current = get(files);
      expect(current[0].name).toBe("sunset.jpg");
    });
  });

  describe("removeFile", () => {
    it("removes a file by its path", () => {
      addFiles(["/path/to/photo1.jpg", "/path/to/photo2.png"]);
      removeFile("/path/to/photo1.jpg");

      const current = get(files);
      expect(current).toHaveLength(1);
      expect(current[0].path).toBe("/path/to/photo2.png");
    });

    it("does nothing when removing a non-existent path", () => {
      addFiles(["/path/to/photo1.jpg"]);
      removeFile("/path/to/nonexistent.jpg");

      const current = get(files);
      expect(current).toHaveLength(1);
    });
  });

  describe("clearFiles", () => {
    it("clears all files from the store", () => {
      addFiles(["/path/to/photo1.jpg", "/path/to/photo2.png"]);
      clearFiles();

      const current = get(files);
      expect(current).toHaveLength(0);
    });
  });

  describe("summary derived store", () => {
    it("returns zero counts for empty files", () => {
      const result = get(summary);
      expect(result.done).toBe(0);
      expect(result.failed).toBe(0);
      expect(result.pending).toBe(0);
      expect(result.savedBytes).toBe(0);
    });

    it("counts done, failed, and pending files correctly", () => {
      files.set([
        { path: "/a.jpg", name: "a.jpg", status: "pending", size: 100 },
        { path: "/b.jpg", name: "b.jpg", status: "compressing", size: 100 },
        { path: "/c.jpg", name: "c.jpg", status: "done", size: 100, compressedSize: 60 },
        { path: "/d.jpg", name: "d.jpg", status: "error", size: 100 },
      ]);

      const result = get(summary);
      expect(result.done).toBe(1);
      expect(result.failed).toBe(1);
      expect(result.pending).toBe(2); // pending + compressing
    });

    it("calculates savedBytes for done files", () => {
      files.set([
        { path: "/a.jpg", name: "a.jpg", status: "done", size: 100, compressedSize: 60 },
        { path: "/b.jpg", name: "b.jpg", status: "done", size: 200, compressedSize: 80 },
      ]);

      const result = get(summary);
      expect(result.savedBytes).toBe(160); // (100-60) + (200-80)
    });

    it("does not count negative savings (when compressed is larger)", () => {
      files.set([
        { path: "/a.jpg", name: "a.jpg", status: "done", size: 60, compressedSize: 100 },
      ]);

      const result = get(summary);
      // savedBytes is clamped to 0 — can't go negative
      expect(result.savedBytes).toBe(0);
    });
  });

  describe("qualityPreset persisted store", () => {
    it("defaults to Balanced", () => {
      const result = get(qualityPreset);
      expect(result).toBe("Balanced");
    });
  });

  describe("format persisted store", () => {
    it("defaults to null", () => {
      const result = get(format);
      expect(result).toBeNull();
    });
  });
});
