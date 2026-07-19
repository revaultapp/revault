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
