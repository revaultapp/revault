<script lang="ts">
  import { ArrowDownToLine, Clock3, RefreshCw, RotateCcw } from "lucide-svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { formatBytes } from "$lib/utils";
  import { t } from "$lib/stores/locale.svelte";
  import { updates } from "$lib/stores/updates";
  import Button from "./Button.svelte";
  import { progressPercent, shouldShowUpdateDialog } from "./updatePresentation";

  const updateStatus = updates.status;
  const pendingUpdate = updates.pendingUpdate;
  const updateProgress = updates.progress;
  const updateError = updates.error;
  const canShowDialog = updates.canShowDialog;

  let percent = $derived(progressPercent($updateProgress));
  let visible = $derived(
    shouldShowUpdateDialog($updateStatus, $canShowDialog, $pendingUpdate !== null),
  );
  let isDownloadError = $derived($updateStatus === "error" && $pendingUpdate !== null);
  let isRestartError = $derived($updateStatus === "readyToRestart" && $updateError !== null);

  function postpone() {
    updates.defer();
  }

  function openReleasePage() {
    void openUrl("https://github.com/revaultapp/revault/releases/latest");
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && $updateStatus === "available") postpone();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="visually-hidden" role="status" aria-live="polite">
  {#if $updateStatus === "available" && $canShowDialog && $pendingUpdate}
    {t("updates.availableAnnouncement", { version: $pendingUpdate.version })}
  {/if}
</div>

{#if visible && $pendingUpdate}
  <div
    class="update-dialog"
    role="dialog"
    aria-modal="false"
    aria-labelledby="update-dialog-title"
    aria-describedby="update-dialog-description"
  >
    <div class="dialog-icon" aria-hidden="true">
      {#if $updateStatus === "readyToRestart"}
        <RotateCcw size={20} strokeWidth={1.8} />
      {:else if isDownloadError}
        <RefreshCw size={20} strokeWidth={1.8} />
      {:else}
        <ArrowDownToLine size={20} strokeWidth={1.8} />
      {/if}
    </div>

    <div class="dialog-copy" aria-live="polite">
      <h2 id="update-dialog-title">
        {#if isRestartError}
          {t("updates.restartErrorTitle")}
        {:else if $updateStatus === "readyToRestart"}
          {t("updates.readyTitle")}
        {:else if isDownloadError}
          {t("updates.errorTitle")}
        {:else}
          {t("updates.availableTitle")}
        {/if}
      </h2>
      <p id="update-dialog-description">
        {#if isRestartError}
          {t("updates.restartErrorDescription")}
        {:else if $updateStatus === "readyToRestart"}
          {t("updates.readyDescription", { version: $pendingUpdate.version })}
        {:else if isDownloadError}
          {t("updates.errorDescription")}
        {:else}
          {t("updates.availableDescription", { version: $pendingUpdate.version })}
        {/if}
      </p>

      {#if $updateStatus === "available" && $pendingUpdate.notes}
        <p class="release-notes">{$pendingUpdate.notes}</p>
      {/if}

      {#if $updateStatus === "downloading"}
        <div class="progress-block">
          <div
            class="progress-track"
            class:progress-track--indeterminate={percent === null}
            role="progressbar"
            aria-label={t("updates.downloadProgressLabel")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent ?? undefined}
          >
            <span class="progress-fill" style:--progress={percent ?? 0}></span>
          </div>
          <span class="progress-copy">
            {#if $updateProgress.total > 0}
              {t("updates.downloadProgress", {
                downloaded: formatBytes($updateProgress.downloaded),
                total: formatBytes($updateProgress.total),
              })}
            {:else}
              {t("updates.downloading")}
            {/if}
          </span>
        </div>
      {/if}
    </div>

    <div class="dialog-actions">
      {#if $updateStatus === "available"}
        <Button variant="ghost" size="sm" onclick={postpone}>
          <Clock3 size={16} strokeWidth={2} />
          {t("updates.later")}
        </Button>
        <Button variant="primary" size="sm" onclick={updates.downloadAndInstall}>
          <ArrowDownToLine size={16} strokeWidth={2} />
          {t("updates.updateNow")}
        </Button>
      {:else if isDownloadError}
        <Button variant="ghost" size="sm" onclick={openReleasePage}>
          {t("updates.downloadManually")}
        </Button>
        <Button variant="primary" size="sm" onclick={updates.downloadAndInstall}>
          <RefreshCw size={16} strokeWidth={2} />
          {t("updates.tryAgain")}
        </Button>
      {:else if $updateStatus === "readyToRestart"}
        <Button variant="primary" size="sm" onclick={updates.restart}>
          <RotateCcw size={16} strokeWidth={2} />
          {isRestartError ? t("updates.tryAgain") : t("updates.restart")}
        </Button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-dialog {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 8;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 12px;
    width: min(400px, calc(100vw - 40px));
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    box-shadow: var(--shadow-lg);
    animation: dialog-in var(--duration-slow) var(--ease-out);
  }

  .dialog-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    color: var(--accent-text);
    background: var(--accent-subtle);
  }

  .dialog-copy {
    min-width: 0;
  }

  .dialog-copy h2 {
    font-size: 15px;
    font-weight: 700;
    line-height: 1.3;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .dialog-copy p {
    margin-top: 4px;
    font-size: 12px;
    color: var(--chart-tick);
  }

  .release-notes {
    display: -webkit-box;
    overflow: hidden;
    max-height: 60px;
    line-height: 1.6;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }

  .progress-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 12px;
  }

  .progress-track {
    position: relative;
    height: 4px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--navy-bg);
  }

  .progress-fill {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--accent);
    transform: scaleX(calc(var(--progress) / 100));
    transform-origin: left;
    transition: transform var(--duration-normal) var(--ease-out);
  }

  .progress-track--indeterminate .progress-fill {
    transform: translateX(-70%) scaleX(0.3);
    animation: progress-indeterminate 1s var(--ease-in-out) infinite;
  }

  .progress-copy {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--chart-tick);
  }

  .dialog-actions {
    display: flex;
    grid-column: 1 / -1;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @keyframes dialog-in {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  @keyframes progress-indeterminate {
    from { transform: translateX(-70%) scaleX(0.3); }
    to { transform: translateX(230%) scaleX(0.3); }
  }
</style>
