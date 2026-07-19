<script lang="ts">
  import { untrack } from "svelte";
  import { donutSegments, groupDonutDisplaySegments, nextChartIndex } from "$lib/charts";
  import { t } from "$lib/stores/locale.svelte";

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
    otherLabel?: string;
    totalLabel: string;
    centerSub: string;
    facts: Fact[];
    formatValue: (n: number) => string;
    formatPercent?: (value: number) => string;
    formatCount?: (value: number) => string;
    ariaSummary: string;
    tableCaption: string;
    view?: "chart" | "table";
  }

  let {
    segments,
    otherLabel = "Other",
    totalLabel,
    centerSub,
    facts,
    formatValue,
    formatPercent = (percent) => `${percent.toFixed(0)}%`,
    formatCount = (count) => String(count),
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

  let selectedKey = $state<string | null>(
    untrack(() => groupDonutDisplaySegments(segments, otherLabel, 5)[0]?.key ?? null),
  );
  let hoverKey = $state<string | null>(null);

  const total = $derived(segments.reduce((sum, segment) => sum + segment.bytes, 0));
  const displaySegments = $derived(groupDonutDisplaySegments(segments, otherLabel, 5));
  const rings = $derived(
    donutSegments(
      displaySegments.map((segment) => segment.bytes),
      { r: R, gapPx: 3, capPx: STROKE / 2 },
    ),
  );
  const selectedIndex = $derived.by(() => {
    const index = selectedKey === null ? -1 : displaySegments.findIndex((segment) => segment.key === selectedKey);
    return index >= 0 ? index : 0;
  });
  const hoverIndex = $derived.by(() => {
    if (hoverKey === null) return null;
    const index = displaySegments.findIndex((segment) => segment.key === hoverKey);
    return index >= 0 ? index : null;
  });
  const visibleIndex = $derived(hoverIndex ?? selectedIndex);

  function percentage(bytes: number): string {
    return formatPercent(total > 0 ? (bytes / total) * 100 : 0);
  }

  function selectIndex(index: number) {
    selectedKey = displaySegments[index]?.key ?? null;
    hoverKey = null;
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      selectIndex(index);
      return;
    }

    const key = event.key === "ArrowUp" ? "ArrowLeft" : event.key === "ArrowDown" ? "ArrowRight" : event.key;
    const nextIndex = nextChartIndex(index, key, displaySegments.length);
    if (nextIndex === null) return;
    event.preventDefault();
    selectIndex(nextIndex);
    const controls = (event.currentTarget as HTMLButtonElement).parentElement?.children;
    (controls?.[nextIndex] as HTMLButtonElement | undefined)?.focus();
  }
</script>

