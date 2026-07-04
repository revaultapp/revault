<script lang="ts">
  import { activePage } from "$lib/stores/nav";
  import { t } from "$lib/stores/locale.svelte";

  let titles: Record<string, [string, string]> = $derived({
    dashboard: [t("topBar.dashboardTitle"), t("topBar.dashboardSubtitle")],
    optimize: [t("topBar.optimizeTitle"), t("topBar.optimizeSubtitle")],
    duplicates: [t("topBar.duplicatesTitle"), t("topBar.duplicatesSubtitle")],
    privacy: [t("topBar.privacyTitle"), t("topBar.privacySubtitle")],
    video: [t("topBar.videoTitle"), t("topBar.videoSubtitle")],
    settings: [t("topBar.settingsTitle"), t("topBar.settingsSubtitle")],
  });

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
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .title-area p {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 400;
  }


</style>
