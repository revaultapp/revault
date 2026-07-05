<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, CheckCircle, AlertCircle, AlertTriangle, X } from "lucide-svelte";
  import ToolShell from "./ToolShell.svelte";
  import HelperTooltip from "./HelperTooltip.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
  import { browseOutputDir, formatBytes } from "$lib/utils";
  import { stripGps } from "$lib/stores/compress";
  import { IMAGE_EXTENSIONS } from "$lib/types";
  import { t } from "$lib/stores/locale.svelte";
  import {
    files, isResizing, outputDir, resolvedOutputDir, resizeMode, width, height, summary,
    upscaleWarning, upscaleCount,
    addFiles, removeFile, clearFiles, willUpscale,
    type ResizeMode,
  } from "$lib/stores/resize";

  type Preset = { label: string; w: number; h: number };
  type PresetGroup = { group: string; presets: Preset[] };

  let presetGroups = $derived<PresetGroup[]>([
    {
      group: t("resize.presetGroupGeneral"),
      presets: [
        { label: t("resize.presetFullHd"), w: 1920, h: 1080 },
        { label: t("resize.presetHd"), w: 1280, h: 720 },
        { label: t("resize.presetThumbnail"), w: 300, h: 300 },
      ],
    },
    {
      group: t("resize.presetGroupSocialMedia"),
      presets: [
        { label: t("resize.presetIgPost"), w: 1080, h: 1350 },
        { label: t("resize.presetIgSquare"), w: 1080, h: 1080 },
        { label: t("resize.presetIg34"), w: 1080, h: 1440 },
        { label: t("resize.presetIgStory"), w: 1080, h: 1920 },
        { label: t("resize.presetYoutube"), w: 1280, h: 720 },
        { label: t("resize.presetTwitter"), w: 1200, h: 675 },
        { label: t("resize.presetLinkedin"), w: 1200, h: 1200 },
        { label: t("resize.presetTiktok"), w: 1080, h: 1920 },
      ],
    },
  ]);

  let targetPct = $derived(
    $files.length === 0 ? 0 : (($summary.done + $summary.failed) / $files.length) * 100
  );

  let headerText = $derived(
    $summary.done > 0 || $summary.failed > 0
      ? t("resize.headerDone", { done: $summary.done, total: $files.length }) +
        ($summary.failed > 0 ? t("common.failedSuffix", { count: $summary.failed }) : "")
      : $files.length === 1
        ? t("common.imagesSelectedOne", { count: $files.length })
        : t("common.imagesSelectedOther", { count: $files.length })
  );

  let modeLabel = $derived($resizeMode === "Fit" ? t("resize.modeFit") : t("resize.modeExact"));

  let modeOptions = $derived<[ResizeMode, string][]>([
    ["Fit", t("resize.modeFit")],
    ["Exact", t("resize.modeExact")],
  ]);

  interface ResizeResult {
    input_path: string;
    output_path: string;
    original_width: number;
    original_height: number;
    new_width: number;
    new_height: number;
    original_size: number;
    resized_size: number;
    error: string | null;
  }

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: t("dropZone.filePickerName"), extensions: [...IMAGE_EXTENSIONS] }],
    });
    if (selected) addFiles(selected);
  }

  async function handleBrowseOutputDir() {
    const dir = await browseOutputDir();
    if (dir) outputDir.set(dir);
  }

  async function startResize() {
    const currentFiles = $files;
    if (currentFiles.length === 0) return;
    const w = $width;
    const h = $height;
    const mode = $resizeMode;
    const outDir = $resolvedOutputDir;
    isResizing.set(true);
    files.update((all) => all.map((f) => ({ ...f, status: "pending" as const })));
    try {
      const allPaths = currentFiles.map((f) => f.path);
      const results = await invoke<ResizeResult[]>("resize_images", {
        paths: allPaths,
        width: w,
        height: h,
        mode,
        outputDir: outDir,
        stripGps: $stripGps,
      });
      const resultMap = new Map(results.map((r) => [r.input_path, r]));
      files.update((all) =>
        all.map((f) => {
          const r = resultMap.get(f.path);
          if (!r) return f;
          if (r.error) return { ...f, status: "error" as const, error: r.error };
          return {
            ...f, status: "done" as const,
            outputPath: r.output_path,
            outputWidth: r.new_width, outputHeight: r.new_height,
            originalWidth: r.original_width, originalHeight: r.original_height,
            size: r.original_size, outputSize: r.resized_size,
          };
        })
      );
    } catch (err) {
      files.update((all) =>
        all.map((f) => f.status === "pending" ? { ...f, status: "error" as const, error: String(err) } : f)
      );
    } finally {
      isResizing.set(false);
    }
  }

