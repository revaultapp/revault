<script lang="ts">
  import {
    HardDrive,
    Layers,
    Minimize2,
    Shield,
    Calendar,
    Share2,
    Download,
    Search,
    ArrowUpRight,
    FolderOpen,
  } from "lucide-svelte";
  import KpiCard from "./KpiCard.svelte";
  import MonthlyBars from "./MonthlyBars.svelte";
  import CategoryLines from "./CategoryLines.svelte";
  import StorageDonut from "./StorageDonut.svelte";
  import Button from "./Button.svelte";
  import { savings } from "$lib/stores/savings";
  import { activePage } from "$lib/stores/nav";
  import { formatBytes } from "$lib/utils";
  import { animatedNumber } from "$lib/motion";
  import { storage, breakdown } from "$lib/stores/storage";
  import { history, monthlySeries, momDeltas, categoryShares, protectedTotal } from "$lib/stores/history";
  import { t, getLocale } from "$lib/stores/locale.svelte";

  let avgCompression = $derived(
    $savings.totalOriginalBytes > 0
      ? Math.max(0, Math.round(($savings.totalOriginalBytes - $savings.totalCompressedBytes) / $savings.totalOriginalBytes * 100))
      : 0
  );

  // Stat-card headline numbers count up toward their target instead of
  // snapping, mirroring CompressPage's estimate hero (reduced-motion guard
  // lives inside animatedNumber itself).
  const spaceSavedTween = animatedNumber(0);
  const avgCompressionTween = animatedNumber(0);
  const protectedTween = animatedNumber(0);
  const analyzedTween = animatedNumber(0);

  $effect(() => {
    spaceSavedTween.set($savings.totalSavedBytes);
    avgCompressionTween.set(avgCompression);
    protectedTween.set($protectedTotal);
    analyzedTween.set($history.lastScan?.total ?? 0);
  });

  function monthLabel(d: Date): string {
    return new Intl.DateTimeFormat(getLocale(), { month: "short" }).format(d);
  }

  const monthlyGrandTotal = $derived($monthlySeries.reduce((acc, m) => acc + m.total, 0));

  const categoryLinesShares = $derived(
    $categoryShares.map((share) => ({
      kind: share.kind,
      label:
        share.kind === "img"
          ? t("dashboard.catImages")
          : share.kind === "vid"
            ? t("dashboard.catVideo")
            : t("dashboard.catPdf"),
      sharePct: share.share,
      delta: share.delta,
    }))
  );

  function sharePctFor(kind: "img" | "vid" | "pdf"): number {
    return Math.round($categoryShares.find((s) => s.kind === kind)?.share ?? 0);
  }

  const categoryAriaSummary = $derived(
    t("dashboard.chartCategoryAria", {
      img: sharePctFor("img"),
      vid: sharePctFor("vid"),
      pdf: sharePctFor("pdf"),
    })
  );

  // Donut source: prefer this session's live scan result; fall back to the
  // last persisted scan (survives app restart) so returning users still see
  // a filled-in panel instead of the idle empty state.
  const hasDonutData = $derived(
    $storage.scanState === "done" || ($storage.scanState === "idle" && $history.lastScan !== null)
  );

  const donutData = $derived.by(() => {
    if ($storage.scanState === "done" && $storage.scanResult) {
      return $breakdown.map((g) => ({ label: g.extension, bytes: g.totalSize, count: g.count }));
    }
    const scan = $history.lastScan;
    if (scan) {
      return scan.types.map(([ext, bytes, count]) => ({ label: ext.toUpperCase(), bytes, count }));
    }
    return [];
  });

  const donutTotalBytes = $derived(donutData.reduce((acc, s) => acc + s.bytes, 0));

  const donutFacts = $derived([
    { value: formatBytes($savings.totalSavedBytes), label: t("dashboard.donutFactFreed") },
    { value: String($savings.filesProcessed), label: t("dashboard.donutFactFiles") },
    { value: String($protectedTotal), label: t("dashboard.donutFactNoMetadata") },
    { value: String($savings.heicCount), label: t("dashboard.donutFactHeic") },
  ]);

  function goToOptimize() {
    activePage.set("optimize");
  }
