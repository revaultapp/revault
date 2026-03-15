<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { ShieldOff, CheckCircle, AlertCircle, X, MapPin, Cpu, Clock, User, Wrench } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import { runWithConcurrency } from "$lib/utils";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import {
    files, selectedCategories, isProcessing, summary,
    addFiles, removeFile, clearFiles,
    type MetaCategory, type PrivacyFile,
  } from "$lib/stores/privacy";

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

  const categories: { value: MetaCategory; label: string; icon: typeof MapPin }[] = [
    { value: "GPS", label: "GPS", icon: MapPin },
    { value: "Device", label: "Device", icon: Cpu },
    { value: "DateTime", label: "Date/Time", icon: Clock },
    { value: "Author", label: "Author", icon: User },
    { value: "Technical", label: "Technical", icon: Wrench },
  ];

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.stripped + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.stripped > 0 || $summary.failed > 0
      ? `${$summary.stripped} of ${$files.length} stripped${$summary.failed > 0 ? ` · ${$summary.failed} failed` : ""}`
      : `${$files.length} image${$files.length > 1 ? "s" : ""} selected`
  );

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

  async function stripFile(file: PrivacyFile): Promise<void> {
    files.update((all) =>
      all.map((f) => f.path === file.path ? { ...f, status: "stripping" as const } : f)
    );
    try {
      const results = await invoke<StripResult[]>("strip_files", {
        paths: [file.path],
        outputDir: null,
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
    files.update((all) =>
      all.map((f) => f.status === "done" ? { ...f, status: "scanned" as const } : f)
    );
    await runWithConcurrency(
      currentFiles.filter((f) => f.status === "scanned" || f.status === "pending"),
      stripFile
    );
    isProcessing.set(false);
  }

  function toggleCategory(cat: MetaCategory) {
    selectedCategories.update((set) => {
      const next = new Set(set);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return next;
    });
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
    {:else if file.status === "scanned" || file.status === "done" || file.status === "stripping"}
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
    <span class="label">Strip</span>
    <div class="pills">
      {#each categories as cat}
        <button
          class="pill"
          class:active={$selectedCategories.has(cat.value)}
          onclick={() => toggleCategory(cat.value)}
        >
          <cat.icon size={11} />
          {cat.label}
        </button>
      {/each}
    </div>
  </div>
</ToolShell>

<style>
  .meta-tag {
    display: inline;
  }

  .meta-tag + .meta-tag::before {
    content: " · ";
  }

  .control-group .pills :global(svg) {
    flex-shrink: 0;
  }
</style>
