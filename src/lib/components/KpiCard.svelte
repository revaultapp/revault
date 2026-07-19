<script lang="ts">
  import type { ComponentType } from "svelte";
  import { ArrowUp, ArrowDown } from "lucide-svelte";

  interface Props {
    label: string;
    icon: ComponentType;
    value: string;
    delta?: { pct: number; up: boolean } | null;
    sub?: string;
    ariaNote?: string;
    /** Translated "vs previous month" suffix — passed in since this component
        takes zero store imports (no i18n access of its own). Only rendered
        when `delta` is present (not undefined). */
    deltaSuffix?: string;
    formatPercent?: (value: number) => string;
  }

  let {
    label,
    icon,
    value,
    delta,
    sub,
    ariaNote,
    deltaSuffix,
    formatPercent = (percent) => `${percent.toFixed(1)}%`,
  }: Props = $props();
</script>

<div class="kpi-card" aria-label={ariaNote}>
  <div class="kpi-top">
    <span class="kpi-label" title={label}>{label}</span>
    <span class="kpi-icon" aria-hidden="true">
      {#if icon}
        {@const Icon = icon}
        <Icon size={12} strokeWidth={2} />
      {/if}
    </span>
  </div>

  <span class="kpi-value">{value}</span>

  {#if delta !== undefined}
    <div class="kpi-delta">
      {#if delta === null}
        <span class="delta-flat">—</span>
      {:else}
        <span class="delta-arrow" class:up={delta.up} class:down={!delta.up} aria-hidden="true">
          {#if delta.up}
            <ArrowUp size={10} strokeWidth={2.5} />
          {:else}
            <ArrowDown size={10} strokeWidth={2.5} />
          {/if}
        </span>
        <span class="delta-pct" class:up={delta.up} class:down={!delta.up}>
          <!-- Signed so the direction survives without the (aria-hidden)
               arrow icon or color — screen readers otherwise hear a bare
               percentage with no up/down information (WCAG 1.4.1). -->
          {delta.up ? "+" : "−"}{formatPercent(delta.pct)}
        </span>
      {/if}
      {#if deltaSuffix}<span class="delta-suffix">{deltaSuffix}</span>{/if}
    </div>
  {:else if sub}
    <span class="kpi-sub">{sub}</span>
  {/if}
</div>

<style>
  .kpi-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
    min-width: 0;
  }

  .kpi-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }

  .kpi-label {
    min-width: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--chart-tick);
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }

  .kpi-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    background: var(--navy-bg);
    color: var(--text-secondary);
  }

  .kpi-value {
    font-size: 22px;
    font-weight: 800;
    color: var(--text-primary);
    line-height: 1;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kpi-delta {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .delta-arrow {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .delta-arrow.up,
  .delta-pct.up {
    color: var(--accent-text);
  }

  .delta-arrow.down,
  .delta-pct.down {
    color: var(--danger-text);
  }

  .delta-pct {
    font-size: 12px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .delta-flat {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .delta-suffix,
  .kpi-sub {
    overflow: hidden;
    font-size: 11px;
    color: var(--chart-tick);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
