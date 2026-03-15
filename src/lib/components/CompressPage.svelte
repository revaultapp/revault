<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { FolderOpen, CheckCircle, AlertCircle, X } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import { formatBytes, runWithConcurrency } from "$lib/utils";
  import {
    files, quality, format, outputDir, isCompressing, summary,
    addFiles, removeFile, clearFiles,
    type OutputFormat, type CompressFile,
  } from "$lib/stores/compress";
  import { savings } from "$lib/stores/savings";
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
    error: string | null;
  }

  const formats: { value: OutputFormat | null; label: string }[] = [
    { value: null, label: "Auto" },
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

  async function browseOutputDir() {
    const selected = await open({ directory: true });
    if (selected) outputDir.set(selected);
  }

  async function openOutputFolder() {
    const firstOutput = $files.find((f) => f.outputPath)?.outputPath;
    if (firstOutput) await revealItemInDir(firstOutput);
  }

  async function compressFile(file: CompressFile, q: number, fmt: OutputFormat | null): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "compressing" as const } : f)
    );
    try {
      const results = await invoke<CompressionResult[]>("compress_images", {
        paths: [file.path],
        quality: q,
        format: fmt,
        outputDir: $outputDir,
      });
      const result = results[0];
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (!result) return { ...f, status: "error" as const, error: "No result returned" };
          if (result.error) return { ...f, status: "error" as const, error: result.error, size: result.original_size };
          return { ...f, status: "done" as const, compressedSize: result.compressed_size, outputPath: result.output_path, size: result.original_size };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  async function startCompression() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isCompressing.set(true);
    const q = $quality;
    const fmt = $format;
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    await runWithConcurrency(currentFiles, (file) => compressFile(file, q, fmt));
    savings.add($summary.savedBytes);
    isCompressing.set(false);
  }

  function savedPercent(file: CompressFile): string {
    if (!file.compressedSize || file.size === 0) return "";
    return `${Math.round(((file.size - file.compressedSize) / file.size) * 100)}% smaller`;
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
  onclear={clearFiles}
  onopenfolder={$summary.done > 0 && $summary.pending === 0 ? openOutputFolder : undefined}
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
    <label for="quality-slider">Quality <span class="quality-value">{$quality}%</span></label>
    <input id="quality-slider" type="range" min="10" max="100" step="5" bind:value={$quality} />
  </div>
  <div class="control-group">
    <span class="label">Output</span>
    <button class="btn-ghost output-btn" onclick={browseOutputDir}>
      <FolderOpen size={14} />
      {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
    </button>
  </div>
</ToolShell>

<style>
  .saved-total {
    font-size: 13px;
    color: var(--accent);
    font-weight: 500;
  }
</style>
