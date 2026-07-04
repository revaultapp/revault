import { describe, it, expect, beforeEach, vi } from "vitest";

function collectKeys(obj: unknown, prefix = ""): string[] {
  if (!obj || typeof obj !== "object") return [prefix];
  return Object.entries(obj as Record<string, unknown>).flatMap(([key, value]) =>
    collectKeys(value, prefix ? `${prefix}.${key}` : key),
  );
}

describe("locale store", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it("t() resolves a key in the current locale for en/es/fr", async () => {
    const mod = await import("./locale.svelte");
    mod.setLocale("en");
    expect(mod.t("common.outputLabel")).toBe("Output");
    mod.setLocale("es");
    expect(mod.t("common.outputLabel")).toBe("Salida");
    mod.setLocale("fr");
    expect(mod.t("common.outputLabel")).toBe("Sortie");
  });

  it("t() interpolates params", async () => {
    const mod = await import("./locale.svelte");
    mod.setLocale("en");
    expect(mod.t("common.savedTotal", { amount: "2 MB" })).toBe("Saved 2 MB");
    mod.setLocale("es");
    expect(mod.t("common.savedTotal", { amount: "2 MB" })).toBe("Ahorrado 2 MB");
    mod.setLocale("fr");
    expect(mod.t("common.savedTotal", { amount: "2 Mo" })).toBe("2 Mo économisés");
  });

  it("t() falls back to the English dictionary when the key is missing in the current locale", async () => {
    const mod = await import("./locale.svelte");
    const { dictionaries } = await import("$lib/i18n");
    mod.setLocale("es");
    delete (dictionaries.es.common as Record<string, unknown>).outputLabel;
    expect(mod.t("common.outputLabel")).toBe("Output");
  });

  it("t() returns the raw key when missing from every locale", async () => {
    const mod = await import("./locale.svelte");
    expect(mod.t("nonexistent.deeply.nested.key")).toBe("nonexistent.deeply.nested.key");
  });

  it("setLocale persists the choice to localStorage", async () => {
    const mod = await import("./locale.svelte");
    mod.setLocale("fr");
    expect(localStorage.getItem("revault:locale")).toBe("fr");
  });

  it("rehydrates from a stored locale on init", async () => {
    localStorage.setItem("revault:locale", "es");
    const mod = await import("./locale.svelte");
    expect(mod.getLocale()).toBe("es");
  });

  it("defaults to en when localStorage is empty (jsdom navigator.language is en-US)", async () => {
    const mod = await import("./locale.svelte");
    expect(mod.getLocale()).toBe("en");
  });

  it("en/es/fr dictionaries expose the exact same set of dot-path keys", async () => {
    const { dictionaries } = await import("$lib/i18n");
    const enKeys = new Set(collectKeys(dictionaries.en));
    const esKeys = new Set(collectKeys(dictionaries.es));
    const frKeys = new Set(collectKeys(dictionaries.fr));
    expect(esKeys).toEqual(enKeys);
    expect(frKeys).toEqual(enKeys);
  });
});
