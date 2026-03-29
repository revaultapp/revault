<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CircleCheck, CircleAlert, X, Eye } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import BeforeAfterSlider from "./BeforeAfterSlider.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { formatBytes, browseOutputDir, openOutputFolder } from "$lib/utils";
  import {
    files, targetFormat, outputDir, isConverting, summary,
    activeProfile, selectedPlatforms,
    addFiles, removeFile, clearFiles,
    type TargetFormat, type ConvertFile,
  } from "$lib/stores/convert";
  import { savings } from "$lib/stores/savings";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS } from "$lib/types";

  let quality = $state(90);
  let stripGps = $state(false);

  const profiles: { id: "Web" | "Email" | "Archive" | "Share" | "Custom"; label: string }[] = [
    { id: "Web", label: "Web" },
    { id: "Email", label: "Email" },
    { id: "Archive", label: "Archive" },
    { id: "Share", label: "Share" },
    { id: "Custom", label: "Custom" },
  ];

  const socialPlatforms = [
    { id: "instagram-portrait", label: "Instagram Portrait", width: 1080, height: 1350 },
    { id: "instagram-square", label: "Instagram Square", width: 1080, height: 1080 },
    { id: "youtube", label: "YouTube", width: 1280, height: 720 },
    { id: "linkedin", label: "LinkedIn", width: 1200, height: 627 },
    { id: "tiktok", label: "TikTok", width: 1080, height: 1920 },
  ];

  function applyProfile(p: "Web" | "Email" | "Archive" | "Share" | "Custom") {
    activeProfile.set(p);
    if (p === "Web") { quality = 75; targetFormat.set("Webp"); }
    else if (p === "Email") { quality = 60; targetFormat.set("Jpeg"); }
    else if (p === "Archive") { quality = 95; targetFormat.set("Png"); }
    else if (p === "Share") { quality = 80; targetFormat.set("Webp"); stripGps = true; }
  }

  function onManualChange() { activeProfile.set("Custom"); }

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

  async function handleOpenOutputFolder() {
    const firstOutput = $files.find((f) => f.outputPath)?.outputPath;
    if (firstOutput) await openOutputFolder(firstOutput);
  }

  async function startConversion() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isConverting.set(true);
    const fmt = $targetFormat;
    const q = quality;
    try {
      files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
      const allPaths = currentFiles.map((f) => f.path);
      const results = await invoke<ConversionResult[]>("convert_images", {
        paths: allPaths,
        format: fmt,
        quality: q,
        outputDir: $outputDir,
        stripGps: stripGps,
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
      const doneFiles = $files.filter((f) => f.status === "done");
      if (doneFiles.length > 0) {
        const originalBytes = doneFiles.reduce((acc, f) => acc + f.size, 0);
        const compressedBytes = doneFiles.reduce((acc, f) => acc + (f.outputSize ?? f.size), 0);
        const savedBytes = originalBytes - compressedBytes;
        savings.incrementOps($summary.done);
        savings.addOriginalBytes(originalBytes);
        savings.addCompressedBytes(compressedBytes);
        if (savedBytes > 0) savings.add(savedBytes);
        activity.add({ type: "convert", fileCount: doneFiles.length, savedBytes });
      }
    } finally {
      isConverting.set(false);
    }
  }

  async function startSocialExport() {
    const platforms = $selectedPlatforms;
    if (platforms.length === 0 || $files.length === 0) return;
    isConverting.set(true);
    const q = quality;
    try {
      files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
      for (const platformId of platforms) {
        const platform = socialPlatforms.find((p) => p.id === platformId);
        if (!platform) continue;
        const platformFiles = $files.filter((f) => f.status === "pending");
        for (const file of platformFiles) {
          files.update((all) =>
            all.map((f) => f.path === file.path ? { ...f, status: "converting" as const } : f)
          );
          try {
            const results = await invoke<ResizeResult[]>("resize_images", {
              paths: [file.path],
              width: platform.width,
              height: platform.height,
              mode: "Fit",
              quality: q,
              outputDir: $outputDir,
              stripGps: stripGps,
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
        savings.incrementOps($summary.done);
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
    onManualChange();
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
  onopenfolder={$summary.done > 0 && $summary.pending === 0 ? handleOpenOutputFolder : undefined}
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
      <button class="btn-icon compare-btn" onclick={() => compareFile = file} title="Compare">
        <Eye size={16} />
      </button>
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
        <button class="pill" class:active={$targetFormat === f.value} onclick={() => { targetFormat.set(f.value); onManualChange(); }}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  {#if $targetFormat !== "Png"}
    <div class="control-group">
      <label for="quality-slider">Quality <span class="quality-value">{quality}%</span></label>
      <input id="quality-slider" type="range" min="10" max="100" step="5" bind:value={quality} oninput={onManualChange} />
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
    <ToggleSwitch bind:checked={stripGps} />
  </div>
  <div class="control-group">
    <span class="label">Social Export</span>
    <div class="social-platforms">
      {#each socialPlatforms as platform}
        <label class="platform-check">
          <input
            type="checkbox"
            checked={$selectedPlatforms.includes(platform.id)}
            onchange={() => handleTogglePlatform(platform.id)}
          />
          <span class="platform-label">{platform.label}</span>
          <span class="platform-res">{platform.width}×{platform.height}</span>
        </label>
      {/each}
    </div>
    {#if $selectedPlatforms.length > 0}
      <button class="btn-primary social-btn" onclick={startSocialExport}>
        Export to {$selectedPlatforms.length} platform{$selectedPlatforms.length > 1 ? "s" : ""}
      </button>
    {/if}
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
    gap: 6px;
  }

  .platform-check {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    background: var(--navy-bg);
    transition: background 0.15s;
  }

  .platform-check:hover { background: var(--border); }

  .platform-check input[type="checkbox"] {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
  }

  .platform-label { font-weight: 500; color: var(--text-secondary); }
  .platform-res { color: var(--text-muted); font-size: 11px; }

  .social-btn {
    margin-top: 8px;
    padding: 8px 20px;
    font-size: 13px;
  }


</style>
