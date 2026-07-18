<script lang="ts">
  import { t } from "$lib/stores/locale.svelte";
  import { donutSegments } from "$lib/charts";
  import ChartTooltip from "./ChartTooltip.svelte";

  interface Segment {
    label: string;
    bytes: number;
    count: number;
  }

  interface Fact {
    value: string;
    label: string;
  }

  interface Props {
    segments: Segment[];
    totalLabel: string;
    centerSub: string;
    facts: Fact[];
    formatValue: (n: number) => string;
    ariaSummary: string;
    tableCaption: string;
    /** "chart" (default) renders the facts/ring/legend grid with its sr-only
        data table; "table" replaces the whole grid with a visible version of
        that same table — the visible table becomes the accessible content,
        so the sr-only duplicate is omitted. */
    view?: "chart" | "table";
  }

  let {
    segments,
    totalLabel,
    centerSub,
    facts,
    formatValue,
    ariaSummary,
    tableCaption,
    view = "chart",
  }: Props = $props();

  const CHART_COLORS = [
    "var(--chart-1)",
    "var(--chart-2)",
    "var(--chart-3)",
    "var(--chart-4)",
    "var(--chart-5)",
  ];

  const R = 73;
  const STROKE = 24;
  const SIZE = 200;
  const CENTER = SIZE / 2;

  let hoverIndex = $state<number | null>(null);
  let containerEl: HTMLDivElement | undefined = $state();

  const total = $derived(segments.reduce((a, s) => a + s.bytes, 0));
  const rings = $derived(
    donutSegments(
      segments.map((s) => s.bytes),
      { r: R, gapPx: 3, capPx: STROKE / 2 }
    )
  );

  const active = $derived(hoverIndex !== null ? segments[hoverIndex] : null);
  const activePct = $derived(active && total > 0 ? (active.bytes / total) * 100 : 0);

  const tooltipRows = $derived(
    active
      ? [
          { swatch: CHART_COLORS[(hoverIndex ?? 0) % CHART_COLORS.length], label: active.label, value: formatValue(active.bytes) },
        ]
      : []
  );
</script>

<div class="storage-donut" class:table-mode={view === "table"}>
  {#if view === "table"}
    <div class="table-scroll">
      <table class="data-table">
        <caption class="visually-hidden">{tableCaption}</caption>
        <thead>
          <tr>
            <th scope="col">{t("dashboard.tableColType")}</th>
            <th scope="col" class="col-num">{t("dashboard.tableColSize")}</th>
            <th scope="col" class="col-num">{t("dashboard.tableColFiles")}</th>
          </tr>
        </thead>
        <tbody>
          {#each segments as seg (seg.label)}
            <tr>
              <td>{seg.label}</td>
              <td class="col-num">{formatValue(seg.bytes)}</td>
              <td class="col-num">{seg.count}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <div class="facts-col">
      {#each facts as fact (fact.label)}
        <div class="fact">
          <span class="fact-value">{fact.value}</span>
          <span class="fact-label">{fact.label}</span>
        </div>
      {/each}
    </div>

    <div class="ring-wrap" bind:this={containerEl} role="img" aria-label={ariaSummary}>
      <svg class="ring-svg" viewBox="0 0 {SIZE} {SIZE}" aria-hidden="true">
        <g transform="rotate(-90 {CENTER} {CENTER})">
          {#each rings as ring, i (segments[i]?.label ?? i)}
            <circle
              cx={CENTER}
              cy={CENTER}
              r={R}
              fill="none"
              stroke={CHART_COLORS[i % CHART_COLORS.length]}
              stroke-width={STROKE}
              stroke-linecap={ring.butt ? "butt" : "round"}
              stroke-dasharray={ring.dasharray}
              stroke-dashoffset={ring.dashoffset}
              class="donut-seg"
              class:active={hoverIndex === i}
              style="transform-origin: {CENTER}px {CENTER}px"
              onmouseenter={() => (hoverIndex = i)}
              onmouseleave={() => (hoverIndex = null)}
              role="presentation"
            />
          {/each}
        </g>
      </svg>

      <div class="ring-center">
        <span class="ring-total">{totalLabel}</span>
        <span class="ring-sub">{centerSub}</span>
      </div>

      <ChartTooltip
        visible={hoverIndex !== null}
        x={CENTER}
        y={CENTER}
        title={active?.label ?? ""}
        sub="{activePct.toFixed(0)}%"
        rows={tooltipRows}
      />
    </div>

    <div class="legend-col">
      {#each segments as seg, i (seg.label)}
        <button
          type="button"
          class="legend-row"
          onmouseenter={() => (hoverIndex = i)}
          onfocus={() => (hoverIndex = i)}
          onmouseleave={() => (hoverIndex = null)}
          onblur={() => (hoverIndex = null)}
          aria-label="{seg.label} — {formatValue(seg.bytes)} · {total > 0 ? ((seg.bytes / total) * 100).toFixed(0) : 0}%"
        >
          <span class="tick" style="background: {CHART_COLORS[i % CHART_COLORS.length]}"></span>
          <span class="legend-name">{seg.label}</span>
          <span class="legend-value">{formatValue(seg.bytes)}</span>
        </button>
      {/each}
    </div>

    <table class="visually-hidden">
      <caption>{tableCaption}</caption>
      <thead>
        <tr><th scope="col">{t("dashboard.tableColType")}</th><th scope="col">{t("dashboard.tableColSize")}</th><th scope="col">{t("dashboard.tableColFiles")}</th></tr>
      </thead>
      <tbody>
        {#each segments as seg (seg.label)}
          <tr>
            <td>{seg.label}</td>
            <td>{formatValue(seg.bytes)}</td>
            <td>{seg.count}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .storage-donut {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 170px minmax(0, 1fr);
    align-items: center;
    gap: 16px;
    height: 100%;
  }

  /* Table mode replaces the 3-column grid with a single scrolling table. */
  .storage-donut.table-mode {
    display: block;
  }

  .table-scroll {
    height: 100%;
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

  .facts-col {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .fact {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .fact-value {
    overflow: hidden;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.01em;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fact-label {
    overflow: hidden;
    font-size: 11px;
    color: var(--chart-tick);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ring-wrap {
    position: relative;
    width: 170px;
    height: 170px;
    margin: 0 auto;
  }

  .ring-svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .donut-seg {
    cursor: pointer;
    transition: transform var(--duration-normal) var(--ease-out);
  }

  .donut-seg.active {
    transform: scale(1.08);
  }

  .ring-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    pointer-events: none;
  }

  .ring-total {
    font-size: 19px;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .ring-sub {
    font-size: 9px;
    font-weight: 600;
    color: var(--chart-tick);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .legend-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .legend-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
  }

  .legend-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .tick {
    width: 3px;
    height: 12px;
    flex-shrink: 0;
    border-radius: 2px;
  }

  .legend-name {
    overflow: hidden;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .legend-value {
    /* Was flex-shrink: 0 with no overflow guard — at narrow widths it could
       push past the legend column's track since nothing clipped it. Letting
       it shrink with the same ellipsis treatment as .legend-name keeps it
       inside the track instead of spilling. */
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    margin-left: auto;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
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
