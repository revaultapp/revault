<script lang="ts">
  import { t } from "$lib/stores/locale.svelte";
  import { untrack } from "svelte";
  import { LineChart } from "lucide-svelte";
  import { smoothPath, niceMax } from "$lib/charts";
  import ChartTooltip from "./ChartTooltip.svelte";

  type Kind = "img" | "vid" | "pdf";

  interface MonthPoint {
    date: Date;
    img: number;
    vid: number;
    pdf: number;
  }

  interface Share {
    kind: Kind;
    label: string;
    sharePct: number;
    delta: { pct: number; up: boolean } | null;
  }

  interface Props {
    series: MonthPoint[];
    shares: Share[];
    monthLabel: (d: Date) => string;
    formatValue: (n: number) => string;
    ariaSummary: string;
    tableCaption: string;
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
    shares,
    monthLabel,
    formatValue,
    ariaSummary,
    tableCaption,
    emptyTitle,
    emptyHint,
    emptyCta,
    onCta,
    view = "chart",
  }: Props = $props();

  const KINDS: Kind[] = ["img", "vid", "pdf"];
  const KIND_VAR: Record<Kind, string> = {
    img: "var(--chart-line-img)",
    vid: "var(--chart-line-vid)",
    pdf: "var(--chart-line-pdf)",
  };

  let wrapperWidth = $state(0);
  let plotWidth = $state(0);
  let plotHeight = $state(0);
  // Default crosshair position is the last month — capture the initial
  // length once; the series array's length never changes after mount.
  let activeIndex = $state(untrack(() => series.length - 1));

  const count = $derived(series.length);
  const showNarrowLabels = $derived(wrapperWidth > 0 && wrapperWidth < 480);
  const allZero = $derived(series.every((s) => s.img === 0 && s.vid === 0 && s.pdf === 0));
  const maxVal = $derived(Math.max(0, ...series.flatMap((s) => [s.img, s.vid, s.pdf])));
  const scaleMax = $derived(niceMax(maxVal || 1));

  const VB_H = 100;
  const xAt = $derived((i: number) => (count > 1 ? (i / (count - 1)) * 100 : 50));
  const yAt = $derived((v: number) => (scaleMax > 0 ? VB_H - (v / scaleMax) * VB_H : VB_H));

  const linePaths = $derived(
    KINDS.map((kind) => ({
      kind,
      d: smoothPath(series.map((s, i) => ({ x: xAt(i), y: yAt(s[kind]) }))),
    }))
  );

  const active = $derived(series[activeIndex] ?? series[series.length - 1]);
  const activeX = $derived(xAt(activeIndex));
  const crosshairPxX = $derived((activeX / 100) * plotWidth);

  function setActive(i: number) {
    activeIndex = i;
  }

  function reset() {
    activeIndex = series.length - 1;
  }

  const tooltipTitle = $derived(active ? `${monthLabel(active.date)} ${active.date.getFullYear()}` : "");
  const tooltipRows = $derived(
    KINDS.map((kind) => ({
      swatch: KIND_VAR[kind],
      label: shares.find((s) => s.kind === kind)?.label ?? kind,
      value: formatValue(active?.[kind] ?? 0),
    }))
  );
  const tooltipY = $derived.by(() => {
    if (!active || plotHeight <= 0) return 0;
    const ys = KINDS.map((k) => yAt(active[k]));
    return (Math.min(...ys) / VB_H) * plotHeight;
  });
</script>

