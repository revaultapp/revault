<script lang="ts">
  import {
    Compass, Zap, ScanSearch,
    Shield, Film, FileText,
    Settings, Database
  } from "lucide-svelte";
  import { Tween, Spring } from 'svelte/motion';
  import { tick } from 'svelte';
  import { activePage } from "$lib/stores/nav";
  import { savings } from "$lib/stores/savings";
  import { formatBytes } from "$lib/utils";

  type NavItem = { icon: typeof Compass; id: string; label: string };
  const navItems: NavItem[] = [
    { icon: Compass, id: "dashboard", label: "Dashboard" },
    { icon: Zap, id: "optimize", label: "Optimize" },
    { icon: ScanSearch, id: "duplicates", label: "Duplicates" },
    { icon: Shield, id: "privacy", label: "Privacy" },
    { icon: Film, id: "video", label: "Video" },
    { icon: FileText, id: "pdf", label: "PDF" },
  ];

  // Savings counter animation
  const displayedBytes = new Tween(0, {
    duration: 800,
    easing: (t: number) => t < 0.5 ? 2*t*t : -1+(4-2*t)*t
  });

  $effect(() => {
    displayedBytes.set($savings.totalSavedBytes);
  });

  let savingsJustUpdated = $state(false);
  let prevSavedBytes = $state($savings.totalSavedBytes);
  let savingsGlowTimer: ReturnType<typeof setTimeout>;

  $effect(() => {
    const current = $savings.totalSavedBytes;
    if (current > prevSavedBytes) {
      savingsJustUpdated = true;
      clearTimeout(savingsGlowTimer);
      savingsGlowTimer = setTimeout(() => { savingsJustUpdated = false; }, 1200);
    }
    prevSavedBytes = current;
  });

  // Sliding active indicator
  let sidebarInnerEl: HTMLElement | undefined = $state();
  let navRefs: Record<string, HTMLElement> = {};
  let indicatorVisible = $state(false);

  const indicatorY = new Spring(0, { stiffness: 0.25, damping: 0.75 });

  $effect(() => {
    const page = $activePage;
    tick().then(() => {
      const activeRef = navRefs[page];
      if (!sidebarInnerEl || !activeRef) return;
      const containerRect = sidebarInnerEl.getBoundingClientRect();
      const itemRect = activeRef.getBoundingClientRect();
      indicatorY.set(itemRect.top - containerRect.top + (itemRect.height / 2) - 14);
      indicatorVisible = true;
    });
  });
</script>

<aside class="sidebar">
  <div class="sidebar-inner" bind:this={sidebarInnerEl}>
    {#if indicatorVisible}
      <span
        class="sliding-indicator"
        style="transform: translateY({indicatorY.current}px)"
      ></span>
    {/if}

    <div class="logo-row">
      <img class="logo-icon" src="/logo2.png" alt="Revault" />
      <span class="logo-text">Revault</span>
    </div>

    <nav class="nav">
      {#each navItems as item (item.id)}
        <button
          type="button"
          class="nav-item"
          class:active={$activePage === item.id}
          onclick={() => activePage.set(item.id)}
          bind:this={navRefs[item.id]}
        >
          <item.icon size={18} strokeWidth={1.8} />
          <span>{item.label}</span>
        </button>
      {/each}

      <div class="saved-badge" class:just-updated={savingsJustUpdated}>
        <Database size={16} strokeWidth={1.8} />
        <span>Saved: {formatBytes(displayedBytes.current)}</span>
      </div>
    </nav>

    <div class="spacer"></div>

    <div class="divider"></div>

    <button
      type="button"
      class="nav-item settings"
      class:active={$activePage === 'settings'}
      onclick={() => activePage.set('settings')}
      bind:this={navRefs['settings']}
    >
      <Settings size={18} strokeWidth={1.8} />
      <span>Settings</span>
    </button>

  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    height: 100%;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: var(--chrome-bg);
  }

  .sidebar-inner {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 24px 18px 20px;
    overflow: hidden;
    position: relative;
  }

  .sliding-indicator {
    position: absolute;
    left: 0;
    top: 0;
    width: 3px;
    height: 28px;
    border-radius: 2px;
    background: var(--accent);
    pointer-events: none;
    will-change: transform;
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
    mix-blend-mode: lighten;
  }

  .logo-text {
    color: var(--chrome-text-primary);
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
    padding: 0 14px 0 8px;
    border-radius: var(--radius-sm);
    color: var(--chrome-text-muted);
    font-size: 14px;
    font-weight: 500;
    transition: background var(--duration-fast), backdrop-filter var(--duration-fast), border-color var(--duration-fast), color var(--duration-fast);
  }

  .nav-item :global(svg) {
    flex-shrink: 0;
  }

  .nav-item span {
    color: var(--chrome-text-muted);
  }

  .nav-item:hover {
    background: var(--chrome-hover-bg);
    color: var(--chrome-text-muted);
  }

  .nav-item:hover span {
    color: var(--chrome-text-muted);
  }

  .nav-item.active {
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(8px) saturate(150%);
    -webkit-backdrop-filter: blur(8px) saturate(150%);
    border: 1px solid rgba(255, 255, 255, 0.09);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.07);
    color: var(--accent);
    border-radius: 10px;
  }

  .nav-item.active span {
    color: var(--chrome-text-primary);
    font-weight: 600;
  }

  .spacer {
    flex: 1;
  }

  .divider {
    height: 1px;
    margin: 16px 0;
    background: var(--chrome-border);
  }

  .saved-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    background: linear-gradient(90deg, rgba(16, 216, 122, 0.12), rgba(14, 207, 116, 0.06));
    border: 1px solid rgba(16, 216, 122, 0.16);
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin: 4px 0;
    transition: box-shadow 300ms var(--ease-out);
  }

  .saved-badge :global(svg) {
    flex-shrink: 0;
  }

  .saved-badge.just-updated {
    animation: savings-pulse 1200ms var(--ease-out) forwards;
  }

  @keyframes savings-pulse {
    0%   { box-shadow: 0 0 0 3px rgba(16, 216, 122, 0.35), 0 0 20px rgba(16, 216, 122, 0.2); }
    100% { box-shadow: 0 0 0 0px rgba(16, 216, 122, 0),   0 0 0px  rgba(16, 216, 122, 0);   }
  }

  @media (prefers-reduced-motion: reduce) {
    .saved-badge { animation: none !important; transition: none !important; }
  }

  @media (prefers-reduced-motion: reduce) {
    /* backdrop-filter no es animación — se mantiene. Solo desactivar transitions */
    .nav-item { transition: none !important; }
  }
</style>
