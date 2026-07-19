import type {
  UpdateErrorOperation,
  UpdateProgress,
  UpdateStatus,
} from "$lib/stores/updates";

export function shouldShowUpdateDialog(
  status: UpdateStatus,
  canShowOffer: boolean,
  hasUpdate: boolean,
  errorOperation: UpdateErrorOperation | null = null,
): boolean {
  if (canShowOffer) return true;
  if (!hasUpdate) return false;
  if (["downloading", "installing", "readyToRestart"].includes(status)) return true;
  return status === "error" && ["download", "install"].includes(errorOperation ?? "");
}

export function progressPercent(progress: UpdateProgress): number | null {
  if (progress.total <= 0) return null;
  return Math.min(100, Math.max(0, Math.round((progress.downloaded / progress.total) * 100)));
}
