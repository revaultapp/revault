<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    CircleCheck, CircleAlert, X, FolderOpen, Trash2,
    Minimize2, Combine, Scissors, ArrowUp, ArrowDown, FileText,
  } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import SegmentedControl from "./SegmentedControl.svelte";
  import DropZone from "./DropZone.svelte";
  import Button from "./Button.svelte";
  import PrivacyToast from "./PrivacyToast.svelte";
  import { browseOutputDir, formatBytes } from "$lib/utils";
  import { activity } from "$lib/stores/activity";
  import {
    files, isProcessing, summary, outputDir, stripMetadata, compressStreams,
    compressImages,
    resolvedOutputDir, addFiles, removeFile, clearFiles, processPdfs,
    revealPdfOutput, type PdfFile,
    mergeFiles, isMerging, mergeResult, mergeError,
    addMergeFiles, removeMergeFile, moveMergeFile, clearMerge, mergePdfs,
    splitFile, isSplitting, splitResults, splitError,
    setSplitFile, clearSplit, splitPdf, type SplitKind,
  } from "$lib/stores/pdf";

  let showToast = $state(false);
  let toastMessage = $state("");
  let toastTimer: ReturnType<typeof setTimeout>;

  const PDF_SUPPORTED_EXTENSIONS = ["pdf"] as const;
  const PDF_SUPPORTED_RE = /\.pdf$/i;

  const modes = [
    { id: "optimize", label: "Optimize", icon: Minimize2 },
    { id: "merge", label: "Merge", icon: Combine },
    { id: "split", label: "Split", icon: Scissors },
  ] as const;

  let mode = $state<"optimize" | "merge" | "split">("optimize");

  const rm = $derived(prefersReducedMotion.current);

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

  // --- Optimize (unchanged) ---

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "PDF files", extensions: [...PDF_SUPPORTED_EXTENSIONS] }],
    });
    if (selected) handleAddFiles(Array.isArray(selected) ? selected : [selected]);
  }

  function handleAddFiles(paths: string[]) {
    const existing = new Set($files.map((f) => f.path));
    const newPaths = paths.filter((p) => !existing.has(p) && PDF_SUPPORTED_RE.test(p));
    addFiles(newPaths);
  }

  function showPrivacyToast(count: number) {
    toastMessage = count === 1 ? "Metadata removed from 1 file" : `Metadata removed from ${count} files`;
    showToast = true;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { showToast = false; }, 3000);
  }

  async function startProcess() {
    if ($files.length === 0) return;
    await processPdfs($resolvedOutputDir, $stripMetadata, $compressStreams, $compressImages);
    const doneCount = $summary.done;
    if (doneCount > 0) {
      const savedBytes = $files.reduce((acc, f) => {
        if (f.status === "done" && f.originalSize && f.outputSize) {
          return acc + Math.max(0, f.originalSize - f.outputSize);
        }
        return acc;
      }, 0);
      activity.add({ type: "compress", fileCount: doneCount, savedBytes });
      if ($stripMetadata) showPrivacyToast(doneCount);
    }
  }

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? `${$summary.done} of ${$files.length} processed${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} PDF${$files.length > 1 ? "s" : ""} selected`
  );

  function sizeDelta(file: PdfFile): string {
    const orig = file.originalSize;
    const out = file.outputSize;
    if (!orig || !out) return "";
    const pct = Math.round(((orig - out) / orig) * 100);
    if (pct > 0) return `${formatBytes(orig)} → ${formatBytes(out)} (${pct}% smaller)`;
    if (pct < 0) return `${formatBytes(orig)} → ${formatBytes(out)} (${Math.abs(pct)}% larger)`;
    return `${formatBytes(orig)} → ${formatBytes(out)} (no change)`;
  }

  // --- Merge ---

  async function browseMergeFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "PDF files", extensions: [...PDF_SUPPORTED_EXTENSIONS] }],
    });
    if (selected) handleMergeAdd(Array.isArray(selected) ? selected : [selected]);
  }

  function handleMergeAdd(paths: string[]) {
    const valid = paths.filter((p) => PDF_SUPPORTED_RE.test(p));
    if (valid.length > 0) addMergeFiles(valid);
  }

  async function startMerge() {
    if ($mergeFiles.length < 2) return;
    const fileCount = $mergeFiles.length;
    await mergePdfs($resolvedOutputDir);
    if ($mergeResult) {
      activity.add({ type: "merge", fileCount, savedBytes: 0 });
    }
  }

  // --- Split ---

  const splitModes = [
    { id: "range", label: "Page range" },
    { id: "each", label: "Individual pages" },
  ] as const;

  let splitModeChoice = $state<SplitKind>("range");
  let rangeStart = $state(1);
  let rangeEnd = $state(1);

  let canSplit = $derived(
    splitModeChoice === "each" ||
    (Number.isInteger(rangeStart) && Number.isInteger(rangeEnd) && rangeStart >= 1 && rangeEnd >= rangeStart)
  );

  function handleSplitAdd(paths: string[]) {
    const valid = paths.filter((p) => PDF_SUPPORTED_RE.test(p));
    if (valid.length > 0) setSplitFile(valid[0]);
  }

  async function startSplit() {
    if (!$splitFile || !canSplit) return;
    const start = splitModeChoice === "range" ? rangeStart : undefined;
    const end = splitModeChoice === "range" ? rangeEnd : undefined;
    await splitPdf(splitModeChoice, start, end, $resolvedOutputDir);
    if ($splitResults.length > 0) {
      activity.add({ type: "split", fileCount: $splitResults.length, savedBytes: 0 });
    }
  }
