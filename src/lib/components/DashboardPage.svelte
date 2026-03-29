<script lang="ts">
  import { HardDrive, Image, Minimize2, Zap, Search, Folder, Shield } from "lucide-svelte";
  import { savings } from "$lib/stores/savings";
  import { activity, formatTimeAgo } from "$lib/stores/activity";
  import { activePage } from "$lib/stores/nav";
  import { formatBytes } from "$lib/utils";

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
    organize: "Organized",
    watermark: "Watermarked",
    rename: "Renamed",
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
    <button class="action-card accent" onclick={() => navigate("tools")}>
      <div class="action-icon">
        <Zap size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Compress Images</span>
        <span class="action-desc">Reduce file size without quality loss</span>
      </div>
    </button>

    <button class="action-card" onclick={() => navigate("analyze")}>
      <div class="action-icon">
        <Search size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Analyze Folder</span>
        <span class="action-desc">Find duplicates and storage hogs</span>
      </div>
    </button>

    <button class="action-card" onclick={() => navigate("organize")}>
      <div class="action-icon">
        <Folder size={18} />
      </div>
      <div class="action-text">
        <span class="action-title">Organize Files</span>
        <span class="action-desc">Sort by date, location, or custom rules</span>
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
      <div class="storage-placeholder">
        <Search size={24} />
        <p>Scan a folder to see storage</p>
      </div>
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
    position: relative;
    overflow: hidden;
  }

  .action-card:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-subtle);
  }

  .action-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .action-card::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent);
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

  .storage-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-muted);
  }

  .storage-placeholder p {
    font-size: 13px;
    font-weight: 500;
  }
</style>
