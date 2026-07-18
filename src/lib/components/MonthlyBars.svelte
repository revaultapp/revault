<script lang="ts">
  import { t } from "$lib/stores/locale.svelte";
  import { untrack } from "svelte";
  import { TrendingUp } from "lucide-svelte";
  import { niceMax } from "$lib/charts";
  import ChartTooltip from "./ChartTooltip.svelte";

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
    /** Row label shown in the hover tooltip (e.g. "Space recovered") — passed
        in since this component takes zero store imports of its own. */
    valueLabel: string;
    emptyTitle?: string;
    emptyHint?: string;
    emptyCta?: string;
    onCta?: () => void;
    /** "chart" (default) renders the plot with its sr-only data table;
        "table" renders only a visible version of that same table — the
        visible table becomes the accessible content, so the sr-only
        duplicate is omitted. */
    view?: "chart" | "table";
  }

  let {
    series,
    heroIndex,
    monthLabel,
    formatValue,
    ariaSummary,
    tableCaption,
    valueLabel,
    emptyTitle,
    emptyHint,
    emptyCta,
    onCta,
    view = "chart",
  }: Props = $props();

  const uid = $props.id();
  const hatchId = `mbars-hatch-${uid}`;
  const heroGradId = `mbars-hero-${uid}`;

  let plotEl: HTMLDivElement | undefined = $state();
  let wrapperWidth = $state(0);
  let plotWidth = $state(0);
  let plotHeight = $state(0);
  // heroIndex is always "current month" (last series entry) and never
  // changes after mount — capture it once as the initial active bar.
  let activeIndex = $state(untrack(() => heroIndex));

  const count = $derived(series.length);
  const allZero = $derived(series.every((s) => s.total === 0));
  const maxTotal = $derived(Math.max(0, ...series.map((s) => s.total)));
  const scaleMax = $derived(niceMax(maxTotal || 1));
  const ticks = $derived([0.75, 0.5, 0.25].map((f) => ({ topPct: (1 - f) * 100, value: scaleMax * f })));
  const showNarrowLabels = $derived(wrapperWidth > 0 && wrapperWidth < 480);
  const active = $derived(series[activeIndex] ?? series[series.length - 1]);

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

  function setActive(i: number) {
    activeIndex = i;
  }

  function reset() {
    activeIndex = heroIndex;
  }

  const tooltipX = $derived(plotWidth > 0 ? (activeIndex + 0.5) * (plotWidth / count) : 0);
  const tooltipY = $derived.by(() => {
    if (plotHeight <= 0 || scaleMax <= 0) return 0;
    const pct = (active?.total ?? 0) / scaleMax;
    return plotHeight * (1 - pct);
  });
  const tooltipTitle = $derived(active ? `${monthLabel(active.date)} ${active.date.getFullYear()}` : "");
</script>

<div class="monthly-bars" bind:clientWidth={wrapperWidth}>
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
    <div class="plot-wrap">
      <div
        class="plot"
        bind:this={plotEl}
        bind:clientWidth={plotWidth}
        bind:clientHeight={plotHeight}
        role="img"
        aria-label={ariaSummary}
      >
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
              class:hero={i === heroIndex}
              fill={i === heroIndex ? `url(#${heroGradId})` : `url(#${hatchId})`}
            />
          {/each}
        </svg>

        {#each ticks as tick (tick.topPct)}
          <span class="y-tick" style="top: {tick.topPct}%">{formatValue(tick.value)}</span>
        {/each}
      </div>

      <!-- Sibling of the role="img" .plot, not a descendant — a role="img"
           subtree is flattened for assistive tech, which would strip these
           focusable hover targets out of the accessibility tree. Same visual
           position via the shared .plot-wrap coordinate space. -->
      <div class="hover-row">
        {#each series as s, i (s.key)}
          <button
            type="button"
            class="hover-col"
            onmouseenter={() => setActive(i)}
            onfocus={() => setActive(i)}
            onmouseleave={reset}
            onblur={reset}
            aria-label="{monthLabel(s.date)} {s.date.getFullYear()}: {formatValue(s.total)}"
          ></button>
        {/each}
      </div>

      <ChartTooltip
        visible={true}
        x={tooltipX}
        y={tooltipY}
        title={tooltipTitle}
        rows={[{ label: valueLabel, value: formatValue(active?.total ?? 0) }]}
      />
    </div>

    <div class="month-row">
      {#each series as s, i (s.key)}
        <span class="month-label" class:hidden-label={showNarrowLabels && i % 2 !== 0}>
          {monthLabel(s.date)}
        </span>
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
    display: flex;
    flex-direction: column;
    gap: 6px;
    height: 100%;
    min-height: 0;
  }

  .plot-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  .table-scroll {
    flex: 1;
    min-height: 0;
    max-height: 100%;
    overflow-y: auto;
    overflow-x: auto;
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
    width: 100%;
    height: 100%;
    display: block;
  }

  .grid-line {
    stroke: var(--chart-grid);
    stroke-width: 1;
  }

  .bar.hero {
    filter: drop-shadow(0 2px 8px var(--accent-glow));
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

  .hover-row {
    position: absolute;
    inset: 0;
    display: flex;
  }

  .hover-col {
    flex: 1;
    height: 100%;
    background: none;
    border: none;
    cursor: pointer;
  }

  .hover-col:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: -2px;
  }

  .month-row {
    display: flex;
    flex-shrink: 0;
  }

  .month-label {
    flex: 1;
    overflow: hidden;
    font-size: 11px;
    color: var(--chart-tick);
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hidden-label {
    visibility: hidden;
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
</style>
