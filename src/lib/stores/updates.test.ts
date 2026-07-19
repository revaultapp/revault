import { beforeEach, describe, expect, it, vi } from "vitest";
import { get, writable } from "svelte/store";
import { createUpdateStore, type UpdateAdapter } from "./updates";

const DAY = 24 * 60 * 60 * 1000;

function setup(update: { version: string; notes?: string } | null = { version: "1.1.0" }) {
  let now = 1_000;
  const isProcessing = writable(false);
  const adapter: UpdateAdapter = {
    check: async () => update,
    downloadAndInstall: async (_update, onProgress) => onProgress({ downloaded: 100, total: 100 }),
    restart: async () => {},
  };

  return {
    store: createUpdateStore({ adapter, isProcessing, now: () => now }),
    isProcessing,
    setNow: (value: number) => { now = value; },
  };
}

function deferred<T>() {
  let resolve: (value: T) => void;
  const promise = new Promise<T>((next) => { resolve = next; });
  return { promise, resolve: resolve! };
}

describe("updates store", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("starts idle", () => {
    expect(get(setup().store.status)).toBe("idle");
  });

  it("is checking until the adapter resolves", async () => {
    const pending = deferred<{ version: string } | null>();
    const { store } = setup();
    store.setAdapter({
      check: () => pending.promise,
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    const check = store.checkForUpdates();
    expect(get(store.status)).toBe("checking");
    pending.resolve({ version: "1.1.0" });
    await check;

    expect(get(store.status)).toBe("available");
  });

  it("records a check error", async () => {
    const { store } = setup();
    store.setAdapter({
      check: async () => { throw new Error("offline"); },
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    await store.checkForUpdates();

    expect(get(store.status)).toBe("error");
    expect(get(store.error)).toBe("Unable to check for updates. offline");
  });

  it("shares an in-flight update check", async () => {
    const pending = deferred<{ version: string } | null>();
    const { store } = setup();
    let checks = 0;
    store.setAdapter({
      check: () => {
        checks += 1;
        return pending.promise;
      },
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    const first = store.checkForUpdates();
    const second = store.checkForUpdates();

    expect(second).toBe(first);
    expect(checks).toBe(1);
    pending.resolve({ version: "1.1.0" });
    await first;
    expect(get(store.status)).toBe("available");
  });

  it("does not allow the dialog for the same deferred version within 24 hours", async () => {
    localStorage.clear();
    const { store, setNow } = setup();

    await store.checkForUpdates();
    store.defer();
    setNow(1_000 + DAY - 1);
    await store.checkForUpdates();

    expect(get(store.canShowDialog)).toBe(false);
    expect(localStorage.getItem("revault_update_deferral")).toBe(
      JSON.stringify({ version: "1.1.0", timestamp: 1_000 }),
    );
  });

  it("allows a newer version immediately after deferring an older version", async () => {
    localStorage.clear();
    const { store } = setup({ version: "1.1.0" });

    await store.checkForUpdates();
    store.defer();
    store.setAdapter({
      check: async () => ({ version: "1.2.0" }),
      downloadAndInstall: async () => {},
      restart: async () => {},
    });
    await store.checkForUpdates();

    expect(get(store.canShowDialog)).toBe(true);
    expect(localStorage.getItem("revault_update_deferral")).toBeNull();
  });

  it("treats a later prerelease as newer", async () => {
    const { store } = setup({ version: "1.0.0-beta.1" });
    await store.checkForUpdates();
    store.defer();
    store.setAdapter({
      check: async () => ({ version: "1.0.0-beta.2+build.9" }),
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    await store.checkForUpdates();

    expect(get(store.canShowDialog)).toBe(true);
    expect(localStorage.getItem("revault_update_deferral")).toBeNull();
  });

  it("treats a stable release as newer than its prerelease", async () => {
    const { store } = setup({ version: "1.0.0-rc.1" });
    await store.checkForUpdates();
    store.defer();
    store.setAdapter({
      check: async () => ({ version: "1.0.0" }),
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    await store.checkForUpdates();

    expect(get(store.canShowDialog)).toBe(true);
    expect(localStorage.getItem("revault_update_deferral")).toBeNull();
  });

  it("only allows the dialog when an update is available and processing is idle", async () => {
    localStorage.clear();
    const { store, isProcessing } = setup();

    isProcessing.set(true);
    await store.checkForUpdates();
    expect(get(store.canShowDialog)).toBe(false);

    isProcessing.set(false);
    expect(get(store.canShowDialog)).toBe(true);
  });

  it("moves through manual check, download, and restart-ready states", async () => {
    localStorage.clear();
    const { store } = setup();

    await store.manualCheck();
    expect(get(store.status)).toBe("available");

    await store.downloadAndInstall();
    expect(get(store.status)).toBe("readyToRestart");
    expect(get(store.progress)).toEqual({ downloaded: 100, total: 100 });
  });

  it("is downloading until installation completes", async () => {
    const pending = deferred<void>();
    const { store } = setup();
    await store.checkForUpdates();
    store.setAdapter({
      check: async () => ({ version: "1.1.0" }),
      downloadAndInstall: async () => pending.promise,
      restart: async () => {},
    });

    const download = store.downloadAndInstall();
    expect(get(store.status)).toBe("downloading");
    pending.resolve();
    await download;

    expect(get(store.status)).toBe("readyToRestart");
  });

  it("records a download error", async () => {
    const { store } = setup();
    await store.checkForUpdates();
    store.setAdapter({
      check: async () => ({ version: "1.1.0" }),
      downloadAndInstall: async () => { throw new Error("disk full"); },
      restart: async () => {},
    });

    await store.downloadAndInstall();

    expect(get(store.status)).toBe("error");
    expect(get(store.error)).toBe("Unable to download the update. disk full");
  });

  it("shares an in-flight download and blocks checks until it completes", async () => {
    const pending = deferred<void>();
    const { store } = setup();
    let downloads = 0;
    let checks = 0;
    store.setAdapter({
      check: async () => {
        checks += 1;
        return { version: "1.1.0" };
      },
      downloadAndInstall: async () => {
        downloads += 1;
        return pending.promise;
      },
      restart: async () => {},
    });
    await store.checkForUpdates();

    const first = store.downloadAndInstall();
    const second = store.downloadAndInstall();
    await store.checkForUpdates();

    expect(second).toBe(first);
    expect(downloads).toBe(1);
    expect(checks).toBe(1);
    expect(get(store.status)).toBe("downloading");
    pending.resolve();
    await first;

    await store.checkForUpdates();
    expect(checks).toBe(1);
    expect(get(store.status)).toBe("readyToRestart");
  });

  it("records a restart error", async () => {
    const { store } = setup();
    await store.checkForUpdates();
    await store.downloadAndInstall();
    store.setAdapter({
      check: async () => ({ version: "1.1.0" }),
      downloadAndInstall: async () => {},
      restart: async () => { throw new Error("restart denied"); },
    });

    await store.restart();

    expect(get(store.status)).toBe("readyToRestart");
    expect(get(store.error)).toBe("Unable to restart ReVault. restart denied");
  });

  it("retries restart after failure without downloading again", async () => {
    const { store } = setup();
    let downloads = 0;
    let restarts = 0;
    store.setAdapter({
      check: async () => ({ version: "1.1.0" }),
      downloadAndInstall: async () => { downloads += 1; },
      restart: async () => {
        restarts += 1;
        if (restarts === 1) throw new Error("restart denied");
      },
    });
    await store.checkForUpdates();
    await store.downloadAndInstall();

    await store.restart();
    expect(get(store.status)).toBe("readyToRestart");
    await store.downloadAndInstall();
    await store.restart();

    expect(downloads).toBe(1);
    expect(restarts).toBe(2);
    expect(get(store.error)).toBeNull();
  });

  it("shares an in-flight restart", async () => {
    const pending = deferred<void>();
    const { store } = setup();
    await store.checkForUpdates();
    await store.downloadAndInstall();
    let restarts = 0;
    store.setAdapter({
      check: async () => ({ version: "1.1.0" }),
      downloadAndInstall: async () => {},
      restart: async () => {
        restarts += 1;
        return pending.promise;
      },
    });

    const first = store.restart();
    const second = store.restart();

    expect(second).toBe(first);
    expect(restarts).toBe(1);
    pending.resolve();
    await first;
  });

  it("does not expose object errors", async () => {
    const { store } = setup();
    store.setAdapter({
      check: async () => { throw { code: "OFFLINE" }; },
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    await store.checkForUpdates();

    expect(get(store.error)).toBe("Unable to check for updates.");
  });

  it("shows upToDate after a manual check without an available update", async () => {
    localStorage.clear();
    const { store } = setup(null);

    await store.manualCheck();

    expect(get(store.status)).toBe("upToDate");
  });

  it("keeps a newer update available when clearing an old deferral fails", async () => {
    const { store } = setup({ version: "1.1.0" });
    await store.checkForUpdates();
    store.defer();
    const removeItem = vi.spyOn(localStorage, "removeItem").mockImplementation(() => {
      throw new Error("storage unavailable");
    });
    store.setAdapter({
      check: async () => ({ version: "1.2.0" }),
      downloadAndInstall: async () => {},
      restart: async () => {},
    });

    await store.checkForUpdates();
    removeItem.mockRestore();

    expect(get(store.status)).toBe("available");
  });

  it("keeps an available update visible when saving a deferral fails", async () => {
    const { store } = setup();
    await store.checkForUpdates();
    const setItem = vi.spyOn(localStorage, "setItem").mockImplementation(() => {
      throw new Error("storage unavailable");
    });

    expect(() => store.defer()).not.toThrow();
    setItem.mockRestore();

    expect(get(store.canShowDialog)).toBe(true);
  });
});
