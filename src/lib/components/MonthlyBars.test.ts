import { afterEach, describe, expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import { createClassComponent } from "svelte/legacy";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import MonthlyBars from "./MonthlyBars.svelte";

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const series = months.map((month, index) => ({
  key: `2026-${String(index + 1).padStart(2, "0")}`,
  date: new Date(2026, index, 1),
  total: (index + 1) * 100,
  month,
}));

const instances: ReturnType<typeof mount>[] = [];
const legacyInstances: { $destroy(): void }[] = [];

function renderMonthlyBars(overrides: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  instances.push(
    mount(MonthlyBars, {
      target,
      props: {
        series,
        heroIndex: 11,
        monthLabel: (date: Date) => months[date.getMonth()],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Savings over the last twelve months",
        tableCaption: "Monthly savings data",
        delta: { pct: 25, up: true },
        deltaSuffix: "vs previous month",
        ...overrides,
      },
    }),
  );
  return target;
}

afterEach(async () => {
  await Promise.all(instances.splice(0).map((instance) => unmount(instance)));
  legacyInstances.splice(0).forEach((instance) => instance.$destroy());
  document.body.replaceChildren();
});

describe("MonthlyBars", () => {
  it("renders all twelve exact month controls with one initial tab stop", () => {
    const target = renderMonthlyBars();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    const group = target.querySelector("[role='radiogroup']");

    expect(group?.getAttribute("aria-label")).toBe("Savings over the last twelve months");
    expect(controls).toHaveLength(12);
    expect(controls.every((control) => control.getAttribute("role") === "radio")).toBe(true);
    expect(controls.map((control) => control.getAttribute("aria-label"))).toEqual(
      months.map((month, index) => `${month} 2026: ${(index + 1) * 100} B`),
    );
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[11]]);
    expect(controls.filter((control) => control.getAttribute("aria-checked") === "true")).toEqual([controls[11]]);
  });

  it("shows the current month summary and comparison without a permanent tooltip", () => {
    const target = renderMonthlyBars();

    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Dec 2026");
    expect(target.querySelector(".active-value")?.textContent?.trim()).toBe("1200 B");
    expect(target.querySelector(".month-comparison")?.textContent?.replace(/\s+/g, " ").trim()).toBe(
      "+25.0% vs previous month",
    );
    expect(target.querySelector(".chart-tooltip")).toBeNull();
  });

  it("uses the supplied locale-aware formatter for its comparison", () => {
    const formatPercent = (value: number) => new Intl.NumberFormat("de", {
      style: "percent",
      maximumFractionDigits: 1,
    }).format(value / 100);
    const target = renderMonthlyBars({
      formatPercent,
    });

    expect(target.querySelector(".month-comparison")?.textContent?.replace(/\s+vs/, " vs").trim()).toBe(
      `+${formatPercent(25)} vs previous month`,
    );
  });

  it("moves selection and focus with ArrowLeft, Home, End, and wrapping ArrowRight", async () => {
    const target = renderMonthlyBars();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[11].focus();
    controls[11].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[10]);
    expect(controls[10].tabIndex).toBe(0);
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Nov 2026");
    expect(target.querySelector(".month-comparison")).toBeNull();

    controls[10].dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[0]);

    controls[0].dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[11]);

    controls[11].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[0]);
  });

  it("previews on hover without changing selection and restores it on mouseleave", async () => {
    const target = renderMonthlyBars();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[11].focus();
    controls[2].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();

    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Mar 2026");
    expect(target.querySelector(".active-value")?.textContent?.trim()).toBe("300 B");
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[11]]);
    expect(controls[2].getAttribute("aria-checked")).toBe("false");
    expect(target.querySelectorAll(".bar.active")).toHaveLength(1);

    controls[2].dispatchEvent(new MouseEvent("mouseleave"));
    await tick();

    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Dec 2026");
    expect(target.querySelector(".active-value")?.textContent?.trim()).toBe("1200 B");
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[11]]);
  });

  it("selects radio months with click, Enter, and Space", async () => {
    const target = renderMonthlyBars();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[2].click();
    await tick();
    expect(controls[2].tabIndex).toBe(0);
    expect(controls[2].getAttribute("aria-checked")).toBe("true");

    controls[5].dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await tick();
    expect(controls[5].getAttribute("aria-checked")).toBe("true");

    controls[8].dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    await tick();
    expect(controls[8].getAttribute("aria-checked")).toBe("true");
  });

  it("preserves selection by key and falls back to hero when the selected key disappears", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    const component = createClassComponent({
      component: MonthlyBars,
      target,
      props: {
        series: [] as (typeof series)[number][],
        heroIndex: -1,
        monthLabel: (date: Date) => months[date.getMonth()],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Savings over the last twelve months",
        tableCaption: "Monthly savings data",
      },
    });
    legacyInstances.push(component);

    component.$set({ series, heroIndex: 11 });
    await tick();
    let controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[11]]);
    expect(controls.filter((control) => control.getAttribute("aria-checked") === "true")).toEqual([controls[11]]);

    controls[4].click();
    await tick();
    const reordered = [series[4], ...series.slice(0, 4), ...series.slice(5)];
    component.$set({ series: reordered, heroIndex: 10 });
    await tick();
    controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[0]]);

    component.$set({ series: series.slice(0, 4), heroIndex: 3 });
    await tick();
    controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[3]]);
    expect(controls.filter((control) => control.getAttribute("aria-checked") === "true")).toEqual([controls[3]]);
  });

  it("ignores a hovered key that disappears from a same-length replacement series", async () => {
    const dynamicTarget = document.createElement("div");
    document.body.append(dynamicTarget);
    const dynamic = createClassComponent({
      component: MonthlyBars,
      target: dynamicTarget,
      props: {
        series: series.slice(0, 4),
        heroIndex: 3,
        monthLabel: (date: Date) => months[date.getMonth()],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Savings over the last twelve months",
        tableCaption: "Monthly savings data",
      },
    });
    legacyInstances.push(dynamic);
    const controls = [...dynamicTarget.querySelectorAll<HTMLButtonElement>(".month-control")];
    controls[1].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();

    dynamic.$set({ series: [series[0], series[2], series[3], series[5]], heroIndex: 2 });
    await tick();
    expect(dynamicTarget.querySelector(".active-month")?.textContent?.trim()).toBe("Apr 2026");
  });

  it("renders one visually hidden caption in visible table mode", () => {
    const target = document.createElement("div");
    document.body.append(target);
    instances.push(
      mount(MonthlyBars, {
        target,
        props: {
          series,
          heroIndex: 11,
          monthLabel: (date: Date) => months[date.getMonth()],
          formatValue: (value: number) => `${value} B`,
          ariaSummary: "Savings over the last twelve months",
          tableCaption: "Monthly savings data",
          view: "table",
        },
      }),
    );

    const captions = target.querySelectorAll(".data-table > caption");
    expect(captions).toHaveLength(1);
    expect(captions[0].classList.contains("visually-hidden")).toBe(true);
    expect(captions[0].textContent).toBe("Monthly savings data");
  });

  it("uses component container queries without JS width measurement or horizontal scrolling", () => {
    const source = readFileSync(resolve("src/lib/components/MonthlyBars.svelte"), "utf8");

    expect(source).toContain("container: monthly-bars / inline-size");
    expect(source).toContain("@container monthly-bars (max-width: 479px)");
    expect(source).toContain("@container monthly-bars (max-width: 399px)");
    expect(source).not.toContain("bind:clientWidth");
    expect(source).not.toContain("$effect");
    expect(source).toContain("selectedKey");
    expect(source).toContain("hoverKey");
    expect(source).not.toMatch(/overflow-x:\s*(auto|scroll)/);
    expect(source).not.toContain('aria-live="polite"');
    expect(source).toMatch(/\.active-value\s*\{[^}]*font-weight:\s*700;/s);
    expect(source).toMatch(/@container monthly-bars \(max-width: 399px\)[\s\S]*?gap:\s*4px;/);
  });

  it("receives the monthly delta and shares the accessible 36px Dashboard toggle target", () => {
    const source = readFileSync(resolve("src/lib/components/DashboardPage.svelte"), "utf8");

    expect(source).toMatch(/<MonthlyBars[\s\S]*?delta=\{\$momDeltas\.saved\}[\s\S]*?deltaSuffix=\{t\("dashboard\.vsPrevMonth"\)\}/);
    expect(source).toContain('class="card-corner"');
    expect(source).toMatch(/\.card-corner\s*\{[^}]*width:\s*36px;[^}]*height:\s*36px;/s);
    expect(source).toMatch(/\.card-corner:focus-visible\s*\{/);
  });

  it("keeps row-a unchanged and constrains table scrolling to the monthly card", () => {
    const source = readFileSync(resolve("src/lib/components/DashboardPage.svelte"), "utf8");
    const rowA = source.match(/\.row-a\s*\{([^}]*)\}/)?.[1] ?? "";
    const monthlyCard = source.match(/\.monthly-card\s*\{([^}]*)\}/)?.[1] ?? "";

    expect(source).toContain('<section class="chart-card monthly-card" aria-labelledby="monthly-savings-title">');
    expect(rowA).toContain("min-height: 230px;");
    expect(rowA.replace("min-height", "")).not.toMatch(/\bheight:/);
    expect(monthlyCard).toContain("contain: size;");
    expect(monthlyCard).toContain("overflow: hidden;");
    expect(monthlyCard).not.toMatch(/\bheight:/);
    expect(source).toMatch(/\.monthly-card \.card-body\s*\{[^}]*overflow:\s*hidden;/s);
  });
});
