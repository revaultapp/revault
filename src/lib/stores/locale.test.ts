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

  it("every dictionary exposes the exact same set of dot-path keys as en", async () => {
    const { dictionaries } = await import("$lib/i18n");
    const enKeys = new Set(collectKeys(dictionaries.en));
    for (const locale of Object.keys(dictionaries) as (keyof typeof dictionaries)[]) {
      if (locale === "en") continue;
      expect(new Set(collectKeys(dictionaries[locale])), `locale ${locale}`).toEqual(enKeys);
    }
  });

  it("every locale explains the Settings choices that need context", async () => {
    const { dictionaries } = await import("$lib/i18n");
    const contextualKeys = [
      "languageDesc",
      "defaultImagePresetDesc",
      "defaultVideoPresetDesc",
      "defaultVideoPrivacyDesc",
    ];

    for (const [locale, dictionary] of Object.entries(dictionaries)) {
      const settings = dictionary.settings as Record<string, string>;
      for (const key of contextualKeys) {
        expect(settings[key], `${locale}.settings.${key}`).toBeTypeOf("string");
        expect(settings[key].trim(), `${locale}.settings.${key}`).not.toBe("");
      }
    }
  });

  it("setLocale keeps document.documentElement.lang in sync, mapping pt to pt-BR", async () => {
    document.documentElement.lang = "";
    const mod = await import("./locale.svelte");
    mod.setLocale("pt");
    expect(document.documentElement.lang).toBe("pt-BR");
    mod.setLocale("es");
    expect(document.documentElement.lang).toBe("es");
  });

  it("module init applies the stored locale to document.documentElement.lang", async () => {
    document.documentElement.lang = "";
    localStorage.setItem("revault:locale", "de");
    await import("./locale.svelte");
    expect(document.documentElement.lang).toBe("de");
  });

  it("falls back through navigator.language to en when the stored locale is unknown", async () => {
    // Stored "zz" is not a registered locale — the full chain is
    // stored → navigator.language → en, so with jsdom's en-US it lands on en...
    localStorage.setItem("revault:locale", "zz");
    const mod = await import("./locale.svelte");
    expect(mod.getLocale()).toBe("en");

    // ...and with a French navigator it lands on fr, proving the middle hop.
    localStorage.clear();
    vi.resetModules();
    localStorage.setItem("revault:locale", "zz");
    vi.stubGlobal("navigator", { ...navigator, language: "fr-FR" });
    const mod2 = await import("./locale.svelte");
    expect(mod2.getLocale()).toBe("fr");
    vi.unstubAllGlobals();
  });

  it("detects de and pt (incl. pt-BR) from navigator.language", async () => {
    for (const [lang, expected] of [
      ["de-DE", "de"],
      ["pt-BR", "pt"],
      ["pt-PT", "pt"],
    ] as const) {
      localStorage.clear();
      vi.resetModules();
      vi.stubGlobal("navigator", { ...navigator, language: lang });
      const mod = await import("./locale.svelte");
      expect(mod.getLocale(), `navigator.language ${lang}`).toBe(expected);
      vi.unstubAllGlobals();
    }
  });

  it("t() resolves keys in the new de and pt locales", async () => {
    const mod = await import("./locale.svelte");
    mod.setLocale("de");
    expect(mod.t("settings.languageGerman")).toBe("Deutsch");
    expect(mod.t("common.outputLabel")).not.toBe("common.outputLabel");
    mod.setLocale("pt");
    expect(mod.t("settings.languagePortuguese")).toBe("Português (Brasil)");
    expect(mod.t("common.outputLabel")).not.toBe("common.outputLabel");
  });
});
