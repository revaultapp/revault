import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const source = readFileSync("src/lib/components/UpdateDialog.svelte", "utf8");

describe("UpdateDialog accessibility contract", () => {
  it("is a labelled non-modal dialog with live progress", () => {
    expect(source).toMatch(/role="dialog"/);
    expect(source).toMatch(/aria-modal="false"/);
    expect(source).toMatch(/aria-labelledby=/);
    expect(source).toMatch(/role="progressbar"/);
    expect(source).toMatch(/aria-live="polite"/);
    expect(source).toMatch(/role="status"/);
  });

  it("exposes update, later, retry, and restart actions", () => {
    expect(source).toContain("updates.downloadAndInstall");
    expect(source).toContain("updates.defer");
    expect(source).toContain("updates.restart");
    expect(source).toContain('t("updates.tryAgain")');
  });

  it("offers the trusted release page when installation fails", () => {
    expect(source).toContain("openUrl");
    expect(source).toContain("https://github.com/revaultapp/revault/releases/latest");
    expect(source).toContain('t("updates.downloadManually")');
  });
});
