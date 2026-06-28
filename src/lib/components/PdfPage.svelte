<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleCheck, CircleAlert, X, FolderOpen } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { browseOutputDir, formatBytes } from "$lib/utils";
  import { activity } from "$lib/stores/activity";
  import {
    files, isProcessing, summary, outputDir, stripMetadata, compressStreams,
    resolvedOutputDir, addFiles, removeFile, clearFiles, processPdfs,
    type PdfFile,
  } from "$lib/stores/pdf";

  const PDF_SUPPORTED_EXTENSIONS = ["pdf"] as const;
  const PDF_SUPPORTED_RE = /\.pdf$/i;

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

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

  function handleRejectedFiles(_paths: string[]) {
    // PDF-only — silently ignore non-PDF drops
  }

  async function startProcess() {
    if ($files.length === 0) return;
    await processPdfs($resolvedOutputDir, $stripMetadata, $compressStreams);
    const doneCount = $summary.done;
    if (doneCount > 0) {
      const savedBytes = $files.reduce((acc, f) => {
        if (f.status === "done" && f.originalSize && f.outputSize) {
          return acc + Math.max(0, f.originalSize - f.outputSize);
        }
        return acc;
      }, 0);
      activity.add({ type: "compress", fileCount: doneCount, savedBytes });
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
</script>

<ToolShell
  files={$files}
  isProcessing={$isProcessing}
  {targetPct}
  progressLabel="{$summary.done + $summary.failed} of {$files.length} files"
  onfiles={handleAddFiles}
  onrejectedfiles={handleRejectedFiles}
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
    <span class="label">Options</span>
    <div class="pdf-options">
      <label><ToggleSwitch bind:checked={$stripMetadata} /> Strip metadata</label>
      <label><ToggleSwitch bind:checked={$compressStreams} /> Compress streams</label>
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
</style>
