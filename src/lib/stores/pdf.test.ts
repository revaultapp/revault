import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { vi } from "vitest";
import {
  files,
  isProcessing,
  outputDir,
  summary,
  addFiles,
  removeFile,
  clearFiles,
  resolvedOutputDir,
} from "./pdf";
import { defaultOutputDir } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("pdf store", () => {
  beforeEach(() => {
    files.set([]);
    isProcessing.set(false);
    outputDir.set(null);
    defaultOutputDir.set(null);
    localStorage.clear();
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
});