<div class="category-lines" bind:clientWidth={wrapperWidth}>
  {#if allZero}
    <div class="chart-empty">
      <span class="empty-icon" aria-hidden="true"><LineChart size={20} /></span>
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
            {#each shares as share (share.kind)}<th scope="col" class="col-num">{share.label}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each series as s (s.date.toISOString())}
            <tr>
              <td>{monthLabel(s.date)} {s.date.getFullYear()}</td>
              {#each KINDS as kind (kind)}<td class="col-num">{formatValue(s[kind])}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="lines-body">
      <div class="mini-legend" aria-hidden="true">
        {#each shares as share (share.kind)}
          <div class="legend-item" style="--rule-color: {KIND_VAR[share.kind]}">
            <span class="legend-share">{share.sharePct.toFixed(0)}%</span>
            {#if share.delta}
              <span class="legend-delta" class:up={share.delta.up} class:down={!share.delta.up}>
                {share.delta.up ? "+" : "-"}{share.delta.pct.toFixed(0)}%
              </span>
            {/if}
            <span class="legend-label">{share.label}</span>
          </div>
        {/each}
      </div>

      <div class="plot-wrap">
        <div class="plot" bind:clientWidth={plotWidth} bind:clientHeight={plotHeight} role="img" aria-label={ariaSummary}>
          <svg class="lines-svg" viewBox="0 0 100 {VB_H}" preserveAspectRatio="none" aria-hidden="true">
            {#each [0.75, 0.5, 0.25] as f (f)}
              <line x1="0" y1={(1 - f) * 100} x2="100" y2={(1 - f) * 100} class="grid-line" vector-effect="non-scaling-stroke" />
            {/each}

            {#each linePaths as line (line.kind)}
              <path
                d={line.d}
                class="series-line"
                style="stroke: {KIND_VAR[line.kind]}"
                vector-effect="non-scaling-stroke"
              />
            {/each}
          </svg>

          <div class="crosshair" style="transform: translateX({crosshairPxX}px)" aria-hidden="true">
            {#each KINDS as kind (kind)}
              <span
                class="crosshair-dot"
                style="top: {(yAt(active?.[kind] ?? 0) / VB_H) * 100}%; background: {KIND_VAR[kind]}"
              ></span>
            {/each}
          </div>
        </div>

        <!-- Sibling of the role="img" .plot, not a descendant — a role="img"
             subtree is flattened for assistive tech, which would strip these
             focusable hover targets out of the accessibility tree. Same
             visual position via the shared .plot-wrap coordinate space. -->
        <div class="hover-row">
          {#each series as s, i (s.date.toISOString())}
            <button
              type="button"
              class="hover-col"
              onmouseenter={() => setActive(i)}
              onfocus={() => setActive(i)}
              onmouseleave={reset}
              onblur={reset}
              aria-label="{monthLabel(s.date)} {s.date.getFullYear()}"
            ></button>
          {/each}
        </div>

        <ChartTooltip visible={true} x={crosshairPxX} y={tooltipY} title={tooltipTitle} rows={tooltipRows} />
      </div>
    </div>

    <div class="month-row">
      {#each series as s, i (s.date.toISOString())}
        <span class="month-label" class:hidden-label={showNarrowLabels && i % 2 !== 0}>{monthLabel(s.date)}</span>
      {/each}
    </div>

    <table class="visually-hidden">
      <caption>{tableCaption}</caption>
      <thead>
        <tr>
          <th scope="col">{t("dashboard.tableColMonth")}</th>
          {#each shares as share (share.kind)}<th scope="col">{share.label}</th>{/each}
        </tr>
      </thead>
      <tbody>
        {#each series as s (s.date.toISOString())}
          <tr>
            <td>{monthLabel(s.date)} {s.date.getFullYear()}</td>
            {#each KINDS as kind (kind)}<td>{formatValue(s[kind])}</td>{/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .category-lines {
    display: flex;
    flex-direction: column;
    gap: 6px;
    height: 100%;
    min-height: 0;
  }

  .lines-body {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .mini-legend {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex-shrink: 0;
    width: 84px;
    padding: 2px 12px 2px 0;
  }

  .legend-item {
    position: relative;
    display: flex;
    flex-direction: column;
    padding-left: 8px;
  }

  .legend-item::before {
    content: "";
    position: absolute;
    top: 1px;
    bottom: 1px;
    left: 0;
    width: 2.5px;
    border-radius: 2px;
    background: var(--rule-color);
  }

  .legend-share {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .legend-delta {
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .legend-delta.up { color: var(--accent-text); }
  .legend-delta.down { color: var(--danger-text); }

  .legend-label {
    font-size: 11px;
    color: var(--chart-tick);
    text-transform: capitalize;
  }

  .plot-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
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

  .lines-svg {
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

  .series-line {
    fill: none;
    stroke-width: 2px;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .crosshair {
    position: absolute;
    top: 0;
    left: 0;
    width: 1px;
    height: 100%;
    border-left: 1px dashed var(--border);
    pointer-events: none;
  }

  .crosshair-dot {
    position: absolute;
    left: 50%;
    width: 6px;
    height: 6px;
    border: 2px solid var(--bg-card);
    border-radius: 50%;
    transform: translate(-50%, -50%);
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
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .month-row {
    display: flex;
    flex-shrink: 0;
    padding-left: 84px;
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