</script>

<div class="pdf-page">
  <div class="mode-header">
    <SegmentedControl segments={modes} bind:selected={mode} label="PDF tool mode" />
  </div>

  <div class="mode-content">
    {#if mode === "optimize"}
      <ToolShell
        files={$files}
        isProcessing={$isProcessing}
        {targetPct}
        progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
        onfiles={handleAddFiles}
        onbrowse={browseFiles}
        onclear={clearFiles}
        dropZoneAcceptedExtensions={PDF_SUPPORTED_RE}
        dropZoneFilePickerName="PDF files"
        dropZoneFilePickerExtensions={[...PDF_SUPPORTED_EXTENSIONS]}
        dropZoneTitle="Drop PDFs here"
        dropZoneFormatTags={["PDF"]}
        placeholderIcon="image"
        actionLabel="Process PDFs"
        onaction={startProcess}
        {headerText}
      >
        {#snippet fileDetail(file)}
          {#if file.status === "processing"}
            Processing...
          {:else if file.status === "error"}
            {file.error}
          {:else if file.status === "done"}
            <span class="size-delta">{sizeDelta(file)}</span>
          {:else}
            Ready
          {/if}
        {/snippet}

        {#snippet fileStatus(file)}
          {#if file.status === "done"}
            <div class="done-actions">
              {#if file.outputPath}
                <button class="btn-icon reveal-btn" aria-label="Reveal in file manager" onclick={() => revealPdfOutput(file.outputPath!)}>
                  <FolderOpen size={16} />
                </button>
              {/if}
              <CircleCheck size={18} />
            </div>
          {:else if file.status === "error"}
            <CircleAlert size={18} />
          {:else}
            <button class="btn-icon" onclick={() => removeFile(file.path)}>
              <X size={16} />
            </button>
          {/if}
        {/snippet}

        <div class="control-group">
          <span class="label">Options</span>
          <div class="pdf-options">
            <label><ToggleSwitch bind:checked={$stripMetadata} label="Strip metadata" /> Strip metadata</label>
            <label><ToggleSwitch bind:checked={$compressStreams} label="Compress streams" /> Compress streams</label>
            <label><ToggleSwitch bind:checked={$compressImages} label="Compress images" /> Compress images <span class="toggle-hint">lossy</span></label>
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
    {:else if mode === "merge"}
      {#if $mergeFiles.length === 0}
        <div class="mode-empty">
          <DropZone
            onfiles={handleMergeAdd}
            dropTitle="Drop PDFs to merge"
            formatTags={["PDF"]}
            acceptedExtensions={PDF_SUPPORTED_RE}
            filePickerName="PDF files"
            filePickerExtensions={[...PDF_SUPPORTED_EXTENSIONS]}
          />
        </div>
      {:else if $mergeResult}
        <div class="result-view">
          <div class="result-card">
            <CircleCheck size={28} color="var(--accent)" />
            <span class="result-name">{$mergeResult.outputPath.split(/[\\/]/).pop()}</span>
            <span class="result-meta">
              {formatBytes($mergeResult.outputSize)} · {$mergeResult.pageCount} page{$mergeResult.pageCount === 1 ? "" : "s"}
            </span>
            <div class="result-actions">
              <button class="btn-primary-sm" onclick={() => revealPdfOutput($mergeResult!.outputPath)}>
                <FolderOpen size={14} />
                Reveal in file manager
              </button>
              <button class="btn-ghost" onclick={clearMerge}>Merge more files</button>
            </div>
          </div>
        </div>
      {:else}
        <div class="tool-view">
          <div class="header">
            <div class="header-left">
              <h2>{$mergeFiles.length} PDF{$mergeFiles.length === 1 ? "" : "s"} to merge</h2>
              <span class="sub">
                {$mergeFiles.length === 1 ? "Add at least one more PDF to enable merging" : "Reorder below — merge follows this order"}
              </span>
            </div>
            <div class="header-actions">
              <button class="btn-ghost" onclick={browseMergeFiles}>Add more</button>
              <button class="btn-ghost danger" onclick={clearMerge}>
                <Trash2 size={14} />
                Clear
              </button>
            </div>
          </div>

          {#if $mergeError}
            <div class="error-card" role="alert">
              <CircleAlert size={14} />
              <span>{$mergeError}</span>
            </div>
          {/if}

          <div class="merge-list">
            {#each $mergeFiles as file, i (file.path)}
              <div
                class="merge-row"
                in:fly={{ y: 8, opacity: 0, duration: rm ? 0 : 220, delay: rm ? 0 : Math.min(i, 9) * 40, easing: cubicOut }}
              >
                <span class="merge-index">{i + 1}</span>
                <FileText size={16} class="merge-file-icon" />
                <span class="merge-name">{file.name}</span>
                <div class="merge-actions">
                  <button class="btn-icon" disabled={i === 0} onclick={() => moveMergeFile(file.path, -1)} aria-label="Move {file.name} up">
                    <ArrowUp size={14} />
                  </button>
                  <button class="btn-icon" disabled={i === $mergeFiles.length - 1} onclick={() => moveMergeFile(file.path, 1)} aria-label="Move {file.name} down">
                    <ArrowDown size={14} />
                  </button>
                  <button class="btn-icon" onclick={() => removeMergeFile(file.path)} aria-label="Remove {file.name}">
                    <X size={14} />
                  </button>
                </div>
              </div>
            {/each}
          </div>

          <div class="controls">
            <div class="control-group">
              <span class="label">Output</span>
              <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
                <FolderOpen size={14} />
                {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
              </button>
            </div>
            <Button class="action-btn" loading={$isMerging} disabled={$mergeFiles.length < 2} onclick={startMerge}>
              Merge PDFs
            </Button>
          </div>
        </div>
      {/if}
    {:else if mode === "split"}
      {#if !$splitFile}
        <div class="mode-empty">
          <DropZone
            onfiles={handleSplitAdd}
            dropTitle="Drop a PDF to split"
            formatTags={["PDF"]}
            acceptedExtensions={PDF_SUPPORTED_RE}
            filePickerName="PDF files"
            filePickerExtensions={[...PDF_SUPPORTED_EXTENSIONS]}
          />
        </div>
      {:else if $splitResults.length > 0}
        <div class="result-view">
          <div class="result-card">
            <CircleCheck size={28} color="var(--accent)" />
            <span class="result-name">{$splitResults.length} file{$splitResults.length === 1 ? "" : "s"} created</span>
            <div class="split-output-list">
              {#each $splitResults as path (path)}
                <div class="split-output-row">
                  <FileText size={14} />
                  <span class="split-output-name">{path.split(/[\\/]/).pop()}</span>
                  <button class="btn-icon reveal-btn" aria-label="Reveal {path.split(/[\\/]/).pop()} in file manager" onclick={() => revealPdfOutput(path)}>
                    <FolderOpen size={14} />
                  </button>
                </div>
              {/each}
            </div>
            <div class="result-actions">
              <button class="btn-ghost" onclick={clearSplit}>Split another PDF</button>
            </div>
          </div>
        </div>
      {:else}
        <div class="tool-view">
          <div class="header">
            <div class="header-left">
              <h2>{$splitFile.name}</h2>
              <span class="sub">Choose how to split this PDF</span>
            </div>
            <div class="header-actions">
              <button class="btn-ghost danger" onclick={clearSplit}>
                <Trash2 size={14} />
                Clear
              </button>
            </div>
          </div>

          {#if $splitError}
            <div class="error-card" role="alert">
              <CircleAlert size={14} />
              <span>{$splitError}</span>
            </div>
          {/if}

          <div class="controls">
            <div class="control-group">
              <span class="label">Mode</span>
              <SegmentedControl segments={splitModes} bind:selected={splitModeChoice} label="Split mode" />
            </div>

            {#if splitModeChoice === "range"}
              <div class="control-group">
                <span class="label">From page</span>
                <input type="number" min="1" step="1" class="page-input" bind:value={rangeStart} aria-label="Start page" />
              </div>
              <div class="control-group">
                <span class="label">To page</span>
                <input type="number" min="1" step="1" class="page-input" bind:value={rangeEnd} aria-label="End page" />
              </div>
              {#if !canSplit}
                <span class="hint">End page must be ≥ start page</span>
              {/if}
            {/if}

            <div class="control-group">
              <span class="label">Output</span>
              <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
                <FolderOpen size={14} />
                {$outputDir?.split(/[\\/]/).pop() ?? "Same as input"}
              </button>
            </div>

            <Button class="action-btn" loading={$isSplitting} disabled={!canSplit} onclick={startSplit}>
              Split PDF
            </Button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<PrivacyToast visible={showToast} message={toastMessage} />

<style>
  .pdf-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .mode-header {
    padding-bottom: 20px;
  }

  .mode-content {
    flex: 1;
    min-height: 0;
  }

  .mode-empty {
    height: 100%;
  }

  .pdf-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .pdf-options label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .size-delta {
    font-variant-numeric: tabular-nums;
  }

  .toggle-hint {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: 4px;
  }

  .done-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .done-actions .reveal-btn {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    transition: color 0.15s;
  }

  .done-actions .reveal-btn:hover {
    color: var(--accent);
  }

  /* --- Shared: tool-view (Merge / Split configuring state) --- */

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
    gap: 4px;
  }

  .header h2 {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .sub {
    font-size: 13px;
    color: var(--text-muted);
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
    background: none;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-ghost:hover { background: var(--navy-bg); }
  .btn-ghost.danger:hover { color: var(--danger); border-color: var(--danger); }

  .error-card {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--danger-bg);
    border: 1px solid var(--danger);
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--danger);
  }

  /* --- Merge list --- */

  .merge-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    min-height: 0;
  }

  .merge-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
  }

  .merge-index {
    flex-shrink: 0;
    width: 20px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .merge-row :global(.merge-file-icon) {
    flex-shrink: 0;
    color: var(--text-muted);
  }

  .merge-name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .merge-actions .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color 0.15s;
  }

  .merge-actions .btn-icon:hover:not(:disabled) { color: var(--accent); }
  .merge-actions .btn-icon:disabled { opacity: 0.3; cursor: not-allowed; }

  /* --- Controls row (Merge / Split) --- */

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    column-gap: 20px;
    row-gap: 10px;
    padding: 12px 16px;
    background: var(--bg-card);
    border-radius: 12px;
    border: 1px solid var(--border);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .output-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 5px 12px;
    font-size: 12px;
  }

  .page-input {
    width: 72px;
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-main);
    color: var(--text-primary);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  .page-input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .hint {
    font-size: 11px;
    color: var(--danger);
    align-self: center;
  }

  .controls :global(.action-btn) {
    margin-left: auto;
    padding: 8px 24px;
    font-size: 13px;
  }

  /* --- Results (Merge / Split) --- */

  .result-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 32px 40px;
    max-width: 420px;
    text-align: center;
  }

  .result-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }

  .result-meta {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .result-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .btn-primary-sm {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 18px;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
  }

  .btn-primary-sm:hover { opacity: 0.9; transform: translateY(-1px); }
  .btn-primary-sm:active { transform: translateY(0) scale(0.98); }

  .split-output-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    max-height: 220px;
    overflow-y: auto;
    margin-top: 4px;
  }

  .split-output-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .split-output-name {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .split-output-row .reveal-btn {
    display: flex;
    align-items: center;
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color 0.15s;
  }

  .split-output-row .reveal-btn:hover { color: var(--accent); }

  @media (prefers-reduced-motion: reduce) {
    .btn-ghost, .btn-primary-sm, .merge-actions .btn-icon, .split-output-row .reveal-btn {
      transition: none;
    }
  }
</style>
