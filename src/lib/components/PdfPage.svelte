<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleCheck, CircleAlert, X, FolderOpen } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import PrivacyToast from "./PrivacyToast.svelte";
  import { browseOutputDir, formatBytes } from "$lib/utils";
  import { activity } from "$lib/stores/activity";
  import {
    files, isProcessing, summary, outputDir, stripMetadata, compressStreams,
    compressImages,
    resolvedOutputDir, addFiles, removeFile, clearFiles, processPdfs,
    revealPdfOutput, type PdfFile,
  } from "$lib/stores/pdf";

  let showToast = $state(false);
  let toastMessage = $state("");
  let toastTimer: ReturnType<typeof setTimeout>;

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
</script>

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

<PrivacyToast visible={showToast} message={toastMessage} />

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
</style>
