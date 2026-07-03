import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  files,
  isProcessing,
  outputDir,
  summary,
  addFiles,
  removeFile,
  clearFiles,
  resolvedOutputDir,
  compressImages,
  mergeFiles,
  isMerging,
  mergeResult,
  mergeError,
  addMergeFiles,
  removeMergeFile,
  moveMergeFile,
  clearMerge,
  mergePdfs,
  splitFile,
  isSplitting,
  splitResults,
  splitError,
  setSplitFile,
  clearSplit,
  splitPdf,
} from "./pdf";
import { defaultOutputDir } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("pdf store", () => {
  beforeEach(() => {
    files.set([]);
    isProcessing.set(false);
    outputDir.set(null);
    defaultOutputDir.set(null);
    localStorage.clear();
    mockInvoke.mockReset();
    clearMerge();
    clearSplit();
  });

  describe("addFiles", () => {
    it("adds files with pending status", () => {
      addFiles(["/docs/report.pdf", "/docs/invoice.pdf"]);
      const curr = get(files);
      expect(curr).toHaveLength(2);
      expect(curr[0]).toMatchObject({ path: "/docs/report.pdf", name: "report.pdf", status: "pending" });
      expect(curr[1]).toMatchObject({ path: "/docs/invoice.pdf", name: "invoice.pdf", status: "pending" });
    });

    it("deduplicates on repeated calls", () => {
      addFiles(["/docs/report.pdf"]);
      addFiles(["/docs/report.pdf", "/docs/invoice.pdf"]);
      expect(get(files)).toHaveLength(2);
    });

    it("deduplicates within a single call", () => {
      addFiles(["/docs/a.pdf", "/docs/a.pdf"]);
      expect(get(files)).toHaveLength(1);
    });
  });

  describe("removeFile", () => {
    it("removes by path", () => {
      addFiles(["/docs/a.pdf", "/docs/b.pdf"]);
      removeFile("/docs/a.pdf");
      const curr = get(files);
      expect(curr).toHaveLength(1);
      expect(curr[0].path).toBe("/docs/b.pdf");
    });

    it("is a no-op for unknown path", () => {
      addFiles(["/docs/a.pdf"]);
      removeFile("/docs/nope.pdf");
      expect(get(files)).toHaveLength(1);
    });
  });

  describe("clearFiles", () => {
    it("empties files and resets isProcessing", () => {
      addFiles(["/docs/a.pdf"]);
      isProcessing.set(true);
      clearFiles();
      expect(get(files)).toHaveLength(0);
      expect(get(isProcessing)).toBe(false);
    });
  });

  describe("summary", () => {
    it("counts done, failed, and pending correctly", () => {
      files.set([
        { path: "/a.pdf", name: "a.pdf", status: "done" },
        { path: "/b.pdf", name: "b.pdf", status: "error" },
        { path: "/c.pdf", name: "c.pdf", status: "pending" },
        { path: "/d.pdf", name: "d.pdf", status: "processing" },
      ]);
      const s = get(summary);
      expect(s.done).toBe(1);
      expect(s.failed).toBe(1);
      expect(s.pending).toBe(2);
    });

    it("returns zeros for empty files", () => {
      const s = get(summary);
      expect(s).toEqual({ done: 0, failed: 0, pending: 0 });
    });
  });

  describe("compressImages", () => {
    it("defaults to false", () => {
      expect(get(compressImages)).toBe(false);
    });

    it("can be set to true", () => {
      compressImages.set(true);
      expect(get(compressImages)).toBe(true);
    });
  });

  describe("resolvedOutputDir", () => {
    it("falls back to defaultOutputDir when outputDir is null", () => {
      outputDir.set(null);
      defaultOutputDir.set("/default/output");
      expect(get(resolvedOutputDir)).toBe("/default/output");
    });

    it("uses local outputDir when set", () => {
      outputDir.set("/local/output");
      defaultOutputDir.set("/default/output");
      expect(get(resolvedOutputDir)).toBe("/local/output");
    });

    it("is null when both are null", () => {
      outputDir.set(null);
      defaultOutputDir.set(null);
      expect(get(resolvedOutputDir)).toBeNull();
    });
  });

  describe("addMergeFiles / removeMergeFile / moveMergeFile", () => {
    it("adds files preserving order and dedupes", () => {
      addMergeFiles(["/a.pdf", "/b.pdf"]);
      addMergeFiles(["/b.pdf", "/c.pdf"]);
      const curr = get(mergeFiles);
      expect(curr.map((f) => f.path)).toEqual(["/a.pdf", "/b.pdf", "/c.pdf"]);
    });

    it("removes a file by path", () => {
      addMergeFiles(["/a.pdf", "/b.pdf"]);
      removeMergeFile("/a.pdf");
      expect(get(mergeFiles).map((f) => f.path)).toEqual(["/b.pdf"]);
    });

    it("moves a file up and down without going out of bounds", () => {
      addMergeFiles(["/a.pdf", "/b.pdf", "/c.pdf"]);
      moveMergeFile("/b.pdf", -1);
      expect(get(mergeFiles).map((f) => f.path)).toEqual(["/b.pdf", "/a.pdf", "/c.pdf"]);

      moveMergeFile("/b.pdf", 1);
      expect(get(mergeFiles).map((f) => f.path)).toEqual(["/a.pdf", "/b.pdf", "/c.pdf"]);

      // no-ops at the boundaries
      moveMergeFile("/a.pdf", -1);
      moveMergeFile("/c.pdf", 1);
      expect(get(mergeFiles).map((f) => f.path)).toEqual(["/a.pdf", "/b.pdf", "/c.pdf"]);
    });
  });

  describe("mergePdfs", () => {
    it("success updates mergeResult and clears merging/error state", async () => {
      addMergeFiles(["/a.pdf", "/b.pdf"]);
      mockInvoke.mockResolvedValueOnce({
        output_path: "/out/merged.pdf",
        output_size: 2048,
        page_count: 7,
      });

      await mergePdfs("/out");

      expect(mockInvoke).toHaveBeenCalledWith("merge_pdfs", {
        paths: ["/a.pdf", "/b.pdf"],
        outputDir: "/out",
      });
      expect(get(mergeResult)).toEqual({ outputPath: "/out/merged.pdf", outputSize: 2048, pageCount: 7 });
      expect(get(isMerging)).toBe(false);
      expect(get(mergeError)).toBeNull();
    });

    it("error path sets mergeError and leaves mergeResult null", async () => {
      addMergeFiles(["/a.pdf", "/b.pdf"]);
      mockInvoke.mockRejectedValueOnce("merge failed: corrupt PDF");

      await mergePdfs(null);

      expect(get(mergeError)).toBe("merge failed: corrupt PDF");
      expect(get(mergeResult)).toBeNull();
      expect(get(isMerging)).toBe(false);
    });

    it("does nothing with fewer than 2 files", async () => {
      addMergeFiles(["/a.pdf"]);
      await mergePdfs(null);
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe("setSplitFile / clearSplit", () => {
    it("sets the split file and resets prior results/error", () => {
      setSplitFile("/docs/report.pdf");
      expect(get(splitFile)).toEqual({ path: "/docs/report.pdf", name: "report.pdf" });
      expect(get(splitResults)).toEqual([]);
      expect(get(splitError)).toBeNull();
    });

    it("clearSplit resets everything", () => {
      setSplitFile("/docs/report.pdf");
      splitResults.set(["/out/1.pdf"]);
      isSplitting.set(true);
      clearSplit();
      expect(get(splitFile)).toBeNull();
      expect(get(splitResults)).toEqual([]);
      expect(get(isSplitting)).toBe(false);
    });
  });

  describe("splitPdf", () => {
    it("range success returns the extracted file path", async () => {
      setSplitFile("/docs/report.pdf");
      mockInvoke.mockResolvedValueOnce(["/out/report_p3-5.pdf"]);

      await splitPdf("range", 3, 5, "/out");

      expect(mockInvoke).toHaveBeenCalledWith("split_pdf", {
        input: "/docs/report.pdf",
        mode: "range",
        start: 3,
        end: 5,
        outputDir: "/out",
      });
      expect(get(splitResults)).toEqual(["/out/report_p3-5.pdf"]);
      expect(get(splitError)).toBeNull();
      expect(get(isSplitting)).toBe(false);
    });

    it("each-page success returns one path per page", async () => {
      setSplitFile("/docs/report.pdf");
      mockInvoke.mockResolvedValueOnce(["/out/1.pdf", "/out/2.pdf", "/out/3.pdf"]);

      await splitPdf("each", undefined, undefined, null);

      expect(get(splitResults)).toHaveLength(3);
    });

    it("error path (out-of-bounds range) sets splitError", async () => {
      setSplitFile("/docs/report.pdf");
      mockInvoke.mockRejectedValueOnce("page range 3-9 out of bounds (document has 5 pages)");

      await splitPdf("range", 3, 9, null);

      expect(get(splitError)).toBe("page range 3-9 out of bounds (document has 5 pages)");
      expect(get(splitResults)).toEqual([]);
      expect(get(isSplitting)).toBe(false);
    });

    it("is a no-op when no split file is set", async () => {
      await splitPdf("each", undefined, undefined, null);
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });
});
