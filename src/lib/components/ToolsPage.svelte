<script lang="ts">
  import { Minimize2, Repeat, Maximize2 } from "lucide-svelte";
  import { activeTool } from "$lib/stores/nav";
  import SegmentedControl from "./SegmentedControl.svelte";
  import CompressPage from "./CompressPage.svelte";
  import ConvertPage from "./ConvertPage.svelte";
  import ResizePage from "./ResizePage.svelte";

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
    {:else if $activeTool === "convert"}
      <ConvertPage />
    {:else if $activeTool === "resize"}
      <ResizePage />
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
</style>
