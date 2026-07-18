import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

describe("privacy store", () => {
  beforeEach(async () => {
    vi.resetModules();
    localStorage.clear();
  });

  it("adds unique pending files with filenames", async () => {
    const { addFiles, files } = await import("./privacy");

    addFiles(["/photos/a.jpg", "/photos/a.jpg", "C:\\photos\\b.png"]);

    expect(get(files)).toEqual([
      { path: "/photos/a.jpg", name: "a.jpg", status: "pending" },
      { path: "C:\\photos\\b.png", name: "b.png", status: "pending" },
    ]);
  });

  it("removes files and clears processing state", async () => {
    const { addFiles, removeFile, clearFiles, files, isProcessing } = await import("./privacy");

    addFiles(["/photos/a.jpg", "/photos/b.jpg"]);
    removeFile("/photos/a.jpg");
    isProcessing.set(true);
    clearFiles();

    expect(get(files)).toEqual([]);
    expect(get(isProcessing)).toBe(false);
  });

  it("summarizes scanned, stripped, and failed files", async () => {
    const { files, summary } = await import("./privacy");

    files.set([
      { path: "/a.jpg", name: "a.jpg", status: "pending" },
      { path: "/b.jpg", name: "b.jpg", status: "scanned" },
      { path: "/c.jpg", name: "c.jpg", status: "done" },
      { path: "/d.jpg", name: "d.jpg", status: "error" },
    ]);

    expect(get(summary)).toEqual({ scanned: 2, stripped: 1, failed: 1 });
  });

  it("per-tool outputDir never writes back to the Settings default", async () => {
    const { defaultOutputDir } = await import("./settings");
    const { outputDir } = await import("./privacy");
    outputDir.set("/tool");
    expect(get(defaultOutputDir)).toBeNull();
    defaultOutputDir.set("/global");
    defaultOutputDir.set(null);
    expect(get(outputDir)).toBe("/tool");
  });

  it("persists outputDir", async () => {
    const { outputDir } = await import("./privacy");

    outputDir.set("/exports");

    expect(localStorage.getItem("revault-privacy-outputDir")).toBe(JSON.stringify("/exports"));
  });

  describe("resolvedOutputDir derived", () => {
    it("prefers the local outputDir over the default", async () => {
      const { outputDir, resolvedOutputDir } = await import("./privacy");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set("/default/output");
      outputDir.set("/custom/output");

      expect(get(resolvedOutputDir)).toBe("/custom/output");
    });

    it("falls back to defaultOutputDir when outputDir is null", async () => {
      const { outputDir, resolvedOutputDir } = await import("./privacy");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set("/default/output");
      outputDir.set(null);

      expect(get(resolvedOutputDir)).toBe("/default/output");
    });

    it("is null when neither outputDir nor defaultOutputDir is set", async () => {
      const { outputDir, resolvedOutputDir } = await import("./privacy");
      const { defaultOutputDir } = await import("./settings");

      defaultOutputDir.set(null);
      outputDir.set(null);

      expect(get(resolvedOutputDir)).toBeNull();
    });
  });
});
