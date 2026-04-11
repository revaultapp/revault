<script lang="ts">
  import { onMount } from "svelte";
  import { Film, Shield, Download, Zap, Wifi, CheckCircle, AlertCircle, X } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ToolShell from "./ToolShell.svelte";
  import { formatBytes } from "$lib/utils";
  import { VIDEO_EXTENSIONS, VIDEO_EXTENSIONS_RE } from "$lib/types";
  import {
    videoFiles,
    videoPreset,
    isCompressing,
    videoSummary,
    ffmpegStatus,
    ffmpegDownloadProgress,
    addVideoFiles,
    removeVideoFile,
    clearVideoFiles,
    compressVideoFile,
    cancelCompression,
    checkFfmpeg,
    downloadFfmpeg,
    type VideoFile,
    type VideoPreset,
  } from "$lib/stores/video";

  const presets: { value: VideoPreset; label: string }[] = [
    { value: "Email", label: "Email" },
    { value: "Web", label: "Web" },
    { value: "Archive", label: "Archive" },
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
    for (const file of pending) {
      await compressVideoFile(file, $videoPreset);
      const updated = $videoFiles.find((f) => f.path === file.path);
      if (updated?.status === "cancelled") break;
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
          <AlertCircle size={14} />
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

{:else if $videoFiles.length === 0}
  <!-- Ready: drop zone -->
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
    onaction={$isCompressing ? cancelCompression : $videoSummary.pending === 0 && $videoFiles.length > 0 ? clearVideoFiles : startCompression}
    {headerText}
  >
    {#snippet headerSub()}
      {#if $videoSummary.savedBytes > 0}
        <span class="saved-total">Saved {formatBytes($videoSummary.savedBytes)}</span>
      {/if}
    {/snippet}

    {#snippet fileDetail(file)}
      Ready
    {/snippet}

    {#snippet fileStatus(file)}
      <button class="btn-icon" onclick={() => removeVideoFile(file.path)}>
        <X size={16} />
      </button>
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
  </ToolShell>

{:else}
  <!-- Ready: file list -->
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
    onaction={$isCompressing ? cancelCompression : $videoSummary.pending === 0 && $videoFiles.length > 0 ? clearVideoFiles : startCompression}
    {headerText}
  >
    {#snippet headerSub()}
      {#if $videoSummary.savedBytes > 0}
        <span class="saved-total">Saved {formatBytes($videoSummary.savedBytes)}</span>
      {/if}
    {/snippet}

    {#snippet fileDetail(file)}
      {#if file.status === "idle"}
        Ready
      {:else if file.status === "compressing"}
        <span class="compressing-detail">
          {#if file.fps > 0}
            {file.fps.toFixed(0)} fps &middot; {file.speed.toFixed(1)}x
          {:else}
            Encoding...
          {/if}
        </span>
        <span class="progress-bar-track">
          <span class="progress-bar-fill" style="width: {file.progress}%"></span>
        </span>
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
      {:else if file.status === "done"}
        <CheckCircle size={18} />
      {:else if file.status === "error" || file.status === "cancelled"}
        <AlertCircle size={18} />
      {:else}
        <button class="btn-icon" onclick={() => removeVideoFile(file.path)}>
          <X size={16} />
        </button>
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
  </ToolShell>
{/if}

<style>
  /* ── FFmpeg state container ── */
  .ffmpeg-state {
    height: 100%;
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
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.2s ease;
  }

  .status-pct {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
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

    .btn-download {
      transition: none;
    }
  }
</style>
