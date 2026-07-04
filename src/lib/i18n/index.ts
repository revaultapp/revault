import { en } from "./locales/en";
import { es } from "./locales/es";
import { fr } from "./locales/fr";
import type { Dictionary } from "./locales/en";

export type Locale = "en" | "es" | "fr";
export type { Dictionary };

export const dictionaries: Record<Locale, Dictionary> = { en, es, fr };
