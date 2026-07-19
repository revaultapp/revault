import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { UpdateOperationError, type AvailableUpdate, type UpdateAdapter } from "$lib/stores/updates";

export interface NativeUpdate {
  version: string;
  body?: string;
  download(onEvent: (event: DownloadEvent) => void): Promise<void>;
  install(): Promise<void>;
  close?(): Promise<void>;
}

export function createTauriUpdateAdapter(
  checkNative: () => Promise<NativeUpdate | null>,
  restartNative: () => Promise<void>,
): UpdateAdapter {
  let candidate: NativeUpdate | null = null;
  let installed = false;

  return {
    async check() {
      const nextCandidate = await checkNative();
      const previousCandidate = candidate;
      candidate = nextCandidate;
      installed = false;
      await previousCandidate?.close?.();
      if (!nextCandidate) return null;
      return {
        version: nextCandidate.version,
        notes: nextCandidate.body?.trim() || undefined,
      };
    },

    async download(update: AvailableUpdate, onProgress) {
      if (!candidate || candidate.version !== update.version) {
        throw new Error("The selected update is no longer available.");
      }

      let downloaded = 0;
      let total = 0;
      const selectedCandidate = candidate;
      await selectedCandidate.download((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          onProgress({ downloaded, total });
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          onProgress({ downloaded, total });
        }
      });
    },

    async restart() {
      if (installed) {
        await restartNative();
        return;
      }
      if (!candidate) throw new Error("The downloaded update is no longer available.");
      const selectedCandidate = candidate;
      try {
        await selectedCandidate.install();
      } catch (cause) {
        throw new UpdateOperationError("install", cause);
      }
      installed = true;
      if (candidate === selectedCandidate) {
        candidate = null;
        try {
          await selectedCandidate.close?.();
        } catch {
          // The update is already installed; retrying must still relaunch it.
        }
      }
      try {
        await restartNative();
      } catch (cause) {
        throw new UpdateOperationError("restart", cause);
      }
    },
  };
}

export const tauriUpdateAdapter = createTauriUpdateAdapter(check, relaunch);
