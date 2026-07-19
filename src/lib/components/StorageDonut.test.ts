import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { mount, tick, unmount } from "svelte";

import StorageDonut from "./StorageDonut.svelte";

const segments = [
  { label: "JPEG", bytes: 600, count: 6 },
  { label: "PNG", bytes: 500, count: 5 },
  { label: "WebP", bytes: 400, count: 4 },
  { label: "AVIF", bytes: 300, count: 3 },
  { label: "HEIC", bytes: 100, count: 2 },
  { label: "TIFF", bytes: 100, count: 1 },
];

const instances: ReturnType<typeof mount>[] = [];

function renderStorageDonut(overrides: Record<string, unknown> = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  instances.push(
    mount(StorageDonut, {
      target,
      props: {
        segments,
        otherLabel: "Other",
        totalLabel: "2 KB",
        centerSub: "Total",
        facts: [
          { value: "21", label: "Files" },
          { value: "6", label: "Formats" },
          { value: "2 KB", label: "Scanned" },
        ],
        formatValue: (value: number) => `${value} B`,
        ariaSummary: "Storage by format",
        tableCaption: "Storage scan data",
        ...overrides,
      },
    }),
  );
  return target;
}

afterEach(async () => {
  await Promise.all(instances.splice(0).map((instance) => unmount(instance)));
  document.body.replaceChildren();
});

