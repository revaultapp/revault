<script lang="ts">
  import { t } from "$lib/stores/locale.svelte";
  import { untrack } from "svelte";
  import { LineChart } from "lucide-svelte";
  import { nextChartIndex, niceMax, smoothPath } from "$lib/charts";

  type Kind = "img" | "vid" | "pdf";

  interface MonthPoint {
    key: string;
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
    formatPercent?: (value: number) => string;
    emptyTitle?: string;
    emptyHint?: string;
    emptyCta?: string;
    onCta?: () => void;
    view?: "chart" | "table";
  }

  let {
    series,
    shares,
    monthLabel,
    formatValue,
    ariaSummary,
    tableCaption,
    formatPercent = (percent) => `${percent.toFixed(0)}%`,
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
  const KIND_DASH: Record<Kind, string | undefined> = { img: undefined, vid: "6 4", pdf: "2 4" };

  let selectedKey: string | null = $state(untrack(() => series[series.length - 1]?.key ?? null));
  let hoverKey: string | null = $state(null);

  const count = $derived(series.length);
  const allZero = $derived(series.every((s) => s.img === 0 && s.vid === 0 && s.pdf === 0));
  const maxVal = $derived(Math.max(0, ...series.flatMap((s) => [s.img, s.vid, s.pdf])));
  const scaleMax = $derived(niceMax(maxVal || 1));
  const selectedIndex = $derived.by(() => {
    const index = selectedKey === null ? -1 : series.findIndex((point) => point.key === selectedKey);
    return index >= 0 ? index : series.length - 1;
  });
  const hoverIndex = $derived.by(() => {
    if (hoverKey === null) return null;
    const index = series.findIndex((point) => point.key === hoverKey);
    return index >= 0 ? index : null;
  });
  const visibleIndex = $derived(hoverIndex ?? selectedIndex);
  const active = $derived(series[visibleIndex] ?? series[series.length - 1]);

  const VB_H = 100;
  const xAt = $derived((index: number) => (count > 1 ? (index / (count - 1)) * 100 : 50));
  const yAt = $derived((value: number) => VB_H - (value / scaleMax) * VB_H);
  const linePaths = $derived(
    KINDS.map((kind) => ({
      kind,
      d: smoothPath(series.map((point, index) => ({ x: xAt(index), y: yAt(point[kind]) }))),
    })),
  );

  function labelFor(kind: Kind): string {
    return shares.find((share) => share.kind === kind)?.label ?? kind;
  }

  function sharePctFor(kind: Kind): number {
    return shares.find((share) => share.kind === kind)?.sharePct ?? 0;
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

<div class="category-lines">
  {#if allZero}
    <div class="chart-empty">
      <span class="empty-icon" aria-hidden="true"><LineChart size={20} /></span>
      {#if emptyTitle}<p class="empty-title">{emptyTitle}</p>{/if}
      {#if emptyHint}<p class="empty-hint">{emptyHint}</p>{/if}
      {#if emptyCta && onCta}<button class="empty-cta" onclick={onCta}>{emptyCta}</button>{/if}
    </div>
  {:else if view === "table"}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (scrollable table region must be keyboard-focusable) -->
    <div class="table-scroll" role="region" tabindex="0" aria-label={tableCaption}>
      <table class="data-table">
        <caption class="visually-hidden">{tableCaption}</caption>
        <thead>
          <tr>
            <th scope="col">{t("dashboard.tableColMonth")}</th>
            {#each shares as share (share.kind)}<th scope="col" class="col-num">{share.label}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each series as point (point.key)}
            <tr>
              <th scope="row">{monthLabel(point.date)} {point.date.getFullYear()}</th>
              {#each KINDS as kind (kind)}<td class="col-num">{formatValue(point[kind])}</td>{/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="summary-row">
      <span class="active-month">{monthLabel(active.date)} {active.date.getFullYear()}</span>
      <div class="active-values">
        {#each KINDS as kind (kind)}
          <span class="active-value"><span>{labelFor(kind)} {formatPercent(sharePctFor(kind))}</span> {formatValue(active[kind])}</span>
        {/each}
      </div>
    </div>

    <div class="lines-body">
      <div class="mini-legend" aria-hidden="true">
        {#each shares as share (share.kind)}
          <div class="legend-item">
            <span class="legend-rule {share.kind}" style="--rule-color: {KIND_VAR[share.kind]}"></span>
            <span class="legend-share">{formatPercent(share.sharePct)}</span>
            <span class="legend-label">{share.label}</span>
          </div>
        {/each}
      </div>

      <div class="plot-wrap">
        <div class="plot" role="img" aria-label={ariaSummary}>
          <svg class="lines-svg" viewBox="0 0 100 {VB_H}" preserveAspectRatio="none" aria-hidden="true">
            {#each [0.75, 0.5, 0.25] as fraction (fraction)}
              <line x1="0" y1={(1 - fraction) * 100} x2="100" y2={(1 - fraction) * 100} class="grid-line" vector-effect="non-scaling-stroke" />
            {/each}
            {#each linePaths as line (line.kind)}
              <path d={line.d} class="series-line {line.kind}" style="stroke: {KIND_VAR[line.kind]}" stroke-dasharray={KIND_DASH[line.kind]} vector-effect="non-scaling-stroke" />
            {/each}
          </svg>

          <div class="crosshair" style="left: {xAt(visibleIndex)}%" aria-hidden="true">
            {#each KINDS as kind (kind)}
              <span class="crosshair-dot" style="top: {yAt(active[kind])}%; background: {KIND_VAR[kind]}"></span>
            {/each}
          </div>
        </div>

        <div class="month-controls" role="radiogroup" aria-label={ariaSummary}>
          {#each series as point, index (point.key)}
            <button
              type="button"
              class="month-control"
              role="radio"
              aria-checked={index === selectedIndex}
              tabindex={index === selectedIndex ? 0 : -1}
              onmouseenter={() => (hoverKey = point.key)}
              onmouseleave={() => (hoverKey = null)}
              onfocus={() => selectIndex(index)}
              onclick={() => selectIndex(index)}
              onkeydown={(event) => handleKeydown(event, index)}
              aria-label="{monthLabel(point.date)} {point.date.getFullYear()}: {labelFor("img")} {formatValue(point.img)}, {labelFor("vid")} {formatValue(point.vid)}, {labelFor("pdf")} {formatValue(point.pdf)}"
            ></button>
          {/each}
        </div>
      </div>
    </div>

    <div class="month-row">
      {#each series as point (point.key)}<span class="month-label">{monthLabel(point.date)}</span>{/each}
    </div>

    <table class="visually-hidden">
      <caption>{tableCaption}</caption>
      <thead>
        <tr><th scope="col">{t("dashboard.tableColMonth")}</th>{#each shares as share (share.kind)}<th scope="col">{share.label}</th>{/each}</tr>
      </thead>
      <tbody>
        {#each series as point (point.key)}
          <tr><th scope="row">{monthLabel(point.date)} {point.date.getFullYear()}</th>{#each KINDS as kind (kind)}<td>{formatValue(point[kind])}</td>{/each}</tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .category-lines {
    container: category-lines / inline-size;
    display: flex;
    flex-direction: column;
    gap: 6px;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .summary-row, .active-values, .legend-item { display: flex; }
  .summary-row { flex-shrink: 0; align-items: baseline; gap: 12px; }
  .active-month { flex-shrink: 0; font-size: 11px; font-weight: 600; color: var(--chart-tick); }
  .active-values { min-width: 0; flex-wrap: wrap; gap: 4px 12px; }
  .active-value { font-size: 13px; font-weight: 700; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .active-value span { font-size: 10px; font-weight: 500; color: var(--chart-tick); }

  .lines-body { display: flex; flex: 1; min-height: 0; }
  .mini-legend { display: flex; width: 84px; flex-shrink: 0; flex-direction: column; gap: 10px; padding: 2px 12px 2px 0; }
  .legend-item { display: grid; grid-template-columns: 18px auto; align-items: center; column-gap: 5px; }
  .legend-rule { width: 18px; border-top: 2px solid var(--rule-color); }
  .legend-rule.vid { border-top-style: dashed; }
  .legend-rule.pdf { border-top-style: dotted; }
  .legend-share { font-size: 14px; font-weight: 700; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .legend-label { grid-column: 2; font-size: 11px; color: var(--chart-tick); text-transform: capitalize; }

  .plot-wrap { position: relative; flex: 1; min-width: 0; min-height: 72px; }
  .plot, .lines-svg, .month-controls { position: absolute; inset: 0; }
  .lines-svg { display: block; width: 100%; height: 100%; }
  .grid-line { stroke: var(--chart-grid); stroke-width: 1; }
  .series-line { fill: none; stroke-width: 2px; stroke-linecap: round; stroke-linejoin: round; }
  .crosshair { position: absolute; top: 0; height: 100%; border-left: 1px dashed var(--border); pointer-events: none; }
  .crosshair-dot { position: absolute; left: 50%; width: 6px; height: 6px; border: 2px solid var(--bg-card); border-radius: 50%; transform: translate(-50%, -50%); }
  .month-controls { display: flex; }
  .month-control { flex: 1; min-width: 0; height: 100%; border: 0; background: none; cursor: pointer; }
  .month-control:focus-visible, .empty-cta:focus-visible { outline: 2px solid var(--accent-text); outline-offset: -2px; }
  .month-row { display: flex; flex-shrink: 0; padding-left: 84px; }
  .month-label { flex: 1; min-width: 0; overflow: hidden; font-size: 11px; color: var(--chart-tick); text-align: center; text-overflow: ellipsis; white-space: nowrap; }

  .table-scroll { flex: 1; min-height: 0; max-height: 100%; overflow-y: auto; }
  .data-table { width: 100%; border-collapse: collapse; }
  .data-table th { padding: 6px 8px; border-bottom: 1px solid var(--border); font-size: 11px; font-weight: 600; color: var(--chart-tick); text-align: left; }
  .data-table td { padding: 6px 8px; border-bottom: 1px solid var(--border); font-size: 12px; color: var(--text-secondary); }
  .data-table tbody th { padding: 6px 8px; border-bottom: 1px solid var(--border); font-size: 12px; font-weight: 400; color: var(--text-secondary); }
  .data-table .col-num { text-align: right; }
  .data-table td.col-num { color: var(--text-primary); font-variant-numeric: tabular-nums; }

  .chart-empty { display: flex; flex: 1; flex-direction: column; align-items: center; justify-content: center; gap: 6px; padding: 12px; text-align: center; }
  .empty-icon { display: flex; align-items: center; justify-content: center; width: 40px; height: 40px; margin-bottom: 2px; border-radius: var(--radius-md); background: var(--accent-subtle); color: var(--accent-text); }
  .empty-title { font-size: 13px; font-weight: 600; color: var(--text-primary); }
  .empty-hint { max-width: 240px; font-size: 11px; color: var(--chart-tick); line-height: 1.4; }
  .empty-cta { padding: 6px 14px; margin-top: 4px; border: 1px solid var(--border); border-radius: var(--radius-sm); font-size: 12px; font-weight: 500; color: var(--text-secondary); transition: background-color var(--duration-normal) var(--ease-out), border-color var(--duration-normal) var(--ease-out); }
  .empty-cta:hover { background: var(--accent-subtle); border-color: var(--accent); color: var(--accent-text); }
  .empty-cta:active { transform: scale(0.98); }

  .visually-hidden { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }

  @container category-lines (max-width: 460px) {
    .lines-body { flex-direction: column; }
    .mini-legend { display: grid; width: auto; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; padding: 0; }
    .month-row { padding-left: 0; }
  }

  @container category-lines (max-width: 420px) {
    .month-label:nth-child(even) { visibility: hidden; }
  }
</style>
