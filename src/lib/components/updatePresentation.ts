import type { UpdateProgress, UpdateStatus } from "$lib/stores/updates";

export function shouldShowUpdateDialog(
  status: UpdateStatus,
  canShowOffer: boolean,
  hasUpdate: boolean,
): boolean {
  if (canShowOffer) return true;
  return hasUpdate && ["downloading", "readyToRestart", "error"].includes(status);
}

export function progressPercent(progress: UpdateProgress): number | null {
  if (progress.total <= 0) return null;
  return Math.min(100, Math.max(0, Math.round((progress.downloaded / progress.total) * 100)));
}
