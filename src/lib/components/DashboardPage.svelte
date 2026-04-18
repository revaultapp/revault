<script lang="ts">
  import { HardDrive, Image, Minimize2, Zap, Search, Shield, FolderOpen } from "lucide-svelte";
  import Button from "./Button.svelte";
  import { savings } from "$lib/stores/savings";
  import { activity, formatTimeAgo } from "$lib/stores/activity";
  import { activePage } from "$lib/stores/nav";
  import { formatBytes } from "$lib/utils";
  import { storage, breakdown } from "$lib/stores/storage";

  let avgCompression = $derived(
    $savings.totalOriginalBytes > 0
      ? Math.round(($savings.totalOriginalBytes - $savings.totalCompressedBytes) / $savings.totalOriginalBytes * 100)
      : 0
  );

  function navigate(page: string) {
    activePage.set(page as typeof $activePage);
  }

  const activityLabels: Record<string, string> = {
    compress: "Compressed",
    convert: "Converted",
    resize: "Resized",
    analyze: "Analyzed",
  };
</script>

<div class="dashboard">
  <!-- Stats Row -->
  <div class="stats-row">
    <div class="stat-card accent">
      <div class="stat-icon">
        <HardDrive size={20} />
      </div>
      <div class="stat-content">
        <span class="stat-value">{formatBytes($savings.totalSavedBytes)}</span>
        <span class="stat-label">Space Saved</span>
        <span class="stat-sub">{$savings.filesProcessed} files processed</span>
      </div>
    </div>

    <div class="stat-card">
      <div class="stat-icon">
        <Image size={20} />
      </div>
      <div class="stat-content">
        <span class="stat-value">{$savings.operationsCount}</span>
        <span class="stat-label">Images Cleaned</span>
        <span class="stat-sub">metadata stripped</span>
      </div>
    </div>

    <div class="stat-card">
      <div class="stat-icon">
        <Minimize2 size={20} />
      </div>
      <div class="stat-content">
        <span class="stat-value">{avgCompression}%</span>
        <span class="stat-label">Avg Compression</span>
        <span class="stat-sub">per file on average</span>
      </div>
    </div>

    <div class="stat-card accent">
      <div class="stat-icon">
        <Zap size={20} />
      </div>
      <div class="stat-content">
        <span class="stat-value">{$savings.heicCount}</span>
        <span class="stat-label">HEIC Converted</span>
        <span class="stat-sub">macOS native decode</span>
      </div>
    </div>
  </div>

  <!-- Quick Actions -->
  <div class="section-title">Quick Actions</div>
  <div class="actions-grid">
    <button class="action-card accent" onclick={() => navigate("optimize")}>
      <div class="action-icon">
        <Zap size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Compress Images</span>
        <span class="action-desc">Reduce file size without quality loss</span>
      </div>
    </button>

    <button class="action-card" onclick={() => navigate("duplicates")}>
      <div class="action-icon">
        <Search size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Analyze Folder</span>
        <span class="action-desc">Find duplicates and storage hogs</span>
      </div>
    </button>

    <button class="action-card" onclick={() => navigate("privacy")}>
      <div class="action-icon">
        <Shield size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Privacy Scan</span>
        <span class="action-desc">Strip GPS and identifying metadata</span>
      </div>
    </button>
  </div>

  <!-- Bottom Row -->
  <div class="bottom-row">
    <!-- Recent Activity -->
    <section class="activity">
      <h3>Recent Activity</h3>
      {#if $activity.length === 0}
        <div class="empty-state">
          <p>No activity yet</p>
          <span>Compress or convert some images to get started</span>
        </div>
      {:else}
        <div class="activity-list">
          {#each $activity as item (item.id)}
            <div class="activity-item">
              <div class="activity-icon">
                {#if item.type === "compress"}
                  <Minimize2 size={14} />
                {:else if item.type === "convert"}
                  <Zap size={14} />
                {:else}
                  <Image size={14} />
                {/if}
              </div>
              <div class="activity-info">
                <span class="activity-label">{activityLabels[item.type] ?? item.type} {item.fileCount} file{item.fileCount > 1 ? "s" : ""}</span>
                <span class="activity-time">{formatTimeAgo(item.timestamp)}</span>
              </div>
              {#if item.savedBytes > 0}
                <span class="activity-saved">{formatBytes(item.savedBytes)}</span>
              {/if}
            </div>
            <div class="activity-divider"></div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- Storage Breakdown -->
    <section class="storage">
      <h3>Storage Breakdown</h3>

      {#if $storage.scanState === "idle"}
        <div class="storage-idle">
          <div class="storage-idle-icon">
            <FolderOpen size={28} />
          </div>
          <p>Scan a folder to see storage breakdown by type</p>
          <Button onclick={() => storage.scanFolder()}>
            <Search size={14} />
            Scan Folder
          </Button>
        </div>

      {:else if $storage.scanState === "scanning"}
        <div class="storage-scanning" role="status" aria-label="Scanning folder">
          <div class="spinner" aria-hidden="true"></div>
          <p>Scanning folder...</p>
          <span class="scan-path">{$storage.folderPath}</span>
        </div>

      {:else if $storage.scanState === "error"}
        <div class="storage-error">
          <p>Scan failed</p>
          <span>{$storage.errorMessage}</span>
          <Button danger style="margin-top: 8px" onclick={() => storage.scanFolder()}>Try Again</Button>
        </div>

      {:else if $storage.scanState === "done" && $storage.scanResult}
        <div class="storage-results">
          <div class="storage-header">
            <div class="storage-hero">
              <span class="hero-value">{formatBytes($storage.scanResult.total_size)}</span>
              <span class="hero-label">total storage</span>
            </div>
            <div class="storage-meta">
              <span class="meta-item">
                <span class="meta-value">{$storage.scanResult.images.length}</span>
                <span class="meta-label">files</span>
              </span>
              {#if $storage.scanResult.skipped > 0}
                <span class="meta-divider"></span>
                <span class="meta-item">
                  <span class="meta-value">{$storage.scanResult.skipped}</span>
                  <span class="meta-label">skipped</span>
                </span>
              {/if}
            </div>
          </div>

          <div class="breakdown-list">
            {#each $breakdown as group (group.extension)}
              <div class="breakdown-row">
                <span class="ext-dot" data-ext={group.extension}></span>
                <span class="ext-name">{group.extension}</span>
                <div class="bar-track">
                  <div
                    class="bar-fill"
                    style="--pct: {group.percentage}%"
                  ></div>
                </div>
                <span class="ext-size">{formatBytes(group.totalSize)}</span>
                <span class="ext-count">{group.count}</span>
              </div>
            {/each}
          </div>

          <Button class="rescan-btn" variant="ghost" onclick={() => storage.scanFolder()}>
            <Search size={12} />
            Scan another folder
          </Button>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 28px;
    overflow-y: auto;
    height: 100%;
  }

  /* Stats Row */
  .stats-row {
    display: flex;
    gap: 16px;
  }

  .stat-card {
    flex: 1;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    padding: 16px;
    display: flex;
    gap: 12px;
    border: 1px solid var(--border);
    position: relative;
    overflow: hidden;
  }

  .stat-card::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent);
  }

  .stat-icon {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .stat-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .stat-value {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
  }

  .stat-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .stat-sub {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  /* Quick Actions */
  .section-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .actions-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .action-card {
    background: var(--bg-card);
    border-radius: var(--radius-md);
    padding: 16px;
    display: flex;
    gap: 12px;
    border: 1px solid var(--border);
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
    text-align: left;
  }

  .action-card:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-subtle);
  }

  .action-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .action-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--accent-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .action-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .action-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .action-desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Bottom Row */
  .bottom-row {
    display: flex;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }

  .activity,
  .storage {
    flex: 1;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    padding: 20px;
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
    min-height: 0;
  }

  .activity h3,
  .storage h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 4px;
    color: var(--text-muted);
  }

  .empty-state p {
    font-size: 13px;
    font-weight: 500;
  }

  .empty-state span {
    font-size: 11px;
  }

  .activity-list {
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .activity-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
  }

  .activity-icon {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--accent-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .activity-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .activity-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .activity-time {
    font-size: 11px;
    color: var(--text-muted);
  }

  .activity-saved {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }

  .activity-divider {
    height: 1px;
    background: var(--border);
  }

  .activity-divider:last-child {
    display: none;
  }

  /* Storage Idle State */
  .storage-idle {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-muted);
    text-align: center;
  }

  .storage-idle-icon {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-md);
    background: var(--accent-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
  }

  .storage-idle p {
    font-size: 13px;
    font-weight: 500;
    max-width: 200px;
    line-height: 1.4;
  }

  /* Storage Scanning State */
  .storage-scanning {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-muted);
    text-align: center;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .storage-scanning p {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .scan-path {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Storage Error State */
  .storage-error {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--danger);
    text-align: center;
  }

  .storage-error p {
    font-size: 13px;
    font-weight: 600;
  }

  .storage-error span {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 200px;
  }

  /* Storage Results */
  .storage-results {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    overflow: hidden;
  }

  .storage-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }

  .storage-hero {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hero-value {
    font-size: 28px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }

  .hero-label {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .storage-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-main);
    border-radius: var(--radius-sm);
  }

  .meta-item {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .meta-value {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .meta-label {
    font-size: 11px;
    color: var(--text-muted);
  }

  .meta-divider {
    width: 1px;
    height: 12px;
    background: var(--border);
  }

  .breakdown-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    overflow-y: auto;
    padding-right: 4px;
  }

  /* Custom scrollbar */
  .breakdown-list::-webkit-scrollbar {
    width: 4px;
  }

  .breakdown-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .breakdown-list::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .breakdown-row {
    display: grid;
    grid-template-columns: 10px 48px 1fr 64px 36px;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    transition: background-color 0.15s;
    min-width: 0;
  }

  .breakdown-row:hover {
    background: var(--bg-main);
  }

  .ext-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--ext-color, var(--accent));
  }

  /* Extension-specific colors */
  .ext-dot[data-ext="JPG"],
  .ext-dot[data-ext="JPEG"] {
    --ext-color: #f59e0b;
  }

  .ext-dot[data-ext="PNG"] {
    --ext-color: #3b82f6;
  }

  .ext-dot[data-ext="HEIC"] {
    --ext-color: #8b5cf6;
  }

  .ext-dot[data-ext="RAW"],
  .ext-dot[data-ext="CR2"],
  .ext-dot[data-ext="NEF"],
  .ext-dot[data-ext="ARW"] {
    --ext-color: #ef4444;
  }

  .ext-dot[data-ext="WEBP"] {
    --ext-color: #06b6d4;
  }

  .ext-dot[data-ext="GIF"] {
    --ext-color: #ec4899;
  }

  .ext-dot[data-ext="TIFF"],
  .ext-dot[data-ext="TIF"] {
    --ext-color: #14b8a6;
  }

  .ext-dot[data-ext="BMP"] {
    --ext-color: #f97316;
  }

  .ext-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar-track {
    height: 6px;
    background: var(--bg-main);
    border-radius: 3px;
    overflow: hidden;
    min-width: 0;
  }

  .bar-fill {
    height: 100%;
    width: var(--pct);
    background: color-mix(in oklch, var(--accent) 40%, var(--text-muted) 60%);
    border-radius: 3px;
    transition: width 0.4s ease-out;
    flex-shrink: 0;
  }

  .ext-size {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    text-align: right;
    font-variant-numeric: tabular-nums;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ext-count {
    font-size: 11px;
    color: var(--text-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.rescan-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 14px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    transition: border-color 0.15s, color 0.15s, background-color 0.15s;
    align-self: flex-start;
  }

  :global(.rescan-btn:hover) {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-subtle);
  }

  :global(.rescan-btn:active) {
    transform: scale(0.98);
  }
</style>