<div class="storage-donut-shell">
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
          {#each segments as seg, index (`${seg.label}-${index}`)}
            <tr>
              <td>{seg.label}</td>
              <td class="col-num">{formatValue(seg.bytes)}</td>
              <td class="col-num">{formatCount(seg.count)}</td>
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

    <div class="ring-wrap" role="img" aria-label={ariaSummary}>
      <svg class="ring-svg" viewBox="0 0 {SIZE} {SIZE}" aria-hidden="true">
        <g transform="rotate(-90 {CENTER} {CENTER})">
          <circle class="donut-track" cx={CENTER} cy={CENTER} r={R} fill="none" stroke-width={STROKE} />
          {#each rings as ring, index (displaySegments[index].key)}
            <circle
              cx={CENTER}
              cy={CENTER}
              r={R}
              fill="none"
              stroke={CHART_COLORS[index % CHART_COLORS.length]}
              stroke-width={STROKE}
              stroke-linecap={ring.butt ? "butt" : "round"}
              stroke-dasharray={ring.dasharray}
              stroke-dashoffset={ring.dashoffset}
              class="donut-seg"
              class:active={visibleIndex === index}
              style="transform-origin: {CENTER}px {CENTER}px"
            />
          {/each}
        </g>
      </svg>

      <div class="ring-center">
        <span class="ring-total">{totalLabel}</span>
        <span class="ring-sub">{centerSub}</span>
      </div>
    </div>

    <div class="legend-col" role="radiogroup" aria-label={ariaSummary}>
      {#each displaySegments as seg, index (seg.key)}
        <button
          type="button"
          class="legend-row"
          class:active={visibleIndex === index}
          role="radio"
          aria-checked={selectedIndex === index}
          tabindex={selectedIndex === index ? 0 : -1}
          onmouseenter={() => (hoverKey = seg.key)}
          onmouseleave={() => (hoverKey = null)}
          onfocus={() => selectIndex(index)}
          onclick={() => selectIndex(index)}
          onkeydown={(event) => handleKeydown(event, index)}
          aria-label="{seg.label}: {percentage(seg.bytes)}, {formatValue(seg.bytes)}"
        >
          <span class="tick" style="background: {CHART_COLORS[index % CHART_COLORS.length]}"></span>
          <span class="legend-name">{seg.label}</span>
          <span class="legend-percent">{percentage(seg.bytes)}</span>
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
        {#each segments as seg, index (`${seg.label}-${index}`)}
          <tr><td>{seg.label}</td><td>{formatValue(seg.bytes)}</td><td>{formatCount(seg.count)}</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
</div>

<style>
  .storage-donut-shell { container: storage-donut / inline-size; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }

  .storage-donut {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(136px, 30cqi, 160px) minmax(0, 1.2fr);
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .storage-donut.table-mode { display: block; }
  .table-scroll { height: 100%; max-height: 100%; overflow-y: auto; overflow-x: hidden; }
  .data-table { width: 100%; table-layout: fixed; border-collapse: collapse; }
  .data-table th, .data-table td { overflow-wrap: anywhere; }
  .data-table th { padding: 6px 8px; border-bottom: 1px solid var(--border); font-size: 11px; font-weight: 600; color: var(--chart-tick); text-align: left; }
  .data-table td { padding: 6px 8px; border-bottom: 1px solid var(--border); font-size: 12px; color: var(--text-secondary); }
  .data-table .col-num { text-align: right; }
  .data-table td.col-num { color: var(--text-primary); font-variant-numeric: tabular-nums; }

  .facts-col { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .fact { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .fact-value { overflow: hidden; font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.01em; font-variant-numeric: tabular-nums; text-overflow: ellipsis; white-space: nowrap; }
  .fact-label { overflow: hidden; font-size: 11px; color: var(--chart-tick); text-overflow: ellipsis; white-space: nowrap; }

  .ring-wrap { position: relative; width: 100%; max-width: 160px; aspect-ratio: 1; margin: 0 auto; }
  .ring-svg { display: block; width: 100%; height: 100%; overflow: visible; }
  .donut-track { stroke: var(--border); }
  .donut-seg { transition: transform var(--duration-normal) var(--ease-out); }
  .donut-seg.active { transform: scale(1.045); }
  .ring-center { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; pointer-events: none; }
  .ring-total { font-size: 19px; font-weight: 800; color: var(--text-primary); letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .ring-sub { font-size: 9px; font-weight: 600; color: var(--chart-tick); letter-spacing: 0.04em; text-transform: uppercase; }

  .legend-col { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .legend-row { display: grid; grid-template-columns: 3px minmax(0, 1fr) auto auto; align-items: center; gap: 6px; min-width: 0; min-height: 36px; padding: 3px 4px; border-radius: var(--radius-sm); text-align: left; cursor: pointer; }
  .legend-row.active { font-weight: 700; background: var(--state-hover); }
  .legend-row:focus-visible { outline: 2px solid var(--accent-text); outline-offset: 2px; }
  .tick { width: 3px; height: 12px; border-radius: 2px; }
  .legend-name { overflow: hidden; font-size: 11px; color: var(--text-secondary); text-overflow: ellipsis; white-space: nowrap; }
  .legend-percent, .legend-value { font-size: 10px; color: var(--text-primary); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .legend-percent { color: var(--chart-tick); }

  .visually-hidden { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }

  @container storage-donut (max-width: 420px) {
    .storage-donut { grid-template-columns: minmax(120px, 0.8fr) minmax(0, 1.2fr); align-content: start; }
    .facts-col { grid-column: 1 / -1; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
    .fact { text-align: center; }
  }

  @container storage-donut (max-width: 320px) {
    .storage-donut { grid-template-columns: minmax(0, 1fr); }
    .facts-col { grid-column: auto; }
    .ring-wrap { width: min(150px, 100%); }
  }

  @media (prefers-reduced-motion: reduce) {
    .donut-seg { transition: none; }
    .donut-seg.active { transform: none; }
  }
</style>
