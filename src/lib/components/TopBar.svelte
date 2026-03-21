<script lang="ts">
  import { activePage } from "$lib/stores/nav";

  const titles: Record<string, [string, string]> = {
    dashboard: ["Dashboard", "Overview and quick actions"],
    tools: ["Tools", "Compress, convert and resize"],
    analyze: ["Analyze", "Scan your storage"],
    organize: ["Organize", "Sort and manage files"],
    privacy: ["Privacy", "Metadata and sensitive data"],
    cloud: ["Cloud", "Sync and backup"],
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
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
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
    color: var(--text-primary);
  }

  .title-area p {
    font-size: 12.5px;
    color: var(--text-muted);
    font-weight: 400;
  }


</style>
