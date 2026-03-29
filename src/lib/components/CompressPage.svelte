<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import { formatBytes, browseOutputDir, openOutputFolder } from "$lib/utils";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import {
    files, quality, format, outputDir, isCompressing, summary,
    compressMode, targetSize, targetUnit, activeProfile, stripGps,
    addFiles, removeFile, clearFiles,
    type OutputFormat, type CompressFile, type CompressionProfile, type CompressMode,
  } from "$lib/stores/compress";
  import { savings } from "$lib/stores/savings";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS } from "$lib/types";

  const profiles: { id: CompressionProfile; label: string }[] = [
    { id: "Web", label: "Web" },
    { id: "Email", label: "Email" },
    { id: "Archive", label: "Archive" },
    { id: "Share", label: "Share" },
    { id: "Custom", label: "Custom" },
  ];

  const targetPresets = [
    { label: "500 KB", size: 500, unit: "KB" as const },
    { label: "1 MB", size: 1, unit: "MB" as const },
    { label: "2 MB", size: 2, unit: "MB" as const },
    { label: "5 MB", size: 5, unit: "MB" as const },
  ];

  function applyProfile(p: CompressionProfile) {
    activeProfile.set(p);
    if (p === "Web") { quality.set(75); format.set("Webp"); compressMode.set("quality"); stripGps.set(false); }
    else if (p === "Email") { quality.set(60); format.set("Jpeg"); compressMode.set("target"); targetSize.set(500); targetUnit.set("KB"); stripGps.set(false); }
    else if (p === "Archive") { quality.set(95); format.set(null); compressMode.set("quality"); stripGps.set(false); }
    else if (p === "Share") { quality.set(80); format.set("Webp"); compressMode.set("quality"); stripGps.set(true); }
  }

  function onManualChange() { activeProfile.set("Custom"); }

  function targetBytes(): number {
    return $targetUnit === "MB" ? $targetSize * 1024 * 1024 : $targetSize * 1024;
  }

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
    { value: null, label: "Auto" },
    { value: "Jpeg", label: "JPEG" },
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

  async function handleOpenOutputFolder() {
    const firstOutput = $files.find((f) => f.outputPath)?.outputPath;
    if (firstOutput) await openOutputFolder(firstOutput);
  }

  async function startCompression() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isCompressing.set(true);
    const q = $quality;
    const fmt = $format;
    const mode = $compressMode;
    const tb = targetBytes();
    const gps = $stripGps;
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    try {
      const allPaths = currentFiles.map((f) => f.path);
      const cmd = mode === "target" ? "compress_to_target" : "compress_images";
      const args = mode === "target"
        ? { paths: allPaths, targetBytes: tb, format: fmt, outputDir: $outputDir, stripGps: gps }
        : { paths: allPaths, quality: q, format: fmt, outputDir: $outputDir, stripGps: gps };
      const results = await invoke<CompressionResult[]>(cmd, args);
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
  onopenfolder={$summary.done > 0 && $summary.pending === 0 ? handleOpenOutputFolder : undefined}
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
    <span class="label">Profile</span>
    <div class="pills">
      {#each profiles as p}
        <button class="pill" class:active={$activeProfile === p.id} onclick={() => applyProfile(p.id)}>
          {p.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Format</span>
    <div class="pills">
      {#each formats as f}
        <button class="pill" class:active={$format === f.value} onclick={() => { format.set(f.value); onManualChange(); }}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Mode</span>
    <div class="pills">
      <button class="pill" class:active={$compressMode === "quality"} onclick={() => { compressMode.set("quality"); onManualChange(); }}>Quality</button>
      <button class="pill" class:active={$compressMode === "target"} onclick={() => { compressMode.set("target"); onManualChange(); }}>Target Size</button>
    </div>
  </div>
  {#if $compressMode === "quality"}
    <div class="control-group">
      <label for="quality-slider">Quality <span class="quality-value">{$quality}%</span></label>
      <input id="quality-slider" type="range" min="10" max="100" step="5" bind:value={$quality} oninput={onManualChange} />
    </div>
  {:else}
    <div class="control-group">
      <span class="label">Target</span>
      <div class="pills">
        {#each targetPresets as tp}
          <button class="pill" class:active={$targetSize === tp.size && $targetUnit === tp.unit}
            onclick={() => { targetSize.set(tp.size); targetUnit.set(tp.unit); onManualChange(); }}>
            {tp.label}
          </button>
        {/each}
      </div>
    </div>
  {/if}
  <div class="control-group">
    <span class="label">Output</span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
    </button>
  </div>
  <div class="control-group">
    <span class="label">Strip GPS</span>
    <ToggleSwitch bind:checked={$stripGps} />
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

  .compare-btn {
    margin-right: 6px;
    color: var(--text-muted);
    transition: color 0.15s;
  }
  .compare-btn:hover { color: var(--accent); }
</style>
