<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { fly, slide } from "svelte/transition";
  import { Film, Shield, ShieldCheck, Download, Zap, Wifi, CircleCheck, CircleAlert, TriangleAlert, FolderOpen, X, ChevronDown } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ToolShell from "./ToolShell.svelte";
  import SegmentedControl from "./SegmentedControl.svelte";
  import { formatBytes } from "$lib/utils";
  import { VIDEO_EXTENSIONS, VIDEO_EXTENSIONS_RE } from "$lib/types";
  import {
    videoFiles,
    videoPreset,
    videoOutputDir,
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
    checkGifski,
    exportGif,
    refreshGifSizeEstimate,
    type VideoFile,
    type VideoPreset,
    type PrivacyMode,
  } from "$lib/stores/video";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  const modeSegments = [
    { id: "compress", label: "Comprimir" },
    { id: "gif", label: "GIF" },
  ] as const;

  const fpSegments = [
    { id: "10", label: "Baja · 10" },
    { id: "15", label: "Media · 15" },
    { id: "24", label: "Alta · 24" },
  ] as const;

  const widthSegments = [
    { id: "320", label: "Chat · 320" },
    { id: "480", label: "WhatsApp · 480" },
    { id: "640", label: "Grande · 640" },
    { id: "720", label: "HD · 720" },
    { id: "1080", label: "Full HD · 1080" },
  ] as const;

  const presets: { value: VideoPreset; label: string }[] = [
    { value: "Smallest", label: "Smallest" },
    { value: "Balanced", label: "Balanced" },
    { value: "HighQuality", label: "High Quality" },
  ];

  const privacySegments = [
    { id: "off" satisfies PrivacyMode, label: "Sin cambios" },
    { id: "smart" satisfies PrivacyMode, label: "Recomendado" },
    { id: "gps_only" satisfies PrivacyMode, label: "Solo ubicación" },
    { id: "full" satisfies PrivacyMode, label: "Máximo" },
  ] as const;

  const privacyTooltips: Record<PrivacyMode, string> = {
    off: "Mantiene el vídeo tal cual, con toda su información original.",
    smart: "Borra dónde se grabó el vídeo y el modelo del móvil, pero conserva la fecha.",
    gps_only: "Solo elimina la ubicación. Conserva fecha, cámara y demás detalles.",
    full: "Borra todo: ubicación, fecha, cámara y cualquier otro rastro del dispositivo.",
  };

  const privacyChipText: Record<Exclude<PrivacyMode, "off">, (n: number) => string> = {
    smart: (n) => `Ubicación y dispositivo eliminados en ${n} vídeo${n !== 1 ? "s" : ""}`,
    gps_only: (n) => `Ubicación eliminada en ${n} vídeo${n !== 1 ? "s" : ""}`,
    full: (n) => `Todos los metadatos eliminados en ${n} vídeo${n !== 1 ? "s" : ""}`,
  };

  // ── Accordion state ─────────────────────────────────────────────────────────
  let settingsOpen = $state(false);

  let accordionHeader = $derived.by(() => {
    const preset = $videoPreset;
    const mode = $videoPrivacyMode;
    const privacyLabel = privacySegments.find(s => s.id === mode)?.label ?? mode;
    if ($videoMode === "compress") {
      return `Ajustes · ${preset} · ${privacyLabel}`;
    }
    const fps = $gifSettings.fps;
    const width = $gifSettings.width;
    return `Ajustes · ${fps} fps · ${width}px`;
  });

  // ── Progress & header ───────────────────────────────────────────────────────
  let targetPct = $derived(
    $videoFiles.length === 0
      ? 0
      : (($videoSummary.done + $videoSummary.failed) / $videoFiles.length) * 100
  );

  let headerText = $derived(
    $videoSummary.done > 0 || $videoSummary.failed > 0
      ? `${$videoSummary.done} of ${$videoFiles.length} compressed${$videoSummary.failed > 0 ? ` · ${$videoSummary.failed} failed` : ""}`
      : `${$videoFiles.length} video${$videoFiles.length !== 1 ? "s" : ""} selected`
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

  let unlistenGifProgress: (() => void) | undefined;

  onMount(async () => {
    checkFfmpeg();
    checkGifski();
    unlistenGifProgress = await listen<{ bytes_done: number; bytes_total: number }>(
      "gifski-download-progress",
      (e) => gifDownloadProgress.set({ done: e.payload.bytes_done, total: e.payload.bytes_total })
    );
  });

  onDestroy(() => unlistenGifProgress?.());

  $effect(() => {
    const settings = $gifSettings;
    if ($videoMode === "gif" && $videoFiles.length > 0 && settings) {
      refreshGifSizeEstimate();
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
    gifDownloadProgress.set({ done: 0, total: 0 });
    try {
      await invoke("download_gifski");
      gifDownloadProgress.set(null);
      isGifInstalling = true;
      await checkGifski();
    } catch (err) {
      gifDownloadProgress.set(null);
      isGifInstalling = false;
      gifDownloadError = "No hemos podido descargar el componente. Comprueba tu conexión e inténtalo de nuevo más tarde.";
    } finally {
      isGifInstalling = false;
    }
  }

  async function startGifExport() {
    const file = $videoFiles[0];
    if (!file) return;
    await exportGif(file);
  }

  async function startCompression() {
    const pending = $videoFiles.filter((f) => f.status === "idle");
    isCompressing.set(true);
    try {
      for (const file of pending) {
        await compressVideoFile(file, $videoPreset, $videoOutputDir);
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
      filters: [{ name: "Videos", extensions: [...VIDEO_EXTENSIONS] }],
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
    if (pct > 0) return `${pct}% smaller`;
    if (pct < 0) return `${Math.abs(pct)}% larger`;
    return "Same size";
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
  let previewDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const filesSnapshot = $videoFiles;
    const presetSnapshot = $videoPreset;
    const privacySnapshot = $videoPrivacyMode;
    const compressing = $isCompressing;

    if (previewDebounceTimer !== null) {
      clearTimeout(previewDebounceTimer);
      previewDebounceTimer = null;
    }

    if (compressing) return;

    void presetSnapshot;
    void privacySnapshot;

    const idlePaths = filesSnapshot
      .filter((f) => f.status === "idle")
      .map((f) => f.path);

    if (idlePaths.length === 0) return;

    previewDebounceTimer = setTimeout(() => {
      for (const path of idlePaths) {
        computeVideoPreview(path);
      }
    }, 300);
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
    <div class="download-hero" role="region" aria-label="FFmpeg setup required">
      <div class="hero-icon">
        <div class="icon-glow"></div>
        <Film size={48} color="var(--accent)" />
      </div>
      <h2>Compresión de vídeo, desbloqueada</h2>
      <p class="subtitle">
        ReVault necesita FFmpeg para comprimir vídeos.<br />
        Una sola descarga. Funciona sin conexión para siempre.
      </p>
      <div class="trust-grid">
        <div class="trust-item">
          <Shield size={16} color="var(--accent)" />
          <span>Privado &mdash; descargado directamente desde FFmpeg.org</span>
        </div>
        <div class="trust-item">
          <Wifi size={16} color="var(--accent)" />
          <span>Solo una vez &mdash; después funciona sin internet</span>
        </div>
        <div class="trust-item">
          <Zap size={16} color="var(--accent)" />
          <span>Estándar del sector &mdash; lo usan YouTube y Netflix</span>
        </div>
      </div>
      {#if downloadError}
        <div class="download-error" role="alert">
          <CircleAlert size={14} />
          <span>No se pudo descargar. Comprueba tu conexión e inténtalo de nuevo.</span>
        </div>
      {/if}
      <button class="btn-download" onclick={handleDownload}>
        <Download size={18} />
        Descargar FFmpeg &middot; Gratis
      </button>
      <p class="fine-print">~80 MB &middot; Solo tu red</p>
    </div>
  </div>

{:else if $ffmpegStatus === "downloading"}
  <div class="ffmpeg-state">
    <div class="download-progress-view" role="region" aria-label="Downloading FFmpeg" aria-live="polite">
      <div class="hero-icon downloading">
        <div class="icon-glow pulse"></div>
        <Film size={48} color="var(--accent)" />
      </div>
      <h2>Preparando FFmpeg&hellip;</h2>
      <p class="subtitle">Solo una vez. No volverás a ver esta pantalla.</p>
      <div class="big-progress-wrap">
        <div class="big-progress-track" role="progressbar" aria-valuenow={Math.round($ffmpegDownloadProgress.percent)} aria-valuemin={0} aria-valuemax={100}>
          <div class="big-progress-fill" style="--progress: {$ffmpegDownloadProgress.percent}%">
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
  <!-- Mode toggle: centered, above DropZone / file list -->
  <div class="mode-bar">
    <SegmentedControl segments={modeSegments} bind:selected={$videoMode} />
  </div>

  <ToolShell
    files={$videoFiles}
    isProcessing={$isCompressing || $gifState === "generating"}
    {targetPct}
    progressLabel="{$videoSummary.done + $videoSummary.failed} of {$videoFiles.length} files"
    progressSublabel={$videoSummary.savedBytes > 0 ? `Saved ${formatBytes($videoSummary.savedBytes)}` : undefined}
    onfiles={handleFiles}
    onbrowse={browseFiles}
    onclear={clearVideoFiles}
    actionLabel={$videoMode === "gif"
      ? (!$gifskiAvailable ? "" : $gifState === "generating" ? "Generando…" : "Exportar GIF")
      : ($isCompressing ? "Cancel" : $videoSummary.pending === 0 && $videoFiles.length > 0 ? "Compress More" : "Compress")}
    onaction={$videoMode === "gif"
      ? startGifExport
      : ($isCompressing ? cancelCompression : $videoSummary.pending === 0 && $videoFiles.length > 0 ? compressMore : startCompression)}
    {headerText}
    dropZoneTitle="Drop videos here"
    dropZoneFormatTags={["MP4", "MOV", "AVI", "MKV", "WebM", "M4V"]}
    dropZoneAcceptedExtensions={VIDEO_EXTENSIONS_RE}
    dropZoneFilePickerName="Videos"
    dropZoneFilePickerExtensions={[...VIDEO_EXTENSIONS]}
    showThumbnails={false}
    placeholderIcon="video"
  >
    {#snippet headerSub()}
      {#if $videoSummary.savedBytes > 0}
        <span class="saved-total">Saved {formatBytes($videoSummary.savedBytes)}</span>
      {/if}
    {/snippet}

    {#snippet fileDetail(file)}
      {#if file.status === "idle"}
        {@const preview = $videoPreviews.get(file.path)}
        {#if !preview || preview.status === "idle"}
          Ready
        {:else if preview.status === "loading"}
          <span class="preview-loading">Calculando estimación&hellip;</span>
        {:else if preview.status === "ready"}
          {#if preview.preview.estimatedSavingsPct < 3}
            <span class="preview-muted">Ya está bien comprimido</span>
          {:else}
            <span class="preview-muted">
              {formatMB(preview.preview.originalSizeBytes)}
              &rarr; ~{formatMB(preview.preview.estimatedSizeBytes)}
              &middot; {Math.round(preview.preview.estimatedSavingsPct)}% menos
            </span>
          {/if}
        {:else}
          Ready
        {/if}
      {:else if file.status === "compressing"}
        <span class="compressing-detail">
          {#if file.fps > 0}
            {file.fps.toFixed(0)} fps &middot; {file.speed.toFixed(1)}x
          {:else}
            Encoding...
          {/if}
        </span>
        <span class="progress-bar-track">
          <span class="progress-bar-fill" style="transform: scaleX({file.progress / 100})"></span>
        </span>
      {:else if file.status === "done" && isOutputLarger(file)}
        <span class="warning-detail">Already optimized &middot; {formatBytes(file.originalSize)} kept</span>
      {:else if file.status === "done"}
        {formatBytes(file.originalSize)} &rarr; {formatBytes(file.compressedSize ?? 0)} &middot; {savedPercent(file)}
      {:else if file.status === "error"}
        {file.error ?? "Compression failed"}
      {:else if file.status === "cancelled"}
        Cancelled
      {/if}
    {/snippet}

    {#snippet fileStatus(file)}
      {#if file.status === "compressing"}
        <span class="status-pct">{Math.round(file.progress)}%</span>
      {:else if file.status === "done" && isOutputLarger(file)}
        <div class="done-actions">
          {#if file.outputPath}
            <button class="btn-icon" aria-label="Reveal in file manager" onclick={() => revealVideoOutput(file.outputPath!)}>
              <FolderOpen size={16} />
            </button>
          {/if}
          <TriangleAlert size={18} color="var(--warning)" />
        </div>
      {:else if file.status === "done"}
        <div class="done-actions">
          {#if file.outputPath}
            <button class="btn-icon" aria-label="Reveal in file manager" onclick={() => revealVideoOutput(file.outputPath!)}>
              <FolderOpen size={16} />
            </button>
          {/if}
          <CircleCheck size={18} />
        </div>
      {:else if file.status === "error" || file.status === "cancelled"}
        <CircleAlert size={18} />
      {:else}
        <button class="btn-icon" aria-label="Remove file" onclick={() => removeVideoFile(file.path)}>
          <X size={16} />
        </button>
      {/if}
    {/snippet}

    {#snippet estimateCard()}
      {#if $videoMode === "compress" && $videoFiles.length > 0}
        {#if estimateState.kind === "loading"}
          <div class="estimate-hero-block">
            <span class="estimate-num estimate-num--loading">…</span>
            <span class="estimate-sub">Calculando ahorro&hellip;</span>
          </div>
        {:else if estimateState.kind === "ready"}
          <div class="estimate-hero-block">
            <span class="estimate-num">-{Math.round(estimateState.savingsPct)}%</span>
            <span class="estimate-sub">
              Estimado: {formatMB(estimateState.totalOriginal)} &rarr; {formatMB(estimateState.totalEstimated)}
            </span>
          </div>
        {:else if estimateState.kind === "no-gain"}
          <div class="estimate-hero-block">
            <span class="estimate-sub estimate-sub--muted">Tus videos ya están bien optimizados</span>
          </div>
        {/if}
      {:else if $videoMode === "gif" && $gifskiAvailable && $videoFiles.length > 0 && $gifSizeEstimate !== null}
        <div class="estimate-hero-block">
          <span class="estimate-num">≈ {formatMB($gifSizeEstimate)}</span>
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
              <span class="label">Preset</span>
              <div class="pills">
                {#each presets as p}
                  <button
                    class="pill"
                    class:active={$videoPreset === p.value}
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
                  <span class="label">Privacidad</span>
                </div>
                <SegmentedControl segments={privacySegments} bind:selected={$videoPrivacyMode} />
              </div>
              <p class="privacy-hint">{privacyTooltips[$videoPrivacyMode]}</p>
            </div>

          {:else}
            <!-- GIF mode controls -->
            {#if $gifskiAvailable === false}
              {#if gifGateState === "idle"}
                <div class="gif-gate" role="status">
                  <CircleAlert size={14} />
                  <span>Para crear GIFs, ReVault necesita descargar un componente extra. Es rápido y solo pasa una vez.</span>
                  <button class="btn-ghost" onclick={handleGifDownload}>Descargar</button>
                </div>
              {:else if gifGateState === "downloading"}
                <div class="gif-gate gif-gate--progress" role="status" aria-live="polite">
                  <div class="gif-gate-progress-wrap">
                    <div class="gif-progress-bar">
                      <div
                        class="gif-progress-fill"
                        style="width: {$gifDownloadProgress && $gifDownloadProgress.total > 0 ? ($gifDownloadProgress.done / $gifDownloadProgress.total) * 100 : 0}%"
                      ></div>
                    </div>
                    <span class="gif-progress-text">
                      {#if $gifDownloadProgress && $gifDownloadProgress.total > 0}
                        Descargando… {formatBytes($gifDownloadProgress.done)} de {formatBytes($gifDownloadProgress.total)}
                        <span class="gif-progress-pct">{Math.round(($gifDownloadProgress.done / $gifDownloadProgress.total) * 100)}%</span>
                      {:else}
                        Descargando…
                      {/if}
                    </span>
                  </div>
                </div>
              {:else if gifGateState === "installing"}
                <div class="gif-gate" role="status" aria-live="polite">
                  <div class="gif-gate-spinner"></div>
                  <span>Casi listo…</span>
                </div>
              {:else if gifGateState === "error"}
                <div class="gif-gate gif-gate--error" role="alert">
                  <CircleAlert size={14} />
                  <div class="gif-gate-error-text">
                    <span>No hemos podido completar la descarga.</span>
                    <small>Comprueba tu conexión e inténtalo de nuevo. Si persiste, prueba más tarde.</small>
                  </div>
                  <button class="btn-ghost" onclick={handleGifDownload}>Reintentar</button>
                </div>
              {/if}
            {/if}

            {#if $gifskiAvailable}
              <div class="control-group gif-range-group">
                <span class="label">Fragmento <span class="sub-hint">(máx 15 s)</span></span>
                <div class="range-inputs">
                  <input
                    type="number"
                    class="num-input"
                    min="0"
                    step="0.5"
                    aria-label="Segundo inicial"
                    value={$gifSettings.startSec}
                    oninput={(e) => gifSettings.update(s => ({ ...s, startSec: Math.max(0, parseFloat((e.target as HTMLInputElement).value) || 0) }))}
                  />
                  <span class="range-sep">–</span>
                  <input
                    type="number"
                    class="num-input"
                    min="0"
                    step="0.5"
                    aria-label="Segundo final"
                    value={$gifSettings.endSec}
                    oninput={(e) => gifSettings.update(s => ({ ...s, endSec: Math.max(s.startSec + 0.5, Math.min(s.startSec + 15, parseFloat((e.target as HTMLInputElement).value) || s.startSec + 3)) }))}
                  />
                </div>
              </div>
              <div class="control-group">
                <span class="label">Fluidez</span>
                <div class="pills">
                  {#each fpSegments as seg}
                    <button
                      class="pill"
                      class:active={$gifSettings.fps === parseInt(seg.id)}
                      onclick={() => gifSettings.update(s => ({ ...s, fps: parseInt(seg.id) as 10 | 15 | 24 }))}
                    >{seg.label}</button>
                  {/each}
                </div>
              </div>
              <div class="control-group">
                <span class="label">Tamaño</span>
                <div class="pills">
                  {#each widthSegments as seg}
                    <button
                      class="pill"
                      class:active={$gifSettings.width === parseInt(seg.id)}
                      onclick={() => gifSettings.update(s => ({ ...s, width: parseInt(seg.id) as 320 | 480 | 640 | 720 | 1080 }))}
                    >{seg.label}</button>
                  {/each}
                </div>
              </div>
              <div class="control-group">
                <span class="label">Calidad</span>
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
          {#if $videoMode === "compress" || $gifskiAvailable}
            <div class="control-group">
              <span class="label">Carpeta</span>
              <button class="btn-ghost output-btn" onclick={browseOutputDir}>
                <FolderOpen size={14} />
                {$videoOutputDir?.split(/[\\/]/).pop() ?? "Misma que el origen"}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>

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
            Abrir carpeta
          </button>
          <button class="btn-ghost" onclick={() => { gifState.set("idle"); gifOutputPath.set(null); gifResult.set(null); }}>
            Crear otro GIF
          </button>
        </div>
      </div>
    {:else if $videoMode === "gif" && $gifState === "error" && $gifError}
      <div class="gif-error-card" role="alert">
        <CircleAlert size={14} />
        <span>{$gifError}</span>
      </div>
    {/if}
  </ToolShell>

  {#if showPrivacyChip}
    <div class="privacy-chip-wrap" transition:fly={{ y: -4, duration: 200 }}>
      <div class="privacy-chip" role="status" aria-live="polite">
        <ShieldCheck size={14} />
        <span>{privacyChipText[privacyChipMode](privacyChipCount)}</span>
      </div>
    </div>
  {/if}
{/if}

<style>
  /* ── Mode bar (above DropZone/ToolShell) ── */
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
    color: #fff;
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
    width: var(--progress);
    background: var(--accent);
    border-radius: 4px;
    position: relative;
    overflow: hidden;
    transition: width 0.3s ease;
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

  .warning-detail { color: var(--warning, #f59e0b); }
  .saved-total { font-size: 13px; color: var(--accent); font-weight: 500; }
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

  /* ── Estimate hero block ── */
  .estimate-hero-block {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 20px 16px 16px;
    text-align: center;
  }

  .estimate-num {
    font-size: 48px;
    font-weight: 800;
    letter-spacing: -0.04em;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .estimate-num--loading {
    color: var(--text-muted);
    font-size: 36px;
  }

  .estimate-sub {
    font-size: 13px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .estimate-sub--muted { color: var(--text-muted); font-style: italic; }

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

  /* ── Privacy chip ── */
  .privacy-chip-wrap {
    position: fixed;
    bottom: 24px;
    right: 28px;
    z-index: 10;
    pointer-events: none;
  }

  .privacy-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.35);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-sm);
  }

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
    background: var(--accent);
    border-radius: 2px;
    transition: width 100ms ease;
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
    color: #fff;
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

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .spinner, .icon-glow.pulse, .progress-shine { animation: none; }
    .big-progress-fill { transition: none; }
    .btn-download, .privacy-icon, .accordion-chevron { transition: none; }
  }
</style>
