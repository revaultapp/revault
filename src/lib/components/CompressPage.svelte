<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { FolderOpen, CheckCircle, AlertCircle, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import HelperTooltip from "./HelperTooltip.svelte";
  import { formatBytes, browseOutputDir } from "$lib/utils";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import {
    files, qualityPreset, format, outputDir, isCompressing, isEstimating, summary,
    stripGps, estimateSavings,
    addFiles, removeFile, clearFiles,
    type QualityPreset, type OutputFormat, type CompressFile,
    type SavingsEstimate,
  } from "$lib/stores/compress";
  import { savings } from "$lib/stores/savings";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS } from "$lib/types";

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? `${$summary.done} of ${$files.length} compressed${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} image${$files.length > 1 ? "s" : ""} selected`
  );

  interface CompressionResult {
    input_path: string;
    output_path: string;
    original_size: number;
    compressed_size: number;
    already_optimal: boolean;
    error: string | null;
  }

  const formats: { value: OutputFormat | null; label: string }[] = [
    { value: null, label: "Auto (smallest)" },
    { value: "Jpeg", label: "JPEG" },
    { value: "Png", label: "PNG" },
    { value: "Webp", label: "WebP" },
    { value: "Avif", label: "AVIF" },
  ];

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: [...IMAGE_EXTENSIONS] }],
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
        outputDir: $outputDir,
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
    }
    isCompressing.set(false);
  }

  function savedPercent(file: CompressFile): string {
    if (!file.compressedSize || file.size === 0) return "";
    if (file.alreadyOptimal) return "Already optimal";
    const pct = Math.round(((file.size - file.compressedSize) / file.size) * 100);
    if (pct > 0) return `${pct}% smaller`;
    if (pct < 0) return `${Math.abs(pct)}% larger`;
    return "Same size";
  }

  async function openOutputFolder() {
    const dir = $outputDir ?? ($files[0]?.path ? $files[0].path.substring(0, $files[0].path.lastIndexOf($files[0].path.includes('/') ? '/' : '\\')) : null);
    if (dir) await openPath(dir);
  }

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
  progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
  progressSublabel={$summary.savedBytes > 0 ? `Saved ${formatBytes($summary.savedBytes)}` : undefined}
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={handleClear}
  actionLabel="Compress {$files.length > 1 ? 'All' : ''}"
  onaction={startCompression}
  {headerText}
>
  {#snippet headerSub()}
    {#if $summary.savedBytes > 0}
      <span class="saved-total">Saved {formatBytes($summary.savedBytes)}</span>
    {/if}
    {#if $summary.done > 0}
      <button class="btn-ghost open-folder-btn" onclick={openOutputFolder}>
        Open output folder
      </button>
    {/if}
  {/snippet}

  {#snippet estimateCard()}
    {#if $isEstimating}
      <div class="estimate-card">
        <div class="estimate-row">
          <span class="estimate-label">Estimated</span>
          <span class="estimate-value estimating">Scanning sample files...</span>
        </div>
      </div>
    {:else if estimatedBanner}
      <div class="estimate-card">
        <div class="estimate-row">
          <span class="estimate-label">Estimated</span>
          <span class="estimate-value">
            {estimatedBanner.count} files:&nbsp;
            {formatBytes(estimatedBanner.totalOriginal)}
            → ~{formatBytes(estimatedBanner.estimated)}
            <span class="estimate-pct">({estimatedBanner.displayPct}% {estimatedBanner.wouldGrow ? 'larger' : 'smaller'})</span>
          </span>
        </div>
        <div class="estimate-meta">
          {#if estimatedBanner.filesMayIncrease > 0}
            <span class="estimate-warn">
              <AlertCircle size={12} />
              {estimatedBanner.filesMayIncrease} file{estimatedBanner.filesMayIncrease > 1 ? 's' : ''} may grow
            </span>
          {/if}
        </div>
      </div>
    {/if}
  {/snippet}

  {#snippet fileDetail(file)}
    {#if file.status === "done"}
      {formatBytes(file.size)} → {formatBytes(file.compressedSize ?? 0)} · {savedPercent(file)}
    {:else if file.status === "error"}
      {file.error}
    {:else}
      Ready
    {/if}
  {/snippet}

  {#snippet fileStatus(file)}
    {#if file.status === "done"}
      <button class="btn-icon compare-btn" onclick={() => compareFile = file} title="Compare" aria-label="Compare before and after">
        <Eye size={16} />
      </button>
      <CheckCircle size={18} />
    {:else if file.status === "error"}
      <AlertCircle size={18} />
    {:else}
      <button class="btn-icon" onclick={() => removeFile(file.path)}>
        <X size={16} />
      </button>
    {/if}
  {/snippet}

  <div class="control-group">
    <span class="label">Format <HelperTooltip tip="Choose the output format. Auto selects the format that produces the smallest file." /></span>
    <div class="pills">
      {#each formats as f}
        <button class="pill" class:active={$format === f.value} onclick={() => format.set(f.value)}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Quality <HelperTooltip tip="Smallest: minimum file size. Balanced: good quality at lower size. High quality: best quality, larger files." /></span>
    <div class="pills">
      <button class="pill" class:active={$qualityPreset === "Smallest"}
        onclick={() => qualityPreset.set("Smallest")}>Smallest</button>
      <button class="pill" class:active={$qualityPreset === "Balanced"}
        onclick={() => qualityPreset.set("Balanced")}>Balanced</button>
      <button class="pill" class:active={$qualityPreset === "HighQuality"}
        onclick={() => qualityPreset.set("HighQuality")}>High quality</button>
    </div>
    {#if $format === "Png"}
      <span class="format-hint">PNG is lossless — quality preset doesn't affect output</span>
    {/if}
  </div>
  <div class="control-group">
    <span class="label">Output <HelperTooltip tip="Folder where compressed images are saved. Defaults to the same folder as the original." /></span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
    </button>
  </div>
  <div class="control-group">
    <div class="toggle-row">
      <div class="toggle-label">
        <span class="label">Strip Location</span>
        <span class="control-hint">Remove location data from photos</span>
      </div>
      <ToggleSwitch bind:checked={$stripGps} />
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

<style>
  .saved-total {
    font-size: 13px;
    color: var(--accent);
    font-weight: 500;
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

  /* Estimate card — prominent pre-compression savings display */
  .estimate-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 16px;
    background: var(--accent-subtle);
    border: 1px solid var(--accent-glow);
    border-radius: var(--radius-sm);
  }

  .estimate-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .estimate-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
  }

  .estimate-value {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .estimate-pct {
    font-weight: 600;
    color: var(--accent);
  }

  .estimate-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .estimate-warn {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--warning, #f59e0b);
  }

  .estimating {
    color: var(--text-muted);
    font-style: italic;
  }

  /* .toggle-row, .toggle-label, .control-hint — styled globally in ToolShell */
</style>
