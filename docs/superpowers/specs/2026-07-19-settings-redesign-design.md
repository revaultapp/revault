# ReVault Settings Redesign

**Date:** 2026-07-19
**Status:** Approved design — ready for implementation planning

## Goal

Redesign the Settings page as a calm, high-precision desktop control room. It must make preferences easier to scan, preserve every existing setting and behavior, and make the page feel native to ReVault rather than a collection of generic form controls.

The approved direction is **A: Control room sereno** with **direct custom Vault glyphs**. This is intentionally restrained: hierarchy, spacing, surfaces and distinctive iconography carry the quality; gradients, glow, ornamental chips and generic UI-library iconography do not.

## Current constraints

- ReVault is a desktop Svelte 5 application using component-scoped CSS and tokens from `src/app.css`; no Tailwind.
- The page must retain its existing persisted settings, keyboard interaction, live region announcements, output-directory picker and reset-focus handling.
- The sidebar's custom icon family is the source pattern. Settings uses a separate, Settings-only family so the sidebar barrel remains scoped to navigation; both families use a complete primary outline plus a non-essential `--icon-duo` fill layer revealed by interaction.
- Plus Jakarta Sans and the existing dark/light theme tokens remain unchanged.
- The Settings page is localized in EN, ES, FR, DE and PT-BR. New visible copy requires parity across every locale and must remain concise.

## Information architecture and layout

The page changes from a single column of visually identical rows into three purposeful zones in a wider, balanced content area. The content container expands from 720px to 920px maximum width.

1. **Workspace**
   - Appearance and language are compact peer cards in a two-column grid from 760px content width upward; below that threshold they stack.
   - Default output spans the grid width below them because folder selection is a location decision, not a quick toggle.
   - Each item has a direct custom glyph, label, one-line contextual description and its existing control.

2. **Processing defaults**
   - Image quality, video quality and video privacy become three distinct setting tiles inside one grouped surface.
   - Each tile shows a custom glyph, a title, a concise explanation and its segmented control directly beneath the heading. This keeps every choice aligned and prevents the longer privacy control from competing with the label column.
   - “Remember last use” remains a first-class option, not an implied fallback.

3. **About ReVault**
   - Privacy remains explicit but becomes a quiet proof point rather than the dominant green object on the page.
   - Version remains a compact metadata row beneath it.

Below 760px, the Workspace grid stacks and every segmented control may wrap to a second line without horizontal page scrolling. The global sidebar remains the only navigation: Settings does not introduce a second internal navigation.

## Visual language

- **Surfaces:** one restrained grouped surface per conceptual area, with a lightly raised internal tile treatment for processing defaults. Borders, not shadows or gradients, establish grouping.
- **Spacing:** generous but purposeful. Group titles introduce zones; controls never float in empty space without a local label and explanation.
- **Accent:** ReVault green indicates active choice, keyboard focus and icon duo-fill interaction. It is not used as a permanent decorative wash.
- **Motion:** existing fast/normal duration and easing tokens only. Duo-fill and surface/border transitions stay subtle and honor reduced-motion preferences.
- **Typography:** retain existing typeface and tokens. Section titles remain concise; descriptions explain consequences instead of repeating labels.

## Custom Settings icon system

Settings will use bespoke Svelte SVG components, not `lucide-svelte` or any other icon library. The components live with the existing custom icon set and follow its documented contract:

- A primary rounded `1.8` stroke silhouette uses `currentColor` and reads completely with no fill.
- A secondary shape uses `var(--icon-duo, transparent)`. It enhances an already-legible element and never reveals essential meaning.
- The SVG viewBox, prop API (`size`, `strokeWidth`, `class`) and `aria-hidden="true"` match the sidebar icons.
- All icons use semantic nearby text and therefore are decorative to assistive technology; icon-only reset affordances retain an accessible label.

The initial family contains:

| Purpose | Custom glyph |
| --- | --- |
| Light appearance | Radiating aperture/sun |
| Dark appearance | Crescent aperture/moon |
| System appearance | Desktop display with adaptive aperture |
| Language | Two-script/translation mark |
| Output directory | ReVault folder with a filled interior plane |
| Reset output directory | Circular restore arrow |
| Image defaults | Image frame with terrain/compression plane |
| Video defaults | Rounded media frame with play aperture |
| Video privacy | Existing custom shield/keyhole glyph, reused where semantics match |
| Privacy statement | Existing custom shield/keyhole glyph, reused rather than creating a competing symbol |

The direct-glyph treatment is approved: no permanently visible square or circular icon chips. A glyph sits next to its setting label. Its duo layer receives a subtle accent tint only when the setting row/control is focused, hovered or has an active state where that state is meaningful.

## Component and state boundaries

The redesign is presentational around the existing state contract.

- `SettingsPage.svelte` continues to own settings orchestration, locale-derived segment arrays, output-directory operations and screen-reader announcements.
- A small local presentation component may be introduced only if it removes repeated markup for a labeled settings tile without absorbing store behavior.
- New custom icons are isolated components exported from `src/lib/components/settings-icons/index.ts`; the existing sidebar barrel remains sidebar-only.
- `SegmentedControl` retains its existing behavioral contract. Any styling additions must be opt-in or scoped so other tools do not regress.
- Existing stores (`theme`, locale, output directory and default presets) remain unchanged; no storage migration is needed.

## Interaction and accessibility requirements

- Every segmented control keeps its accessible label and keyboard behavior.
- Changing a setting continues to announce `label: value` through the existing polite live region.
- Output-directory reset still returns focus to the picker after the reset button unmounts.
- No meaning depends exclusively on color, icon fill or hover state.
- Text and controls meet existing dark and light mode contrast requirements. Focus remains visible on cards, picker, reset and all segmented options.
- The layout remains usable at the app's supported narrow desktop size with no clipped labels or horizontal page scrolling.

## Verification strategy

- Preserve and update frontend tests covering settings store propagation, persisted defaults and locale parity when copy or component contracts change.
- Run `pnpm check` and `pnpm test` after implementation.
- Perform a focused manual pass in dark, light and system theme for all five locales, including output-directory selection/reset and keyboard-only segmented-control navigation.
- Verify the custom icons at their actual rendered size in the sidebar-adjacent visual context; they must remain recognizable before duo-fill activates.

## Non-goals

- No new preference categories, settings persistence format or backend/Tauri command changes.
- No internal Settings sub-navigation.
- No gradients, glassmorphism, permanent neon glow, emoji or third-party icon library usage in Settings.
- No redesign of the global sidebar or existing feature pages as part of this scope.
