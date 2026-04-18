<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minus, Square, X, Copy } from "lucide-svelte";

  let isMacOS = $state(false);
  let isMaximized = $state(false);

  const win = getCurrentWindow();
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    isMacOS = /Mac|iPod|iPhone|iPad/.test(navigator.platform);
    if (isMacOS) return;

    isMaximized = await win.isMaximized();
    unlisten = await win.onResized(async () => {
      isMaximized = await win.isMaximized();
    });
  });

  onDestroy(() => unlisten?.());

  async function toggleMax() {
    await win.toggleMaximize();
  }
</script>

{#if !isMacOS}
  <div class="window-controls">
    <button class="ctrl min" onclick={() => win.minimize()} aria-label="Minimizar">
      <Minus size={14} />
    </button>
    <button class="ctrl max" onclick={toggleMax} aria-label={isMaximized ? "Restaurar" : "Maximizar"}>
      {#if isMaximized}
        <Copy size={12} />
      {:else}
        <Square size={12} />
      {/if}
    </button>
    <button class="ctrl close" onclick={() => win.close()} aria-label="Cerrar">
      <X size={14} />
    </button>
  </div>
{/if}

<style>
  .window-controls {
    display: flex;
    height: 28px;
    align-items: stretch;
  }

  .ctrl {
    width: 46px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 0;
    color: var(--chrome-text-muted);
    cursor: pointer;
    transition: background 120ms, color 120ms;
  }

  .ctrl:hover {
    background: var(--chrome-hover-bg);
    color: var(--chrome-text-primary);
  }

  .ctrl.close:hover {
    background: #e81123;
    color: white;
  }
</style>
