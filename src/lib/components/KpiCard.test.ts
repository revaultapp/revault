import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { mount, unmount } from "svelte";
import { Circle } from "lucide-svelte";

import KpiCard from "./KpiCard.svelte";

const instances: ReturnType<typeof mount>[] = [];

afterEach(async () => {
  await Promise.all(instances.splice(0).map((instance) => unmount(instance)));
  document.body.replaceChildren();
});

describe("KpiCard", () => {
  it("shows the full wrapping label with a title fallback", () => {
    const target = document.createElement("div");
    document.body.append(target);
    instances.push(mount(KpiCard, {
      target,
      props: { label: "A very long localized KPI label", icon: Circle, value: "42" },
    }));
    const label = target.querySelector(".kpi-label");
    const source = readFileSync(resolve("src/lib/components/KpiCard.svelte"), "utf8");

    expect(label?.getAttribute("title")).toBe("A very long localized KPI label");
    expect(source).toMatch(/\.kpi-label\s*\{[^}]*overflow-wrap:\s*anywhere;/s);
    expect(source).not.toMatch(/\.kpi-label\s*\{[^}]*white-space:\s*nowrap;/s);
    expect(source).not.toMatch(/\.kpi-label\s*\{[^}]*text-overflow:\s*ellipsis;/s);
  });

  it("uses the supplied locale-aware formatter for its delta", () => {
    const formatPercent = (value: number) => new Intl.NumberFormat("es", {
      style: "percent",
      maximumFractionDigits: 1,
    }).format(value / 100);
    const target = document.createElement("div");
    document.body.append(target);
    instances.push(mount(KpiCard, {
      target,
      props: {
        label: "Recuperado",
        icon: Circle,
        value: "2,84 GB",
        delta: { pct: 12.5, up: true },
        formatPercent,
      },
    }));

    expect(target.querySelector(".delta-pct")?.textContent?.trim()).toBe(`+${formatPercent(12.5)}`);
  });
});
