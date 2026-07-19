import { describe, expect, it, vi } from "vitest";
import { createTauriUpdateAdapter, type NativeUpdate } from "./tauriUpdateAdapter";

describe("Tauri update adapter", () => {
  it("maps metadata and accumulates chunk progress", async () => {
    const nativeUpdate: NativeUpdate = {
      version: "0.2.0",
      body: "Faster exports",
      download: async (onEvent) => {
        onEvent({ event: "Started", data: { contentLength: 100 } });
        onEvent({ event: "Progress", data: { chunkLength: 30 } });
        onEvent({ event: "Progress", data: { chunkLength: 20 } });
        onEvent({ event: "Finished" });
      },
      install: async () => {},
    };
    const adapter = createTauriUpdateAdapter(async () => nativeUpdate, vi.fn());
    const available = await adapter.check();
    const progress = vi.fn();

    expect(available).toEqual({ version: "0.2.0", notes: "Faster exports" });
    await adapter.downloadAndInstall(available!, progress);
    expect(progress).toHaveBeenLastCalledWith({ downloaded: 50, total: 100 });
  });

  it("installs the downloaded update before restarting", async () => {
    const restart = vi.fn(async () => {});
    const install = vi.fn(async () => {});
    const nativeUpdate: NativeUpdate = {
      version: "0.2.0",
      download: async () => {},
      install,
    };
    const adapter = createTauriUpdateAdapter(async () => nativeUpdate, restart);
    const available = await adapter.check();
    await adapter.downloadAndInstall(available!, vi.fn());
    await adapter.restart();

    expect(install).toHaveBeenCalledOnce();
    expect(restart).toHaveBeenCalledOnce();
  });

  it("retries relaunch without installing the update again", async () => {
    const install = vi.fn(async () => {});
    const restart = vi.fn().mockRejectedValueOnce(new Error("relaunch failed")).mockResolvedValueOnce(undefined);
    const nativeUpdate: NativeUpdate = {
      version: "0.2.0",
      download: async () => {},
      install,
    };
    const adapter = createTauriUpdateAdapter(async () => nativeUpdate, restart);
    const available = await adapter.check();
    await adapter.downloadAndInstall(available!, vi.fn());

    await expect(adapter.restart()).rejects.toThrow("relaunch failed");
    await adapter.restart();

    expect(install).toHaveBeenCalledOnce();
    expect(restart).toHaveBeenCalledTimes(2);
  });

  it("releases a previous native update before replacing it", async () => {
    const first: NativeUpdate = {
      version: "0.2.0",
      download: async () => {},
      install: async () => {},
      close: vi.fn(async () => {}),
    };
    const second: NativeUpdate = {
      version: "0.3.0",
      download: async () => {},
      install: async () => {},
      close: vi.fn(async () => {}),
    };
    const check = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second);
    const adapter = createTauriUpdateAdapter(check, vi.fn());

    await adapter.check();
    await adapter.check();

    expect(first.close).toHaveBeenCalledOnce();
  });
});
