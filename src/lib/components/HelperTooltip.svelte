<script lang="ts">
  import { Info } from "lucide-svelte";

  interface Props {
    tip: string;
  }

  let { tip }: Props = $props();

  let buttonEl = $state<HTMLButtonElement | null>(null);
  let tooltipEl = $state<HTMLDivElement | null>(null);

  function getPosition(): { top: number; left: number } | null {
    if (!buttonEl || !tooltipEl) return null;
    const btnRect = buttonEl.getBoundingClientRect();
    const tooltipRect = tooltipEl.getBoundingClientRect();
    const viewportWidth = window.innerWidth;

    // Default: center horizontally, place above
    let top = btnRect.top - tooltipRect.height - 8;
    let left = btnRect.left + (btnRect.width - tooltipRect.width) / 2;

    // Boundary checks
    if (top < 8) {
      // Flip below if not enough space above
      top = btnRect.bottom + 8;
    }
    if (left < 8) {
      left = 8;
    }
    if (left + tooltipRect.width > viewportWidth - 8) {
      left = viewportWidth - tooltipRect.width - 8;
    }

    return { top, left };
  }

  $effect(() => {
    if (tooltipEl) {
      const pos = getPosition();
      if (pos) {
        tooltipEl.style.top = `${pos.top}px`;
        tooltipEl.style.left = `${pos.left}px`;
        tooltipEl.style.bottom = "auto";
        tooltipEl.style.right = "auto";
      }
    }
  });
</script>

<span class="helper-tooltip-wrap">
  <button
    bind:this={buttonEl}
    class="helper-btn"
    aria-label={tip}
  >
    <Info size={16} />
  </button>

  <div
    bind:this={tooltipEl}
    class="helper-tooltip"
    role="tooltip"
  >{tip}</div>
</span>

<style>
  .helper-tooltip-wrap {
    display: inline-flex;
    align-items: center;
    position: relative;
  }

  .helper-btn {
    display: inline-block;
    text-align: center;
    width: 28px;
    height: 28px;
    line-height: 28px;
    border-radius: 50%;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: color 0.15s;
    margin-left: 4px;
    padding: 0;
  }

  .helper-btn:hover {
    color: var(--accent);
  }

  .helper-btn :global(svg) {
    vertical-align: middle;
    margin-top: -1px;
  }

  .helper-tooltip {
    position: fixed;
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 400;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-md);
    max-width: 280px;
    line-height: 1.4;
    z-index: 100;
    pointer-events: none;
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.15s, visibility 0.15s;
  }

  .helper-tooltip-wrap:hover .helper-tooltip,
  .helper-btn:focus + .helper-tooltip {
    opacity: 1;
    visibility: visible;
  }

</style>
