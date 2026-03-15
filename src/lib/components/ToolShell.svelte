<script lang="ts" generics="T extends BaseFile">
  import type { Snippet } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { FolderOpen, Trash2, ImageIcon } from "lucide-svelte";
  import DropZone from "./DropZone.svelte";
  import ProgressRing from "./ProgressRing.svelte";
  import type { BaseFile } from "$lib/types";

  let thumbnails = $state<Record<string, string>>({});

  interface Props {
    files: T[];
    isProcessing: boolean;
    targetPct: number;
    progressLabel: string;
    progressSublabel?: string;
    onfiles: (paths: string[]) => void;
    onbrowse: () => void;
    onclear: () => void;
    onopenfolder?: () => void;
    actionLabel: string;
    onaction: () => void;
    headerText: string;
    headerSub?: Snippet;
    children?: Snippet;
    fileDetail: Snippet<[T]>;
    fileStatus: Snippet<[T]>;
  }

  let {
    files,
    isProcessing,
    targetPct,
    progressLabel,
    progressSublabel,
    onfiles,
    onbrowse,
    onclear,
    onopenfolder,
    actionLabel,
    onaction,
    headerText,
    headerSub,
    children,
    fileDetail,
    fileStatus,
  }: Props = $props();

  $effect(() => {
    for (const file of files) {
      if (thumbnails[file.path] !== undefined) continue;
      thumbnails[file.path] = ""; // mark as loading
      invoke<string>("generate_thumbnail", { path: file.path })
        .then((src) => { thumbnails[file.path] = src; })
        .catch(() => { thumbnails[file.path] = "error"; });
    }
  });
</script>

{#if files.length === 0}
  <DropZone {onfiles} />
{:else if isProcessing}
  <ProgressRing {targetPct} label={progressLabel} sublabel={progressSublabel} />
{:else}
  <div class="tool-view">
    <div class="header">
      <div class="header-left">
        <h2>{headerText}</h2>
        {#if headerSub}{@render headerSub()}{/if}
      </div>
      <div class="header-actions">
        {#if onopenfolder}
          <button class="btn-ghost" onclick={onopenfolder}>
            <FolderOpen size={14} />
            Open Folder
          </button>
        {/if}
        <button class="btn-ghost" onclick={onbrowse}>Add more</button>
        <button class="btn-ghost danger" onclick={onclear}>
          <Trash2 size={14} />
          Clear
        </button>
      </div>
    </div>

    <div class="file-list">
      {#each files as file (file.path)}
        <div class="file-row" class:failed={file.status === "error"}>
          {#if thumbnails[file.path] && thumbnails[file.path] !== "error"}
            <img class="file-thumb" src={thumbnails[file.path]} alt="" draggable="false" />
          {:else}
            <div class="file-thumb placeholder"><ImageIcon size={18} /></div>
          {/if}
          <div class="file-info">
            <span class="file-name">{file.name}</span>
            <span class="file-detail">{@render fileDetail(file)}</span>
          </div>
          <div class="file-status">{@render fileStatus(file)}</div>
        </div>
      {/each}
    </div>

    <div class="controls">
      {#if children}{@render children()}{/if}
      <button class="btn-primary" onclick={onaction}>{actionLabel}</button>
    </div>
  </div>
{/if}

<style>
  .tool-view {
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

  .header-left {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .header h2 {
    font-size: 18px;
    font-weight: 600;
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

  .btn-ghost:hover { background: var(--navy-bg); }
  .btn-ghost.danger:hover { color: #ef4444; border-color: #ef4444; }

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
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    transition: background 0.15s;
  }

  .file-row.failed { background: rgba(239, 68, 68, 0.06); }

  .file-thumb {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border);
  }

  .file-thumb.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--navy-bg);
    color: var(--text-muted);
  }

  .file-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .file-name {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-detail { font-size: 12px; color: var(--text-muted); }
  .file-row.failed .file-detail { color: #ef4444; }

  .file-status {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .file-status :global(svg) { color: var(--accent); }
  .file-row.failed .file-status :global(svg) { color: #ef4444; }

  .file-status :global(.btn-icon) {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    transition: color 0.15s;
  }

  .file-status :global(.btn-icon:hover) { color: #ef4444; }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 24px;
    row-gap: 12px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-radius: 12px;
    border: 1px solid var(--border);
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

  .btn-primary:hover { opacity: 0.9; }

  /*
   * Shared slot styles — these :global() rules style content passed via
   * Svelte snippets from CompressPage, ConvertPage, ResizePage.
   * Child components MUST use these class names: .control-group, .pill,
   * .quality-value, .btn-ghost, .output-btn, .btn-icon
   * This is an intentional design contract, not an accident.
   */
  .controls :global(.control-group) {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .controls :global(.control-group label),
  .controls :global(.control-group .label) {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .controls :global(.pills) { display: flex; gap: 4px; }

  .controls :global(.pill) {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--navy-bg);
    transition: background 0.15s, color 0.15s;
  }

  .controls :global(.pill.active) { background: var(--accent); color: #fff; }

  .controls :global(.quality-value) { color: var(--accent); text-transform: none; letter-spacing: 0; }

  .controls :global(input[type="range"]) { width: 160px; accent-color: var(--accent); }

  .controls :global(.btn-ghost) {
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

  .controls :global(.btn-ghost:hover) { background: var(--navy-bg); }

  .controls :global(.output-btn) {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
