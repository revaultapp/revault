<script lang="ts">
  import {
    Compass, Wand2, ScanSearch,
    EyeOff, CloudCog,
    Settings, Database
  } from "lucide-svelte";
  import { activePage } from "$lib/stores/nav";
  import { savings } from "$lib/stores/savings";
  import { formatBytes } from "$lib/utils";

  type NavItem = { icon: typeof Compass; id: string; label: string };
  const navItems: NavItem[] = [
    { icon: Compass, id: "dashboard", label: "Dashboard" },
    { icon: Wand2, id: "tools", label: "Tools" },
    { icon: ScanSearch, id: "analyze", label: "Analyze" },
    { icon: EyeOff, id: "privacy", label: "Privacy" },
    { icon: CloudCog, id: "cloud", label: "Cloud" },
  ];
</script>

<aside class="sidebar">
  <div class="sidebar-inner">
    <div class="logo-row">
      <img class="logo-icon" src="/logo.png" alt="Revault" />
      <span class="logo-text">Revault</span>
    </div>

    <nav class="nav">
      {#each navItems as item (item.id)}
        <button type="button" class="nav-item" class:active={$activePage === item.id} onclick={() => activePage.set(item.id)}>
          {#if $activePage === item.id}
            <span class="accent-bar"></span>
          {/if}
          <item.icon size={18} strokeWidth={1.8} />
          <span>{item.label}</span>
        </button>
      {/each}

      <div class="saved-badge">
        <Database size={16} strokeWidth={1.8} />
        <span>Saved: {formatBytes($savings.totalSavedBytes)}</span>
      </div>
    </nav>

    <div class="spacer"></div>

    <div class="divider"></div>

    <button type="button" class="nav-item settings" class:active={$activePage === 'settings'} onclick={() => activePage.set('settings')}>
      {#if $activePage === 'settings'}
        <span class="accent-bar"></span>
      {/if}
      <Settings size={18} strokeWidth={1.8} />
      <span>Settings</span>
    </button>

  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: linear-gradient(180deg, #070d1a 0%, #0b1120 40%, #0f1729 100%);
  }

  .sidebar-inner {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 24px 18px 22px;
    overflow: hidden;
  }

  .logo-row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 34px;
    margin-bottom: 20px;
  }

  .logo-icon {
    width: 34px;
    height: 34px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .logo-text {
    color: #fff;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 14px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 13.5px;
    font-weight: 500;
    transition: background 0.15s, color 0.15s;
  }

  .nav-item :global(svg) {
    flex-shrink: 0;
  }

  .nav-item span {
    color: var(--text-muted);
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #94a3b8;
  }

  .nav-item:hover span {
    color: #94a3b8;
  }

  .nav-item.active {
    background: rgba(255, 255, 255, 0.03);
    color: var(--accent);
    border-radius: 10px;
    gap: 10px;
    padding: 0 12px;
  }

  .nav-item.active span {
    color: #e2e8f0;
    font-weight: 600;
  }

  .accent-bar {
    width: 3px;
    height: 28px;
    border-radius: 2px;
    background: var(--accent);
    flex-shrink: 0;
  }

  .spacer {
    flex: 1;
  }

  .divider {
    height: 1px;
    margin: 16px 0;
    background: linear-gradient(90deg, transparent, #293548 30%, #293548 70%, transparent);
  }

  .saved-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    background: linear-gradient(90deg, rgba(16, 185, 129, 0.15), rgba(5, 150, 105, 0.08));
    border: 1px solid rgba(16, 185, 129, 0.19);
    color: var(--accent);
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin: 4px 0;
  }

  .saved-badge :global(svg) {
    flex-shrink: 0;
  }


</style>
