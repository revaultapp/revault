<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { CheckCircle, AlertCircle, X, FolderOpen } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { runWithConcurrency, browseOutputDir } from "$lib/utils";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import {
    files, isProcessing, summary, outputDir,
    stripGps, stripDevice, stripDatetime, stripAuthor,
    addFiles, removeFile, clearFiles,
    type PrivacyFile,
  } from "$lib/stores/privacy";

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

  interface ScanResult {
    path: string;
    gps: { latitude: number; longitude: number; altitude: number | null } | null;
    device: string | null;
    datetime: string | null;
    author: string | null;
    technical: string | null;
    error: string | null;
  }

  interface StripResult {
    input_path: string;
    output_path: string;
    original_size: number;
    stripped_size: number;
    error: string | null;
  }

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.stripped + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.stripped > 0 || $summary.failed > 0
      ? `${$summary.stripped} of ${$files.length} stripped${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} image${$files.length > 1 ? "s" : ""} selected`
  );

  let gpsCount = $derived($files.filter(f => f.gps).length);
  let deviceCount = $derived($files.filter(f => f.device).length);
  let datetimeCount = $derived($files.filter(f => f.datetime).length);
  let authorCount = $derived($files.filter(f => f.author).length);
  let techCount = $derived($files.filter(f => f.technical).length);
  let totalFound = $derived(gpsCount + deviceCount + datetimeCount + authorCount + techCount);
  let allStripped = $derived($summary.stripped > 0 && $summary.stripped === $files.filter(f => f.hasMetadata !== undefined).length);

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: [...IMAGE_EXTENSIONS] }],
    });
    if (selected) handleAddFiles(selected);
  }

  async function scanFile(file: PrivacyFile): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "scanning" as const } : f)
    );
    try {
      const result = await invoke<ScanResult>("read_metadata", { path: file.path });
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (result.error) return { ...f, status: "error" as const, error: result.error };
          const gpsStr = result.gps
            ? `${result.gps.latitude.toFixed(4)}, ${result.gps.longitude.toFixed(4)}`
            : undefined;
          return {
            ...f,
            status: "scanned" as const,
            gps: gpsStr,
            device: result.device ?? undefined,
            datetime: result.datetime ?? undefined,
            author: result.author ?? undefined,
            technical: result.technical ?? undefined,
            hasMetadata: !!(result.gps || result.device || result.datetime || result.author || result.technical),
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  function handleAddFiles(paths: string[]) {
    const currentFiles = $files;
    const existing = new Set(currentFiles.map((f) => f.path));
    const newPaths = paths.filter((p) => !existing.has(p));
    addFiles(newPaths);
    // Scan new files silently in background (no processing state, no progress ring)
    const newFiles = newPaths.map((p) => ({
      path: p,
      name: p.split(/[\\/]/).pop() ?? p,
      status: "pending" as const,
    }));
    runWithConcurrency(newFiles, scanFile);
  }

  interface StripOpts { gps: boolean; device: boolean; datetime: boolean; author: boolean; }

  async function stripFile(file: PrivacyFile, opts: StripOpts): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "stripping" as const } : f)
    );
    try {
      const results = await invoke<StripResult[]>("strip_files_selective", {
        paths: [file.path],
        outputDir: $outputDir,
        stripGps: opts.gps,
        stripDevice: opts.device,
        stripDatetime: opts.datetime,
        stripAuthor: opts.author,
      });
      const result = results[0];
      files.update((all) =>
        all.map((f) => {
          if (f.path !== file.path) return f;
          if (!result || result.error) return { ...f, status: "error" as const, error: result?.error ?? "No result" };
          return {
            ...f,
            status: "done" as const,
            outputPath: result.output_path,
            originalSize: result.original_size,
            strippedSize: result.stripped_size,
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.path === file.path ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    }
  }

  async function startStrip() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    isProcessing.set(true);
    const opts: StripOpts = { gps: $stripGps, device: $stripDevice, datetime: $stripDatetime, author: $stripAuthor };
    files.update((all) =>
      all.map((f) => f.status === "done" ? { ...f, status: "scanned" as const } : f)
    );
    await runWithConcurrency(
      currentFiles.filter((f) => f.status === "scanned" || f.status === "pending"),
      (file) => stripFile(file, opts)
    );
    isProcessing.set(false);
  }
</script>

<ToolShell
  files={$files}
  isProcessing={$isProcessing}
  {targetPct}
  progressLabel="{$summary.stripped + $summary.failed} of {$files.length} files"
  onfiles={handleAddFiles}
  onbrowse={browseFiles}
  onclear={clearFiles}
  actionLabel="Strip Metadata"
  onaction={startStrip}
  {headerText}
>
  {#snippet fileDetail(file)}
    {#if file.status === "scanning"}
      Scanning...
    {:else if file.status === "error"}
      {file.error}
    {:else if file.status === "done"}
      {#if file.hasMetadata}
        <span class="meta-removed">{[file.gps && "GPS", file.device, file.datetime, file.author && "Author", file.technical && "Technical"].filter(Boolean).join(" · ")}</span>
      {/if}
      {#if file.outputPath}<span class="meta-tag">{file.outputPath.split(/[\\/]/).pop()}</span>{/if}
    {:else if file.status === "scanned" || file.status === "stripping"}
      {#if file.hasMetadata}
        {#if file.gps}<span class="meta-tag">GPS</span>{/if}
        {#if file.device}<span class="meta-tag">{file.device}</span>{/if}
        {#if file.datetime}<span class="meta-tag">{file.datetime}</span>{/if}
        {#if file.author}<span class="meta-tag">{file.author}</span>{/if}
        {#if file.technical}<span class="meta-tag">Technical</span>{/if}
      {:else}
        No metadata found
      {/if}
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
    <span class="label">{allStripped ? "Removed" : "Found"}</span>
    <div class="pills">
      {#if totalFound === 0}
        <span class="pill">No metadata</span>
      {:else}
        {#if gpsCount > 0}<span class="pill" class:active={allStripped}>GPS · {gpsCount}</span>{/if}
        {#if deviceCount > 0}<span class="pill" class:active={allStripped}>Device · {deviceCount}</span>{/if}
        {#if datetimeCount > 0}<span class="pill" class:active={allStripped}>Date · {datetimeCount}</span>{/if}
        {#if authorCount > 0}<span class="pill" class:active={allStripped}>Author · {authorCount}</span>{/if}
        {#if techCount > 0}<span class="pill" class:active={allStripped}>Technical · {techCount}</span>{/if}
      {/if}
    </div>
  </div>
  <div class="control-group">
    <span class="label">Strip</span>
    <div class="strip-options">
      <label><ToggleSwitch bind:checked={$stripGps} /> GPS</label>
      <label><ToggleSwitch bind:checked={$stripDevice} /> Device</label>
      <label><ToggleSwitch bind:checked={$stripDatetime} /> Date &amp; Time</label>
      <label><ToggleSwitch bind:checked={$stripAuthor} /> Author</label>
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
  .strip-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .strip-options label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .meta-tag {
    display: inline;
  }

  .meta-tag + .meta-tag::before {
    content: " · ";
  }

  .meta-removed {
    text-decoration: line-through;
    opacity: 0.45;
  }
</style>
