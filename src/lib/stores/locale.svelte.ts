import { dictionaries } from "$lib/i18n";
import type { Locale } from "$lib/i18n";

const STORAGE_KEY = "revault:locale";

function detectLocale(): Locale {
  const stored = typeof localStorage !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null;
  if (stored !== null && stored in dictionaries) return stored as Locale;

  // "pt" deliberately catches both pt-BR and pt-PT: the dictionary is written
  // in pt-BR, which Portuguese users read without friction (and Brazil is the
  // overwhelming majority of the lusophone audience).
  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  if (nav.startsWith("es")) return "es";
  if (nav.startsWith("fr")) return "fr";
  if (nav.startsWith("de")) return "de";
  if (nav.startsWith("pt")) return "pt";
  return "en";
}

// The pt dictionary is written in pt-BR — advertise the regional subtag so
// screen readers pick the Brazilian voice (WCAG 3.1.1 Language of Page).
const BCP47: Record<Locale, string> = { en: "en", es: "es", fr: "fr", de: "de", pt: "pt-BR" };

function applyDocumentLang(locale: Locale): void {
  if (typeof document !== "undefined") {
    document.documentElement.lang = BCP47[locale];
  }
}

const initialLocale = detectLocale();
let currentLocale = $state<Locale>(initialLocale);

// app.html's static lang="en" only covers the first paint of an en session;
// detectLocale() can land on es/fr/de/pt before the user ever opens Settings.
applyDocumentLang(initialLocale);

export function getLocale(): Locale {
  return currentLocale;
}

export function setLocale(next: Locale): void {
  currentLocale = next;
  applyDocumentLang(next);
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
  const raw = resolve(dictionaries[currentLocale], key) ?? resolve(dictionaries.en, key) ?? key;
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (match, name) => (name in params ? String(params[name]) : match));
}
