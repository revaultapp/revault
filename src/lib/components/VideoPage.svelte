<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { Film, Shield, ShieldCheck, Download, Zap, Wifi, CircleCheck, CircleAlert, TriangleAlert, FolderOpen, X } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ToolShell from "./ToolShell.svelte";
  import ToggleSwitch from "./ToggleSwitch.svelte";
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
    videoStripPrivacy,
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
    type VideoFile,
    type VideoPreset,
  } from "$lib/stores/video";

  const presets: { value: VideoPreset; label: string }[] = [
    { value: "Smallest", label: "Smallest" },
    { value: "Balanced", label: "Balanced" },
    { value: "HighQuality", label: "High Quality" },
  ];

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

  onMount(() => {
    checkFfmpeg();
  });

  async function handleDownload() {
    downloadError = null;
    try {
      await downloadFfmpeg();
    } catch (e) {
      downloadError = String(e);
    }
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

  // ── Inline preview formatting ───────────────────────────────────────────────
  function formatMB(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    if (mb >= 100) return `${Math.round(mb)} MB`;
    if (mb >= 10) return `${mb.toFixed(0)} MB`;
    return `${mb.toFixed(1)} MB`;
  }

  // ── Debounced preview trigger ───────────────────────────────────────────────
  let previewDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Subscribe to the signals that should re-trigger preview estimation.
    const filesSnapshot = $videoFiles;
    const presetSnapshot = $videoPreset;
    const stripSnapshot = $videoStripPrivacy;
    const compressing = $isCompressing;

    // Cancel pending debounce whenever inputs change.
    if (previewDebounceTimer !== null) {
      clearTimeout(previewDebounceTimer);
      previewDebounceTimer = null;
    }

    // Guard: don't fire previews during real encode.
    if (compressing) return;

    // Silence the "unused variable" for reactive tracking only.
    void presetSnapshot;
    void stripSnapshot;

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

  // ── Post-success privacy micro-moment ──────────────────────────────────────
  let showPrivacyChip = $state(false);
  let privacyChipCount = $state(0);
  let privacyChipTimer: ReturnType<typeof setTimeout> | null = null;
  let lastAllDoneSignature = "";

  $effect(() => {
    const files = $videoFiles;
    const stripping = $videoStripPrivacy;

    // Detect "all files reached done" edge with a stable signature so we only
    // fire the chip once per completed batch.
    const signature = files.length > 0
      ? files.map((f) => `${f.path}:${f.status}`).join("|")
      : "";

    if (signature === lastAllDoneSignature) return;

    if (
      files.length > 0 &&
      files.every((f) => f.status === "done") &&
      stripping
    ) {
      lastAllDoneSignature = signature;
      privacyChipCount = files.length;
      showPrivacyChip = true;

      if (privacyChipTimer !== null) clearTimeout(privacyChipTimer);
      privacyChipTimer = setTimeout(() => {
        showPrivacyChip = false;
      }, 4000);
    } else if (files.length === 0) {
      // Reset when the list is cleared.
      lastAllDoneSignature = "";
      showPrivacyChip = false;
      if (privacyChipTimer !== null) {
        clearTimeout(privacyChipTimer);
        privacyChipTimer = null;
      }
    }
  });

  // ── Estimate card helpers ──────────────────────────────────────────────────
  let estimateState = $derived.by(() => {
    const summary = $videoPreviewSummary;
    if (!summary || summary.filesTotal === 0) return { kind: "hidden" as const };
    if (summary.filesReady < summary.filesTotal) return { kind: "loading" as const };
    if (summary.totalSaved > 0) {
      return {
        kind: "ready" as const,
        totalSaved: summary.totalSaved,
        savingsPct: summary.savingsPct,
        filesTotal: summary.filesTotal,
      };
    }
    return { kind: "no-gain" as const };
  });

</script>

{#if $ffmpegStatus === "checking"}
  <!-- Checking state: minimal spinner -->
  <div class="ffmpeg-state">
    <div class="checking-icon">
      <Film size={40} color="var(--accent)" />
      <div class="spinner"></div>
    </div>
  </div>

{:else if $ffmpegStatus === "needs_download"}
  <!-- Welcome / download prompt -->
  <div class="ffmpeg-state">
    <div class="download-hero" role="region" aria-label="FFmpeg setup required">
      <div class="hero-icon">
        <div class="icon-glow"></div>
        <Film size={48} color="var(--accent)" />
      </div>

      <h2>Video compression, unlocked</h2>
      <p class="subtitle">
        ReVault needs FFmpeg to compress videos.<br />
        One download. Works offline forever after.
      </p>

      <div class="trust-grid">
        <div class="trust-item">
          <Shield size={16} color="var(--accent)" />
          <span>Private &mdash; downloaded directly from FFmpeg.org</span>
        </div>
        <div class="trust-item">
          <Wifi size={16} color="var(--accent)" />
          <span>One-time only &mdash; works offline after</span>
        </div>
        <div class="trust-item">
          <Zap size={16} color="var(--accent)" />
          <span>Industry standard &mdash; used by YouTube, Netflix</span>
        </div>
      </div>

      {#if downloadError}
        <div class="download-error" role="alert">
          <CircleAlert size={14} />
          <span>Download failed. Check your connection and try again.</span>
        </div>
      {/if}

      <button class="btn-download" onclick={handleDownload}>
        <Download size={18} />
        Download FFmpeg &middot; Free
      </button>
      <p class="fine-print">~80 MB &middot; Your network only</p>
    </div>
  </div>

{:else if $ffmpegStatus === "downloading"}
  <!-- Download progress -->
  <div class="ffmpeg-state">
    <div class="download-progress-view" role="region" aria-label="Downloading FFmpeg" aria-live="polite">
      <div class="hero-icon downloading">
        <div class="icon-glow pulse"></div>
        <Film size={48} color="var(--accent)" />
      </div>

      <h2>Setting up FFmpeg&hellip;</h2>
      <p class="subtitle">Just once. You'll never see this screen again.</p>

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
  <ToolShell
    files={$videoFiles}
    isProcessing={$isCompressing}
    {targetPct}
    progressLabel="{$videoSummary.done + $videoSummary.failed} of {$videoFiles.length} files"
    progressSublabel={$videoSummary.savedBytes > 0 ? `Saved ${formatBytes($videoSummary.savedBytes)}` : undefined}
    onfiles={handleFiles}
    onbrowse={browseFiles}
    onclear={clearVideoFiles}
    actionLabel={$isCompressing ? "Cancel" : $videoSummary.pending === 0 && $videoFiles.length > 0 ? "Compress More" : "Compress"}
    onaction={$isCompressing ? cancelCompression : $videoSummary.pending === 0 && $videoFiles.length > 0 ? compressMore : startCompression}
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
      {#if estimateState.kind === "loading"}
        <div class="estimate-card">
          <div class="estimate-row">
            <span class="estimate-label">Estimado</span>
            <span class="estimate-value estimating">Calculando ahorro&hellip;</span>
          </div>
        </div>
      {:else if estimateState.kind === "ready"}
        <div class="estimate-card">
          <div class="estimate-hero">
            <span class="estimate-hero-text">
              Ahorrarás <span class="estimate-hero-number">~{formatMB(estimateState.totalSaved)}</span>
              <span class="estimate-hero-pct">({Math.round(estimateState.savingsPct)}%)</span>
            </span>
          </div>
          <span class="estimate-sub">
            en {estimateState.filesTotal} video{estimateState.filesTotal !== 1 ? "s" : ""}
          </span>
        </div>
      {:else if estimateState.kind === "no-gain"}
        <div class="estimate-card muted">
          <div class="estimate-row">
            <span class="estimate-value estimating">Tus videos ya están bien optimizados</span>
          </div>
        </div>
      {/if}
    {/snippet}

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
      <div class="toggle-row privacy-row">
        <div class="toggle-label privacy-toggle-label">
          <div class="privacy-icon" class:on={$videoStripPrivacy} aria-hidden="true">
            <ShieldCheck size={16} />
          </div>
          <div class="privacy-text">
            <span class="label">Proteger mi privacidad</span>
            <span class="control-hint">
              {$videoStripPrivacy
                ? "Se eliminará la ubicación y los datos del dispositivo"
                : "Tus videos conservarán la ubicación original"}
            </span>
          </div>
        </div>
        <ToggleSwitch bind:checked={$videoStripPrivacy} />
      </div>
    </div>
    <div class="control-group">
      <span class="label">Output</span>
      <button class="btn-ghost output-btn" onclick={browseOutputDir}>
        <FolderOpen size={14} />
        {$videoOutputDir?.split(/[\\/]/).pop() ?? "Same as input"}
      </button>
    </div>
  </ToolShell>

  {#if showPrivacyChip}
    <div class="privacy-chip-wrap" transition:fly={{ y: -4, duration: 200 }}>
      <div class="privacy-chip" role="status" aria-live="polite">
        <ShieldCheck size={14} />
        <span>
          Ubicación eliminada &middot; {privacyChipCount} video{privacyChipCount !== 1 ? "s" : ""}
        </span>
      </div>
    </div>
  {/if}
{/if}

<style>
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

  /* ── Hero icon (shared by needs_download + downloading) ── */
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

  /* ── Download hero (needs_download) ── */
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

  /* ── Trust grid ── */
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

  .trust-item:hover {
    border-color: var(--accent-glow);
  }

  /* ── Download error ── */
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

  /* ── Download button ── */
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

  .btn-download:hover {
    opacity: 0.92;
    transform: translateY(-1px);
  }

  .btn-download:active {
    transform: translateY(0) scale(0.98);
  }

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

  .big-progress-wrap {
    margin-top: 32px;
  }

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
    top: 0;
    right: 0;
    bottom: 0;
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

  /* ── Done actions row ── */
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

  .done-actions .btn-icon:hover {
    color: var(--accent);
  }

  /* ── Warning detail for "already optimized" ── */
  .warning-detail {
    color: var(--warning, #f59e0b);
  }

  /* ── Existing video file list styles ── */
  .saved-total {
    font-size: 13px;
    color: var(--accent);
    font-weight: 500;
  }

  .compressing-detail {
    font-variant-numeric: tabular-nums;
  }

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

  /* ── Inline preview in file rows ── */
  .preview-loading {
    font-style: italic;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .preview-muted {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  /* ── Estimate card (pre-compression savings banner) ── */
  .estimate-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 16px;
    background: var(--accent-subtle);
    border: 1px solid var(--accent-glow);
    border-radius: var(--radius-sm);
  }

  .estimate-card.muted {
    background: var(--navy-bg);
    border-color: var(--border);
  }

  .estimate-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .estimate-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
  }

  .estimate-value {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .estimating {
    color: var(--text-muted);
    font-style: italic;
  }

  .estimate-hero {
    font-size: 15px;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .estimate-hero-text {
    font-weight: 500;
  }

  .estimate-hero-number {
    font-size: 20px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -0.02em;
  }

  .estimate-hero-pct {
    font-size: 14px;
    font-weight: 600;
    color: var(--accent);
  }

  .estimate-sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* ── Privacy toggle ── */
  .privacy-group {
    flex: 1 1 280px;
    min-width: 260px;
  }

  .privacy-row {
    align-items: center;
  }

  .privacy-toggle-label {
    display: flex;
    align-items: center;
    gap: 10px;
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

  .privacy-icon.on {
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .privacy-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  /* ── Privacy success chip ── */
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

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .spinner,
    .icon-glow.pulse,
    .progress-shine {
      animation: none;
    }

    .big-progress-fill {
      transition: none;
    }

    .btn-download,
    .privacy-icon {
      transition: none;
    }
  }
</style>
