<script lang="ts">
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import "@fontsource-variable/plus-jakarta-sans";
  import "../app.css";
  import { updates } from "$lib/stores/updates";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import UpdateDialog from "$lib/components/UpdateDialog.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";
  import { scheduleStartupUpdateCheck } from "$lib/components/startUpdateCheck";
  import { tauriUpdateAdapter } from "$lib/components/tauriUpdateAdapter";

  let { children } = $props();

  onMount(() => {
    updates.setAdapter(tauriUpdateAdapter);
    return scheduleStartupUpdateCheck(
      isTauri(),
      requestAnimationFrame,
      cancelAnimationFrame,
      () => { void updates.checkForUpdates(); },
    );
  });
</script>

<div class="titlebar" data-tauri-drag-region>
  <WindowControls />
</div>

<UpdateDialog />

<div class="shell">
  <Sidebar />
  <div class="main">
    <TopBar />
    <div class="content-area">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    justify-content: flex-end;
    align-items: stretch;
    width: 100%;
    height: 28px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .shell {
    display: flex;
    width: 100vw;
    height: calc(100vh - 28px);
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-main);
  }

  .content-area {
    flex: 1;
    padding: 28px;
    overflow-y: auto;
    /* Defense-in-depth: no child (chart cards, tables, etc.) should ever be
       able to push horizontal scroll onto the shell — it must stay contained
       and scroll internally instead of escaping this area. */
    overflow-x: hidden;
  }
</style>
