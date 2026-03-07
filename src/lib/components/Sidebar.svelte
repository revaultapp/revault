<script lang="ts">
  const navItems: { icon: string; label: string; active?: boolean; badge?: boolean }[] = [
    { icon: "layout-dashboard", label: "Dashboard", active: true },
    { icon: "minimize-2", label: "Compress" },
    { icon: "arrow-left-right", label: "Convert" },
    { icon: "bar-chart-3", label: "Analyze", badge: true },
    { icon: "folder-tree", label: "Organize" },
    { icon: "paintbrush", label: "Edit" },
    { icon: "shield", label: "Privacy" },
    { icon: "cloud", label: "Cloud", badge: true },
  ];
</script>

<aside class="sidebar">
  <div class="accent-line"></div>

  <div class="sidebar-inner">
    <div class="logo-row">
      <div class="logo-icon">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M2 4l6-3 6 3v8l-6 3-6-3V4z" fill="white" opacity="0.9"/>
          <path d="M8 1v14M2 4l6 3 6-3" stroke="white" stroke-width="0.5" opacity="0.4"/>
        </svg>
      </div>
      <span class="logo-text">ReVault</span>
      <span class="status-dot"></span>
    </div>

    <nav class="nav">
      {#each navItems as item}
        <button class="nav-item" class:active={item.active}>
          <i data-lucide={item.icon}></i>
          <span>{item.label}</span>
          {#if item.badge}
            <span class="nav-badge"></span>
          {/if}
        </button>
      {/each}

      <div class="saved-badge">
        <i data-lucide="database"></i>
        <span>Saved: 12.4 GB</span>
      </div>
    </nav>

    <div class="spacer"></div>

    <div class="divider"></div>

    <button class="nav-item settings">
      <i data-lucide="settings"></i>
      <span>Settings</span>
    </button>

    <div class="user-section">
      <div class="user-avatar">M</div>
      <div class="user-info">
        <span class="user-name">Mike</span>
        <span class="user-role">ImFan</span>
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
    background: linear-gradient(180deg, #0c0c0c 0%, #171717 40%, #1a1a1a 100%);
  }

  .accent-line {
    height: 3px;
    background: linear-gradient(90deg, #059669, #10b981, #c9a96e, #8b6f47);
    flex-shrink: 0;
  }

  .sidebar-inner {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 24px 18px 18px;
    gap: 0;
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
    background: linear-gradient(135deg, #10b981, #059669);
    display: flex;
    align-items: center;
    justify-content: center;
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
    color: #484848;
    font-size: 14px;
    font-weight: 400;
    transition: background 0.15s, color 0.15s;
    position: relative;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #888;
  }

  .nav-item.active {
    background: rgba(16, 185, 129, 0.09);
    color: var(--accent);
    box-shadow: 0 2px 16px rgba(16, 185, 129, 0.15);
    border-radius: 10px;
  }

  .nav-item i {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .nav-badge {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    margin-left: auto;
  }

  .spacer {
    flex: 1;
  }

  .divider {
    height: 1px;
    margin: 8px 0;
    background: linear-gradient(90deg, transparent, #2a2a2a 30%, #2a2a2a 70%, transparent);
  }

  .settings {
    color: #484848;
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

  .saved-badge i {
    width: 16px;
    height: 16px;
  }

  .user-section {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    padding: 0 6px;
    border-radius: 10px;
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 16px;
    background: linear-gradient(135deg, #8b6f47, #c9a96e);
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
    color: #666;
    font-size: 11px;
  }
</style>
