<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { browseOutputDir } from "$lib/utils";
  import { stripGps } from "$lib/stores/compress";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import { formatBytes } from "$lib/utils";
  import {
    files, isResizing, outputDir, resizeMode, width, height, summary,
    addFiles, removeFile, clearFiles,
    type ResizeFile,
  } from "$lib/stores/resize";

  type Preset = { label: string; w: number; h: number };
  type PresetGroup = { group: string; presets: Preset[] };

  const presetGroups: PresetGroup[] = [
    {
      group: "General",
      presets: [
        { label: "Full HD", w: 1920, h: 1080 },
        { label: "HD", w: 1280, h: 720 },
        { label: "Thumbnail", w: 300, h: 300 },
      ],
    },
    {
      group: "Social Media",
      presets: [
        { label: "IG Post", w: 1080, h: 1350 },
        { label: "IG Square", w: 1080, h: 1080 },
        { label: "IG Story", w: 1080, h: 1920 },
        { label: "YouTube", w: 1280, h: 720 },
        { label: "Twitter/X", w: 1200, h: 675 },
        { label: "LinkedIn", w: 1200, h: 1200 },
        { label: "TikTok", w: 1080, h: 1920 },
      ],
    },
  ];

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? `${$summary.done} of ${$files.length} resized${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} image${$files.length > 1 ? "s" : ""} selected`
  );

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

  async function startResize() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    const w = $width;
    const h = $height;
    const mode = $resizeMode;
    const outDir = $outputDir;
    isResizing.set(true);
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    try {
      const allPaths = currentFiles.map((f) => f.path);
      const results = await invoke<ResizeResult[]>("resize_images", {
        paths: allPaths,
        width: w,
        height: h,
        mode,
        outputDir: outDir,
        stripGps: $stripGps,
      });
      const resultMap = new Map(results.map((r) => [r.input_path, r]));
      files.update((all) =>
        all.map((f) => {
          const r = resultMap.get(f.path);
          if (!r) return f;
          if (r.error) return { ...f, status: "error" as const, error: r.error };
          return {
            ...f, status: "done" as const,
            outputPath: r.output_path,
            outputWidth: r.new_width, outputHeight: r.new_height,
            originalWidth: r.original_width, originalHeight: r.original_height,
            size: r.original_size, outputSize: r.resized_size,
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.status === "pending" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    } finally {
      isResizing.set(false);
    }
  }

</script>

<ToolShell
  files={$files}
  isProcessing={$isResizing}
  {targetPct}
  progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={clearFiles}
  actionLabel="Resize {$files.length > 1 ? 'All' : ''}"
  onaction={startResize}
  {headerText}
>
  {#snippet fileDetail(file)}
    {#if file.status === "done"}
      {file.size ? formatBytes(file.size) : '—'} → {file.outputSize ? formatBytes(file.outputSize) : '—'} ({file.originalWidth}×{file.originalHeight} → {file.outputWidth}×{file.outputHeight})
    {:else if file.status === "error"}
      {file.error}
    {:else}
      {$width}×{$height} · {$resizeMode}
    {/if}
  {/snippet}

  {#snippet fileStatus(file)}
    {#if file.status === "done"}
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
    <span class="label">Presets</span>
    <div class="preset-sections">
      {#each presetGroups as group}
        <div class="preset-section">
          <span class="preset-group-label">{group.group}</span>
          <div class="preset-grid">
            {#each group.presets as p}
              <button
                class="pill"
                class:active={$width === p.w && $height === p.h}
                onclick={() => { width.set(p.w); height.set(p.h); }}
                title="{p.w}×{p.h}"
              >{p.label}</button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Size</span>
    <div class="dimension-inputs">
      <input type="number" min="1" max="10000" bind:value={$width} />
      <span class="dim-sep">×</span>
      <input type="number" min="1" max="10000" bind:value={$height} />
    </div>
  </div>
  <div class="control-group">
    <span class="label">Mode</span>
    <div class="pills">
      {#each ([["Fit", "Fit"], ["Exact", "Stretch"]] as const) as [value, label]}
        <button class="pill" class:active={$resizeMode === value} onclick={() => resizeMode.set(value)}>
          {label}
        </button>
      {/each}
    </div>
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

<style>
  .preset-sections {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preset-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .preset-group-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    opacity: 0.7;
  }

  .preset-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .dimension-inputs {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dimension-inputs input {
    width: 72px;
    padding: 6px 8px;
    border-radius: 6px;
    font-size: 13px;
    border: 1px solid var(--border);
    background: var(--navy-bg);
    color: var(--text-primary);
    text-align: center;
  }

  .dim-sep {
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
