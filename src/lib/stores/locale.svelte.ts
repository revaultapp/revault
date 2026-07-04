import { dictionaries } from "$lib/i18n";
import type { Locale } from "$lib/i18n";

const STORAGE_KEY = "revault:locale";

function detectLocale(): Locale {
  const stored = typeof localStorage !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null;
  if (stored === "en" || stored === "es" || stored === "fr") return stored;

  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  if (nav.startsWith("es")) return "es";
  if (nav.startsWith("fr")) return "fr";
  return "en";
}

export let locale = $state<Locale>(detectLocale());

export function setLocale(next: Locale): void {
  locale = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, next);
  }
}

function resolve(dict: unknown, key: string): string | undefined {
  const value = key
    .split(".")
    .reduce<unknown>(
      (acc, part) => (acc && typeof acc === "object" && part in acc ? (acc as Record<string, unknown>)[part] : undefined),
      dict,
    );
  return typeof value === "string" ? value : undefined;
}

export function t(key: string, params?: Record<string, string | number>): string {
  const raw = resolve(dictionaries[locale], key) ?? resolve(dictionaries.en, key) ?? key;
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (match, name) => (name in params ? String(params[name]) : match));
}
