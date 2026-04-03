<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import HelperTooltip from "./HelperTooltip.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { formatBytes, browseOutputDir } from "$lib/utils";
  import {
    files, targetFormat, outputDir, isConverting, summary,
    selectedPlatforms,
    addFiles, removeFile, clearFiles,
    type TargetFormat, type ConvertFile,
  } from "$lib/stores/convert";
  import { qualityPreset, stripGps } from "$lib/stores/compress";
  import { savings } from "$lib/stores/savings";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS } from "$lib/types";

  const socialPlatforms = [
    { id: "instagram-portrait", label: "Instagram Portrait", width: 1080, height: 1350 },
    { id: "instagram-square", label: "Instagram Square", width: 1080, height: 1080 },
    { id: "youtube", label: "YouTube", width: 1280, height: 720 },
    { id: "linkedin", label: "LinkedIn", width: 1200, height: 627 },
    { id: "tiktok", label: "TikTok", width: 1080, height: 1920 },
  ];

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? `${$summary.done} of ${$files.length} converted${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} image${$files.length > 1 ? "s" : ""} selected`
  );

  interface ConversionResult {
    input_path: string;
    output_path: string;
    original_size: number;
    compressed_size: number;
    error: string | null;
  }

  interface ResizeResult {
    input_path: string;
    output_path: string;
    original_width: number;
    original_height: number;
    new_width: number;
    new_height: number;
    original_size: number;
    resized_size: number;
    error: string | null;
  }

  const formats: { value: TargetFormat; label: string }[] = [
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

  async function startConversion() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isConverting.set(true);
    const fmt = $targetFormat;
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    try {
      const allPaths = currentFiles.map((f) => f.path);
      const results = await invoke<ConversionResult[]>("convert_images", {
        paths: allPaths,
        format: fmt,
        qualityPreset: $qualityPreset,
        outputDir: $outputDir,
        stripGps: $stripGps,
      });
      const resultMap = new Map(results.map((r) => [r.input_path, r]));
      files.update((all) =>
        all.map((f) => {
          const r = resultMap.get(f.path);
          if (!r) return f;
          if (r.error) return { ...f, status: "error" as const, error: r.error };
          return { ...f, status: "done" as const, outputPath: r.output_path, outputSize: r.compressed_size, size: r.original_size };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.status === "pending" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
    const doneFiles = $files.filter((f) => f.status === "done");
    if (doneFiles.length > 0) {
      const originalBytes = doneFiles.reduce((acc, f) => acc + f.size, 0);
      const compressedBytes = doneFiles.reduce((acc, f) => acc + (f.outputSize ?? f.size), 0);
      const savedBytes = originalBytes - compressedBytes;
      savings.incrementOps(doneFiles.length);
      savings.addOriginalBytes(originalBytes);
      savings.addCompressedBytes(compressedBytes);
      if (savedBytes > 0) savings.add(savedBytes);
      activity.add({ type: "convert", fileCount: doneFiles.length, savedBytes });
    }
    isConverting.set(false);
  }

  async function startSocialExport() {
    const platforms = $selectedPlatforms;
    if (platforms.length === 0 || $files.length === 0) return;
    const filesToProcess = [...$files];
    isConverting.set(true);
    try {
      files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
      for (const platformId of platforms) {
        const platform = socialPlatforms.find((p) => p.id === platformId);
        if (!platform) continue;
        for (const file of filesToProcess) {
          files.update((all) =>
            all.map((f) => f.path === file.path ? { ...f, status: "converting" as const } : f)
          );
          try {
            const results = await invoke<ResizeResult[]>("resize_images", {
              paths: [file.path],
              width: platform.width,
              height: platform.height,
              mode: "Fit",
              quality: null,
              outputDir: $outputDir,
              stripGps: $stripGps,
              suffix: `_${platform.id}`,
            });
            const result = results[0];
            files.update((all) =>
              all.map((f) => {
                if (f.path !== file.path) return f;
                if (!result) return { ...f, status: "error" as const, error: "No result returned" };
                if (result.error) return { ...f, status: "error" as const, error: result.error };
                return { ...f, status: "done" as const, outputPath: result.output_path, outputSize: result.resized_size, size: result.original_size };
              })
            );
          } catch (err) {
            files.update((all) =>
              all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
            );
          }
        }
      }
      const doneFiles = $files.filter((f) => f.status === "done");
      if (doneFiles.length > 0) {
        const originalBytes = doneFiles.reduce((acc, f) => acc + f.size, 0);
        const compressedBytes = doneFiles.reduce((acc, f) => acc + (f.outputSize ?? f.size), 0);
        const savedBytes = originalBytes - compressedBytes;
        savings.incrementOps(doneFiles.length);
        savings.addOriginalBytes(originalBytes);
        savings.addCompressedBytes(compressedBytes);
        if (savedBytes > 0) savings.add(savedBytes);
        activity.add({ type: "convert", fileCount: doneFiles.length, savedBytes });
      }
    } finally {
      isConverting.set(false);
    }
  }

  function handleTogglePlatform(id: string) {
    selectedPlatforms.update((current) =>
      current.includes(id) ? current.filter((p) => p !== id) : [...current, id]
    );
  }

  let compareFile = $state<ConvertFile | null>(null);

  function handleClear() {
    compareFile = null;
    clearFiles();
  }
</script>

<ToolShell
  files={$files}
  isProcessing={$isConverting}
  {targetPct}
  progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
  progressSublabel={$summary.savedBytes > 0 ? `Saved ${formatBytes($summary.savedBytes)}` : undefined}
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={handleClear}
  actionLabel="Convert {$files.length > 1 ? 'All' : ''}"
  onaction={startConversion}
  {headerText}
>
  {#snippet headerSub()}
    {#if $summary.savedBytes > 0}
      <span class="saved-total">Saved {formatBytes($summary.savedBytes)}</span>
    {/if}
  {/snippet}

  {#snippet fileDetail(file)}
    {@const label = formats.find((f) => f.value === $targetFormat)?.label ?? ""}
    {#if file.status === "done"}
      {file.sourceFormat} → {label} · {formatBytes(file.outputSize ?? 0)}
    {:else if file.status === "error"}
      {file.error}
    {:else}
      {file.sourceFormat} → {label}
    {/if}
  {/snippet}

  {#snippet fileStatus(file)}
    {#if file.status === "done"}
      <button class="btn-icon compare-btn" onclick={() => compareFile = file} aria-label="Compare before and after">
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

  <div class="controls-row">
    <div class="control-group">
      <span class="label">Format <HelperTooltip tip="Choose the output image format. JPEG is best for photos, PNG for graphics, WebP/AVIF for modern compression." /></span>
      <div class="pills">
        {#each formats as f}
          <button class="pill" class:active={$targetFormat === f.value} onclick={() => targetFormat.set(f.value)}>
            {f.label}
          </button>
        {/each}
      </div>
    </div>
    {#if $targetFormat !== "Png"}
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
      </div>
    {/if}
    <div class="control-group">
      <span class="label">Output <HelperTooltip tip="Where to save converted files. Defaults to the same folder as the source images." /></span>
      <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
        <FolderOpen size={14} />
        {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
      </button>
    </div>
  </div>

  <div class="controls-divider"></div>

  <div class="controls-row">
    <div class="control-group">
      <div class="toggle-row">
        <div class="toggle-label">
          <span class="label">Strip GPS <HelperTooltip tip="Removes GPS coordinates and other location metadata from converted images to protect your privacy." /></span>
        </div>
        <ToggleSwitch bind:checked={$stripGps} />
      </div>
    </div>
  </div>

  <div class="controls-divider"></div>

  <div class="controls-row">
    <div class="social-row">
      <div class="control-group">
        <span class="label">Social export <HelperTooltip tip="Export images optimized for each social platform's dimensions and format requirements." /></span>
        <div class="social-platforms">
          {#each socialPlatforms as platform}
            <button
              class="pill platform-pill"
              class:active={$selectedPlatforms.includes(platform.id)}
              onclick={() => handleTogglePlatform(platform.id)}
            >
              {platform.label}
              <span class="platform-res">{platform.width}×{platform.height}</span>
            </button>
          {/each}
        </div>
      </div>
      {#if $selectedPlatforms.length > 0}
        <div class="social-action">
          <button class="social-export-btn" onclick={startSocialExport}>
            Export to {$selectedPlatforms.length} platform{$selectedPlatforms.length > 1 ? "s" : ""}
          </button>
        </div>
      {/if}
    </div>
  </div>
</ToolShell>

{#if compareFile?.outputPath}
  <BeforeAfterSlider
    beforePath={compareFile.path}
    afterPath={compareFile.outputPath}
    beforeSize={compareFile.size}
    afterSize={compareFile.outputSize ?? 0}
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

  .social-platforms {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .platform-pill {
    gap: 4px;
  }

  .platform-res {
    font-size: 11px;
    opacity: 0.45;
    font-variant-numeric: tabular-nums;
  }

  .platform-pill.active .platform-res {
    opacity: 0.65;
  }

  .social-export-btn {
    margin-top: 4px;
    padding: 5px 14px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 8px;
    color: var(--accent);
    border: 1px solid color-mix(in oklch, var(--accent) 40%, transparent);
    background: var(--accent-subtle);
    width: fit-content;
    transition: background 0.15s, border-color 0.15s;
  }

  .social-export-btn:hover {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    border-color: var(--accent);
  }

  .social-export-btn:active {
    transform: scale(0.98);
  }

  .social-row {
    display: flex;
    align-items: flex-start;
    gap: 20px;
    width: 100%;
  }

  .social-action {
    margin-left: auto;
    flex-shrink: 0;
    align-self: center;
  }
</style>
