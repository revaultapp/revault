import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

describe("nav store", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("defaults to the dashboard and compress tool", async () => {
    const { activePage, activeTool } = await import("./nav");

    expect(get(activePage)).toBe("dashboard");
    expect(get(activeTool)).toBe("compress");
  });

  it("updates active page and tool", async () => {
    const { activePage, activeTool } = await import("./nav");

    activePage.set("privacy");
    activeTool.set("resize");

    expect(get(activePage)).toBe("privacy");
    expect(get(activeTool)).toBe("resize");
  });
});
