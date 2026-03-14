<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    files, quality, format, outputDir, isCompressing, summary,
    addFiles, removeFile, clearFiles,
    type OutputFormat, type CompressFile,
  } from "$lib/stores/compress";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { X, CheckCircle, AlertCircle, Trash2, FolderOpen } from "lucide-svelte";
  import DropZone from "./DropZone.svelte";
  import ProgressRing from "./ProgressRing.svelte";

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
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
  ];

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "bmp", "gif"] }],
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
      all.map((f) => f.path === file.path ? { ...f, status: "compressing" as const } : f),
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
          if (result.error) {
            return { ...f, status: "error" as const, error: result.error, size: result.original_size };
          }
          return {
            ...f,
            status: "done" as const,
            compressedSize: result.compressed_size,
            outputPath: result.output_path,
            size: result.original_size,
          };
        }),
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) =>
          f.path === file.path
            ? { ...f, status: "error" as const, error: String(err) }
            : f,
        ),
      );
    }
  }

  async function startCompression() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isCompressing.set(true);

    const q = $quality;
    const fmt = $format;

    const concurrency = Math.min(
      Math.max(2, (navigator.hardwareConcurrency || 4) - 2),
      currentFiles.length,
    );

    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    await new Promise((r) => setTimeout(r, 0));

    let nextIndex = 0;
    async function worker() {
      while (nextIndex < currentFiles.length) {
        const file = currentFiles[nextIndex++];
        await compressFile(file, q, fmt);
      }
    }

    await Promise.all(Array.from({ length: concurrency }, () => worker()));
    isCompressing.set(false);
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function savedPercent(file: CompressFile): string {
    if (!file.compressedSize || file.size === 0) return "";
    const pct = Math.round(((file.size - file.compressedSize) / file.size) * 100);
    return `${pct}% smaller`;
  }
</script>

{#if $files.length === 0}
  <DropZone onfiles={(paths) => addFiles(paths)} />
{:else if $isCompressing}
  <ProgressRing
    {targetPct}
    label="{$summary.done + $summary.failed} of {$files.length} files"
    sublabel={$summary.savedBytes > 0 ? `Saved ${formatBytes($summary.savedBytes)}` : undefined}
  />
{:else}
  <!-- File List State -->
  <div class="compress-view">
    <div class="header">
      <div class="header-left">
        <h2>
          {#if $summary.done > 0 || $summary.failed > 0}
            {$summary.done} of {$files.length} compressed
            {#if $summary.failed > 0}· {$summary.failed} failed{/if}
          {:else}
            {$files.length} image{$files.length > 1 ? "s" : ""} selected
          {/if}
        </h2>
        {#if $summary.savedBytes > 0}
          <span class="saved-total">Saved {formatBytes($summary.savedBytes)}</span>
        {/if}
      </div>
      <div class="header-actions">
        {#if $summary.done > 0 && $summary.pending === 0}
          <button class="btn-ghost" onclick={openOutputFolder}>
            <FolderOpen size={14} />
            Open Folder
          </button>
        {:else}
          <button class="btn-ghost" onclick={browseFiles}>Add more</button>
        {/if}
        <button class="btn-ghost danger" onclick={clearFiles}>
          <Trash2 size={14} />
          Clear
        </button>
      </div>
    </div>

    <div class="file-list">
      {#each $files as file (file.path)}
        <div class="file-row" class:failed={file.status === "error"}>
          <div class="file-info">
            <span class="file-name">{file.name}</span>
            <span class="file-detail">
              {#if file.status === "done"}
                {formatBytes(file.size)} → {formatBytes(file.compressedSize ?? 0)} · {savedPercent(file)}
              {:else if file.status === "error"}
                {file.error}
              {:else}
                Ready
              {/if}
            </span>
          </div>
          <div class="file-status">
            {#if file.status === "done"}
              <CheckCircle size={18} />
            {:else if file.status === "error"}
              <AlertCircle size={18} />
            {:else}
              <button class="btn-icon" onclick={() => removeFile(file.path)}>
                <X size={16} />
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <!-- Controls -->
    <div class="controls">
      <div class="control-group">
        <span class="label">Format</span>
        <div class="pills">
          {#each formats as f}
            <button
              class="pill"
              class:active={$format === f.value}
              onclick={() => format.set(f.value)}
            >{f.label}</button>
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
      <button class="btn-primary" onclick={startCompression}>
        Compress {$files.length > 1 ? "All" : ""}
      </button>
    </div>
  </div>
{/if}

<style>
  /* Compress View */
  .compress-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 16px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header h2 {
    font-size: 18px;
    font-weight: 600;
  }

  .saved-total {
    font-size: 13px;
    color: var(--accent);
    font-weight: 500;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .btn-ghost {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .btn-ghost:hover {
    background: var(--navy-bg);
  }

  .output-btn {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn-ghost.danger:hover {
    color: #ef4444;
    border-color: #ef4444;
  }

  /* File List */
  .file-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    min-height: 0;
  }

  .file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    transition: background 0.15s;
  }

  .file-row.failed {
    background: rgba(239, 68, 68, 0.06);
  }

  .file-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .file-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-detail {
    font-size: 12px;
    color: var(--text-muted);
  }

  .file-row.failed .file-detail {
    color: #ef4444;
  }

  .file-status {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .file-status :global(svg) {
    color: var(--accent);
  }

  .file-row.failed .file-status :global(svg) {
    color: #ef4444;
  }

  .btn-icon {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    transition: color 0.15s;
  }

  .btn-icon:hover {
    color: #ef4444;
  }

  /* Controls */
  .controls {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-radius: 12px;
    border: 1px solid var(--border);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .control-group label,
  .control-group .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .quality-value {
    color: var(--accent);
    text-transform: none;
    letter-spacing: 0;
  }

  .pills {
    display: flex;
    gap: 4px;
  }

  .pill {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--navy-bg);
    transition: background 0.15s, color 0.15s;
  }

  .pill.active {
    background: var(--accent);
    color: #fff;
  }

  input[type="range"] {
    width: 160px;
    accent-color: var(--accent);
  }

  .btn-primary {
    margin-left: auto;
    padding: 10px 28px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    transition: opacity 0.15s;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }
</style>
