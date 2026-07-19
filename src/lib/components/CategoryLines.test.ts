import { afterEach, describe, expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";
import { createClassComponent } from "svelte/legacy";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import CategoryLines from "./CategoryLines.svelte";

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const series = months.map((month, index) => ({
  key: `2026-${String(index + 1).padStart(2, "0")}`,
  date: new Date(2026, index, 1),
  img: (index + 1) * 100,
  vid: (index + 1) * 20,
  pdf: index + 1,
  month,
}));
const shares = [
  { kind: "img" as const, label: "Images", sharePct: 80, delta: { pct: 4, up: true } },
  { kind: "vid" as const, label: "Video", sharePct: 15, delta: null },
  { kind: "pdf" as const, label: "PDF", sharePct: 5, delta: { pct: 2, up: false } },
];

const instances: ReturnType<typeof mount>[] = [];
const legacyInstances: { $destroy(): void }[] = [];

function renderCategoryLines(overrides: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  instances.push(
    mount(CategoryLines, {
      target,
      props: {
        series,
        shares,
        monthLabel: (date: Date) => months[date.getMonth()],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Savings by file type",
        tableCaption: "Category savings data",
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

describe("CategoryLines", () => {
  it("renders twelve exact month radios with one tab stop and a stable active summary", () => {
    const target = renderCategoryLines();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    expect(target.querySelector("[role='radiogroup']")?.getAttribute("aria-label")).toBe("Savings by file type");
    expect(controls).toHaveLength(12);
    expect(controls.map((control) => control.getAttribute("aria-label"))).toEqual(
      months.map(
        (month, index) =>
          `${month} 2026: Images ${(index + 1) * 100} B, Video ${(index + 1) * 20} B, PDF ${index + 1} B`,
      ),
    );
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[11]]);
    expect(controls.filter((control) => control.getAttribute("aria-checked") === "true")).toEqual([controls[11]]);
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Dec 2026");
    expect(
      [...target.querySelectorAll(".active-value")].map((value) => value.textContent?.replace(/\s+/g, " ").trim()),
    ).toEqual(["Images 80% 1200 B", "Video 15% 240 B", "PDF 5% 12 B"]);
    expect(target.querySelector(".chart-tooltip")).toBeNull();
  });

  it("uses the supplied locale-aware formatter for visible shares", () => {
    const formatPercent = (value: number) => new Intl.NumberFormat("de", {
      style: "percent",
      maximumFractionDigits: 0,
    }).format(value / 100);
    const target = renderCategoryLines({
      formatPercent,
    });

    expect([...target.querySelectorAll(".legend-share")].map((node) => node.textContent?.trim())).toEqual([
      formatPercent(80),
      formatPercent(15),
      formatPercent(5),
    ]);
    expect(target.querySelector(".active-value span")?.textContent?.trim()).toBe(`Images ${formatPercent(80)}`);
  });

  it("moves selection and focus with arrows, Home, End, and wrapping", async () => {
    const target = renderCategoryLines();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[11].focus();
    controls[11].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[10]);
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Nov 2026");

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

  it("moves down to the next month and up to the previous month with wrapping", async () => {
    const target = renderCategoryLines();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[11].focus();
    controls[11].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[0]);

    controls[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[11]);
  });

  it("previews hover without selecting and restores the selected month", async () => {
    const target = renderCategoryLines();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[11].focus();
    controls[2].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Mar 2026");
    expect(controls[11].getAttribute("aria-checked")).toBe("true");
    expect(controls[2].getAttribute("aria-checked")).toBe("false");

    controls[2].dispatchEvent(new MouseEvent("mouseleave"));
    await tick();
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Dec 2026");
  });

  it("selects months with click, Enter, Space, and focus", async () => {
    const target = renderCategoryLines();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];

    controls[2].click();
    await tick();
    expect(controls[2].getAttribute("aria-checked")).toBe("true");

    controls[5].dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await tick();
    expect(controls[5].getAttribute("aria-checked")).toBe("true");

    controls[8].dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    await tick();
    expect(controls[8].getAttribute("aria-checked")).toBe("true");

    controls[1].focus();
    await tick();
    expect(controls[1].getAttribute("aria-checked")).toBe("true");
  });

  it("handles empty-to-data, reorder, shrink, and a stale hover by stable key", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    const component = createClassComponent({
      component: CategoryLines,
      target,
      props: {
        series: [] as (typeof series)[number][],
        shares,
        monthLabel: (date: Date) => months[date.getMonth()],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Savings by file type",
        tableCaption: "Category savings data",
        emptyTitle: "No history",
      },
    });
    legacyInstances.push(component);

    expect(target.querySelector(".chart-empty")?.textContent).toContain("No history");
    component.$set({ series });
    await tick();
    let controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls[11].getAttribute("aria-checked")).toBe("true");

    controls[4].click();
    controls[1].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();
    const reordered = [series[4], series[0], ...series.slice(2, 4), ...series.slice(5)];
    component.$set({ series: reordered });
    await tick();
    controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls[0].getAttribute("aria-checked")).toBe("true");
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("May 2026");

    controls[0].click();
    component.$set({ series: series.slice(0, 4) });
    await tick();
    controls = [...target.querySelectorAll<HTMLButtonElement>(".month-control")];
    expect(controls[3].getAttribute("aria-checked")).toBe("true");
    expect(target.querySelector(".active-month")?.textContent?.trim()).toBe("Apr 2026");
  });

  it("renders the complete 12x3 table with one accessible caption", () => {
    const target = document.createElement("div");
    document.body.append(target);
    instances.push(
      mount(CategoryLines, {
        target,
        props: {
          series,
          shares,
          monthLabel: (date: Date) => months[date.getMonth()],
          formatValue: (value: number) => `${value} B`,
          ariaSummary: "Savings by file type",
          tableCaption: "Category savings data",
          view: "table",
        },
      }),
    );

    expect(target.querySelectorAll(".data-table > caption")).toHaveLength(1);
    expect(target.querySelector("caption")?.textContent).toBe("Category savings data");
    expect(target.querySelectorAll("tbody tr")).toHaveLength(12);
    expect(target.querySelectorAll("tbody tr:first-child td")).toHaveLength(3);
    const region = target.querySelector(".table-scroll");
    expect(region?.getAttribute("role")).toBe("region");
    expect(region?.getAttribute("tabindex")).toBe("0");
    expect(region?.getAttribute("aria-label")).toBe("Category savings data");
    expect(target.querySelectorAll("tbody tr > th[scope='row']")).toHaveLength(12);
  });

  it("uses row headers in its screen-reader table", () => {
    const target = renderCategoryLines();
    expect(target.querySelectorAll(".visually-hidden tbody tr > th[scope='row']")).toHaveLength(12);
  });

  it("uses container queries, keyed state, distinct line styles, and a delta-free legend", () => {
    const target = renderCategoryLines();
    const source = readFileSync(resolve("src/lib/components/CategoryLines.svelte"), "utf8");

    expect(source).toContain("container: category-lines / inline-size");
    expect(source).toContain("@container category-lines (max-width: 460px)");
    expect(source).toContain("@container category-lines (max-width: 420px)");
    expect(source).not.toContain("bind:clientWidth");
    expect(source).not.toContain("bind:clientHeight");
    expect(source).not.toContain("$effect");
    expect(source).toContain("selectedKey");
    expect(source).toContain("hoverKey");
    expect(source).not.toMatch(/overflow-x:\s*(auto|scroll)/);
    expect(source).toContain('img: undefined, vid: "6 4", pdf: "2 4"');
    expect(target.querySelectorAll(".legend-delta")).toHaveLength(0);
    const rules = [...target.querySelectorAll(".legend-rule")];
    expect(rules.map((rule) => ["img", "vid", "pdf"].find((kind) => rule.classList.contains(kind)))).toEqual([
      "img",
      "vid",
      "pdf",
    ]);
  });
});
