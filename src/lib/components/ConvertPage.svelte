<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CircleCheck, CircleAlert, X } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import { formatBytes, runWithConcurrency, browseOutputDir, openOutputFolder } from "$lib/utils";
  import {
    files, targetFormat, outputDir, isConverting, summary,
    addFiles, removeFile, clearFiles,
    type TargetFormat, type ConvertFile,
  } from "$lib/stores/convert";
  import { IMAGE_EXTENSIONS } from "$lib/types";

  let quality = $state(90);

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

  async function convertFile(file: ConvertFile, fmt: TargetFormat, q: number): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "converting" as const } : f)
    );
    try {
      const results = await invoke<ConversionResult[]>("convert_images", {
        paths: [file.path],
        format: fmt,
        quality: q,
        outputDir: $outputDir,
      });
      const result = results[0];
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (!result) return { ...f, status: "error" as const, error: "No result returned" };
          if (result.error) return { ...f, status: "error" as const, error: result.error };
          return { ...f, status: "done" as const, outputPath: result.output_path, outputSize: result.compressed_size, size: result.original_size };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  async function startConversion() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isConverting.set(true);
    const fmt = $targetFormat;
    const q = quality;
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    await runWithConcurrency(currentFiles, (file) => convertFile(file, fmt, q));
    isConverting.set(false);
  }
</script>

<ToolShell
  files={$files}
  isProcessing={$isConverting}
  {targetPct}
  progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={clearFiles}
  onopenfolder={$summary.done > 0 && $summary.pending === 0 ? handleOpenOutputFolder : undefined}
  actionLabel="Convert {$files.length > 1 ? 'All' : ''}"
  onaction={startConversion}
  {headerText}
>
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
    <span class="label">To</span>
    <div class="pills">
      {#each formats as f}
        <button class="pill" class:active={$targetFormat === f.value} onclick={() => targetFormat.set(f.value)}>
          {f.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    {#if $targetFormat === "Png"}
      <span class="label">Quality</span>
      <span class="hint">Lossless — no quality setting</span>
    {:else}
      <label for="quality-slider">Quality <span class="quality-value">{quality}%</span></label>
      <input id="quality-slider" type="range" min="10" max="100" step="5" bind:value={quality} />
    {/if}
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
  .hint {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
