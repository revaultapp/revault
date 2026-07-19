import { derived, get, writable, type Readable } from "svelte/store";
import { isCompressing as isImageCompressing, isEstimating } from "./compress";
import { isConverting } from "./convert";
import { isScanning as isDuplicateScanRunning } from "./dedupe";
import {
  isBuildingPdf,
  isMerging,
  isProcessing as isPdfProcessing,
  isRasterizing,
  isSplitting,
} from "./pdf";
import { isProcessing as isPrivacyProcessing } from "./privacy";
import { isResizing } from "./resize";
import { storage } from "./storage";
import { audioState, gifState, isCompressing as isVideoCompressing, trimState } from "./video";

const DEFERRAL_KEY = "revault_update_deferral";
const DEFERRAL_DURATION = 24 * 60 * 60 * 1000;

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
  | "readyToRestart"
  | "error"
  | "upToDate";

export interface AvailableUpdate {
  version: string;
  notes?: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
}

export type UpdateErrorOperation = "check" | "download" | "install" | "restart";

export class UpdateOperationError extends Error {
  constructor(public readonly operation: "install" | "restart", cause: unknown) {
    super(cause instanceof Error ? cause.message : typeof cause === "string" ? cause : "");
    this.name = "UpdateOperationError";
  }
}

export interface UpdateAdapter {
  check(): Promise<AvailableUpdate | null>;
  download(
    update: AvailableUpdate,
    onProgress: (progress: UpdateProgress) => void,
  ): Promise<void>;
  restart(): Promise<void>;
}

interface DeferredUpdate {
  version: string;
  timestamp: number;
}

interface UpdateStoreOptions {
  adapter: UpdateAdapter;
  isProcessing: Readable<boolean>;
  now?: () => number;
}

function readDeferral(): DeferredUpdate | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(DEFERRAL_KEY) ?? "null");
    if (
      typeof value === "object" &&
      value !== null &&
      "version" in value &&
      "timestamp" in value &&
      typeof value.version === "string" &&
      typeof value.timestamp === "number"
    ) {
      return value as DeferredUpdate;
    }
  } catch {
    // Ignore unavailable or malformed persisted state.
  }
  return null;
}

function clearDeferral(): void {
  try {
    localStorage.removeItem(DEFERRAL_KEY);
  } catch {
    // Deferral persistence must not block update discovery.
  }
}

function saveDeferral(deferral: DeferredUpdate): void {
  try {
    localStorage.setItem(DEFERRAL_KEY, JSON.stringify(deferral));
  } catch {
    // Deferral persistence must not block the update UI.
  }
}

interface SemVer {
  core: number[];
  prerelease: string[] | null;
}

function parseSemVer(version: string): SemVer | null {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.exec(version);
  if (!match) return null;
  return {
    core: [Number(match[1]), Number(match[2]), Number(match[3])],
    prerelease: match[4]?.split(".") ?? null,
  };
}

function isNewerVersion(next: string, current: string): boolean {
  const nextVersion = parseSemVer(next);
  const currentVersion = parseSemVer(current);
  if (!nextVersion || !currentVersion) return false;

  for (let index = 0; index < nextVersion.core.length; index += 1) {
    const difference = nextVersion.core[index] - currentVersion.core[index];
    if (difference !== 0) return difference > 0;
  }
  if (nextVersion.prerelease === null) return currentVersion.prerelease !== null;
  if (currentVersion.prerelease === null) return false;
  for (let index = 0; index < Math.max(nextVersion.prerelease.length, currentVersion.prerelease.length); index += 1) {
    const nextPart = nextVersion.prerelease[index];
    const currentPart = currentVersion.prerelease[index];
    if (nextPart === undefined) return false;
    if (currentPart === undefined) return true;
    if (nextPart === currentPart) continue;
    const nextNumber = /^\d+$/.test(nextPart);
    const currentNumber = /^\d+$/.test(currentPart);
    if (nextNumber && currentNumber) return Number(nextPart) > Number(currentPart);
    if (nextNumber !== currentNumber) return !nextNumber;
    return nextPart > currentPart;
  }
  return false;
}

function isDeferred(version: string, now: () => number): boolean {
  const deferred = readDeferral();
  return deferred?.version === version && now() - deferred.timestamp < DEFERRAL_DURATION;
}

