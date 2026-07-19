<script lang="ts">
  import { t } from "$lib/stores/locale.svelte";
  import { untrack } from "svelte";
  import { TrendingUp } from "lucide-svelte";
  import { nextChartIndex, niceMax, normalizeChartIndex } from "$lib/charts";

  interface MonthPoint {
    key: string;
    date: Date;
    total: number;
  }

  interface Props {
    series: MonthPoint[];
    heroIndex: number;
    monthLabel: (d: Date) => string;
    formatValue: (n: number) => string;
    ariaSummary: string;
    tableCaption: string;
    delta?: { pct: number; up: boolean } | null;
    deltaSuffix?: string;
    emptyTitle?: string;
    emptyHint?: string;
    emptyCta?: string;
    onCta?: () => void;
    view?: "chart" | "table";
  }

  let {
    series,
    heroIndex,
    monthLabel,
    formatValue,
    ariaSummary,
    tableCaption,
    delta,
    deltaSuffix,
    emptyTitle,
    emptyHint,
    emptyCta,
    onCta,
    view = "chart",
  }: Props = $props();

  const uid = $props.id();
  const hatchId = `mbars-hatch-${uid}`;
  const heroGradId = `mbars-hero-${uid}`;

  let selectedKey: string | null = $state(
    untrack(() => series[normalizeChartIndex(heroIndex, series.length) ?? series.length - 1]?.key ?? null),
  );
  let hoverKey: string | null = $state(null);

  const count = $derived(series.length);
  const allZero = $derived(series.every((s) => s.total === 0));
  const maxTotal = $derived(Math.max(0, ...series.map((s) => s.total)));
  const scaleMax = $derived(niceMax(maxTotal || 1));
  const ticks = $derived([0.75, 0.5, 0.25].map((f) => ({ topPct: (1 - f) * 100, value: scaleMax * f })));
  const effectiveHeroIndex = $derived(normalizeChartIndex(heroIndex, count) ?? (count > 0 ? count - 1 : -1));
  const selectedIndex = $derived.by(() => {
    const index = selectedKey === null ? -1 : series.findIndex((point) => point.key === selectedKey);
    return index >= 0 ? index : effectiveHeroIndex;
  });
  const hoverIndex = $derived.by(() => {
    if (hoverKey === null) return null;
    const index = series.findIndex((point) => point.key === hoverKey);
    return index >= 0 ? index : null;
  });
  const visibleIndex = $derived(hoverIndex ?? selectedIndex);
  const active = $derived(series[visibleIndex] ?? series[effectiveHeroIndex] ?? series[series.length - 1]);

  const VB_H = 100;
  const colW = $derived(count > 0 ? 100 / count : 0);
  const barW = $derived(colW * 0.5);

  function barPath(i: number, total: number): string {
    const pct = scaleMax > 0 ? total / scaleMax : 0;
    const h = pct * VB_H;
    const x = i * colW + (colW - barW) / 2;
    const y = VB_H - h;
    const r = Math.min(2.4, barW / 2, h);
    if (h <= 0) return "";
    if (r <= 0) return `M ${x} ${y} L ${x + barW} ${y} L ${x + barW} ${VB_H} L ${x} ${VB_H} Z`;
    return `M ${x} ${y + r}
            Q ${x} ${y} ${x + r} ${y}
            L ${x + barW - r} ${y}
            Q ${x + barW} ${y} ${x + barW} ${y + r}
            L ${x + barW} ${VB_H}
            L ${x} ${VB_H}
            Z`;
  }

  function selectIndex(index: number) {
    selectedKey = series[index]?.key ?? null;
    hoverKey = null;
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      selectIndex(index);
      return;
    }

    const nextIndex = nextChartIndex(index, event.key, count);
    if (nextIndex === null) return;
    event.preventDefault();
    selectIndex(nextIndex);
    const controls = (event.currentTarget as HTMLButtonElement).parentElement?.children;
    (controls?.[nextIndex] as HTMLButtonElement | undefined)?.focus();
  }
</script>

