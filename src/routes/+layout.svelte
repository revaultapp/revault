<script lang="ts">
  import "@fontsource-variable/plus-jakarta-sans";
  import "../app.css";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";

  let { children } = $props();
</script>

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
  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
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