</script>

<div class="dashboard">
  <header class="dash-head">
    <h2>{t("dashboard.panelTitle")}</h2>
    <div class="dash-head-actions">
      <button class="range-pill" type="button" disabled>
        <Calendar size={12} />
        {t("dashboard.rangeLastYear")}
      </button>
      <button class="icon-btn" type="button" aria-label={t("dashboard.shareAria")} disabled>
        <Share2 size={14} />
      </button>
      <button class="icon-btn" type="button" aria-label={t("dashboard.exportAria")} disabled>
        <Download size={14} />
      </button>
      <Button size="sm" class="scan-cta" onclick={() => storage.scanFolder()}>
        <Search size={14} />
        {t("dashboard.scanFolderButton")}
      </Button>
    </div>
  </header>

  <div class="row row-a">
    <div class="kpi-grid">
      <KpiCard
        label={t("dashboard.kpiSpaceRecovered")}
        icon={HardDrive}
        value={formatBytes(spaceSavedTween.current)}
        delta={$momDeltas.saved}
        deltaSuffix={t("dashboard.vsPrevMonth")}
      />
      <KpiCard
        label={t("dashboard.kpiAnalyzedSize")}
        icon={Layers}
        value={$history.lastScan ? formatBytes(analyzedTween.current) : "—"}
      />
      <KpiCard
        label={t("dashboard.avgCompression")}
        icon={Minimize2}
        value="{Math.round(avgCompressionTween.current)}%"
        delta={$momDeltas.compression}
        deltaSuffix={t("dashboard.vsPrevMonth")}
      />
      <KpiCard
        label={t("dashboard.kpiProtectedFiles")}
        icon={Shield}
        value={Math.round(protectedTween.current).toString()}
        sub={t("dashboard.kpiProtectedSub")}
      />
    </div>

    <section class="chart-card">
      <div class="card-head">
        <span class="card-title">{t("dashboard.chartMonthlyTitle")}</span>
        <button class="card-corner" type="button" tabindex="-1" aria-hidden="true">
          <ArrowUpRight size={14} />
        </button>
      </div>
      <div class="card-body">
        <MonthlyBars
          series={$monthlySeries}
          heroIndex={$monthlySeries.length - 1}
          {monthLabel}
          formatValue={formatBytes}
          ariaSummary={t("dashboard.chartMonthlyAria", { total: formatBytes(monthlyGrandTotal) })}
          tableCaption={t("dashboard.tableCaptionMonthly")}
          valueLabel={t("dashboard.kpiSpaceRecovered")}
          emptyTitle={t("dashboard.emptyHistoryTitle")}
          emptyHint={t("dashboard.emptyHistoryHint")}
          emptyCta={t("dashboard.emptyHistoryCta")}
          onCta={goToOptimize}
        />
      </div>
    </section>
  </div>

  <div class="row row-b">
    <section class="chart-card">
      <div class="card-head">
        <span class="card-title">{t("dashboard.chartCategoryTitle")}</span>
        <button class="card-corner" type="button" tabindex="-1" aria-hidden="true">
          <ArrowUpRight size={14} />
        </button>
      </div>
      <div class="card-body">
        <CategoryLines
          series={$monthlySeries}
          shares={categoryLinesShares}
          {monthLabel}
          formatValue={formatBytes}
          ariaSummary={categoryAriaSummary}
          tableCaption={t("dashboard.tableCaptionCategory")}
          emptyTitle={t("dashboard.emptyHistoryTitle")}
          emptyHint={t("dashboard.emptyHistoryHint")}
          emptyCta={t("dashboard.emptyHistoryCta")}
          onCta={goToOptimize}
        />
      </div>
    </section>

    <section class="chart-card">
      <div class="card-head">
        <span class="card-title">{t("dashboard.chartDonutTitle")}</span>
        <button class="card-corner" type="button" tabindex="-1" aria-hidden="true">
          <ArrowUpRight size={14} />
        </button>
      </div>
      <div class="card-body">
        {#if $storage.scanState === "scanning"}
          <div class="donut-status" role="status" aria-label={t("dashboard.scanningAriaLabel")}>
            <div class="spinner" aria-hidden="true"></div>
            <p>{t("dashboard.scanningText")}</p>
            <span class="status-path">{$storage.folderPath}</span>
          </div>
        {:else if $storage.scanState === "error"}
          <div class="donut-status error" role="alert">
            <p>{t("dashboard.scanFailed")}</p>
            <span>{$storage.errorMessage}</span>
            <Button danger size="sm" style="margin-top: 8px" onclick={() => storage.scanFolder()}>
              {t("dashboard.tryAgain")}
            </Button>
          </div>
        {:else if hasDonutData}
          <StorageDonut
            segments={donutData}
            totalLabel={formatBytes(donutTotalBytes)}
            centerSub={t("dashboard.totalStorage")}
            facts={donutFacts}
            formatValue={formatBytes}
            ariaSummary={t("dashboard.chartDonutAria", { total: formatBytes(donutTotalBytes) })}
            tableCaption={t("dashboard.tableCaptionDonut")}
          />
        {:else}
          <div class="donut-status">
            <span class="status-icon" aria-hidden="true"><FolderOpen size={20} /></span>
            <p>{t("dashboard.scanIdleHint")}</p>
            <button class="status-cta" type="button" onclick={() => storage.scanFolder()}>
              <Search size={12} />
              {t("dashboard.scanFolderButton")}
            </button>
          </div>
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 14px;
    /* .content-area already contributes 28px; no extra padding here. */
    padding: 0;
    height: 100%;
    overflow-y: auto;
  }

  .dash-head {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .dash-head h2 {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .dash-head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .range-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: default;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-secondary);
    transition: background-color var(--duration-normal) var(--ease-out), color var(--duration-normal) var(--ease-out);
  }

  .icon-btn:hover {
    background: var(--navy-bg);
    color: var(--text-primary);
  }

  .icon-btn:active {
    transform: scale(0.96);
  }

  :global(button.scan-cta) {
    background: linear-gradient(135deg, var(--chart-hero-a), var(--chart-hero-b));
    color: var(--text-on-accent);
  }

  .row {
    display: grid;
    flex-shrink: 0;
    gap: 14px;
    min-height: 0;
  }

  .row-a {
    grid-template-columns: 0.95fr 1.35fr;
    min-height: 230px;
  }

  .row-b {
    grid-template-columns: 1.25fr 1fr;
    flex: 1;
    min-height: 260px;
  }

  .kpi-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 12px;
    min-height: 0;
  }

  .chart-card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
  }

  .card-head {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .card-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--chart-tick);
    letter-spacing: 0.02em;
  }

  .card-corner {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition: background-color var(--duration-normal) var(--ease-out), color var(--duration-normal) var(--ease-out);
  }

  .card-corner:hover {
    background: var(--navy-bg);
    color: var(--accent-text);
  }

  .card-body {
    flex: 1;
    min-height: 0;
  }

  .donut-status {
    display: flex;
    height: 100%;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-muted);
    text-align: center;
  }

  .donut-status.error {
    color: var(--danger-text);
  }

  .donut-status p {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .donut-status.error p {
    color: var(--danger-text);
    font-weight: 600;
  }

  .donut-status span {
    max-width: 220px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .status-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md);
    background: var(--accent-subtle);
    color: var(--accent-text);
  }

  .status-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .status-cta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    margin-top: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: background-color var(--duration-normal) var(--ease-out), border-color var(--duration-normal) var(--ease-out);
  }

  .status-cta:hover {
    background: var(--accent-subtle);
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .status-cta:active {
    transform: scale(0.98);
  }
</style>
