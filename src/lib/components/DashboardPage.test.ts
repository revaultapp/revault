import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = readFileSync(resolve("src/lib/components/DashboardPage.svelte"), "utf8");

describe("DashboardPage integration contracts", () => {
  it("only renders a donut for positive scan data and localizes its grouped label", () => {
    expect(source).toMatch(/const hasDonutData = \$derived\(donutData\.length > 0 && donutTotalBytes > 0\)/);
    expect(source).toContain('otherLabel={t("dashboard.donutOther")}');
  });

  it("labels every chart section with a unique h3", () => {
    expect(source).toContain('aria-labelledby="monthly-savings-title"');
    expect(source).toContain('<h3 id="monthly-savings-title"');
    expect(source).toContain('aria-labelledby="category-savings-title"');
    expect(source).toContain('<h3 id="category-savings-title"');
    expect(source).toContain('aria-labelledby="last-scan-title"');
    expect(source).toContain('<h3 id="last-scan-title"');
  });

  it("announces each toggle's current action and gives every toggle a shared focusable target", () => {
    expect(source).toContain('dashboard.showChart');
    expect(source).toContain('dashboard.showTable');
    expect(source).not.toContain('dashboard.cardTableToggle');
    expect(source).toMatch(/\.card-corner\s*\{[^}]*width:\s*36px;[^}]*height:\s*36px;/s);
    expect(source).toMatch(/\.card-corner:focus-visible\s*\{/);
    expect(source).not.toContain("monthly-card-toggle");
  });

  it("uses an outer query container, keeps content scrolling at the shell, and stacks naturally", () => {
    const dashboardRule = source.match(/\.dashboard\s*\{([^}]*)\}/)?.[1] ?? "";

    expect(source).toContain('<div class="dashboard-shell">');
    expect(source).toMatch(/\.dashboard-shell\s*\{[^}]*container:\s*dashboard\s*\/\s*inline-size;/s);
    expect(source).toContain("@container dashboard (max-width: 820px)");
    expect(source).toMatch(/@container dashboard \(max-width: 820px\)[\s\S]*?\.row-a,[\s\S]*?\.row-b\s*\{[^}]*grid-template-columns:\s*minmax\(0, 1fr\);/s);
    expect(dashboardRule).toContain("min-height: 100%;");
    expect(dashboardRule.replace("min-height", "")).not.toMatch(/\bheight:\s*100%/);
    expect(dashboardRule).not.toMatch(/overflow-y:\s*auto/);
    expect(source).not.toMatch(/overflow-x:\s*(auto|scroll)/);
  });

  it("uses locale-aware formatters for dashboard values and visible counts", () => {
    expect(source).toContain("formatBytes(bytes, getLocale())");
    expect(source).toContain("new Intl.NumberFormat(getLocale()).format");
    expect(source).toContain("formatValue={formatDashboardBytes}");
  });

  it("uses a retry class and disables spinner motion when reduced motion is preferred", () => {
    expect(source).toContain('class="retry-button"');
    expect(source).not.toContain('style="margin-top: 8px"');
    expect(source).toMatch(/@media \(prefers-reduced-motion: reduce\)[\s\S]*?\.spinner\s*\{[^}]*animation:\s*none;/s);
  });
});
