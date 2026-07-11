<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { FolderOpen, CheckCircle, AlertCircle, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import HelperTooltip from "./HelperTooltip.svelte";
  import PrivacyToast from "./PrivacyToast.svelte";
  import { formatBytes, browseOutputDir } from "$lib/utils";
  import { animatedNumber } from "$lib/motion";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import {
    files, qualityPreset, format, outputDir, resolvedOutputDir, isCompressing, isEstimating, summary,
    stripGps, estimateSavings,
    addFiles, removeFile, clearFiles,
    type QualityPreset, type OutputFormat, type CompressFile,
    type SavingsEstimate,
  } from "$lib/stores/compress";
  import { savings } from "$lib/stores/savings";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import { t } from "$lib/stores/locale.svelte";

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? t("compress.headerDone", { done: $summary.done, total: $files.length }) +
        ($summary.failed > 0 ? t("common.failedSuffix", { count: $summary.failed }) : "")
      : $files.length === 1
        ? t("common.imagesSelectedOne", { count: $files.length })
        : t("common.imagesSelectedOther", { count: $files.length })
  );

  interface CompressionResult {
    input_path: string;
    output_path: string;
    original_size: number;
    compressed_size: number;
    already_optimal: boolean;
    error: string | null;
  }

  let formats = $derived<{ value: OutputFormat | null; label: string }[]>([
    { value: null, label: t("compress.formatAuto") },
    { value: "Jpeg", label: t("common.formatJpeg") },
    { value: "Png", label: t("common.formatPng") },
    { value: "Webp", label: t("common.formatWebp") },
    { value: "Avif", label: t("common.formatAvif") },
  ]);

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: t("dropZone.filePickerName"), extensions: [...IMAGE_EXTENSIONS] }],
    });
    if (selected) addFiles(selected);
  }

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

  async function startCompression() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isCompressing.set(true);
    const fmt = $format;
    const gps = $stripGps;
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    try {
      const allPaths = currentFiles.map((f) => f.path);
      const results = await invoke<CompressionResult[]>("compress_images", {
        paths: allPaths,
        qualityPreset: $qualityPreset,
        format: fmt,
        outputDir: $resolvedOutputDir,
        stripGps: gps,
      });
      const resultMap = new Map(results.map((r) => [r.input_path, r]));
      files.update((all) =>
        all.map((f) => {
          const r = resultMap.get(f.path);
          if (!r) return f;
          if (r.error) return { ...f, status: "error" as const, error: r.error, size: r.original_size };
          return { ...f, status: "done" as const, compressedSize: r.compressed_size, outputPath: r.output_path, size: r.original_size, alreadyOptimal: r.already_optimal };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.status === "pending" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
    if ($summary.done > 0) {
      const doneFiles = $files.filter((f) => f.status === "done");
      const originalBytes = doneFiles.reduce((acc, f) => acc + f.size, 0);
      const compressedBytes = doneFiles.reduce((acc, f) => acc + (f.compressedSize ?? f.size), 0);
      savings.incrementOps($summary.done);
      savings.addOriginalBytes(originalBytes);
      savings.addCompressedBytes(compressedBytes);
      savings.add($summary.savedBytes);
      activity.add({ type: "compress", fileCount: $summary.done, savedBytes: $summary.savedBytes });
      if (gps) {
        const n = $summary.done;
        toastMessage = n === 1
          ? t("compress.gpsRemovedOne", { count: n })
          : t("compress.gpsRemovedOther", { count: n });
        showToast = true;
        clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { showToast = false; }, 3000);
      }
    }
    if ($summary.done > 0 || $summary.failed > 0) {
      compressSuccess = true;
      clearTimeout(successTimer);
      successTimer = setTimeout(() => { compressSuccess = false; }, 1500);
    }
    isCompressing.set(false);
  }

  function savedPercent(file: CompressFile): string {
    if (!file.compressedSize || file.size === 0) return "";
    if (file.alreadyOptimal) return t("compress.alreadyOptimal");
    const pct = Math.round(((file.size - file.compressedSize) / file.size) * 100);
    if (pct > 0) return t("compress.pctSmaller", { pct });
    if (pct < 0) return t("compress.pctLarger", { pct: Math.abs(pct) });
    return t("compress.sameSize");
  }

  async function openOutputFolder() {
    const dir = $resolvedOutputDir ?? ($files[0]?.path ? $files[0].path.substring(0, $files[0].path.lastIndexOf($files[0].path.includes('/') ? '/' : '\\')) : null);
    if (dir) await openPath(dir);
  }

  let compressSuccess = $state(false);
  let successTimer: ReturnType<typeof setTimeout>;

  let showToast = $state(false);
  let toastMessage = $state('');
  let toastTimer: ReturnType<typeof setTimeout>;

  // Real savings estimate from preview compression
  let savingsEstimate = $state<SavingsEstimate | null>(null);
  let currentEstimateId = 0;

  // Re-estimate when files, quality, or format changes
  $effect(() => {
    if ($files.length === 0 || $isCompressing || $summary.done > 0) {
      savingsEstimate = null;
      return;
    }
    // Copy values to avoid tracking issues with async
    const currentFiles = $files;
    const currentPreset = $qualityPreset;
    const currentFormat = $format;
    const estimateId = ++currentEstimateId;

    estimateSavings(currentFiles, currentPreset, currentFormat).then((result) => {
      // Only use result if this is still the latest estimate
      if (estimateId !== currentEstimateId) return;
      if (currentFiles.length > 0) {
        savingsEstimate = result;
      }
    });
  });

  // Derived banner from real estimate
  let estimatedBanner = $derived.by(() => {
    if ($files.length === 0 || $isCompressing || $summary.done > 0) return null;
    if (!savingsEstimate) return null;
    const { sampleRatio, filesMayIncrease, totalOriginalBytes } = savingsEstimate;
    const totalOriginal = totalOriginalBytes;
    const estimated = Math.round(totalOriginal * sampleRatio);
    const pct = Math.round((1 - sampleRatio) * 100);
    const wouldGrow = pct < 0;
    return {
      count: $files.length,
      totalOriginal,
      estimated,
      pct,
      displayPct: Math.abs(pct),
      wouldGrow,
      filesMayIncrease,
    };
  });

  // Estimate hero number + % count up toward the target instead of snapping,
  // per animatedNumber's own reduced-motion guard.
  const estimatedBytesTween = animatedNumber(0);
  const estimatedPctTween = animatedNumber(0);

  $effect(() => {
    if (estimatedBanner) {
      estimatedBytesTween.set(estimatedBanner.estimated);
      estimatedPctTween.set(estimatedBanner.displayPct);
    }
  });

  let compareFile = $state<CompressFile | null>(null);

  function handleClear() {
    compareFile = null;
    clearFiles();
  }
</script>

<ToolShell
  files={$files}
  isProcessing={$isCompressing}
  {targetPct}
  progressLabel={t("common.progressLabel", { done: $summary.done + $summary.failed, total: $files.length })}
  progressSublabel={$summary.savedBytes > 0 ? t("common.savedTotal", { amount: formatBytes($summary.savedBytes) }) : undefined}
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={handleClear}
  actionLabel={$files.length > 1 ? t("compress.actionButtonAll") : t("compress.actionButton")}
  onaction={startCompression}
  actionLoading={$isCompressing}
  actionSuccess={compressSuccess}
  {headerText}
>
  {#snippet headerSub()}
    {#if $summary.savedBytes > 0}
      <span class="saved-total">{t("common.savedTotal", { amount: formatBytes($summary.savedBytes) })}</span>
    {/if}
    {#if $summary.done > 0}
      <button class="btn-ghost open-folder-btn" onclick={openOutputFolder}>
        {t("compress.openOutputFolder")}
      </button>
    {/if}
  {/snippet}

  {#snippet estimateCard()}
    {#if $isEstimating}
      <div class="estimate-card">
        <span class="estimate-label">{t("compress.estimatedLabel")}</span>
        <span class="estimate-scanning">{t("compress.scanningSampleFiles")}</span>
      </div>
    {:else if estimatedBanner}
      <div class="estimate-card">
        <span class="estimate-label">{t("compress.estimatedLabel")}</span>
        <div class="estimate-hero">
          <span class="estimate-hero-num" class:grow={estimatedBanner.wouldGrow}>
            {Math.round(estimatedPctTween.current)}<small>%</small>
          </span>
          <span class="estimate-hero-word">
            {t(estimatedBanner.wouldGrow ? "compress.larger" : "compress.smaller")}
          </span>
        </div>
        <div class="estimate-bar-track">
          <div
            class="estimate-bar-fill"
            class:grow={estimatedBanner.wouldGrow}
            style="transform: scaleX({Math.min(Math.abs(estimatedPctTween.current), 100) / 100})"
          ></div>
        </div>
        <span class="estimate-range">
          {estimatedBanner.count === 1
            ? t("compress.estimateSummaryOne", { count: estimatedBanner.count, original: formatBytes(estimatedBanner.totalOriginal), estimated: formatBytes(estimatedBytesTween.current) })
            : t("compress.estimateSummaryOther", { count: estimatedBanner.count, original: formatBytes(estimatedBanner.totalOriginal), estimated: formatBytes(estimatedBytesTween.current) })}
        </span>
        {#if estimatedBanner.filesMayIncrease > 0}
          <span class="estimate-warn">
            <AlertCircle size={12} />
            {estimatedBanner.filesMayIncrease === 1
              ? t("compress.mayGrowOne", { count: estimatedBanner.filesMayIncrease })
              : t("compress.mayGrowOther", { count: estimatedBanner.filesMayIncrease })}
          </span>
        {/if}
      </div>
    {/if}
  {/snippet}

  {#snippet fileDetail(file)}
    {#if file.status === "done"}
      {formatBytes(file.size)} → {formatBytes(file.compressedSize ?? 0)} · {savedPercent(file)}
    {:else if file.status === "error"}
      {file.error}
    {:else}
      {t("compress.ready")}
    {/if}
  {/snippet}

  {#snippet fileStatus(file)}
    {#if file.status === "done"}
      <button class="btn-icon compare-btn" onclick={() => compareFile = file} title={t("compress.compareTitle")} aria-label={t("common.compareAriaLabel")}>
        <Eye size={16} />
      </button>
      <CheckCircle size={18} />
    {:else if file.status === "error"}
      <AlertCircle size={18} />
    {:else}
      <button class="btn-icon" onclick={() => removeFile(file.path)} aria-label={t("common.removeFileAriaLabel", { name: file.name })}>
        <X size={16} />
      </button>
    {/if}
  {/snippet}

  <div class="control-group">
    <span class="label">{t("common.formatLabel")} <HelperTooltip tip={t("compress.formatTooltip")} /></span>
    <div class="pills">
      {#each formats as f}
        <button class="pill" class:active={$format === f.value} aria-pressed={$format === f.value} onclick={() => format.set(f.value)}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">{t("common.qualityLabel")} <HelperTooltip tip={t("common.qualityTooltip")} /></span>
    <div class="pills">
      <button class="pill" class:active={$qualityPreset === "Smallest"} aria-pressed={$qualityPreset === "Smallest"}
        onclick={() => qualityPreset.set("Smallest")}>{t("common.qualitySmallest")}</button>
      <button class="pill" class:active={$qualityPreset === "Balanced"} aria-pressed={$qualityPreset === "Balanced"}
        onclick={() => qualityPreset.set("Balanced")}>{t("common.qualityBalanced")}</button>
      <button class="pill" class:active={$qualityPreset === "HighQuality"} aria-pressed={$qualityPreset === "HighQuality"}
        onclick={() => qualityPreset.set("HighQuality")}>{t("common.qualityHighQuality")}</button>
    </div>
    {#if $format === "Png"}
      <span class="format-hint">{t("compress.pngLosslessHint")}</span>
    {/if}
  </div>
  <div class="control-group">
    <span class="label">{t("common.outputLabel")} <HelperTooltip tip={t("compress.outputTooltip")} /></span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$resolvedOutputDir?.split(/[\\/]/).pop() ?? t("common.sameAsInput")}
    </button>
  </div>
  <div class="control-group">
    <div class="toggle-row">
      <div class="toggle-label">
        <span class="label">{t("compress.stripLocationLabel")}</span>
        <span class="control-hint">{t("compress.stripLocationHint")}</span>
      </div>
      <ToggleSwitch bind:checked={$stripGps} label={t("compress.stripLocationLabel")} />
    </div>
  </div>
</ToolShell>

{#if compareFile?.outputPath}
  <BeforeAfterSlider
    beforePath={compareFile.path}
    afterPath={compareFile.outputPath}
    beforeSize={compareFile.size}
    afterSize={compareFile.compressedSize ?? 0}
    onclose={() => compareFile = null}
  />
{/if}

<PrivacyToast visible={showToast} message={toastMessage} />

<style>
  .saved-total {
    font-size: 13px;
    color: var(--accent-text);
    font-weight: 500;
  }

  .pill.active {
    font-weight: 600;
    box-shadow: var(--shadow-xs);
  }

  .format-hint {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .compare-btn {
    margin-right: 6px;
    color: var(--text-muted);
    transition: color 0.15s;
  }
  .compare-btn:hover { color: var(--accent); }

  .open-folder-btn {
    margin-left: 8px;
    font-size: 12px;
  }

  /* Estimate card — the pre-compression savings preview, the app's signature
     panel. Hero number carries the sole accent usage; everything else stays
     on the neutral scale so the number keeps all the visual weight. */
  .estimate-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 20px;
    background: var(--accent-subtle);
    border: 1px solid var(--accent-glow);
    border-radius: var(--radius-sm);
  }

  .estimate-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .estimate-hero {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .estimate-hero-num {
    font-size: 38px;
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.02em;
    color: var(--accent-text);
    font-variant-numeric: tabular-nums;
  }

  .estimate-hero-num.grow { color: var(--warning-text); }

  .estimate-hero-num small {
    font-size: 18px;
    font-weight: 500;
  }

  .estimate-hero-word {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .estimate-bar-track {
    width: 100%;
    height: 6px;
    border-radius: 999px;
    background: var(--navy-bg);
    overflow: hidden;
  }

  .estimate-bar-fill {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transform-origin: left center;
    transition: background-color var(--duration-normal) var(--ease-out);
  }

  .estimate-bar-fill.grow { background: var(--warning); }

  .estimate-range {
    font-size: 13px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .estimate-scanning {
    font-size: 13px;
    color: var(--text-muted);
    font-style: italic;
  }

  .estimate-warn {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--warning-text);
  }

  /* .toggle-row, .toggle-label, .control-hint — styled globally in ToolShell */
</style>
