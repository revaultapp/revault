import { describe, it, expect, beforeEach, vi } from "vitest";

const reducedMotion = vi.hoisted(() => ({ current: false }));

vi.mock("svelte/motion", async (importOriginal) => {
  const actual = await importOriginal<typeof import("svelte/motion")>();
  return { ...actual, prefersReducedMotion: reducedMotion };
});

import { animatedNumber } from "./motion";

describe("animatedNumber", () => {
  beforeEach(() => {
    reducedMotion.current = false;
  });

  it("eases toward the target when reduced motion is off", async () => {
    const n = animatedNumber(0, 30);
    const done = n.set(100);
    expect(n.current).not.toBe(100);
    await done;
    expect(n.current).toBe(100);
  });

  it("snaps immediately when reduced motion is on", async () => {
    reducedMotion.current = true;
    const n = animatedNumber(0, 30);
    await n.set(100);
    expect(n.current).toBe(100);
  });
});
