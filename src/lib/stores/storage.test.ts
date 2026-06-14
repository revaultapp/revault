import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

describe("storage store", () => {
  beforeEach(() => {
    vi.resetModules();
    mockInvoke.mockReset();
    mockOpen.mockReset();
  });

  it("does nothing when folder selection is canceled", async () => {
    mockOpen.mockResolvedValueOnce(null);
    const { storage } = await import("./storage");

    await storage.scanFolder();

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(get(storage).scanState).toBe("idle");
  });

  it("scans a selected folder", async () => {
    mockOpen.mockResolvedValueOnce("/photos");
    mockInvoke.mockResolvedValueOnce({ images: [], total_size: 0, skipped: 0 });
    const { storage } = await import("./storage");

    await storage.scanFolder();

    expect(mockInvoke).toHaveBeenCalledWith("scan_folder", { path: "/photos", recursive: true });
    expect(get(storage)).toEqual({
      scanState: "done",
      scanResult: { images: [], total_size: 0, skipped: 0 },
      errorMessage: null,
      folderPath: "/photos",
    });
  });

  it("records scan errors", async () => {
    mockOpen.mockResolvedValueOnce("/photos");
    mockInvoke.mockRejectedValueOnce(new Error("scan failed"));
    const { storage } = await import("./storage");

    await storage.scanFolder();

    expect(get(storage).scanState).toBe("error");
    expect(get(storage).errorMessage).toBe("Error: scan failed");
  });

  it("derives extension breakdown sorted by total size", async () => {
    mockOpen.mockResolvedValueOnce("/photos");
    mockInvoke.mockResolvedValueOnce({
      images: [
        { path: "/a.jpg", relative_path: "a.jpg", size: 100, extension: "jpg" },
        { path: "/b.png", relative_path: "b.png", size: 300, extension: "png" },
        { path: "/c.jpg", relative_path: "c.jpg", size: 100, extension: "jpg" },
      ],
      total_size: 500,
      skipped: 0,
    });
    const { storage, breakdown } = await import("./storage");

    await storage.scanFolder();

    expect(get(breakdown)).toEqual([
      { extension: "PNG", count: 1, totalSize: 300, percentage: 60 },
      { extension: "JPG", count: 2, totalSize: 200, percentage: 40 },
    ]);
  });
});
