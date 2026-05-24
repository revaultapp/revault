import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

describe("activity store", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
  });

  it("clamps negative savedBytes when adding activity", async () => {
    const { activity } = await import("./activity");

    activity.add({ type: "convert", fileCount: 1, savedBytes: -10 });

    expect(get(activity)[0].savedBytes).toBe(0);
  });

  it("clamps persisted negative savedBytes on load", async () => {
    localStorage.setItem(
      "revault_activity",
      JSON.stringify([
        {
          id: "old",
          type: "convert",
          fileCount: 1,
          savedBytes: -10,
          timestamp: 1,
        },
      ]),
    );

    const { activity } = await import("./activity");

    expect(get(activity)[0].savedBytes).toBe(0);
  });
});
