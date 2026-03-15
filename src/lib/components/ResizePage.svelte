<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import { runWithConcurrency, browseOutputDir, openOutputFolder } from "$lib/utils";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import {
    files, isResizing, outputDir, resizeMode, width, height, summary,
    addFiles, removeFile, clearFiles,
    type ResizeFile,
  } from "$lib/stores/resize";

  const presets = [
    { label: "Full HD", w: 1920, h: 1080 },
    { label: "HD", w: 1280, h: 720 },
    { label: "Instagram", w: 1080, h: 1080 },
    { label: "Thumbnail", w: 300, h: 300 },
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

  async function handleOpenOutputFolder() {
    const firstOutput = $files.find((f) => f.outputPath)?.outputPath;
    if (firstOutput) await openOutputFolder(firstOutput);
  }

  async function resizeFile(file: ResizeFile, w: number, h: number, mode: string, outDir: string | null): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "resizing" as const } : f)
    );
    try {
      const results = await invoke<ResizeResult[]>("resize_images", {
        paths: [file.path],
        width: w,
        height: h,
        mode,
        outputDir: outDir,
      });
      const result = results[0];
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (!result) return { ...f, status: "error" as const, error: "No result returned" };
          if (result.error) return { ...f, status: "error" as const, error: result.error };
          return {
            ...f, status: "done" as const,
            outputPath: result.output_path,
            outputWidth: result.new_width, outputHeight: result.new_height,
            originalWidth: result.original_width, originalHeight: result.original_height,
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
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
    await runWithConcurrency(currentFiles, (file) => resizeFile(file, w, h, mode, outDir));
    isResizing.set(false);
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
  onopenfolder={$summary.done > 0 && $summary.pending === 0 ? handleOpenOutputFolder : undefined}
  actionLabel="Resize {$files.length > 1 ? 'All' : ''}"
  onaction={startResize}
  {headerText}
>
  {#snippet fileDetail(file)}
    {#if file.status === "done"}
      {file.originalWidth}×{file.originalHeight} → {file.outputWidth}×{file.outputHeight}
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
    <div class="preset-grid">
      {#each presets as p}
        <button
          class="pill"
          class:active={$width === p.w && $height === p.h}
          onclick={() => { width.set(p.w); height.set(p.h); }}
        >{p.label}</button>
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
</ToolShell>

<style>
  .preset-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .dimension-inputs {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dimension-inputs input {
    width: 72px;
    padding: 5px 8px;
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
