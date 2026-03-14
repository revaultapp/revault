<script lang="ts">
  import { Minimize2, Repeat, Maximize2 } from "lucide-svelte";
  import { activeTool } from "$lib/stores/nav";
  import SegmentedControl from "./SegmentedControl.svelte";
  import CompressPage from "./CompressPage.svelte";

  const tools = [
    { id: "compress", label: "Compress", icon: Minimize2 },
    { id: "convert", label: "Convert", icon: Repeat },
    { id: "resize", label: "Resize", icon: Maximize2 },
  ];
</script>

<div class="tools-page">
  <div class="tools-header">
    <SegmentedControl segments={tools} bind:selected={$activeTool} />
  </div>

  <div class="tools-content">
    {#if $activeTool === "compress"}
      <CompressPage />
    {:else}
      <div class="placeholder">
        <p>{tools.find(t => t.id === $activeTool)?.label}</p>
        <span>Coming soon</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .tools-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tools-header {
    padding-bottom: 20px;
  }

  .tools-content {
    flex: 1;
    min-height: 0;
  }

  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
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
