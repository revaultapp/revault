<script lang="ts">
  import { Search, Bell } from "lucide-svelte";
</script>

<header class="topbar">
  <div class="title-area">
    <h1>Good morning, Mike</h1>
    <p>Dashboard · 3 pending actions</p>
  </div>

  <div class="search-box">
    <Search size={15} strokeWidth={2} />
    <span class="search-text">Jump to anything...</span>
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
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .title-area p {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    margin-left: auto;
    background: var(--wood-bg);
    border-radius: 10px;
    color: var(--wood-medium);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .search-box:hover {
    background: var(--wood-border);
  }

  .search-text {
    white-space: nowrap;
  }

  kbd {
    font-family: var(--font);
    font-size: 11px;
    padding: 2px 6px;
    background: var(--wood-border);
    border-radius: 5px;
    color: var(--wood);
    margin-left: 8px;
  }

  .bell {
    position: relative;
    color: var(--wood-medium);
    padding: 8px;
    border-radius: var(--radius-sm);
    transition: color 0.15s;
  }

  .bell:hover {
    color: var(--wood);
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
