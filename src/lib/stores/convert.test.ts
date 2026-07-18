import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("convert store", () => {
  let store: typeof import("./convert");

  beforeEach(async () => {
    vi.resetModules();
    mockInvoke.mockReset();
    localStorage.clear();
    store = await import("./convert");
    store.clearFiles();
    store.targetFormat.set("Jpeg");
    store.outputDir.set(null);
    store.heicBannerDismissed.set(false);
  });

  describe("addFiles", () => {
    it("adds new files as pending with parsed name and format", async () => {
      mockInvoke.mockResolvedValueOnce([2048]);
      await store.addFiles(["/photos/shot.jpg"]);
      const [f] = get(store.files);
      expect(f.name).toBe("shot.jpg");
      expect(f.status).toBe("pending");
      expect(f.sourceFormat).toBe("JPG");
      expect(f.size).toBe(2048);
    });

    it("skips duplicate paths", async () => {
      mockInvoke.mockResolvedValue([500]);
      await store.addFiles(["/a.png"]);
      await store.addFiles(["/a.png", "/b.png"]);
      expect(get(store.files)).toHaveLength(2);
    });

    it("resets heicBannerDismissed when a HEIC file is added", async () => {
      mockInvoke.mockResolvedValueOnce([1000]);
      store.heicBannerDismissed.set(true);
      await store.addFiles(["/photos/IMG_0001.HEIC"]);
      expect(get(store.heicBannerDismissed)).toBe(false);
    });

    it("does not override user-selected targetFormat", async () => {
      mockInvoke.mockResolvedValueOnce([1000]);
      store.targetFormat.set("Webp");
      await store.addFiles(["/photos/IMG_0001.HEIC"]);
      expect(get(store.targetFormat)).toBe("Webp");
    });
  });

  describe("removeFile", () => {
    it("removes by path", async () => {
      mockInvoke.mockResolvedValue([100, 100]);
      await store.addFiles(["/a.jpg", "/b.jpg"]);
      store.removeFile("/a.jpg");
      const current = get(store.files);
      expect(current).toHaveLength(1);
      expect(current[0].path).toBe("/b.jpg");
    });
  });

  describe("clearFiles", () => {
    it("empties files and resets derived flags", async () => {
      mockInvoke.mockResolvedValueOnce([100]);
      await store.addFiles(["/a.jpg"]);
      store.clearFiles();
      expect(get(store.files)).toHaveLength(0);
      expect(get(store.isConverting)).toBe(false);
    });
  });

  describe("hasHeicFiles derived", () => {
    it("is true only when a .heic file is present", async () => {
      mockInvoke.mockResolvedValue([100, 100]);
      await store.addFiles(["/photo.jpg"]);
      expect(get(store.hasHeicFiles)).toBe(false);
      await store.addFiles(["/shot.heic"]);
      expect(get(store.hasHeicFiles)).toBe(true);
      store.removeFile("/shot.heic");
      expect(get(store.hasHeicFiles)).toBe(false);
    });
  });

  describe("summary derived", () => {
    it("counts done, failed, and pending correctly", () => {
      store.files.set([
        { path: "/a.jpg", name: "a.jpg", size: 100, status: "pending", sourceFormat: "JPG" },
        { path: "/b.jpg", name: "b.jpg", size: 100, status: "converting", sourceFormat: "JPG" },
        { path: "/c.jpg", name: "c.jpg", size: 200, status: "done", sourceFormat: "JPG", outputSize: 80 },
        { path: "/d.jpg", name: "d.jpg", size: 100, status: "error", sourceFormat: "JPG" },
      ]);
      const s = get(store.summary);
      expect(s.done).toBe(1);
      expect(s.failed).toBe(1);
      expect(s.pending).toBe(2);
      expect(s.savedBytes).toBe(120); // 200-80
    });

    it("clamps savedBytes to 0 when output is larger than input", () => {
      store.files.set([
        { path: "/a.jpg", name: "a.jpg", size: 50, status: "done", sourceFormat: "JPG", outputSize: 100 },
      ]);
      expect(get(store.summary).savedBytes).toBe(0);
    });
  });

  describe("resolvedOutputDir derived", () => {
    it("falls back to defaultOutputDir when outputDir is null", async () => {
      const { defaultOutputDir } = await import("./settings");
      defaultOutputDir.set("/default/output");
      store.outputDir.set(null);
      expect(get(store.resolvedOutputDir)).toBe("/default/output");
    });

    it("prefers the local outputDir over default", async () => {
      const { defaultOutputDir } = await import("./settings");
      defaultOutputDir.set("/default/output");
      store.outputDir.set("/custom/output");
      expect(get(store.resolvedOutputDir)).toBe("/custom/output");
    });

    it("per-tool outputDir never writes back to the Settings default", async () => {
      const { defaultOutputDir } = await import("./settings");
      defaultOutputDir.set(null);
      store.outputDir.set("/tool");
      expect(get(defaultOutputDir)).toBeNull();
      defaultOutputDir.set("/global");
      defaultOutputDir.set(null);
      expect(get(store.outputDir)).toBe("/tool");
    });
  });
});
