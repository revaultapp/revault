# Dashboard Monthly Savings Responsive Design

## Scope

This package changes only the Dashboard's **Monthly savings** card. It must not redesign or alter the behavior, data model, or visual presentation of **Savings by file type** or **Last scan**.

## User outcome

At a glance, a user can understand how much space the active month recovered, whether that changed from the previous month, and how the last twelve months compare. The card remains readable from ReVault's default `1250 × 820` window down to its supported minimum `900 × 600` window without horizontal scrolling.

## Data semantics

- The chart always receives the existing twelve-entry `monthlySeries`, ordered oldest to newest.
- The summary value is the active month's `total`, initially the current month.
- The comparison is the existing `momDeltas.saved` value and is shown only while the current month is active and a valid previous-month baseline exists.
- Hovering or keyboard-selecting another month updates the visible month/value summary. It does not display the current-month comparison for a historical month.
- Zero-value months retain their position. No month is removed to make the chart appear denser.
- The empty state remains the existing localized Dashboard empty state and CTA.

## Visual hierarchy

- The existing card title and chart/table toggle remain in the card header.
- A compact summary row appears above the plot: active month and year as context, active value as the primary number, and the month-over-month comparison as supporting text when applicable.
- The current month keeps the Dashboard's approved `--chart-hero-a/b` gradient. Other bars retain the existing neutral hatch treatment.
- The chart keeps three quiet grid lines, tabular numbers, and localized month labels.
- The permanent floating tooltip is removed. The stable summary row carries the selected value; no information should float over the plot at rest.

## Responsive behavior

- Responsiveness is driven by the Monthly savings component's container width, not the global viewport.
- Twelve bars are always rendered and must fit without horizontal scrolling.
- At a container width below `480px`, alternate month labels are visually hidden while all twelve bars and the accessible data table remain present.
- At a container width below `400px`, the summary comparison moves below the primary value instead of reducing typography.
- The Monthly card uses size containment at the grid-item boundary so the twelve-row table scrolls inside the card instead of contributing intrinsic height to the Dashboard row. The card continues to stretch to the grid track; no fixed card height is introduced.
- The Dashboard's existing overall grid is otherwise unchanged in this package.

## Interaction and accessibility

- The twelve month hit regions use roving tabindex: one tab stop enters the chart, then Arrow Left/Right, Home, and End select months.
- Pointer hover and keyboard selection both update the same active month summary.
- Pointer hover is a temporary preview. Leaving the plot restores the user's current keyboard/click selection without changing the single roving tab stop.
- Each month control includes localized month/year and the exact formatted value in its accessible name.
- The chart retains its `role="img"` summary and screen-reader table; visible table mode remains available.
- The card's table toggle receives a minimum `36 × 36px` target and visible focus treatment as part of this card only.
- Reduced-motion behavior remains governed by existing global and motion tokens; no new decorative animation is added.

## Testing

- Pure chart navigation logic is unit-tested for Arrow Left/Right wrapping, Home, End, and ignored keys.
- Component contract tests verify twelve month controls, a single initial tab stop, keyboard selection, active summary updates, no permanent tooltip, and the responsive container-query rules.
- Existing history-store tests continue to prove the twelve-month data contract.
- Required gates: focused tests, `pnpm test`, `pnpm check`, and `pnpm build`.

## Out of scope

- Category chart redesign.
- Last-scan donut redesign.
- Dashboard-wide header, KPI, row-B, or navigation changes.
- New persistence or Rust/Tauri changes.