export function createUpdateStore({ adapter: initialAdapter, isProcessing, now = Date.now }: UpdateStoreOptions) {
  let adapter = initialAdapter;
  let checkTask: Promise<void> | null = null;
  let downloadTask: Promise<void> | null = null;
  let restartTask: Promise<void> | null = null;
  let restarted = false;
  const status = writable<UpdateStatus>("idle");
  const pendingUpdate = writable<AvailableUpdate | null>(null);
  const progress = writable<UpdateProgress>({ downloaded: 0, total: 0 });
  const error = writable<string | null>(null);
  const errorOperation = writable<UpdateErrorOperation | null>(null);
  const canShowDialog = derived(
    [status, pendingUpdate, isProcessing],
    ([$status, $update, $isProcessing]) =>
      $status === "available" &&
      $update !== null &&
      !$isProcessing &&
      !isDeferred($update.version, now),
  );

  function errorMessage(operation: UpdateErrorOperation, cause: unknown): string {
    const prefix = {
      check: "Unable to check for updates.",
      download: "Unable to download the update.",
      install: "Unable to install the update.",
      restart: "Unable to restart ReVault.",
    }[operation];
    const detail = cause instanceof Error ? cause.message : typeof cause === "string" ? cause : "";
    return detail ? `${prefix} ${detail}` : prefix;
  }

  function checkForUpdates(): Promise<void> {
    if (checkTask) return checkTask;
    if (get(isProcessing)) return Promise.resolve();
    if (["downloading", "installing", "readyToRestart"].includes(get(status))) {
      return Promise.resolve();
    }
    status.set("checking");
    error.set(null);
    errorOperation.set(null);
    let task: Promise<void>;
    try {
      task = adapter.check().then((update) => {
        pendingUpdate.set(update);
        if (update) {
          const deferred = readDeferral();
          if (deferred && isNewerVersion(update.version, deferred.version)) {
            clearDeferral();
          }
          status.set("available");
        } else {
          status.set("upToDate");
        }
      }).catch((cause) => {
        status.set("error");
        errorOperation.set("check");
        error.set(errorMessage("check", cause));
      });
    } catch (cause) {
      status.set("error");
      errorOperation.set("check");
      error.set(errorMessage("check", cause));
      return Promise.resolve();
    }
    checkTask = task;
    void task.finally(() => { if (checkTask === task) checkTask = null; });
    return task;
  }

  function download(): Promise<void> {
    if (downloadTask) return downloadTask;
    if (get(isProcessing)) return Promise.resolve();
    const currentStatus = get(status);
    const canRetryDownload = currentStatus === "error" && get(errorOperation) === "download";
    if (currentStatus !== "available" && !canRetryDownload) return Promise.resolve();
    const update = get(pendingUpdate);
    if (!update) return Promise.resolve();

    status.set("downloading");
    error.set(null);
    errorOperation.set(null);
    progress.set({ downloaded: 0, total: 0 });
    let task: Promise<void>;
    try {
      task = adapter.download(update, progress.set).then(() => {
        status.set("readyToRestart");
      }).catch((cause) => {
        status.set("error");
        errorOperation.set("download");
        error.set(errorMessage("download", cause));
      });
    } catch (cause) {
      status.set("error");
      errorOperation.set("download");
      error.set(errorMessage("download", cause));
      return Promise.resolve();
    }
    downloadTask = task;
    void task.finally(() => { if (downloadTask === task) downloadTask = null; });
    return task;
  }

  function restart(): Promise<void> {
    if (restartTask) return restartTask;
    if (get(isProcessing)) return Promise.resolve();
    const currentStatus = get(status);
    const canRetryInstall = currentStatus === "error" && get(errorOperation) === "install";
    if (restarted || (currentStatus !== "readyToRestart" && !canRetryInstall)) return Promise.resolve();
    restarted = true;
    status.set("installing");
    error.set(null);
    errorOperation.set(null);
    let task: Promise<void>;
    try {
      task = adapter.restart().catch((cause) => {
        restarted = false;
        const operation = cause instanceof UpdateOperationError
          ? cause.operation
          : typeof cause === "object" && cause !== null && "operation" in cause && cause.operation === "install"
            ? "install"
            : "restart";
        status.set(operation === "install" ? "error" : "readyToRestart");
        errorOperation.set(operation);
        error.set(errorMessage(operation, cause));
      });
    } catch (cause) {
      restarted = false;
      status.set("readyToRestart");
      errorOperation.set("restart");
      error.set(errorMessage("restart", cause));
      return Promise.resolve();
    }
    restartTask = task;
    void task.finally(() => { if (restartTask === task) restartTask = null; });
    return task;
  }

  function defer(): void {
    const update = get(pendingUpdate);
    if (!update) return;
    saveDeferral({ version: update.version, timestamp: now() });
    pendingUpdate.set({ ...update });
  }

  return {
    status,
    pendingUpdate,
    progress,
    error,
    errorOperation,
    canShowDialog,
    checkForUpdates,
    manualCheck: checkForUpdates,
    download,
    restart,
    defer,
    setAdapter: (next: UpdateAdapter) => { adapter = next; },
  };
}

const unavailableAdapter: UpdateAdapter = {
  check: async () => { throw new Error("Updater is not configured"); },
  download: async () => { throw new Error("Updater is not configured"); },
  restart: async () => { throw new Error("Updater is not configured"); },
};

export const isUpdateProcessing = derived(
  [
    isImageCompressing,
    isEstimating,
    isConverting,
    isResizing,
    isDuplicateScanRunning,
    isPrivacyProcessing,
    isPdfProcessing,
    isMerging,
    isBuildingPdf,
    isSplitting,
    isRasterizing,
    isVideoCompressing,
    gifState,
    audioState,
    trimState,
    storage,
  ],
  ([
    imageCompressing,
    estimatingImage,
    converting,
    resizing,
    scanningDuplicates,
    processingPrivacy,
    processingPdf,
    mergingPdf,
    buildingPdf,
    splittingPdf,
    rasterizingPdf,
    compressingVideo,
    currentGifState,
    currentAudioState,
    currentTrimState,
    currentStorage,
  ]) =>
    imageCompressing ||
    estimatingImage ||
    converting ||
    resizing ||
    scanningDuplicates ||
    processingPrivacy ||
    processingPdf ||
    mergingPdf ||
    buildingPdf ||
    splittingPdf ||
    rasterizingPdf ||
    compressingVideo ||
    currentGifState === "generating" ||
    currentAudioState === "extracting" ||
    currentTrimState === "trimming" ||
    currentStorage.scanState === "scanning",
);
export const updates = createUpdateStore({ adapter: unavailableAdapter, isProcessing: isUpdateProcessing });
