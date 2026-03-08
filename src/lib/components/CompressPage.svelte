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

  let displayPct = $state(0);

  $effect(() => {
    if (!$isCompressing) return;

    displayPct = 0;
    let lastRealPct = 0;
    let lastCompletionTime = performance.now();
    let rafId: number;

    function tick() {
      const total = $files.length;
      if (total === 0) return;

      const completed = $summary.done + $summary.failed;
      const realPct = (completed / total) * 100;
      const stepSize = 100 / total;

      if (realPct > lastRealPct) {
        lastRealPct = realPct;
        lastCompletionTime = performance.now();
      }

      // Between completions, creep forward slowly over ~2.5s
      const elapsed = performance.now() - lastCompletionTime;
      const creep = Math.min(elapsed / 2500, 0.85) * stepSize;
      const target = Math.min(realPct + creep, realPct >= 100 ? 100 : 99);

      // Exponential smoothing — buttery 60fps
      displayPct += (target - displayPct) * 0.06;

      if (realPct >= 100 && displayPct > 99.5) {
        displayPct = 100;
        return;
      }

      rafId = requestAnimationFrame(tick);
    }

    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId);
  });

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

  async function compressFile(file: CompressFile, q: number, fmt: OutputFormat | null): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "compressing" as const } : f),
    );

    try {
      const results = await invoke<CompressionResult[]>("compress_images", {
        paths: [file.path],
        quality: q,
        format: fmt,
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

    // Leave 2 cores free for OS + UI thread to prevent beach ball
    const concurrency = Math.min(
      Math.max(2, (navigator.hardwareConcurrency || 4) - 2),
      currentFiles.length,
    );

    // Mark all as pending, yield to let UI paint before heavy work
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
{:else if $isCompressing}
  <!-- Compressing State — circular progress -->
  {@const circumference = 2 * Math.PI * 54}
  {@const offset = circumference - (displayPct / 100) * circumference}
  <div class="progress-screen">
    <div class="circle-wrap">
      <svg width="180" height="180" viewBox="0 0 120 120">
        <circle cx="60" cy="60" r="54" fill="none" stroke="var(--navy-bg)" stroke-width="6" />
        <circle
          cx="60" cy="60" r="54" fill="none"
          stroke="var(--accent)" stroke-width="6"
          stroke-linecap="round"
          stroke-dasharray={circumference}
          stroke-dashoffset={offset}
          transform="rotate(-90 60 60)"
          class="arc"
        />
      </svg>
      <span class="pct">{Math.round(displayPct)}<small>%</small></span>
    </div>
    <p class="progress-label">{$summary.done + $summary.failed} of {$files.length} files</p>
    {#if $summary.savedBytes > 0}
      <p class="progress-saved">Saved {formatBytes($summary.savedBytes)}</p>
    {/if}
  </div>
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
        <button class="btn-ghost" onclick={browseFiles}>Add more</button>
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
      <button class="btn-primary" onclick={startCompression}>
        Compress {$files.length > 1 ? "All" : ""}
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

  /* Progress Screen */
  .progress-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    gap: 20px;
  }

  .circle-wrap {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .circle-wrap svg {
    filter: drop-shadow(0 0 10px rgba(16, 185, 129, 0.25));
    animation: glow-pulse 2.5s ease-in-out infinite;
  }

  .arc {
    filter: drop-shadow(0 0 4px rgba(16, 185, 129, 0.5));
  }

  @keyframes glow-pulse {
    0%, 100% { filter: drop-shadow(0 0 8px rgba(16, 185, 129, 0.2)); }
    50% { filter: drop-shadow(0 0 16px rgba(16, 185, 129, 0.45)); }
  }

  .pct {
    position: absolute;
    font-size: 38px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .pct small {
    font-size: 18px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .progress-label {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .progress-saved {
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
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

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
