<script lang="ts">
  import "@fontsource-variable/plus-jakarta-sans";
  import "../app.css";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import WindowControls from "$lib/components/WindowControls.svelte";

  let { children } = $props();
</script>

<div class="titlebar" data-tauri-drag-region>
  <WindowControls />
</div>

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
    background: var(--chrome-bg);
    border-bottom: 1px solid var(--chrome-border);
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
  }
</style>