</script>

<ToolShell
  files={$files}
  isProcessing={$isResizing}
  {targetPct}
  progressLabel={t("common.progressLabel", { done: $summary.done + $summary.failed, total: $files.length })}
  onfiles={(paths) => addFiles(paths)}
  onbrowse={browseFiles}
  onclear={clearFiles}
  actionLabel={$files.length > 1 ? t("resize.actionButtonAll") : t("resize.actionButton")}
  onaction={startResize}
  {headerText}
>
  {#snippet headerSub()}
    {#if $upscaleWarning}
      <span class="upscale-chip">
        <AlertTriangle size={12} />
        {$upscaleCount === 1
          ? t("resize.willUpscaleChipOne", { count: $upscaleCount })
          : t("resize.willUpscaleChipOther", { count: $upscaleCount })}
      </span>
    {/if}
  {/snippet}

  {#snippet fileDetail(file)}
    {#if file.status === "done"}
      {file.size ? formatBytes(file.size) : '—'} → {file.outputSize ? formatBytes(file.outputSize) : '—'} ({file.originalWidth}×{file.originalHeight} → {file.outputWidth}×{file.outputHeight})
    {:else if file.status === "error"}
      {file.error}
    {:else}
      <span class="file-detail-row">
        <span>{$width}×{$height} · {modeLabel}</span>
        {#if willUpscale(file, $width, $height, $resizeMode)}
          <span class="upscale-flag">
            <AlertTriangle size={14} />
            {t("resize.willUpscaleFlag")}
          </span>
        {/if}
      </span>
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
    <span class="label">{t("resize.presetsLabel")}</span>
    <div class="preset-sections">
      {#each presetGroups as group}
        <div class="preset-section">
          <span class="preset-group-label">{group.group}</span>
          <div class="preset-grid">
            {#each group.presets as p}
              <button
                class="pill"
                class:active={$width === p.w && $height === p.h}
                onclick={() => { width.set(p.w); height.set(p.h); }}
                title="{p.w}×{p.h}"
              >{p.label}</button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">{t("resize.sizeLabel")}</span>
    <div class="dimension-inputs">
      <input type="number" min="1" max="10000" bind:value={$width} aria-label={t("resize.widthAriaLabel")} />
      <span class="dim-sep">×</span>
      <input type="number" min="1" max="10000" bind:value={$height} aria-label={t("resize.heightAriaLabel")} />
    </div>
  </div>
  <div class="control-group">
    <span class="label">
      {t("resize.modeLabel")}
      <HelperTooltip tip={t("resize.modeTooltip")} />
    </span>
    <div class="pills">
      {#each modeOptions as [value, label]}
        <button class="pill" class:active={$resizeMode === value} onclick={() => resizeMode.set(value)}>
          {label}
        </button>
      {/each}
    </div>
  </div>
  <div class="control-group">
    <span class="label">{t("common.outputLabel")}</span>
    <button class="btn-ghost output-btn" onclick={handleBrowseOutputDir}>
      <FolderOpen size={14} />
      {$resolvedOutputDir?.split(/[\\/]/).pop() ?? t("common.sameAsInput")}
    </button>
  </div>
  <div class="control-group">
    <div class="toggle-row">
      <div class="toggle-label">
        <span class="label">{t("resize.stripGpsLabel")}</span>
        <span class="control-hint">{t("resize.stripGpsHint")}</span>
      </div>
      <ToggleSwitch bind:checked={$stripGps} label={t("resize.stripGpsLabel")} />
    </div>
  </div>
</ToolShell>

<style>
  .preset-sections {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preset-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .preset-group-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    opacity: 0.7;
  }

  .preset-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .dimension-inputs {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dimension-inputs input {
    width: 72px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    border: 1px solid var(--border);
    background: var(--navy-bg);
    color: var(--text-primary);
    text-align: center;
  }

  .dim-sep {
    font-size: 13px;
    color: var(--text-muted);
  }

  .upscale-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--warning-bg);
    color: var(--warning-text);
    font-size: 11px;
    font-weight: 500;
  }

  .file-detail-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .upscale-flag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--warning-text);
    font-size: 12px;
    font-weight: 500;
  }
</style>
