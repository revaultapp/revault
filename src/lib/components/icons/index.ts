// Custom "duotone vault" icon set for the sidebar only — the rest of the
// app keeps using lucide-svelte. Every icon here follows the same duo-fill
// mechanism: a primary stroke layer (currentColor, always visible) plus a
// secondary "duo" layer whose fill reads `var(--icon-duo, transparent)`.
// The consuming component (Sidebar.svelte) drives the reveal by setting
// `--icon-duo` on ancestor elements — e.g. `.nav-item:hover`/`.active` for
// a low/high-alpha accent tint, or a permanent tint on a static badge.
// Hard rule (learned in the v1 redesign): every glyph must read COMPLETE at
// rest — the duo fill is a pure tint on shapes that are already part of the
// stroked drawing, never a load-bearing element that only appears on
// activation (a compass without its needle is an empty ring). The only
// stroke-less duo fills are ones whose outline the primary layer already
// draws (the PDF dog-ear, the shield's evenodd-punched keyhole plate).
export { default as DashboardIcon } from "./DashboardIcon.svelte";
export { default as OptimizeIcon } from "./OptimizeIcon.svelte";
export { default as DuplicatesIcon } from "./DuplicatesIcon.svelte";
export { default as PrivacyIcon } from "./PrivacyIcon.svelte";
export { default as VideoIcon } from "./VideoIcon.svelte";
export { default as PdfIcon } from "./PdfIcon.svelte";
export { default as SavedIcon } from "./SavedIcon.svelte";
export { default as SettingsIcon } from "./SettingsIcon.svelte";
export { default as AppearanceLightIcon } from "./AppearanceLightIcon.svelte";
export { default as AppearanceDarkIcon } from "./AppearanceDarkIcon.svelte";
export { default as AppearanceSystemIcon } from "./AppearanceSystemIcon.svelte";
export { default as LanguageIcon } from "./LanguageIcon.svelte";
export { default as OutputFolderIcon } from "./OutputFolderIcon.svelte";
export { default as ResetIcon } from "./ResetIcon.svelte";
export { default as ImageDefaultsIcon } from "./ImageDefaultsIcon.svelte";
export { default as VideoDefaultsIcon } from "./VideoDefaultsIcon.svelte";
