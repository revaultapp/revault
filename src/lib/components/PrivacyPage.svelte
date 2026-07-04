<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleCheck, CircleAlert, X, FolderOpen } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import PrivacyToast from "./PrivacyToast.svelte";
  import { runWithConcurrency, browseOutputDir } from "$lib/utils";
  import { activity } from "$lib/stores/activity";
  import {
    files, isProcessing, summary, outputDir,
    stripGps, stripDevice, stripDatetime, stripAuthor,
    addFiles, removeFile, clearFiles,
    type PrivacyFile,
  } from "$lib/stores/privacy";
  import { t } from "$lib/stores/locale.svelte";

  const PRIVACY_SUPPORTED_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "heic", "heif"] as const;
  const PRIVACY_SUPPORTED_RE = /\.(jpe?g|png|webp|heic|heif)$/i;

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

  interface ScanResult {
    path: string;
    gps: { latitude: number; longitude: number; altitude: number | null } | null;
    device: string | null;
    datetime: string | null;
    author: string | null;
    technical: string | null;
    error: string | null;
  }

  interface StripResult {
    input_path: string;
    output_path: string;
    original_size: number;
    stripped_size: number;
    error: string | null;
  }

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.stripped + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.stripped > 0 || $summary.failed > 0
      ? t("privacy.headerDone", { done: $summary.stripped, total: $files.length }) +
        ($summary.failed > 0 ? t("common.failedSuffix", { count: $summary.failed }) : "")
      : $files.length === 1
        ? t("common.imagesSelectedOne", { count: $files.length })
        : t("common.imagesSelectedOther", { count: $files.length })
  );

  let gpsCount = $derived($files.filter(f => f.gps).length);
  let deviceCount = $derived($files.filter(f => f.device).length);
  let datetimeCount = $derived($files.filter(f => f.datetime).length);
  let authorCount = $derived($files.filter(f => f.author).length);
  let techCount = $derived($files.filter(f => f.technical).length);
  let totalFound = $derived(gpsCount + deviceCount + datetimeCount + authorCount + techCount);
  let allStripped = $derived($summary.stripped > 0 && $summary.stripped === $files.filter(f => f.hasMetadata !== undefined).length);

  let showToast = $state(false);
  let toastMessage = $state('');
  let toastTimer: ReturnType<typeof setTimeout>;

  function showPrivacyToast(strippedCount: number, hadGps: number) {
    if (strippedCount === 0) return;
    toastMessage = hadGps > 0
      ? (hadGps === 1 ? t("privacy.gpsRemovedOne", { count: hadGps }) : t("privacy.gpsRemovedOther", { count: hadGps }))
      : (strippedCount === 1 ? t("privacy.metadataRemovedOne", { count: strippedCount }) : t("privacy.metadataRemovedOther", { count: strippedCount }));
    showToast = true;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { showToast = false; }, 3000);
  }

  function showToastMessage(message: string) {
    toastMessage = message;
    showToast = true;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { showToast = false; }, 3000);
  }

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: t("privacy.filePickerName"), extensions: [...PRIVACY_SUPPORTED_EXTENSIONS] }],
    });
    if (selected) handleAddFiles(selected);
  }

  async function scanFile(file: PrivacyFile): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "scanning" as const } : f)
    );
    try {
      const result = await invoke<ScanResult>("read_metadata", { path: file.path });
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (result.error) return { ...f, status: "error" as const, error: result.error };
          const gpsStr = result.gps
            ? `${result.gps.latitude.toFixed(4)}, ${result.gps.longitude.toFixed(4)}`
            : undefined;
          return {
            ...f,
            status: "scanned" as const,
            gps: gpsStr,
            device: result.device ?? undefined,
            datetime: result.datetime ?? undefined,
            author: result.author ?? undefined,
            technical: result.technical ?? undefined,
            hasMetadata: !!(result.gps || result.device || result.datetime || result.author || result.technical),
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  function handleAddFiles(paths: string[]) {
    const currentFiles = $files;
    const existing = new Set(currentFiles.map((f) => f.path));
    const newPaths = paths.filter((p) => !existing.has(p) && PRIVACY_SUPPORTED_RE.test(p));
    addFiles(newPaths);
    // Scan new files silently in background (no processing state, no progress ring)
    const newFiles = newPaths.map((p) => ({
      path: p,
      name: p.split(/[\\/]/).pop() ?? p,
      status: "pending" as const,
    }));
    runWithConcurrency(newFiles, scanFile);
  }

  function handleRejectedFiles(paths: string[]) {
    showToastMessage(
      paths.length === 1
        ? t("privacy.unsupportedFileSkippedOne", { count: paths.length })
        : t("privacy.unsupportedFileSkippedOther", { count: paths.length })
    );
  }

  interface StripOpts { gps: boolean; device: boolean; datetime: boolean; author: boolean; }

  async function stripFile(file: PrivacyFile, opts: StripOpts): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "stripping" as const } : f)
    );
    try {
      const results = await invoke<StripResult[]>("strip_files_selective", {
        paths: [file.path],
        outputDir: $outputDir,
        stripGps: opts.gps,
        stripDevice: opts.device,
        stripDatetime: opts.datetime,
        stripAuthor: opts.author,
      });
      const result = results[0];
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (!result || result.error) return { ...f, status: "error" as const, error: result?.error ?? t("privacy.noResultError") };
          return {
            ...f,
            status: "done" as const,
            outputPath: result.output_path,
            originalSize: result.original_size,
            strippedSize: result.stripped_size,
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  async function startStrip() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    const opts: StripOpts = { gps: $stripGps, device: $stripDevice, datetime: $stripDatetime, author: $stripAuthor };
    if (!opts.gps && !opts.device && !opts.datetime && !opts.author) {
      showToastMessage(t("privacy.selectAtLeastOneCategory"));
      return;
    }
    isProcessing.set(true);
    files.update((all) =>
      all.map((f) => f.status === "done" ? { ...f, status: "scanned" as const } : f)
    );
    try {
      const toStrip = currentFiles.filter((f) => f.status === "scanned" || f.status === "pending");
      if (toStrip.length === 0) return;
      const allPaths = toStrip.map((f) => f.path);
      const results = await invoke<StripResult[]>("strip_files_selective", {
        paths: allPaths,
        outputDir: $outputDir,
        stripGps: opts.gps,
        stripDevice: opts.device,
        stripDatetime: opts.datetime,
        stripAuthor: opts.author,
      });
      const resultMap = new Map(results.map((r) => [r.input_path, r]));
      let successCount = 0;
      let gpsStrippedCount = 0;
      files.update((all) =>
        all.map((f) => {
          const r = resultMap.get(f.path);
          if (!r) return f;
          if (r.error) return { ...f, status: "error" as const, error: r.error };
          successCount++;
          if (f.gps && opts.gps) gpsStrippedCount++;
          return {
            ...f,
            status: "done" as const,
            outputPath: r.output_path,
            originalSize: r.original_size,
            strippedSize: r.stripped_size,
          };
        })
      );
      if (successCount > 0) {
        activity.add({ type: "privacy", fileCount: successCount, savedBytes: 0 });
      }
      showPrivacyToast(successCount, gpsStrippedCount);
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.status === "stripping" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    } finally {
      isProcessing.set(false);
    }
  }
</script>

<ToolShell
  files={$files}
  isProcessing={$isProcessing}
  {targetPct}
  progressLabel={t("common.progressLabel", { done: $summary.stripped + $summary.failed, total: $files.length })}
  onfiles={handleAddFiles}
  onrejectedfiles={handleRejectedFiles}
  onbrowse={browseFiles}
  onclear={clearFiles}
  dropZoneAcceptedExtensions={PRIVACY_SUPPORTED_RE}
  dropZoneFilePickerName={t("privacy.filePickerName")}
  dropZoneFilePickerExtensions={[...PRIVACY_SUPPORTED_EXTENSIONS]}
  actionLabel={t("privacy.actionLabel")}
  onaction={startStrip}
  {headerText}
>
  {#snippet fileDetail(file)}
    {#if file.status === "scanning"}
      {t("privacy.scanning")}
    {:else if file.status === "error"}
      {file.error}
    {:else if file.status === "done"}
      {#if file.hasMetadata}
        <span class="meta-removed">{[file.gps && t("privacy.gpsWord"), file.device, file.datetime, file.author && t("privacy.authorWord"), file.technical && t("privacy.technicalWord")].filter(Boolean).join(" · ")}</span>
      {/if}
      {#if file.outputPath}<span class="meta-tag">{file.outputPath.split(/[\\/]/).pop()}</span>{/if}
    {:else if file.status === "scanned" || file.status === "stripping"}
      {#if file.hasMetadata}
        {#if file.gps}<span class="meta-tag">{t("privacy.gpsWord")}</span>{/if}
        {#if file.device}<span class="meta-tag">{file.device}</span>{/if}
        {#if file.datetime}<span class="meta-tag">{file.datetime}</span>{/if}
        {#if file.author}<span class="meta-tag">{file.author}</span>{/if}
        {#if file.technical}<span class="meta-tag">{t("privacy.technicalWord")}</span>{/if}
      {:else}
        {t("privacy.noMetadataFound")}
      {/if}
    {:else}
      {t("privacy.ready")}
    {/if}
  {/snippet}

  {#snippet fileStatus(file)}
    {#if file.status === "done"}
      <CircleCheck size={18} />
    {:else if file.status === "error"}
      <CircleAlert size={18} />
    {:else}
      <button class="btn-icon" onclick={() => removeFile(file.path)}>
        <X size={16} />
      </button>
    {/if}
  {/snippet}

  <div class="control-group">
    <span class="label">{allStripped ? t("privacy.removedLabel") : t("privacy.foundLabel")}</span>
    <div class="pills">
      {#if totalFound === 0}
        <span class="pill">{t("privacy.noMetadataPill")}</span>
      {:else}
        {#if gpsCount > 0}<span class="pill" class:active={allStripped}>{t("privacy.pillGps", { count: gpsCount })}</span>{/if}
        {#if deviceCount > 0}<span class="pill" class:active={allStripped}>{t("privacy.pillDevice", { count: deviceCount })}</span>{/if}
        {#if datetimeCount > 0}<span class="pill" class:active={allStripped}>{t("privacy.pillDate", { count: datetimeCount })}</span>{/if}
        {#if authorCount > 0}<span class="pill" class:active={allStripped}>{t("privacy.pillAuthor", { count: authorCount })}</span>{/if}
        {#if techCount > 0}<span class="pill" class:active={allStripped}>{t("privacy.pillTechnical", { count: techCount })}</span>{/if}
      {/if}
    </div>
  </div>
  <div class="control-group">
    <span class="label">{t("privacy.stripLabel")}</span>
    <div class="strip-options">
      <label><ToggleSwitch bind:checked={$stripGps} label={t("privacy.stripGpsToggleLabel")} /> {t("privacy.gpsWord")}</label>
      <label><ToggleSwitch bind:checked={$stripDevice} label={t("privacy.stripDeviceToggleLabel")} /> {t("privacy.deviceWord")}</label>
      <label><ToggleSwitch bind:checked={$stripDatetime} label={t("privacy.stripDatetimeToggleLabel")} /> {t("privacy.dateTimeWord")}</label>
      <label><ToggleSwitch bind:checked={$stripAuthor} label={t("privacy.stripAuthorToggleLabel")} /> {t("privacy.authorWord")}</label>
    </div>
  </div>
  <div class="control-group">
    <span class="label">{t("common.outputLabel")}</span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$outputDir?.split(/[\\/]/).pop() ?? t("common.sameAsInput")}
    </button>
  </div>

</ToolShell>

<PrivacyToast visible={showToast} message={toastMessage} />

<style>
  .strip-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .strip-options label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .meta-tag {
    display: inline;
  }

  .meta-tag + .meta-tag::before {
    content: " · ";
  }

  .meta-removed {
    text-decoration: line-through;
    opacity: 0.45;
  }
</style>
