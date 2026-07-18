<script lang="ts">
  import { onMount } from "svelte";
  import { slide, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "svelte/motion";
  import { Film, Shield, ShieldCheck, Download, Zap, Wifi, CircleCheck, CircleAlert, TriangleAlert, FolderOpen, X, ChevronDown, Minimize2, ImagePlay, Scissors, AudioLines } from "lucide-svelte";
  import PrivacyToast from "./PrivacyToast.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ToolShell from "./ToolShell.svelte";
  import SegmentedControl from "./SegmentedControl.svelte";
  import { formatBytes } from "$lib/utils";
  import { animatedNumber } from "$lib/motion";
  import { VIDEO_EXTENSIONS, VIDEO_EXTENSIONS_RE } from "$lib/types";
  import {
    videoFiles,
    videoPreset,
    videoOutputDir,
    resolvedVideoOutputDir,
    isCompressing,
    videoSummary,
    ffmpegStatus,
    ffmpegDownloadProgress,
    videoPrivacyMode,
    videoPreviews,
    videoPreviewSummary,
    computeVideoPreview,
    addVideoFiles,
    removeVideoFile,
    clearVideoFiles,
    resetVideoFilesToIdle,
    compressVideoFile,
    cancelCompression,
    revealVideoOutput,
    checkFfmpeg,
    downloadFfmpeg,
    videoMode,
    gifSettings,
    gifState,
    gifOutputPath,
    gifError,
    gifskiAvailable,
    gifDownloadProgress,
    gifResult,
    gifSizeEstimate,
    gifProgress,
    gifPhase,
    checkGifski,
    downloadGifski,
    exportGif,
    cancelGifExport,
    refreshGifSizeEstimate,
    trimSettings,
    trimState,
    trimResult,
    trimOutputPath,
    trimError,
    trimVideoFile,
    audioSettings,
    audioState,
    audioResult,
    audioError,
    audioProgress,
    extractAudioFile,
    cancelAudioExtract,
    type AudioFormat,
    type AudioBitrate,
    type VideoFile,
    type VideoPreset,
    type PrivacyMode,
  } from "$lib/stores/video";
  import { t } from "$lib/stores/locale.svelte";

  const rm = $derived(prefersReducedMotion.current);

  // Icon rule: same concept → same glyph across the app. Minimize2 mirrors
  // PdfPage's Optimize mode and Scissors mirrors its Split mode.
  let modeSegments = $derived([
    { id: "compress", label: t("video.modeCompress"), icon: Minimize2 },
    { id: "gif", label: t("video.modeGif"), icon: ImagePlay },
    { id: "trim", label: t("video.modeTrim"), icon: Scissors },
    { id: "audio", label: t("video.modeAudio"), icon: AudioLines },
  ] as const);

  let audioFormatSegments = $derived([
    { id: "auto", label: t("video.audioFormatAuto") },
    { id: "mp3", label: t("video.audioFormatMp3") },
  ] as const);

  const audioBitrateOptions = [128, 192, 320] as const;

  let fpSegments = $derived([
    { id: "10", label: t("video.fpLow") },
    { id: "15", label: t("video.fpMedium") },
    { id: "24", label: t("video.fpHigh") },
  ] as const);

  let widthSegments = $derived([
    { id: "320", label: t("video.widthChat") },
    { id: "480", label: t("video.widthWhatsapp") },
    { id: "640", label: t("video.widthLarge") },
    { id: "720", label: t("video.widthHd") },
    { id: "1080", label: t("video.widthFullHd") },
  ] as const);

  let presets = $derived<{ value: VideoPreset; label: string }[]>([
    { value: "Smallest", label: t("video.presetSmallest") },
    { value: "Balanced", label: t("video.presetBalanced") },
    { value: "HighQuality", label: t("video.presetHighQuality") },
  ]);

  let privacySegments = $derived([
    { id: "off" satisfies PrivacyMode, label: t("video.privacyOff") },
    { id: "smart" satisfies PrivacyMode, label: t("video.privacySmart") },
    { id: "gps_only" satisfies PrivacyMode, label: t("video.privacyGpsOnly") },
    { id: "full" satisfies PrivacyMode, label: t("video.privacyFull") },
  ] as const);

  let privacyTooltips = $derived<Record<PrivacyMode, string>>({
    off: t("video.privacyTooltipOff"),
    smart: t("video.privacyTooltipSmart"),
    gps_only: t("video.privacyTooltipGpsOnly"),
    full: t("video.privacyTooltipFull"),
  });

  let privacyChipText = $derived<Record<Exclude<PrivacyMode, "off">, (n: number) => string>>({
    smart: (n) => n === 1 ? t("video.privacyChipSmartOne", { count: n }) : t("video.privacyChipSmartOther", { count: n }),
    gps_only: (n) => n === 1 ? t("video.privacyChipGpsOnlyOne", { count: n }) : t("video.privacyChipGpsOnlyOther", { count: n }),
    full: (n) => n === 1 ? t("video.privacyChipFullOne", { count: n }) : t("video.privacyChipFullOther", { count: n }),
  });

  // ── Accordion state ─────────────────────────────────────────────────────────
  let settingsOpen = $state(false);

  let accordionHeader = $derived.by(() => {
    const preset = $videoPreset;
    const presetLabel = presets.find(p => p.value === preset)?.label ?? preset;
    const mode = $videoPrivacyMode;
    const privacyLabel = privacySegments.find(s => s.id === mode)?.label ?? mode;
    if ($videoMode === "compress") {
      return t("video.accordionHeaderCompress", { preset: presetLabel, privacy: privacyLabel });
    }
    if ($videoMode === "trim") {
      const len = Math.max(0, $trimSettings.endSec - $trimSettings.startSec);
      return t("video.accordionHeaderTrim", { length: len.toFixed(1) });
    }
    if ($videoMode === "audio") {
      return $audioSettings.format === "auto"
        ? t("video.accordionHeaderAudioAuto")
        : t("video.accordionHeaderAudioMp3", { bitrate: $audioSettings.bitrateKbps });
    }
    const fps = $gifSettings.fps;
    const width = $gifSettings.width;
    return t("video.accordionHeaderGif", { fps, width });
  });

  // ── Trim validity ───────────────────────────────────────────────────────────
  let trimClipLength = $derived(Math.max(0, $trimSettings.endSec - $trimSettings.startSec));
  let trimValid = $derived($trimSettings.endSec >= $trimSettings.startSec + 0.1);

  // ── Progress & header ───────────────────────────────────────────────────────
  let targetPct = $derived(
    $videoFiles.length === 0
      ? 0
      : (($videoSummary.done + $videoSummary.failed) / $videoFiles.length) * 100
  );

  let headerText = $derived(
    $videoSummary.done > 0 || $videoSummary.failed > 0
      ? t("video.headerDone", { done: $videoSummary.done, total: $videoFiles.length }) +
        ($videoSummary.failed > 0 ? t("common.failedSuffix", { count: $videoSummary.failed }) : "")
      : $videoFiles.length === 1
        ? t("video.videosSelectedOne", { count: $videoFiles.length })
        : t("video.videosSelectedOther", { count: $videoFiles.length })
  );

  let downloadError = $state<string | null>(null);
  let gifDownloadError = $state<string | null>(null);
  let isGifInstalling = $state(false);

  let gifGateState = $derived.by<"idle" | "downloading" | "installing" | "error">(() => {
    if ($gifskiAvailable) return "idle";
    if (gifDownloadError) return "error";
    if (isGifInstalling) return "installing";
    if ($gifDownloadProgress !== null) return "downloading";
    return "idle";
  });

  onMount(() => {
    checkFfmpeg();
    checkGifski();
  });

  $effect(() => {
    const settings = $gifSettings;
    if ($videoMode === "gif" && $videoFiles.length > 0 && settings) {
      // Debounced like the compression-preview trigger below: the range/quality
      // sliders call gifSettings.update() on every drag tick, and each estimate
      // is a Tauri invoke — without the delay we'd spam IPC mid-drag.
      const timer = setTimeout(() => refreshGifSizeEstimate(), 300);
      return () => clearTimeout(timer);
    }
  });

  async function handleDownload() {
    downloadError = null;
    try {
      await downloadFfmpeg();
    } catch (e) {
      downloadError = String(e);
    }
  }

  async function handleGifDownload() {
    gifDownloadError = null;
    isGifInstalling = false;
    try {
      await downloadGifski();
      isGifInstalling = true;
    } catch {
      gifDownloadError = t("video.gifDownloadErrorMessage");
    } finally {
      isGifInstalling = false;
    }
  }

  async function startGifExport() {
    const file = $videoFiles[0];
    if (!file) return;
    await exportGif(file);
  }

  async function startTrim() {
    const file = $videoFiles[0];
    if (!file || !trimValid) return;
    await trimVideoFile(file);
  }

  async function startAudioExtract() {
    const file = $videoFiles[0];
    if (!file) return;
    await extractAudioFile(file);
  }

  async function startCompression() {
    const pending = $videoFiles.filter((f) => f.status === "idle");
    isCompressing.set(true);
    try {
      for (const file of pending) {
        await compressVideoFile(file, $videoPreset, $resolvedVideoOutputDir);
        const updated = $videoFiles.find((f) => f.path === file.path);
        if (updated?.status === "cancelled") break;
      }
    } finally {
      isCompressing.set(false);
    }
  }

  async function compressMore() {
    resetVideoFilesToIdle();
    await startCompression();
  }

  async function browseOutputDir() {
    const selected = await open({ directory: true, multiple: false });
    if (selected && typeof selected === "string") {
      videoOutputDir.set(selected);
    }
  }

  async function browseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: t("video.filePickerName"), extensions: [...VIDEO_EXTENSIONS] }],
    });
    if (selected) handleFiles(selected);
  }

  function handleFiles(paths: string[]) {
    const videoPaths = paths.filter((p) => VIDEO_EXTENSIONS_RE.test(p));
    if (videoPaths.length > 0) addVideoFiles(videoPaths);
  }

  function savedPercent(file: VideoFile): string {
    if (!file.compressedSize || file.originalSize === 0) return "";
    const pct = Math.round(
      ((file.originalSize - file.compressedSize) / file.originalSize) * 100
    );
    if (pct > 0) return t("video.pctSmaller", { pct });
    if (pct < 0) return t("video.pctLarger", { pct: Math.abs(pct) });
    return t("video.sameSize");
  }

  function isOutputLarger(file: VideoFile): boolean {
    if (!file.compressedSize || file.originalSize === 0) return false;
    return file.compressedSize >= file.originalSize;
  }

  function formatMB(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    if (mb >= 100) return `${Math.round(mb)} MB`;
    if (mb >= 10) return `${mb.toFixed(0)} MB`;
    return `${mb.toFixed(1)} MB`;
  }

  // ── Debounced preview trigger ───────────────────────────────────────────────
  $effect(() => {
    const filesSnapshot = $videoFiles;
    const presetSnapshot = $videoPreset;
    const privacySnapshot = $videoPrivacyMode;
    const compressing = $isCompressing;

    if (compressing) return;

    void presetSnapshot;
    void privacySnapshot;

    const idlePaths = filesSnapshot
      .filter((f) => f.status === "idle")
      .map((f) => f.path);

    if (idlePaths.length === 0) return;

    const timer = setTimeout(() => {
      for (const path of idlePaths) {
        computeVideoPreview(path);
      }
    }, 300);
    return () => clearTimeout(timer);
  });

  // ── Post-success privacy chip ───────────────────────────────────────────────
  let showPrivacyChip = $state(false);
  let privacyChipCount = $state(0);
  let privacyChipMode = $state<Exclude<PrivacyMode, "off">>("smart");
  let privacyChipTimer: ReturnType<typeof setTimeout> | null = null;
  let lastAllDoneSignature = "";

  $effect(() => {
    const files = $videoFiles;
    const privacyActive = $videoPrivacyMode !== "off";
    const signature = files.length > 0
      ? files.map((f) => `${f.path}:${f.status}`).join("|")
      : "";

    if (signature === lastAllDoneSignature) return;

    if (files.length > 0 && files.every((f) => f.status === "done") && privacyActive) {
      lastAllDoneSignature = signature;
      privacyChipCount = files.length;
      privacyChipMode = $videoPrivacyMode as Exclude<PrivacyMode, "off">;
      showPrivacyChip = true;

      if (privacyChipTimer !== null) clearTimeout(privacyChipTimer);
      privacyChipTimer = setTimeout(() => { showPrivacyChip = false; }, 4000);
    } else if (files.length === 0) {
      lastAllDoneSignature = "";
      showPrivacyChip = false;
      if (privacyChipTimer !== null) {
        clearTimeout(privacyChipTimer);
        privacyChipTimer = null;
      }
    }
  });

  // ── Estimate state ──────────────────────────────────────────────────────────
  let estimateState = $derived.by(() => {
    const summary = $videoPreviewSummary;
    if (!summary || summary.filesTotal === 0) return { kind: "hidden" as const };
    if (summary.filesReady < summary.filesTotal) return { kind: "loading" as const };
    if (summary.totalSaved > 0) {
      return {
        kind: "ready" as const,
        totalOriginal: summary.totalOriginal,
        totalEstimated: summary.totalEstimated,
        totalSaved: summary.totalSaved,
        savingsPct: summary.savingsPct,
        filesTotal: summary.filesTotal,
      };
    }
    return { kind: "no-gain" as const };
  });

  // Estimate hero number + % count up toward the target instead of snapping,
  // per animatedNumber's own reduced-motion guard (mirrors CompressPage).
  const estimatedPctTween = animatedNumber(0);
  const estimatedBytesTween = animatedNumber(0);
  const gifSizeTween = animatedNumber(0);

  $effect(() => {
    if (estimateState.kind === "ready") {
      estimatedPctTween.set(estimateState.savingsPct);
      estimatedBytesTween.set(estimateState.totalEstimated);
    }
  });

  $effect(() => {
    if ($gifSizeEstimate !== null) {
      gifSizeTween.set($gifSizeEstimate);
    }
  });
