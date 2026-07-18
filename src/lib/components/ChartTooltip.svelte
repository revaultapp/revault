<script lang="ts">
  interface TooltipRow {
    swatch?: string;
    label: string;
    value: string;
    delta?: { text: string; up: boolean };
  }

  interface Props {
    visible: boolean;
    x: number;
    y: number;
    title: string;
    sub?: string;
    rows?: TooltipRow[];
  }

  let { visible, x, y, title, sub, rows = [] }: Props = $props();

  let el: HTMLDivElement | undefined = $state();
  let clampedLeft = $state(0);
  let clampedTop = $state(0);
  let flipped = $state(false);

  // Re-measure and clamp into the offsetParent every time the anchor point
  // moves. Reads el/x/y/visible so it reruns on mount and on every hover
  // update — no ResizeObserver needed since the parent chart re-renders the
  // tooltip's position props on every relevant change.
  $effect(() => {
    if (!visible || !el) return;
    // Track the anchor coordinates so this effect reruns when they change.
    void x;
    void y;

    const parent = el.offsetParent as HTMLElement | null;
    const parentW = parent?.clientWidth ?? 0;
    const parentH = parent?.clientHeight ?? 0;
    const w = el.offsetWidth;
    const h = el.offsetHeight;

    const margin = 10;
    const wouldOverflowRight = x + margin + w > parentW;
    flipped = wouldOverflowRight;
    // Both branches clamp into [0, parentW - w] — the flipped branch used to
    // only floor at 0, so a stale (mid-resize) measured width could still
    // push the tooltip past the right edge.
    clampedLeft = wouldOverflowRight
      ? Math.max(0, Math.min(x - margin - w, parentW - w))
      : Math.min(x + margin, Math.max(0, parentW - w));
    clampedTop = Math.min(Math.max(0, y - h / 2), Math.max(0, parentH - h));
  });
</script>

{#if visible}
  <div
    class="chart-tooltip"
    class:flipped
    style="left: {clampedLeft}px; top: {clampedTop}px;"
    bind:this={el}
  >
    <span class="tooltip-title">{title}</span>
    {#if sub}<span class="tooltip-sub">{sub}</span>{/if}
    {#if rows.length}
      <div class="tooltip-rows">
        {#each rows as row (row.label)}
          <div class="tooltip-row">
            {#if row.swatch}
              <span class="swatch" style="background: {row.swatch}"></span>
            {/if}
            <span class="row-label">{row.label}</span>
            <span class="row-value">{row.value}</span>
            {#if row.delta}
              <span class="row-delta" class:up={row.delta.up} class:down={!row.delta.up}>
                {row.delta.text}
              </span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .chart-tooltip {
    position: absolute;
    z-index: 5;
    min-width: 120px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    box-shadow: var(--shadow-md);
    pointer-events: none;
  }

  .tooltip-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  .tooltip-sub {
    font-size: 11px;
    color: var(--chart-tick);
  }

  .tooltip-rows {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 2px;
  }

  .tooltip-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: 2px;
  }

  .row-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 11px;
    color: var(--chart-tick);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-value {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .row-delta {
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .row-delta.up {
    color: var(--accent-text);
  }

  .row-delta.down {
    color: var(--danger-text);
  }
</style>
