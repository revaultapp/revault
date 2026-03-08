<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import {
    files, quality, format, isCompressing, summary,
    addFiles, removeFile, clearFiles,
    type OutputFormat, type CompressFile,
  } from "$lib/stores/compress";
  import { X, Upload, CheckCircle, AlertCircle, Clock, Trash2 } from "lucide-svelte";

  interface CompressionResult {
    input_path: string;
    output_path: string;
    original_size: number;
    compressed_size: number;
    error: string | null;
  }

  const IMAGE_EXTENSIONS = /\.(jpe?g|png|webp|heic|heif|tiff?|bmp|gif)$/i;

  const formats: { value: OutputFormat | null; label: string }[] = [
    { value: null, label: "Auto" },
    { value: "Jpeg", label: "JPEG" },
    { value: "Png", label: "PNG" },
    { value: "Webp", label: "WebP" },
  ];

  let isDragging = $state(false);
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        const paths = event.payload.paths.filter((p) => IMAGE_EXTENSIONS.test(p));
        if (paths.length > 0) addFiles(paths);
      } else {
        isDragging = false;
      }
    });
  });

  onDestroy(() => unlisten?.());

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "bmp", "gif"] }],
    });
    if (selected) addFiles(selected);
  }

  async function startCompression() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isCompressing.set(true);

    const paths = currentFiles.map((f) => f.path);
    const q = $quality;
    const fmt = $format;

    files.update((all) => all.map((f) => ({ ...f, status: "compressing" as const })));

    try {
      const results = await invoke<CompressionResult[]>("compress_images", {
        paths,
        quality: q,
        format: fmt,
      });

      files.update((all) =>
        all.map((f) => {
          const result = results.find((r) => r.input_path === f.path);
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
          f.status === "compressing"
            ? { ...f, status: "error" as const, error: String(err) }
            : f,
        ),
      );
    } finally {
      isCompressing.set(false);
    }
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
  <!-- Empty State -->
  <div class="empty" class:dragging={isDragging} role="button" tabindex="0" onclick={browseFiles} onkeydown={(e) => e.key === "Enter" && browseFiles()}>
    <div class="drop-zone">
      <Upload size={40} strokeWidth={1.5} />
      <p class="drop-title">Drop images here</p>
      <p class="drop-sub">or click to browse</p>
      <div class="format-tags">
        <span class="tag">JPEG</span>
        <span class="tag">PNG</span>
        <span class="tag">WebP</span>
        <span class="tag">HEIC</span>
        <span class="tag">TIFF</span>
        <span class="tag">BMP</span>
        <span class="tag">GIF</span>
      </div>
    </div>
  </div>
{:else}
  <!-- File List State -->
  <div class="compress-view">
    <div class="header">
      <div class="header-left">
        <h2>
          {#if $isCompressing}
            Compressing {$files.length} image{$files.length > 1 ? "s" : ""}...
          {:else if $summary.done > 0 || $summary.failed > 0}
            {$summary.done} of {$files.length} compressed
            {#if $summary.failed > 0}
              · {$summary.failed} failed
            {/if}
          {:else}
            {$files.length} image{$files.length > 1 ? "s" : ""} selected
          {/if}
        </h2>
        {#if $summary.savedBytes > 0}
          <span class="saved-total">Saved {formatBytes($summary.savedBytes)}</span>
        {/if}
      </div>
      <div class="header-actions">
        <button class="btn-ghost" onclick={browseFiles}>Add more</button>
        <button class="btn-ghost danger" onclick={clearFiles}>
          <Trash2 size={14} />
          Clear
        </button>
      </div>
    </div>

    {#if $isCompressing || $summary.done > 0}
      <div class="progress-bar">
        <div class="progress-fill" style="width: {(($summary.done + $summary.failed) / $files.length) * 100}%"></div>
      </div>
    {/if}

    <div class="file-list">
      {#each $files as file (file.path)}
        <div class="file-row" class:active={file.status === "compressing"} class:failed={file.status === "error"}>
          <div class="file-info">
            <span class="file-name">{file.name}</span>
            <span class="file-detail">
              {#if file.status === "done"}
                {formatBytes(file.size)} → {formatBytes(file.compressedSize ?? 0)} · {savedPercent(file)}
              {:else if file.status === "error"}
                {file.error}
              {:else if file.status === "compressing"}
                Compressing...
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
            {:else if file.status === "compressing"}
              <div class="spinner"></div>
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
      <button class="btn-primary" onclick={startCompression} disabled={$isCompressing}>
        {#if $isCompressing}
          Compressing...
        {:else}
          Compress {$files.length > 1 ? "All" : ""}
        {/if}
      </button>
    </div>
  </div>
{/if}

<style>
  /* Empty State */
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    cursor: pointer;
  }

  .drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 80px;
    border: 2px dashed var(--border);
    border-radius: 16px;
    color: var(--text-muted);
    transition: border-color 0.2s, color 0.2s;
  }

  .empty:hover .drop-zone,
  .empty.dragging .drop-zone {
    border-color: var(--accent);
    color: var(--accent);
  }

  .drop-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .drop-sub {
    font-size: 13px;
    color: var(--text-muted);
  }

  .format-tags {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .tag {
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    background: var(--navy-bg);
    color: var(--text-secondary);
  }

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

  .btn-ghost.danger:hover {
    color: #ef4444;
    border-color: #ef4444;
  }

  /* Progress Bar */
  .progress-bar {
    height: 4px;
    border-radius: 2px;
    background: var(--navy-bg);
  }

  .progress-fill {
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.3s ease;
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

  .file-row.active {
    outline: 1px solid var(--accent);
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

  /* Spinner */
  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