</script>

{#if $ffmpegStatus === "checking"}
  <div class="ffmpeg-state">
    <div class="checking-icon">
      <Film size={40} color="var(--accent)" />
      <div class="spinner"></div>
    </div>
  </div>

{:else if $ffmpegStatus === "needs_download"}
  <div class="ffmpeg-state">
    <div class="download-hero" role="region" aria-label={t("video.ffmpegSetupAriaLabel")}>
      <div class="hero-icon">
        <div class="icon-glow"></div>
        <Film size={48} color="var(--accent)" />
      </div>
      <h2>{t("video.unlockHeroTitle")}</h2>
      <p class="subtitle">
        {t("video.ffmpegNeedLine1")}<br />
        {t("video.ffmpegNeedLine2")}
      </p>
      <div class="trust-grid">
        <div class="trust-item">
          <Shield size={16} color="var(--accent)" />
          <span>{t("video.trustPrivate")}</span>
        </div>
        <div class="trust-item">
          <Wifi size={16} color="var(--accent)" />
          <span>{t("video.trustOffline")}</span>
        </div>
        <div class="trust-item">
          <Zap size={16} color="var(--accent)" />
          <span>{t("video.trustIndustry")}</span>
        </div>
      </div>
      {#if downloadError}
        <div class="download-error" role="alert">
          <CircleAlert size={14} />
          <span>{t("video.downloadErrorMessage")}</span>
        </div>
      {/if}
      <button class="btn-download" onclick={handleDownload}>
        <Download size={18} />
        {t("video.downloadButton")}
      </button>
      <p class="fine-print">{t("video.downloadFinePrint")}</p>
    </div>
  </div>

{:else if $ffmpegStatus === "downloading"}
  <div class="ffmpeg-state">
    <div class="download-progress-view" role="region" aria-label={t("video.downloadingAriaLabel")} aria-live="polite">
      <div class="hero-icon downloading">
        <div class="icon-glow pulse"></div>
        <Film size={48} color="var(--accent)" />
      </div>
      <h2>{t("video.preparingTitle")}</h2>
      <p class="subtitle">{t("video.preparingSubtitle")}</p>
      <div class="big-progress-wrap">
        <div class="big-progress-track" role="progressbar" aria-valuenow={Math.round($ffmpegDownloadProgress.percent)} aria-valuemin={0} aria-valuemax={100}>
          <div class="big-progress-fill" style="transform: scaleX({$ffmpegDownloadProgress.percent / 100})">
            <div class="progress-shine"></div>
          </div>
        </div>
        <div class="progress-meta">
          <span>{formatBytes($ffmpegDownloadProgress.downloaded)} / {formatBytes($ffmpegDownloadProgress.total)}</span>
          <span>{Math.round($ffmpegDownloadProgress.percent)}%</span>
        </div>
      </div>
    </div>
  </div>

{:else}
  <!-- Columna a altura completa: la mode-bar arriba y ToolShell ocupando el
       resto REAL. Sin esto, el wrapper vacío de ToolShell (height:100%) se
       suma a la barra y el DropZone queda descentrado + scroll fantasma. -->
  <div class="video-flow">
  <!-- Mode toggle: centered, above DropZone / file list -->
  <div class="mode-bar">
    <SegmentedControl segments={modeSegments} bind:selected={$videoMode} label={t("video.modeAriaLabel")} />
  </div>

  <div class="tool-area">
  <ToolShell
    files={$videoFiles}
    isProcessing={$isCompressing || $gifState === "generating" || $audioState === "extracting"}
    {targetPct}
    progressLabel={t("common.progressLabel", { done: $videoSummary.done + $videoSummary.failed, total: $videoFiles.length })}
    progressSublabel={$videoSummary.savedBytes > 0 ? t("common.savedTotal", { amount: formatBytes($videoSummary.savedBytes) }) : undefined}
    onfiles={handleFiles}
    onbrowse={browseFiles}
    onclear={clearVideoFiles}
    actionLabel={$videoMode === "gif"
      ? (!$gifskiAvailable ? "" : $gifState === "generating" ? t("video.gifExportCancel") : t("video.gifExportAction"))
      : $videoMode === "trim"
        ? (trimValid ? t("video.trimAction") : "")
        : $videoMode === "audio"
          ? ($audioState === "extracting" ? t("video.audioExtractCancel") : t("video.audioExtractAction"))
          : ($isCompressing ? t("video.compressCancel") : $videoSummary.pending === 0 && $videoFiles.length > 0 ? t("video.compressMoreAction") : t("video.compressAction"))}
    onaction={$videoMode === "gif"
      ? ($gifState === "generating" ? cancelGifExport : startGifExport)
      : $videoMode === "trim"
        ? startTrim
        : $videoMode === "audio"
          ? ($audioState === "extracting" ? cancelAudioExtract : startAudioExtract)
          : ($isCompressing ? cancelCompression : $videoSummary.pending === 0 && $videoFiles.length > 0 ? compressMore : startCompression)}
    actionLoading={$videoMode === "trim" && $trimState === "trimming"}
    {headerText}
    dropZoneTitle={t("video.dropZoneTitle")}
    dropZoneFormatTags={["MP4", "MOV", "AVI", "MKV", "WebM", "M4V"]}
    dropZoneAcceptedExtensions={VIDEO_EXTENSIONS_RE}
    dropZoneFilePickerName={t("video.filePickerName")}
    dropZoneFilePickerExtensions={[...VIDEO_EXTENSIONS]}
    showThumbnails={false}
    placeholderIcon="video"
  >
    {#snippet headerSub()}
      {#if $videoSummary.savedBytes > 0}
        <span class="saved-total">{t("common.savedTotal", { amount: formatBytes($videoSummary.savedBytes) })}</span>
      {/if}
    {/snippet}

    {#snippet fileDetail(file)}
      {#if file.status === "idle"}
        {@const preview = $videoPreviews.get(file.path)}
        {#if !preview || preview.status === "idle"}
          {t("video.readyLabel")}
        {:else if preview.status === "loading"}
          <span class="preview-loading">{t("video.calculatingEstimate")}</span>
        {:else if preview.status === "ready"}
          {#if preview.preview.estimatedSavingsPct < 3}
            <span class="preview-muted">{t("video.alreadyWellCompressed")}</span>
          {:else}
            <span class="preview-muted">
              {t("video.estimateSavings", {
                original: formatMB(preview.preview.originalSizeBytes),
                estimated: formatMB(preview.preview.estimatedSizeBytes),
                pct: Math.round(preview.preview.estimatedSavingsPct),
              })}
            </span>
          {/if}
        {:else}
          {t("video.readyLabel")}
        {/if}
      {:else if file.status === "compressing"}
        <span class="compressing-detail">
          {#if file.fps > 0}
            {file.fps.toFixed(0)} fps &middot; {file.speed.toFixed(1)}x
          {:else}
            {t("video.encodingLabel")}
          {/if}
        </span>
        <span class="progress-bar-track">
          <span class="progress-bar-fill" style="transform: scaleX({file.progress / 100})"></span>
        </span>
      {:else if file.status === "done" && isOutputLarger(file)}
        <span class="warning-detail">{t("video.alreadyOptimized", { size: formatBytes(file.originalSize) })}</span>
      {:else if file.status === "done"}
        {formatBytes(file.originalSize)} &rarr; {formatBytes(file.compressedSize ?? 0)} &middot; {savedPercent(file)}
      {:else if file.status === "error"}
        {file.error ?? t("video.compressionFailedFallback")}
      {:else if file.status === "cancelled"}
        {t("video.cancelledLabel")}
      {/if}
    {/snippet}

    {#snippet fileStatus(file)}
      {#if file.status === "compressing"}
        <span class="status-pct">{Math.round(file.progress)}%</span>
      {:else if file.status === "done" && isOutputLarger(file)}
        <div class="done-actions">
          {#if file.outputPath}
            <button class="btn-icon" aria-label={t("video.revealAriaLabel")} onclick={() => revealVideoOutput(file.outputPath!)}>
              <FolderOpen size={16} />
            </button>
          {/if}
          <TriangleAlert size={18} color="var(--warning)" />
        </div>
      {:else if file.status === "done"}
        <div class="done-actions">
          {#if file.outputPath}
            <button class="btn-icon" aria-label={t("video.revealAriaLabel")} onclick={() => revealVideoOutput(file.outputPath!)}>
              <FolderOpen size={16} />
            </button>
          {/if}
          <CircleCheck size={18} />
        </div>
      {:else if file.status === "error" || file.status === "cancelled"}
        <CircleAlert size={18} />
      {:else}
        <button class="btn-icon" aria-label={t("video.removeFileAriaLabel")} onclick={() => removeVideoFile(file.path)}>
          <X size={16} />
        </button>
      {/if}
    {/snippet}

    {#snippet estimateCard()}
      {#if $videoMode === "compress" && $videoFiles.length > 0}
        {#if estimateState.kind === "loading"}
          <div class="estimate-card">
            <span class="estimate-label">{t("video.estimatedLabel")}</span>
            <span class="estimate-scanning">{t("video.calculatingSavings")}</span>
          </div>
        {:else if estimateState.kind === "ready"}
          <div class="estimate-card">
            <span class="estimate-label">{t("video.estimatedLabel")}</span>
            <div class="estimate-hero">
              <span class="estimate-hero-num">{Math.round(estimatedPctTween.current)}<small>%</small></span>
              <span class="estimate-hero-word">{t("video.smaller")}</span>
            </div>
            <div class="estimate-bar-track">
              <div
                class="estimate-bar-fill"
                style="transform: scaleX({Math.min(Math.max(estimatedPctTween.current, 0), 100) / 100})"
              ></div>
            </div>
            <span class="estimate-range">
              {estimateState.filesTotal === 1
                ? t("video.estimateSummaryOne", { count: estimateState.filesTotal, original: formatBytes(estimateState.totalOriginal), estimated: formatBytes(estimatedBytesTween.current) })
                : t("video.estimateSummaryOther", { count: estimateState.filesTotal, original: formatBytes(estimateState.totalOriginal), estimated: formatBytes(estimatedBytesTween.current) })}
            </span>
          </div>
        {:else if estimateState.kind === "no-gain"}
          <div class="estimate-card">
            <span class="estimate-scanning">{t("video.videosAlreadyOptimized")}</span>
          </div>
        {/if}
      {:else if $videoMode === "gif" && $gifskiAvailable && $videoFiles.length > 0 && $gifSizeEstimate !== null}
        <div class="estimate-card">
          <div class="estimate-hero">
            <span class="estimate-hero-num">≈ {formatBytes(gifSizeTween.current)}</span>
          </div>
        </div>
      {/if}
    {/snippet}

    <!-- Ajustes accordion -->
    <div class="control-group accordion">
      <button
        class="accordion-header"
        onclick={() => settingsOpen = !settingsOpen}
        type="button"
        aria-expanded={settingsOpen}
        aria-controls="video-settings-body"
      >
        <span class="label">{accordionHeader}</span>
        <span class="accordion-chevron" class:open={settingsOpen}>
          <ChevronDown size={14} />
        </span>
      </button>

      {#if settingsOpen}
        <div id="video-settings-body" class="accordion-body" transition:slide={{ duration: 180 }}>
          {#if $videoMode === "compress"}
            <div class="control-group">
              <span class="label">{t("video.presetLabel")}</span>
              <div class="pills">
                {#each presets as p}
                  <button
                    class="pill"
                    class:active={$videoPreset === p.value}
                    aria-pressed={$videoPreset === p.value}
                    onclick={() => videoPreset.set(p.value)}
                  >{p.label}</button>
                {/each}
              </div>
            </div>

            <div class="control-group privacy-group">
              <div class="privacy-control-row">
                <div class="privacy-icon-label">
                  <div class="privacy-icon" class:on={$videoPrivacyMode !== "off"} aria-hidden="true">
                    <ShieldCheck size={16} />
                  </div>
                  <span class="label">{t("video.privacyLabel")}</span>
                </div>
                <SegmentedControl segments={privacySegments} bind:selected={$videoPrivacyMode} label={t("video.privacyModeAriaLabel")} />
              </div>
              {#key $videoPrivacyMode}
                <p
                  class="privacy-hint"
                  in:fade={{ duration: rm ? 0 : 150, easing: cubicOut }}
                  out:fade={{ duration: rm ? 0 : 100, easing: cubicOut }}
                >{privacyTooltips[$videoPrivacyMode]}</p>
              {/key}
            </div>

          {:else if $videoMode === "trim"}
            <!-- Trim mode controls -->
            <div class="control-group gif-range-group">
              <span class="label">
                {t("video.clipRangeLabel")}
                <span class="lossless-badge"><CircleCheck size={11} />{t("video.losslessBadge")}</span>
              </span>
              <div class="range-inputs">
                <input
                  type="number"
                  class="num-input"
                  min="0"
                  step="0.5"
                  aria-label={t("video.startSecondAriaLabel")}
                  value={$trimSettings.startSec}
                  oninput={(e) => trimSettings.update(s => ({ ...s, startSec: Math.max(0, parseFloat((e.target as HTMLInputElement).value) || 0) }))}
                />
                <span class="range-sep">–</span>
                <input
                  type="number"
                  class="num-input"
                  min="0"
                  step="0.5"
                  aria-label={t("video.endSecondAriaLabel")}
                  value={$trimSettings.endSec}
                  oninput={(e) => trimSettings.update(s => ({ ...s, endSec: Math.max(0, parseFloat((e.target as HTMLInputElement).value) || 0) }))}
                />
              </div>
              <p class="trim-length-hint" class:invalid={!trimValid}>
                {trimValid ? t("video.clipLengthHint", { length: trimClipLength.toFixed(1) }) : t("video.endAfterStartError")}
              </p>
            </div>

          {:else if $videoMode === "audio"}
            <!-- Audio extraction controls -->
            <div class="control-group">
              <span class="label">{t("video.audioFormatLabel")}</span>
              <div class="pills">
                {#each audioFormatSegments as seg}
                  <button
                    class="pill"
                    class:active={$audioSettings.format === seg.id}
                    aria-pressed={$audioSettings.format === seg.id}
                    onclick={() => audioSettings.update(s => ({ ...s, format: seg.id as AudioFormat }))}
                  >{seg.label}</button>
                {/each}
              </div>
              {#key $audioSettings.format}
                <p
                  class="privacy-hint"
                  in:fade={{ duration: rm ? 0 : 150, easing: cubicOut }}
                  out:fade={{ duration: rm ? 0 : 100, easing: cubicOut }}
                >{$audioSettings.format === "auto" ? t("video.audioFormatAutoHint") : t("video.audioFormatMp3Hint")}</p>
              {/key}
            </div>

            <div class="control-group">
              <span class="label">{t("video.audioBitrateLabel")}</span>
              <div class="pills">
                {#each audioBitrateOptions as kbps}
                  <button
                    class="pill"
                    class:active={$audioSettings.bitrateKbps === kbps}
                    aria-pressed={$audioSettings.bitrateKbps === kbps}
                    onclick={() => audioSettings.update(s => ({ ...s, bitrateKbps: kbps as AudioBitrate }))}
                  >{kbps} kbps</button>
                {/each}
              </div>
            </div>

          {:else}
            <!-- GIF mode controls -->
            {#if $gifskiAvailable === false}
              {#if gifGateState === "idle"}
                <div class="gif-gate" role="status">
                  <CircleAlert size={14} />
                  <span>{t("video.gifGateIdleMessage")}</span>
                  <button class="btn-ghost" onclick={handleGifDownload}>{t("video.downloadAction")}</button>
                </div>
              {:else if gifGateState === "downloading"}
                <div class="gif-gate gif-gate--progress" role="status" aria-live="polite">
                  <div class="gif-gate-progress-wrap">
                    <div class="gif-progress-bar">
                      <div
                        class="gif-progress-fill"
                        style="transform: scaleX({$gifDownloadProgress && $gifDownloadProgress.total > 0 ? $gifDownloadProgress.done / $gifDownloadProgress.total : 0})"
                      ></div>
                    </div>
                    <span class="gif-progress-text">
                      {#if $gifDownloadProgress && $gifDownloadProgress.total > 0}
                        {t("video.downloadingProgress", { done: formatBytes($gifDownloadProgress.done), total: formatBytes($gifDownloadProgress.total) })}
                        <span class="gif-progress-pct">{Math.round(($gifDownloadProgress.done / $gifDownloadProgress.total) * 100)}%</span>
                      {:else}
                        {t("video.downloadingEllipsis")}
                      {/if}
                    </span>
                  </div>
                </div>
              {:else if gifGateState === "installing"}
                <div class="gif-gate" role="status" aria-live="polite">
                  <div class="gif-gate-spinner"></div>
                  <span>{t("video.almostReady")}</span>
                </div>
              {:else if gifGateState === "error"}
                <div class="gif-gate gif-gate--error" role="alert">
                  <CircleAlert size={14} />
                  <div class="gif-gate-error-text">
                    <span>{t("video.downloadIncompleteError")}</span>
                    <small>{t("video.downloadRetryHint")}</small>
                  </div>
                  <button class="btn-ghost" onclick={handleGifDownload}>{t("video.retryAction")}</button>
                </div>
              {/if}
            {/if}

            {#if $gifskiAvailable}
              <div class="control-group gif-range-group">
                <span class="label">{t("video.fragmentLabel")} <span class="sub-hint">{t("video.fragmentMaxHint")}</span></span>
                <div class="range-inputs">
                  <input
                    type="number"
                    class="num-input"
                    min="0"
                    step="0.5"
                    aria-label={t("video.startSecondGifAriaLabel")}
                    value={$gifSettings.startSec}
                    oninput={(e) => gifSettings.update(s => ({ ...s, startSec: Math.max(0, parseFloat((e.target as HTMLInputElement).value) || 0) }))}
                  />
                  <span class="range-sep">–</span>
                  <input
                    type="number"
                    class="num-input"
                    min="0"
                    step="0.5"
                    aria-label={t("video.endSecondGifAriaLabel")}
                    value={$gifSettings.endSec}
                    oninput={(e) => gifSettings.update(s => ({ ...s, endSec: Math.max(s.startSec + 0.5, Math.min(s.startSec + 15, parseFloat((e.target as HTMLInputElement).value) || s.startSec + 3)) }))}
                  />
                </div>
              </div>
              <div class="control-group">
                <span class="label">{t("video.fluidityLabel")}</span>
                <div class="pills">
                  {#each fpSegments as seg}
                    <button
                      class="pill"
                      class:active={$gifSettings.fps === parseInt(seg.id)}
                      aria-pressed={$gifSettings.fps === parseInt(seg.id)}
                      onclick={() => gifSettings.update(s => ({ ...s, fps: parseInt(seg.id) as 10 | 15 | 24 }))}
                    >{seg.label}</button>
                  {/each}
                </div>
              </div>
              <div class="control-group">
                <span class="label">{t("video.sizeLabel")}</span>
                <div class="pills">
                  {#each widthSegments as seg}
                    <button
                      class="pill"
                      class:active={$gifSettings.width === parseInt(seg.id)}
                      aria-pressed={$gifSettings.width === parseInt(seg.id)}
                      onclick={() => gifSettings.update(s => ({ ...s, width: parseInt(seg.id) as 320 | 480 | 640 | 720 | 1080 }))}
                    >{seg.label}</button>
                  {/each}
                </div>
              </div>
              <div class="control-group">
                <span class="label">{t("video.qualityLabel")}</span>
                <input
                  type="range"
                  min="1"
                  max="100"
                  class="quality-slider"
                  value={$gifSettings.quality}
                  oninput={(e) => gifSettings.update(s => ({ ...s, quality: parseInt((e.target as HTMLInputElement).value) }))}
                />
                <span class="quality-val">{$gifSettings.quality}</span>
              </div>
            {/if}
          {/if}

          <!-- Carpeta output — visible in both modes -->
          {#if $videoMode === "compress" || $videoMode === "trim" || $videoMode === "audio" || $gifskiAvailable}
            <div class="control-group">
              <span class="label">{t("video.folderLabel")}</span>
              <button class="btn-ghost output-btn" onclick={browseOutputDir}>
                <FolderOpen size={14} />
                {$resolvedVideoOutputDir?.split(/[\\/]/).pop() ?? t("video.sameAsInput")}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- GIF encoding progress -->
    {#if $videoMode === "gif" && $gifState === "generating"}
      <div class="gif-progress-block">
        <div class="gif-progress-header">
          <span class="gif-progress-label">
            {$gifPhase === "complete" ? t("video.finalizingLabel") : t("video.encodingGifLabel")}
          </span>
          <span class="gif-progress-pct">{$gifProgress}%</span>
        </div>
        <div class="gif-enc-track" role="progressbar" aria-label={t("video.exportingGifAriaLabel")} aria-valuenow={$gifProgress} aria-valuemin={0} aria-valuemax={100}>
          <div class="gif-enc-fill" style="transform: scaleX({$gifProgress / 100})"></div>
        </div>
      </div>
    {/if}

    <!-- GIF done state -->
    {#if $videoMode === "gif" && $gifState === "done" && $gifOutputPath}
      <div class="gif-done">
        <CircleCheck size={28} color="var(--accent)" />
        <span class="gif-done-name">{$gifOutputPath.split(/[\\/]/).pop()}</span>
        {#if $gifResult}
          <span class="gif-done-meta">
            {$gifResult.duration_sec.toFixed(1)}s &middot; {$gifResult.width}×{$gifResult.height} &middot; {$gifResult.fps}fps &middot; {formatMB($gifResult.size_bytes)}
          </span>
        {/if}
        <div class="gif-done-actions">
          <button class="btn-primary-sm" onclick={() => revealVideoOutput($gifOutputPath!)}>
            <FolderOpen size={14} />
            {t("video.openFolderAction")}
          </button>
          <button class="btn-ghost" onclick={() => { gifState.set("idle"); gifOutputPath.set(null); gifResult.set(null); }}>
            {t("video.createAnotherGif")}
          </button>
        </div>
      </div>
    {:else if $videoMode === "gif" && $gifState === "error" && $gifError}
      <div class="gif-error-card" role="alert">
        <CircleAlert size={14} />
        <span>{$gifError}</span>
      </div>
    {/if}

    <!-- Trim done state -->
    {#if $videoMode === "trim" && $trimState === "done" && $trimOutputPath}
      <div class="gif-done">
        <CircleCheck size={28} color="var(--accent)" />
        <span class="gif-done-name">{$trimOutputPath.split(/[\\/]/).pop()}</span>
        <div class="gif-done-actions">
          <button class="btn-primary-sm" onclick={() => revealVideoOutput($trimOutputPath!)}>
            <FolderOpen size={14} />
            {t("video.showInFolderAction")}
          </button>
          <button class="btn-ghost" onclick={() => { trimState.set("idle"); trimOutputPath.set(null); trimResult.set(null); }}>
            {t("video.trimAnotherClip")}
          </button>
        </div>
      </div>
    {:else if $videoMode === "trim" && $trimState === "error" && $trimError}
      <div class="gif-error-card" role="alert">
        <CircleAlert size={14} />
        <span>{$trimError}</span>
      </div>
    {/if}

    <!-- Audio extraction progress -->
    {#if $videoMode === "audio" && $audioState === "extracting"}
      <div class="gif-progress-block">
        <div class="gif-progress-header">
          <span class="gif-progress-label">{t("video.extractingAudioLabel")}</span>
          <span class="gif-progress-pct">{$audioProgress}%</span>
        </div>
        <div class="gif-enc-track" role="progressbar" aria-label={t("video.extractingAudioAriaLabel")} aria-valuenow={$audioProgress} aria-valuemin={0} aria-valuemax={100}>
          <div class="gif-enc-fill" style="transform: scaleX({$audioProgress / 100})"></div>
        </div>
      </div>
    {/if}

    <!-- Audio done state -->
    {#if $videoMode === "audio" && $audioState === "done" && $audioResult}
      <div class="gif-done">
        <CircleCheck size={28} color="var(--accent)" />
        <span class="gif-done-name">{$audioResult.output_path.split(/[\\/]/).pop()}</span>
        <span class="gif-done-meta">
          {formatMB($audioResult.output_size)}
          {#if $audioResult.was_lossless_copy}
            <span class="lossless-badge"><CircleCheck size={11} />{t("video.losslessBadge")}</span>
          {/if}
        </span>
        <div class="gif-done-actions">
          <button class="btn-primary-sm" onclick={() => revealVideoOutput($audioResult!.output_path)}>
            <FolderOpen size={14} />
            {t("video.showInFolderAction")}
          </button>
          <button class="btn-ghost" onclick={() => { audioState.set("idle"); audioResult.set(null); audioProgress.set(0); }}>
            {t("video.extractAnotherAudio")}
          </button>
        </div>
      </div>
    {:else if $videoMode === "audio" && $audioState === "error" && $audioError}
      <div class="gif-error-card" role="alert">
        <CircleAlert size={14} />
        <span>{$audioError}</span>
      </div>
    {/if}
  </ToolShell>
  </div>
  </div>

  <PrivacyToast visible={showPrivacyChip} message={privacyChipText[privacyChipMode](privacyChipCount)} />
{/if}

<style>
  /* ── Mode bar (above DropZone/ToolShell) ── */
  .video-flow {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  /* El estado vacío de ToolShell (height:100%) se resuelve contra esta área
     flexada — el DropZone se centra en el hueco real bajo la barra. En el
     estado de resultados el contenido alto desborda visible y sigue
     scrolleando via .content-area, como antes. */
  .tool-area {
    flex: 1;
    min-height: 0;
  }

  .mode-bar {
    display: flex;
    justify-content: center;
    padding: 0 0 20px;
    flex-shrink: 0;
  }

  /* ── FFmpeg state container ── */
  .ffmpeg-state {
    height: 100%;
    padding: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* ── Checking spinner ── */
  .checking-icon {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── Hero icon ── */
  .hero-icon {
    position: relative;
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 28px;
  }

  .icon-glow {
    position: absolute;
    inset: -12px;
    border-radius: 50%;
    background: radial-gradient(circle, var(--accent-glow) 0%, transparent 70%);
  }

  .icon-glow.pulse {
    animation: pulse-glow 2s ease-in-out infinite;
  }

  @keyframes pulse-glow {
    0%, 100% { opacity: 0.6; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.12); }
  }

  /* ── Download hero ── */
  .download-hero {
    max-width: 420px;
    text-align: center;
  }

  .download-hero h2 {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin: 0 0 10px;
  }

  .subtitle {
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1.6;
    margin: 0 0 28px;
  }

  .trust-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 28px;
    text-align: left;
  }

  .trust-item {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
    color: var(--text-secondary);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px 16px;
    transition: border-color 0.2s ease;
  }

  .trust-item:hover { border-color: var(--accent-glow); }

  .download-error {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-bottom: 16px;
    padding: 10px 16px;
    font-size: 13px;
    color: var(--danger);
    background: var(--danger-bg);
    border-radius: var(--radius-sm);
  }

  .btn-download {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 14px 28px;
    background: var(--accent);
    color: var(--text-on-accent);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease, transform 0.15s ease;
  }

  .btn-download:hover { opacity: 0.92; transform: translateY(-1px); }
  .btn-download:active { transform: translateY(0) scale(0.98); }

  .fine-print {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 12px;
  }

  /* ── Download progress view ── */
  .download-progress-view {
    max-width: 380px;
    width: 100%;
    text-align: center;
  }

  .download-progress-view h2 {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin: 0 0 8px;
  }

  .big-progress-wrap { margin-top: 32px; }

  .big-progress-track {
    height: 8px;
    background: var(--navy-bg);
    border-radius: 4px;
    overflow: hidden;
  }

  .big-progress-fill {
    height: 100%;
    width: 100%;
    background: var(--accent);
    border-radius: 4px;
    position: relative;
    overflow: hidden;
    transform-origin: left;
    transition: transform 0.3s ease;
  }

  .progress-shine {
    position: absolute;
    top: 0; right: 0; bottom: 0;
    width: 60px;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.25), transparent);
    animation: shine 1.5s ease-in-out infinite;
  }

  @keyframes shine {
    0% { transform: translateX(-100px); }
    100% { transform: translateX(100px); }
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 8px;
    font-variant-numeric: tabular-nums;
  }

  /* ── File list status ── */
  .done-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .done-actions .btn-icon {
    padding: 4px;
    border-radius: 4px;
    color: var(--text-muted);
    transition: color 0.15s ease;
  }

  .done-actions .btn-icon:hover { color: var(--accent); }

  .warning-detail { color: var(--warning-text, #b45309); }
  .saved-total { font-size: 13px; color: var(--accent-text); font-weight: 500; }
  .compressing-detail { font-variant-numeric: tabular-nums; }

  .progress-bar-track {
    display: block;
    height: 3px;
    background: var(--navy-bg);
    border-radius: 2px;
    margin-top: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    display: block;
    height: 100%;
    width: 100%;
    background: var(--accent);
    border-radius: 2px;
    transform-origin: left center;
    transform: scaleX(0);
    transition: transform 0.2s ease;
  }

  .status-pct {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .preview-loading { font-style: italic; color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .preview-muted { color: var(--text-muted); font-variant-numeric: tabular-nums; }

  /* ── Estimate card — mirrors CompressPage's "Estimated" hero panel ── */
  .estimate-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 20px;
    background: var(--accent-subtle);
    border: 1px solid var(--accent-glow);
    border-radius: var(--radius-sm);
  }

  .estimate-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .estimate-hero {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .estimate-hero-num {
    font-size: 38px;
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.02em;
    color: var(--accent-text);
    font-variant-numeric: tabular-nums;
  }

  .estimate-hero-num small {
    font-size: 18px;
    font-weight: 500;
  }

  .estimate-hero-word {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .estimate-bar-track {
    width: 100%;
    height: 6px;
    border-radius: 999px;
    background: var(--navy-bg);
    overflow: hidden;
  }

  .estimate-bar-fill {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transform-origin: left center;
  }

  .estimate-range {
    font-size: 13px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .estimate-scanning {
    font-size: 13px;
    color: var(--text-muted);
    font-style: italic;
  }

  /* ── Accordion ── */
  .accordion {
    width: 100%;
  }

  .accordion-header {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 0;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .accordion-chevron {
    color: var(--text-muted);
    transition: transform 0.2s ease;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .accordion-chevron.open { transform: rotate(180deg); }

  .accordion-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 12px;
  }

  /* Reinforces ToolShell's global accent-bg active state, same treatment as
     CompressPage/ConvertPage/ResizePage's preset/format/quality pills. */
  .pill.active {
    font-weight: 600;
    box-shadow: var(--shadow-xs);
  }

  /* ── Privacy controls ── */
  .privacy-group { flex: 1 1 auto; }

  .privacy-control-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .privacy-hint {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  .privacy-icon-label {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .privacy-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--navy-bg);
    color: var(--text-muted);
    transition: background 0.2s ease, color 0.2s ease;
    flex-shrink: 0;
  }

  .privacy-icon.on { background: var(--accent-subtle); color: var(--accent); }

  /* ── GIF controls ── */
  .num-input {
    width: 72px;
    padding: 6px 10px;
    background: var(--navy-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  .gif-range-group .range-inputs {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .range-sep { color: var(--text-muted); font-size: 13px; }
  .sub-hint { color: var(--text-muted); font-weight: 400; margin-left: 6px; }

  .lossless-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: 6px;
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent) 32%, transparent);
    vertical-align: middle;
  }

  .lossless-badge :global(svg) {
    /* --accent-text, not --accent: the raw green is 1.71:1 against this
       badge's tinted light-mode background (WCAG 1.4.11 needs 3:1). */
    color: var(--accent-text);
    flex-shrink: 0;
  }

  .trim-length-hint {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .trim-length-hint.invalid { color: var(--danger); }

  .quality-slider { flex: 1; accent-color: var(--accent); }

  .quality-val {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    min-width: 28px;
    text-align: right;
  }

  .gif-gate {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--navy-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .gif-gate--error { border-color: var(--danger); background: var(--danger-bg); color: var(--danger); }
  .gif-gate--progress { flex-direction: column; align-items: stretch; }

  .gif-gate-progress-wrap { display: flex; flex-direction: column; gap: 6px; width: 100%; }

  .gif-progress-bar {
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }

  .gif-progress-fill {
    height: 100%;
    width: 100%;
    background: var(--accent);
    border-radius: 2px;
    transform-origin: left;
    transition: transform 100ms ease;
  }

  .gif-progress-text { font-size: 12px; color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .gif-gate-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  .gif-gate-error-text { display: flex; flex-direction: column; gap: 2px; flex: 1; }

  /* ── GIF done state ── */
  .gif-done {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 24px 16px;
    text-align: center;
  }

  .gif-done-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
    font-variant-numeric: tabular-nums;
  }

  .gif-done-meta {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .gif-done-actions {
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
    color: var(--text-on-accent);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease, transform 0.1s ease;
  }

  .btn-primary-sm:hover { opacity: 0.9; transform: translateY(-1px); }
  .btn-primary-sm:active { transform: translateY(0) scale(0.98); }

  .gif-error-card {
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

  /* ── GIF encoding progress block ── */
  .gif-progress-block {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .gif-progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .gif-progress-label {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .gif-progress-pct {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .gif-enc-track {
    height: 4px;
    background: var(--navy-bg);
    border-radius: 2px;
    overflow: hidden;
  }

  .gif-enc-fill {
    height: 100%;
    width: 100%;
    background: var(--accent);
    border-radius: 2px;
    transform-origin: left center;
    transform: scaleX(0);
    transition: transform 0.2s ease;
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .spinner, .icon-glow.pulse, .progress-shine { animation: none; }
    .big-progress-fill, .gif-enc-fill { transition: none; }
    .btn-download, .privacy-icon, .accordion-chevron { transition: none; }
  }
</style>
