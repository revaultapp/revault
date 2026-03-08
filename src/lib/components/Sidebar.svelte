<script lang="ts">
  import {
    Compass, Zap, Shuffle, ScanSearch,
    Boxes, Sparkles, EyeOff, CloudCog,
    Settings, Database
  } from "lucide-svelte";
  import { activePage } from "$lib/stores/nav";

  type NavItem = { icon: typeof Compass; id: string; label: string; badge?: string; dot?: boolean };
  const navItems: NavItem[] = [
    { icon: Compass, id: "dashboard", label: "Dashboard" },
    { icon: Zap, id: "compress", label: "Compress" },
    { icon: Shuffle, id: "convert", label: "Convert" },
    { icon: ScanSearch, id: "analyze", label: "Analyze", badge: "3" },
    { icon: Boxes, id: "organize", label: "Organize" },
    { icon: Sparkles, id: "edit", label: "Edit" },
    { icon: EyeOff, id: "privacy", label: "Privacy" },
    { icon: CloudCog, id: "cloud", label: "Cloud", dot: true },
  ];
</script>

<aside class="sidebar">
  <div class="accent-line"></div>

  <div class="sidebar-inner">
    <div class="logo-row">
      <img class="logo-icon" src="/logo.png" alt="Revault" />
      <span class="logo-text">Revault</span>
      <span class="status-dot"></span>
    </div>

    <nav class="nav">
      {#each navItems as item}
        <button class="nav-item" class:active={$activePage === item.id} onclick={() => activePage.set(item.id)}>
          {#if $activePage === item.id}
            <span class="accent-bar"></span>
          {/if}
          <item.icon size={18} strokeWidth={1.8} />
          <span>{item.label}</span>
          {#if item.badge}
            <span class="nav-spacer"></span>
            <span class="nav-badge">{item.badge}</span>
          {:else if item.dot}
            <span class="nav-spacer"></span>
            <span class="nav-dot"></span>
          {/if}
        </button>
      {/each}

      <div class="saved-badge">
        <Database size={16} strokeWidth={1.8} />
        <span>Saved: 12.4 GB</span>
      </div>
    </nav>

    <div class="spacer"></div>

    <div class="divider"></div>

    <button class="nav-item settings" class:active={$activePage === 'settings'} onclick={() => activePage.set('settings')}>
      {#if $activePage === 'settings'}
        <span class="accent-bar"></span>
      {/if}
      <Settings size={18} strokeWidth={1.8} />
      <span>Settings</span>
    </button>

    <div class="user-section">
      <div class="user-avatar">M</div>
      <div class="user-info">
        <span class="user-name">Mike</span>
        <span class="user-role">Pro Plan</span>
      </div>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100vh;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: linear-gradient(180deg, #070d1a 0%, #0b1120 40%, #0f1729 100%);
  }

  .accent-line {
    height: 3px;
    background: linear-gradient(90deg, #059669, #10b981, #3b82f6, #1e3a5f);
    flex-shrink: 0;
  }

  .sidebar-inner {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 24px 18px 18px;
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
    font-size: 17px;
    font-weight: 700;
    letter-spacing: 0.5px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--accent);
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
    color: #475569;
    font-size: 14px;
    font-weight: 400;
    transition: background 0.15s, color 0.15s;
  }

  .nav-item :global(svg) {
    flex-shrink: 0;
  }

  .nav-item span {
    color: #64748b;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #7b8da6;
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
    color: #e8e8e8;
    font-weight: 600;
  }

  .accent-bar {
    width: 3px;
    height: 28px;
    border-radius: 2px;
    background: var(--accent);
    flex-shrink: 0;
  }

  .nav-badge {
    min-width: 20px;
    height: 18px;
    border-radius: 9px;
    background: var(--accent);
    color: #fff !important;
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .nav-dot {
    width: 8px;
    height: 8px;
    border-radius: 4px;
    background: var(--accent);
    flex-shrink: 0;
  }

  .nav-spacer {
    flex: 1;
  }

  .spacer {
    flex: 1;
  }

  .divider {
    height: 1px;
    margin: 12px 0;
    background: linear-gradient(90deg, transparent, #1e293b 30%, #1e293b 70%, transparent);
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
    font-size: 13px;
    font-weight: 700;
    margin: 4px 0;
  }

  .saved-badge :global(svg) {
    flex-shrink: 0;
  }

  .user-section {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    padding: 0 6px;
    margin-top: 8px;
    border-radius: 10px;
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 16px;
    background: linear-gradient(135deg, #1e3a5f, #3b82f6);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .user-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .user-name {
    color: #e0e0e0;
    font-size: 13px;
    font-weight: 500;
  }

  .user-role {
    color: #64748b;
    font-size: 11px;
  }
</style>