describe("StorageDonut", () => {
  it("groups six formats into five visual segments while preserving all table rows", () => {
    const target = renderStorageDonut();

    expect(target.querySelectorAll(".donut-seg")).toHaveLength(5);
    expect(target.querySelectorAll(".legend-row")).toHaveLength(5);
    expect(target.querySelector(".legend-row:last-child")?.textContent).toContain("Other");
    expect(target.querySelectorAll(".visually-hidden tbody tr")).toHaveLength(6);
  });

  it("keeps five or fewer formats ungrouped and sorted", () => {
    const target = renderStorageDonut({ segments: segments.slice(1) });
    const labels = [...target.querySelectorAll(".legend-name")].map((node) => node.textContent?.trim());

    expect(labels).toEqual(["PNG", "WebP", "AVIF", "HEIC", "TIFF"]);
    expect(target.textContent).not.toContain("Other");
  });

  it("uses a safe Other fallback until the localized label is provided", () => {
    const target = renderStorageDonut({ otherLabel: undefined });

    expect(target.querySelector(".legend-row:last-child")?.textContent).toContain("Other");
  });

  it("uses one selected tab stop and supports keyboard, focus, and click selection", async () => {
    const target = renderStorageDonut();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".legend-row")];

    expect(target.querySelector("[role='radiogroup']")?.getAttribute("aria-label")).toBe("Storage by format");
    expect(controls.every((control) => control.getAttribute("role") === "radio")).toBe(true);
    expect(controls.filter((control) => control.tabIndex === 0)).toEqual([controls[0]]);
    expect(controls.filter((control) => control.getAttribute("aria-checked") === "true")).toEqual([controls[0]]);

    controls[0].focus();
    controls[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[4]);
    expect(controls[4].getAttribute("aria-checked")).toBe("true");

    controls[4].dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[0]);

    controls[0].dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
    await tick();
    expect(document.activeElement).toBe(controls[4]);

    controls[2].focus();
    await tick();
    expect(controls[2].getAttribute("aria-checked")).toBe("true");

    controls[1].click();
    await tick();
    expect(controls[1].getAttribute("aria-checked")).toBe("true");

    controls[3].dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    await tick();
    expect(controls[3].getAttribute("aria-checked")).toBe("true");
  });

  it("previews hover without changing selection and restores the selected segment", async () => {
    const target = renderStorageDonut();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".legend-row")];

    controls[1].click();
    controls[3].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();
    expect(controls[1].getAttribute("aria-checked")).toBe("true");
    expect(controls[3].classList.contains("active")).toBe(true);
    expect(target.querySelectorAll(".donut-seg.active")).toHaveLength(1);

    controls[3].dispatchEvent(new MouseEvent("mouseleave"));
    await tick();
    expect(controls[1].classList.contains("active")).toBe(true);
    expect(controls[3].classList.contains("active")).toBe(false);
  });

  it("starts with the total and previews exact localized segment summaries", async () => {
    const formatPercent = (value: number) => new Intl.NumberFormat("de", {
      style: "percent",
      maximumFractionDigits: 0,
    }).format(value / 100);
    const target = renderStorageDonut({ formatPercent });
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".legend-row")];

    expect(target.querySelector(".ring-total")?.textContent).toBe("2 KB");
    expect(target.querySelector(".ring-sub")?.textContent).toBe("Total");

    controls[1].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("500 B");
    expect(target.querySelector(".ring-sub")?.textContent).toBe(`PNG · ${formatPercent(25)}`);

    controls[1].dispatchEvent(new MouseEvent("mouseleave"));
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("2 KB");
    expect(target.querySelector(".ring-sub")?.textContent).toBe("Total");
  });

  it("keeps the selected summary after hover leaves and focus blurs", async () => {
    const target = renderStorageDonut();
    const controls = [...target.querySelectorAll<HTMLButtonElement>(".legend-row")];

    controls[1].click();
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("500 B");
    controls[3].dispatchEvent(new MouseEvent("mouseenter"));
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("300 B");
    controls[3].dispatchEvent(new MouseEvent("mouseleave"));
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("500 B");

    controls[2].focus();
    await tick();
    controls[2].blur();
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("400 B");

    controls[2].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    await tick();
    expect(target.querySelector(".ring-total")?.textContent).toBe("300 B");
    expect(target.querySelector(".chart-tooltip")).toBeNull();
  });

  it("shows and announces only label, percentage, and exact bytes", () => {
    const target = renderStorageDonut();
    const first = target.querySelector<HTMLButtonElement>(".legend-row");

    expect(first?.textContent?.replace(/\s+/g, " ").trim()).toBe("JPEG 30% 600 B");
    expect(first?.getAttribute("aria-label")).toBe("JPEG: 30%, 600 B");
    expect(target.querySelector(".legend-count")).toBeNull();
  });

  it("renders one caption and all original rows in table mode", () => {
    const target = renderStorageDonut({ view: "table" });

    expect(target.querySelectorAll(".data-table > caption")).toHaveLength(1);
    expect(target.querySelector("caption")?.textContent).toBe("Storage scan data");
    expect(target.querySelectorAll("tbody tr")).toHaveLength(6);
    expect(target.querySelectorAll(".legend-row")).toHaveLength(0);
    const region = target.querySelector(".table-scroll");
    expect(region?.getAttribute("role")).toBe("region");
    expect(region?.getAttribute("tabindex")).toBe("0");
    expect(region?.getAttribute("aria-label")).toBe("Storage scan data");
    expect(target.querySelectorAll("tbody tr > th[scope='row']")).toHaveLength(6);
  });

  it("uses row headers in its screen-reader table", () => {
    const target = renderStorageDonut();
    expect(target.querySelectorAll(".visually-hidden tbody tr > th[scope='row']")).toHaveLength(6);
  });

  it("uses supplied locale-aware percent and count formatters", () => {
    const formatPercent = (value: number) => new Intl.NumberFormat("de", {
      style: "percent",
      maximumFractionDigits: 0,
    }).format(value / 100);
    const formatCount = (value: number) => new Intl.NumberFormat("de", { maximumFractionDigits: 0 }).format(value);
    const chart = renderStorageDonut({
      segments: [{ label: "JPEG", bytes: 3, count: 1234 }, { label: "PNG", bytes: 1, count: 5 }],
      formatPercent,
      formatCount,
    });

    expect(chart.querySelector(".legend-percent")?.textContent).toBe(formatPercent(75));
    expect(chart.querySelector(".legend-row")?.getAttribute("aria-label")).toContain(formatPercent(75));

    const table = renderStorageDonut({
      view: "table",
      segments: [{ label: "JPEG", bytes: 3, count: 1234 }],
      formatPercent,
      formatCount,
    });
    expect(table.querySelector("tbody tr td:last-child")?.textContent).toBe(formatCount(1234));
  });

  it("uses responsive container styles, reduced motion, a neutral track, and a passive SVG", () => {
    const target = renderStorageDonut();
    const source = readFileSync(resolve("src/lib/components/StorageDonut.svelte"), "utf8");
    const svg = target.querySelector("svg");

    expect(source).toContain(".storage-donut-shell { container: storage-donut / inline-size;");
    expect(source).not.toMatch(/\.storage-donut\s*\{[^}]*container:/s);
    expect(target.querySelector(".storage-donut-shell > .storage-donut")).not.toBeNull();
    expect(source).toContain("@container storage-donut (max-width: 379px)");
    expect(source).not.toContain("@container storage-donut (max-width: 420px)");
    expect(source).toContain("@container storage-donut (max-width: 320px)");
    expect(source).toContain("@media (prefers-reduced-motion: reduce)");
    expect(source).not.toMatch(/overflow-x:\s*(auto|scroll)/);
    expect(target.querySelectorAll(".donut-track")).toHaveLength(1);
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.querySelector("[onclick], [role='button']")).toBeNull();
  });
});
