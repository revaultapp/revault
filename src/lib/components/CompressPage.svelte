<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import { formatBytes, browseOutputDir } from "$lib/utils";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import {
    files, qualityPreset, format, outputDir, isCompressing, summary,
    stripGps,
    addFiles, removeFile, clearFiles,
    type QualityPreset, type OutputFormat, type CompressFile,
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
      <button class="btn-icon compare-btn" onclick={() => compareFile = file} title="Compare">
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
    <span class="label">Format</span>
    <div class="pills">
      {#each formats as f}
        <button class="pill" class:active={$format === f.value} onclick={() => format.set(f.value)}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Quality</span>
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
    <span class="label">Output</span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
    </button>
  </div>
  <div class="control-group">
    <div class="toggle-row">
      <div class="toggle-label">
        <span class="label">Strip GPS</span>
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

  /* .toggle-row, .toggle-label, .control-hint — styled globally in ToolShell */
</style>
