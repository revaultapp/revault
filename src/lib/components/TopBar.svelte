<script lang="ts">
  import { activePage } from "$lib/stores/nav";

  const titles: Record<string, [string, string]> = {
    dashboard: ["Dashboard", "Overview and quick actions"],
    optimize: ["Optimize", "Compress, convert and resize"],
    duplicates: ["Duplicates", "Find duplicate photos"],
    privacy: ["Privacy", "Metadata and sensitive data"],
    video: ["Video", "Compress and optimize videos"],
    watermark: ["Watermark", "Add text or logo watermarks"],
    settings: ["Settings", "Manage your preferences"],
  };

  let current = $derived(titles[$activePage] ?? titles.dashboard);
</script>

<header class="topbar">
  <div class="title-area">
    <h1>{current[0]}</h1>
    <p>{current[1]}</p>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    height: var(--topbar-height);
    padding: 0 28px;
    background: var(--chrome-bg);
    border-bottom: 1px solid var(--chrome-border);
    flex-shrink: 0;
  }

  .title-area {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title-area h1 {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.025em;
    color: var(--chrome-text-primary);
  }

  .title-area p {
    font-size: 12px;
    color: var(--chrome-text-muted);
    font-weight: 400;
  }


</style>
