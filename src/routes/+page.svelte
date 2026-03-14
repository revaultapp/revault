<script lang="ts">
  import { activePage } from "$lib/stores/nav";
  import SettingsPage from "$lib/components/SettingsPage.svelte";
  import ToolsPage from "$lib/components/ToolsPage.svelte";
</script>

{#if $activePage === "tools"}
  <ToolsPage />
{:else if $activePage === "settings"}
  <SettingsPage />
{:else}
  <div class="placeholder">
    <p>This section is not yet available.</p>
    <span>Coming soon</span>
  </div>
{/if}

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 100%;
  }

  .placeholder p {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .placeholder span {
    font-size: 12px;
    color: var(--navy);
    background: var(--navy-bg);
    padding: 4px 12px;
    border-radius: 6px;
  }
</style>