<div class="monthly-bars">
  {#if allZero}
    <div class="chart-empty">
      <span class="empty-icon" aria-hidden="true"><TrendingUp size={20} /></span>
      {#if emptyTitle}<p class="empty-title">{emptyTitle}</p>{/if}
      {#if emptyHint}<p class="empty-hint">{emptyHint}</p>{/if}
      {#if emptyCta && onCta}
        <button class="empty-cta" onclick={onCta}>{emptyCta}</button>
      {/if}
    </div>
  {:else if view === "table"}
    <div class="table-scroll">
      <table class="data-table">
        <caption class="visually-hidden">{tableCaption}</caption>
        <thead>
          <tr>
            <th scope="col">{t("dashboard.tableColMonth")}</th>
            <th scope="col" class="col-num">{t("dashboard.tableColValue")}</th>
          </tr>
        </thead>
        <tbody>
          {#each series as s (s.key)}
            <tr>
              <td>{monthLabel(s.date)} {s.date.getFullYear()}</td>
              <td class="col-num">{formatValue(s.total)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="summary-row">
      <div class="summary-primary">
        <span class="active-month">{monthLabel(active.date)} {active.date.getFullYear()}</span>
        <strong class="active-value">{formatValue(active.total)}</strong>
      </div>
      {#if visibleIndex === effectiveHeroIndex && delta}
        <span class="month-comparison" class:up={delta.up} class:down={!delta.up}>
          {delta.up ? "+" : "−"}{delta.pct.toFixed(1)}%
          {#if deltaSuffix}<span>{deltaSuffix}</span>{/if}
        </span>
      {/if}
    </div>

    <div class="plot-wrap">
      <div class="plot" role="img" aria-label={ariaSummary}>
        <svg class="bars-svg" viewBox="0 0 100 {VB_H}" preserveAspectRatio="none" aria-hidden="true">
          <defs>
            <pattern id={hatchId} width="3" height="3" patternTransform="rotate(45)" patternUnits="userSpaceOnUse">
              <rect width="3" height="3" fill="var(--chart-hatch-bg)" />
              <line x1="0" y1="0" x2="0" y2="3" stroke="var(--chart-hatch)" stroke-width="1.1" />
            </pattern>
            <linearGradient id={heroGradId} x1="0" y1="1" x2="0" y2="0">
              <stop offset="0%" stop-color="var(--chart-hero-b)" />
              <stop offset="100%" stop-color="var(--chart-hero-a)" />
            </linearGradient>
          </defs>

          {#each ticks as tick (tick.topPct)}
            <line x1="0" y1={tick.topPct} x2="100" y2={tick.topPct} class="grid-line" vector-effect="non-scaling-stroke" />
          {/each}

          {#each series as s, i (s.key)}
            <path
              d={barPath(i, s.total)}
              class="bar"
              class:hero={i === effectiveHeroIndex}
              class:active={i === visibleIndex}
              fill={i === effectiveHeroIndex ? `url(#${heroGradId})` : `url(#${hatchId})`}
            />
          {/each}
        </svg>

        {#each ticks as tick (tick.topPct)}
          <span class="y-tick" style="top: {tick.topPct}%">{formatValue(tick.value)}</span>
        {/each}
      </div>

      <div class="month-controls" role="radiogroup" aria-label={ariaSummary}>
        {#each series as s, i (s.key)}
          <button
            type="button"
            class="month-control"
            role="radio"
            aria-checked={i === selectedIndex}
            tabindex={i === selectedIndex ? 0 : -1}
            onmouseenter={() => (hoverKey = s.key)}
            onmouseleave={() => (hoverKey = null)}
            onfocus={() => selectIndex(i)}
            onclick={() => selectIndex(i)}
            onkeydown={(event) => handleKeydown(event, i)}
            aria-label="{monthLabel(s.date)} {s.date.getFullYear()}: {formatValue(s.total)}"
          ></button>
        {/each}
      </div>
    </div>

    <div class="month-row">
      {#each series as s (s.key)}
        <span class="month-label">{monthLabel(s.date)}</span>
      {/each}
    </div>

    <table class="visually-hidden">
      <caption>{tableCaption}</caption>
      <thead>
        <tr><th scope="col">{t("dashboard.tableColMonth")}</th><th scope="col">{t("dashboard.tableColValue")}</th></tr>
      </thead>
      <tbody>
        {#each series as s (s.key)}
          <tr>
            <td>{monthLabel(s.date)} {s.date.getFullYear()}</td>
            <td>{formatValue(s.total)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .monthly-bars {
    container: monthly-bars / inline-size;
    display: flex;
    flex-direction: column;
    gap: 6px;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .summary-row {
    display: flex;
    flex-shrink: 0;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
  }

  .summary-primary {
    display: flex;
    min-width: 0;
    align-items: baseline;
    gap: 8px;
  }

  .active-month {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--chart-tick);
  }

  .active-value {
    overflow: hidden;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .month-comparison {
    display: flex;
    flex-shrink: 0;
    gap: 4px;
    font-size: 11px;
    font-weight: 700;
    color: var(--danger-text);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .month-comparison.up {
    color: var(--accent-text);
  }

  .month-comparison span {
    font-weight: 500;
    color: var(--chart-tick);
  }

  .plot-wrap {
    position: relative;
    flex: 1;
    min-height: 72px;
  }

  .table-scroll {
    flex: 1;
    min-height: 0;
    max-height: 100%;
    overflow-y: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
  }

  .data-table th {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    font-weight: 600;
    color: var(--chart-tick);
    text-align: left;
  }

  .data-table td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .data-table .col-num {
    text-align: right;
  }

  .data-table td.col-num {
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .plot {
    position: absolute;
    inset: 0;
  }

  .bars-svg {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }

  .grid-line {
    stroke: var(--chart-grid);
    stroke-width: 1;
  }

  .bar.hero {
    filter: drop-shadow(0 2px 8px var(--accent-glow));
  }

  .bar.active {
    stroke: var(--accent);
    stroke-width: 1.4;
    vector-effect: non-scaling-stroke;
  }

  .y-tick {
    position: absolute;
    left: 0;
    transform: translateY(-50%);
    font-size: 9px;
    color: var(--chart-tick);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }

  .month-controls {
    position: absolute;
    inset: 0;
    display: flex;
  }

  .month-control {
    flex: 1;
    height: 100%;
    min-width: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }

  .month-control:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: -2px;
  }

  .month-row {
    display: flex;
    flex-shrink: 0;
  }

  .month-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 11px;
    color: var(--chart-tick);
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chart-empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 12px;
    text-align: center;
  }

  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    margin-bottom: 2px;
    border-radius: var(--radius-md);
    background: var(--accent-subtle);
    color: var(--accent-text);
  }

  .empty-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-hint {
    max-width: 240px;
    font-size: 11px;
    color: var(--chart-tick);
    line-height: 1.4;
  }

  .empty-cta {
    padding: 6px 14px;
    margin-top: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background-color var(--duration-normal) var(--ease-out), border-color var(--duration-normal) var(--ease-out);
  }

  .empty-cta:hover {
    background: var(--accent-subtle);
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .empty-cta:active {
    transform: scale(0.98);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @container monthly-bars (max-width: 479px) {
    .month-label:nth-child(even) {
      visibility: hidden;
    }
  }

  @container monthly-bars (max-width: 399px) {
    .summary-row {
      align-items: flex-start;
      flex-direction: column;
      gap: 4px;
    }
  }
</style>
