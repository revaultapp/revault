<script lang="ts">
  import { Search, Bell } from "lucide-svelte";
  import { activePage } from "$lib/stores/nav";

  const titles: Record<string, [string, string]> = {
    dashboard: ["Good morning, Mike", "Dashboard · 3 pending actions"],
    compress: ["Compress", "Reduce file size"],
    convert: ["Convert", "Change image formats"],
    analyze: ["Analyze", "Scan your storage"],
    organize: ["Organize", "Sort and manage files"],
    edit: ["Edit", "Batch image editing"],
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

  <div class="search-box">
    <Search size={15} strokeWidth={2} />
    <span>Jump to anything...</span>
    <kbd>⌘K</kbd>
  </div>

  <button class="bell" aria-label="Notifications">
    <Bell size={18} strokeWidth={2} />
    <span class="bell-dot"></span>
  </button>
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

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    margin-left: auto;
    background: var(--navy-bg);
    border-radius: 10px;
    color: var(--navy-medium);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .search-box:hover {
    background: var(--navy-border);
  }

  kbd {
    font-family: var(--font);
    font-size: 10.5px;
    font-weight: 600;
    padding: 2px 6px;
    background: var(--navy-border);
    border-radius: 5px;
    color: var(--navy);
    margin-left: 8px;
  }

  .bell {
    position: relative;
    color: var(--navy-medium);
    padding: 8px;
    border-radius: var(--radius-sm);
    transition: color 0.15s;
  }

  .bell:hover {
    color: var(--navy);
  }

  .bell-dot {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg-card);
  }
</style>
