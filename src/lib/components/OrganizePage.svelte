<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, X, Files, FolderInput, FolderOutput } from "lucide-svelte";
  import {
    activeTab, renameFiles, template, sourceDir, destDir, organizeMode,
    isOrganizing, organizeResult, renameSummary,
    addRenameFiles, removeRenameFile, clearRenameFiles,
    setOrganizeResult, clearOrganizeResult,
    type OrganizeFile,
  } from "$lib/stores/organize";
  import { activity } from "$lib/stores/activity";
  import { IMAGE_EXTENSIONS_RE } from "$lib/types";
  import ToolShell from "./ToolShell.svelte";

  // Tauri drag-and-drop adds a non-standard .path property to File objects
  interface FileWithPath extends File {
    path?: string;
  }

  // Progress state for streaming organize
  let progress = $state<{ processed: number; total: number; currentFile: string; moved: number; skipped: number } | null>(null);

  let progressPct = $derived(
    progress && progress.total > 0
      ? Math.round((progress.processed / progress.total) * 100)
      : 0
  );

  const tokens = [
    { label: "{name}", desc: "Original filename" },
    { label: "{counter}", desc: "Sequential number (001)" },
    { label: "{counter:5}", desc: "Counter with N digits" },
    { label: "{date}", desc: "EXIF date (2024-01-15)" },
    { label: "{datetime}", desc: "Date + time" },
    { label: "{year}", desc: "Year from date" },
    { label: "{month}", desc: "Month (01-12)" },
    { label: "{day}", desc: "Day (01-31)" },
    { label: "{ext}", desc: "File extension" },
  ];

  function appendToken(token: string) {
    template.update((t) => t + token);
  }

  async function browseRenameFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["jpg","jpeg","png","webp","heic","heif","tiff","bmp","avif","jxl"] }],
    });
    if (selected) addRenameFiles(selected);
  }

  async function browseSourceDir() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") sourceDir.set(dir);
  }

  async function browseDestDir() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") destDir.set(dir);
  }

  function previewRename(file: OrganizeFile, tmpl: string): string {
    const name = file.name.replace(/\.[^.]+$/, "");
    const ext = file.name.split(".").pop() ?? "";
    // Simple client-side preview — counter will be 001
    return tmpl
      .replace("{name}", name)
      .replace("{ext}", ext)
      .replace(/\{counter(?::\d+)?\}/g, "001")
      .replace("{date}", "YYYY-MM-DD")
      .replace("{datetime}", "YYYY-MM-DD_HH-mm-ss")
      .replace("{year}", "YYYY")
      .replace("{month}", "MM")
      .replace("{day}", "DD");
  }

  interface RenameResult {
    original_path: string;
    new_path: string;
    success: boolean;
    error: string | null;
  }

  async function startRename() {
    const files = $renameFiles.filter((f) => f.status === "pending");
    if (files.length === 0) return;

    renameFiles.update((all) => all.map((f) => f.status === "pending" ? { ...f, status: "done" as const } : f));

    const requests = files.map((f) => ({ original_path: f.path, template: $template }));
    try {
      const results = await invoke<RenameResult[]>("rename_batch", { requests });
      renameFiles.update((all) =>
        all.map((f) => {
          const r = results.find((r) => r.original_path === f.path);
          if (!r) return f;
          if (r.success) return { ...f, status: "done" as const, newPath: r.new_path };
          return { ...f, status: "error" as const, error: r.error ?? "Unknown error" };
        })
      );
      const successCount = results.filter((r) => r.success).length;
      if (successCount > 0) {
        activity.add({ type: "rename", fileCount: successCount, savedBytes: 0 });
      }
    } catch (err) {
      renameFiles.update((all) =>
        all.map((f) => f.status === "pending" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  interface OrganizeRes {
    moved: number;
    skipped: number;
    errors: string[];
  }

  async function startOrganize() {
    if (!$sourceDir || !$destDir) return;
    isOrganizing.set(true);
    clearOrganizeResult();
    progress = null;

    let unlistenProgress: UnlistenFn | undefined;
    let unlistenComplete: UnlistenFn | undefined;

    try {
      // Listen to organize-progress events
      unlistenProgress = await listen<{ processed: number; total: number; current_file: string; moved: number; skipped: number }>("organize-progress", (event) => {
        progress = {
          processed: event.payload.processed,
          total: event.payload.total,
          currentFile: event.payload.current_file,
          moved: event.payload.moved,
          skipped: event.payload.skipped,
        };
      });

      // Listen to organize-complete event
      unlistenComplete = await listen<OrganizeRes>("organize-complete", (event) => {
        setOrganizeResult({ moved: event.payload.moved, skipped: event.payload.skipped, errors: event.payload.errors });
        if (event.payload.moved > 0) {
          activity.add({ type: "organize", fileCount: event.payload.moved, savedBytes: 0 });
        }
      });

      // Call the streaming command (fire-and-forget, results come via events)
      await invoke("organize_by_date_stream", {
        source: $sourceDir,
        dest: $destDir,
        copy: $organizeMode === "copy",
      });
    } catch (err) {
      setOrganizeResult({ moved: 0, skipped: 0, errors: [String(err)] });
    } finally {
      isOrganizing.set(false);
      progress = null;
      unlistenProgress?.();
      unlistenComplete?.();
    }
  }

  function handleClear() {
    clearRenameFiles();
    clearOrganizeResult();
  }

  let headerText = $derived(
    $activeTab === "rename"
      ? $renameSummary.done + $renameSummary.failed + $renameSummary.pending > 0
        ? `${$renameSummary.done} renamed${$renameSummary.failed > 0 ? ` · ${$renameSummary.failed} failed` : ""}`
        : `${$renameFiles.length} file${$renameFiles.length !== 1 ? "s" : ""} selected`
      : $organizeResult
        ? `${$organizeResult.moved} moved${$organizeResult.skipped > 0 ? ` · ${$organizeResult.skipped} skipped` : ""}`
        : $sourceDir
          ? "Ready to organize"
          : "Select source folder"
  );
</script>

<div class="page">
  <div class="page-header">
    <div class="tab-row">
      <button class="tab" class:active={$activeTab === "rename"} onclick={() => activeTab.set("rename")}>
        <Files size={15} />
        Rename
      </button>
      <button class="tab" class:active={$activeTab === "organize"} onclick={() => activeTab.set("organize")}>
        <FolderInput size={15} />
        Organize by Date
      </button>
    </div>
  </div>

  {#if $activeTab === "rename"}
    {#if $renameFiles.length === 0}
      <div class="empty-state">
        <div class="drop-zone" role="button" tabindex="0"
          ondragover={(e) => e.preventDefault()}
          ondrop={(e) => { e.preventDefault(); const files = e.dataTransfer?.files; if (files) { addRenameFiles(Array.from(files as unknown as FileWithPath[]).map((f) => f.path).filter((p): p is string => !!p && IMAGE_EXTENSIONS_RE.test(p))); } }}
          onclick={browseRenameFiles}
          onkeydown={(e) => e.key === "Enter" && browseRenameFiles()}>
          <Files size={40} strokeWidth={1.5} />
          <p>Drop images here or click to browse</p>
          <span>JPEG, PNG, WebP, HEIC, TIFF, AVIF</span>
        </div>
      </div>
    {:else}
      <ToolShell
        files={$renameFiles}
        isProcessing={false}
        targetPct={0}
        progressLabel=""
        onfiles={(paths) => addRenameFiles(paths)}
        onbrowse={browseRenameFiles}
        onclear={handleClear}
        actionLabel="Rename All"
        onaction={startRename}
        {headerText}
      >
        {#snippet fileDetail(file)}
          {#if file.status === "done"}
            {file.newPath?.split(/[\\/]/).pop() ?? "Renamed"}
          {:else if file.status === "error"}
            {file.error}
          {:else}
            {previewRename(file, $template)}
          {/if}
        {/snippet}

        {#snippet fileStatus(file)}
          {#if file.status === "done"}
            <CheckCircle size={18} />
          {:else if file.status === "error"}
            <AlertCircle size={18} />
          {:else}
            <button class="btn-icon" onclick={() => removeRenameFile(file.path)}>
              <X size={16} />
            </button>
          {/if}
        {/snippet}

        <div class="control-group template-group">
          <span class="label">Template</span>
          <div class="token-input-row">
            <input
              class="template-input"
              type="text"
              bind:value={$template}
              placeholder="e.g. photo_001.jpg"
            />
          </div>
          <div class="token-chips">
            {#each tokens as token}
              <button class="token-chip" onclick={() => appendToken(token.label)} title={token.desc}>
                {token.label}
              </button>
            {/each}
          </div>
        </div>
      </ToolShell>
    {/if}

  {:else}
    <div class="organize-panel">
      <div class="organize-header">
        <h2>{progress ? `Processing ${progress.processed} of ${progress.total}` : headerText}</h2>
      </div>

      {#if progress}
        <div class="organize-progress">
          <div class="progress-bar-wrap">
            <div class="progress-bar" style="width: {progressPct}%"></div>
          </div>
          <p class="progress-detail">
            {#if progress.currentFile}
              <span class="progress-filename">{progress.currentFile.split(/[\\/]/).pop()}</span>
            {/if}
            {#if progress.moved > 0 || progress.skipped > 0}
              <span class="progress-stats">{progress.moved} moved · {progress.skipped} skipped</span>
            {/if}
          </p>
        </div>
      {/if}

      <div class="organize-controls">
        <div class="folder-row">
          <div class="folder-picker">
            <span class="label">Source Folder</span>
            <button class="btn-folder" onclick={browseSourceDir}>
              <FolderOpen size={14} />
              {$sourceDir?.split(/[\\/]/).pop() ?? "Select folder..."}
            </button>
            {#if $sourceDir}
              <span class="folder-path">{$sourceDir}</span>
            {/if}
          </div>

          <div class="folder-picker">
            <span class="label">Destination Folder</span>
            <button class="btn-folder" onclick={browseDestDir}>
              <FolderOpen size={14} />
              {$destDir?.split(/[\\/]/).pop() ?? "Select folder..."}
            </button>
            {#if $destDir}
              <span class="folder-path">{$destDir}</span>
            {/if}
          </div>
        </div>

        <div class="mode-row">
          <span class="label">Mode</span>
          <div class="mode-pills">
            <button class="pill" class:active={$organizeMode === "copy"} onclick={() => organizeMode.set("copy")}>
              <FolderOutput size={14} />
              Copy
            </button>
            <button class="pill" class:active={$organizeMode === "move"} onclick={() => organizeMode.set("move")}>
              <FolderInput size={14} />
              Move
            </button>
          </div>
        </div>

        {#if $organizeResult}
          <div class="result-card" class:has-errors={$organizeResult.errors.length > 0}>
            <div class="result-stat">
              <span class="stat-value">{$organizeResult.moved}</span>
              <span class="stat-label">Files organized</span>
            </div>
            {#if $organizeResult.skipped > 0}
              <div class="result-stat">
                <span class="stat-value">{$organizeResult.skipped}</span>
                <span class="stat-label">Skipped</span>
              </div>
            {/if}
            {#if $organizeResult.errors.length > 0}
              <div class="result-errors">
                {#each $organizeResult.errors.slice(0, 5) as err}
                  <p class="error-item">{err}</p>
                {/each}
                {#if $organizeResult.errors.length > 5}
                  <p class="error-item">...and {$organizeResult.errors.length - 5} more</p>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

        <button
          class="btn-primary"
          onclick={startOrganize}
          disabled={!$sourceDir || !$destDir || $isOrganizing}
        >
          {$isOrganizing ? "Organizing..." : $organizeMode === "copy" ? "Copy to Folder" : "Move to Folder"}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 16px;
    padding: 28px;
    overflow-y: auto;
  }

  .page-header {
    display: flex;
    align-items: center;
  }

  .tab-row {
    display: flex;
    gap: 4px;
    background: var(--navy-bg);
    padding: 4px;
    border-radius: 10px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    transition: background 0.15s, color 0.15s;
  }

  .tab:hover { color: var(--text-secondary); }

  .tab.active {
    background: var(--bg-card);
    color: var(--text-primary);
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    width: 480px;
    height: 280px;
    border: 2px dashed var(--border);
    border-radius: 16px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    color: var(--text-muted);
  }

  .drop-zone:hover {
    border-color: var(--accent);
    background: rgba(16, 185, 129, 0.04);
    color: var(--accent);
  }

  .drop-zone p {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .drop-zone span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .token-input-row {
    display: flex;
    gap: 8px;
  }

  .template-input {
    flex: 1;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-main);
    color: var(--text-primary);
    font-size: 13px;
    font-family: monospace;
    min-width: 240px;
  }

  .template-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .token-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .token-chip {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-family: monospace;
    background: var(--navy-bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background 0.15s, color 0.15s;
  }

  .token-chip:hover {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .template-group {
    min-width: 300px;
  }

  .btn-icon {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    transition: color 0.15s;
  }

  .btn-icon:hover { color: var(--accent); }

  /* Organize tab */
  .organize-panel {
    display: flex;
    flex-direction: column;
    gap: 20px;
    flex: 1;
  }

  .organize-header h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  .organize-controls {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 600px;
  }

  .folder-row {
    display: flex;
    gap: 20px;
  }

  .folder-picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
  }

  .btn-folder {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    transition: background 0.15s;
    cursor: pointer;
    max-width: 280px;
  }

  .btn-folder:hover { background: var(--navy-bg); }

  .folder-path {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 280px;
  }

  .mode-row {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mode-pills {
    display: flex;
    gap: 4px;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--navy-bg);
    transition: background 0.15s, color 0.15s;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .pill.active {
    background: var(--accent);
    color: #fff;
  }

  .result-card {
    display: flex;
    gap: 24px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-radius: 12px;
    border: 1px solid var(--border);
  }

  .result-card.has-errors {
    border-color: var(--danger);
    background: var(--danger-bg);
  }

  .result-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-value {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
  }

  .has-errors .stat-value { color: var(--danger); }

  .stat-label {
    font-size: 12px;
    color: var(--text-muted);
  }

  .result-errors {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-left: auto;
  }

  .error-item {
    font-size: 12px;
    color: var(--danger);
    margin: 0;
  }

  .btn-primary {
    align-self: flex-start;
    padding: 10px 28px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    transition: opacity 0.15s;
    cursor: pointer;
    border: none;
  }

  .btn-primary:hover:not(:disabled) { opacity: 0.9; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Organize progress */
  .organize-progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-radius: 12px;
    border: 1px solid var(--border);
    max-width: 600px;
  }

  .progress-bar-wrap {
    height: 8px;
    background: var(--navy-bg);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--accent);
    border-radius: 4px;
    transition: width 0.2s ease;
  }

  .progress-detail {
    display: flex;
    gap: 12px;
    align-items: center;
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .progress-filename {
    color: var(--text-secondary);
    font-family: monospace;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .progress-stats {
    margin-left: auto;
  }
</style>
