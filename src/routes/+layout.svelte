<script lang="ts">
  import "../app.css";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";

  let { children } = $props();
</script>

<div class="shell">
  <Sidebar />
  <div class="main">
    <TopBar />
    <div class="content-wrapper">
      <div class="content">
        {@render children()}
      </div>
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

  .content-wrapper {
    flex: 1;
    padding: 28px;
    overflow-y: auto;
  }

  .content {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    padding: 32px 40px;
    min-height: 100%;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.03);
  }
</style>
