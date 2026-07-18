import { en } from "./locales/en";
import { es } from "./locales/es";
import { fr } from "./locales/fr";
import { de } from "./locales/de";
import { pt } from "./locales/pt";
import type { Dictionary } from "./locales/en";

export type Locale = "en" | "es" | "fr" | "de" | "pt";
export type { Dictionary };

export const dictionaries: Record<Locale, Dictionary> = { en, es, fr, de, pt };
