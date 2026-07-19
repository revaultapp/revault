# Dashboard Monthly Savings Responsive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Dashboard's Monthly savings card immediately understandable, keyboard-efficient, and responsive across ReVault's supported desktop window sizes without changing the other two analytics cards.

**Architecture:** Keep `DashboardPage.svelte` as the data orchestrator and `MonthlyBars.svelte` as the presentation component. Add one pure roving-index helper to `charts.ts`, pass the existing month-over-month delta into `MonthlyBars`, and let component container queries handle visual recomposition.

**Tech Stack:** Svelte 5 runes, TypeScript, Vitest, component-scoped CSS, existing ReVault design tokens.

---

### Task 1: Keyboard navigation contract

**Files:**
- Create: `src/lib/charts.test.ts`
- Modify: `src/lib/charts.ts`

- [ ] **Step 1: Write the failing navigation tests**

Add focused tests for a pure `nextChartIndex(current, key, count)` helper: ArrowRight increments and wraps, ArrowLeft decrements and wraps, Home selects `0`, End selects `count - 1`, and unsupported keys return `null`.

- [ ] **Step 2: Run the focused test and confirm RED**

Run: `pnpm vitest run src/lib/charts.test.ts`

Expected: FAIL because `nextChartIndex` is not exported.

- [ ] **Step 3: Implement the minimal pure helper**

Add this public contract to `src/lib/charts.ts`:

```ts
export function nextChartIndex(current: number, key: string, count: number): number | null
```

Return `null` for an empty series or unsupported key; otherwise clamp the current index and apply wrapping Arrow navigation plus Home/End behavior.

- [ ] **Step 4: Run the focused test and confirm GREEN**

Run: `pnpm vitest run src/lib/charts.test.ts`

Expected: the new chart tests pass with no warnings.

### Task 2: Monthly card behavior and responsive presentation

**Files:**
- Create: `src/lib/components/MonthlyBars.test.ts`
- Modify: `src/lib/components/MonthlyBars.svelte`
- Modify: `src/lib/components/DashboardPage.svelte`

- [ ] **Step 1: Write failing component contract tests**

Mount `MonthlyBars` with twelve deterministic month points and assert:

```ts
expect(monthButtons).toHaveLength(12);
expect(monthButtons.filter((button) => button.tabIndex === 0)).toHaveLength(1);
```

Dispatch `ArrowLeft` from the current-month control and assert the previous month becomes selected, its summary value is visible, and the current-month comparison is not. Also assert the component source contains a named inline-size container and the `<480px` / `<400px` container-query contracts, and does not render `ChartTooltip`.

- [ ] **Step 2: Run the component test and confirm RED**

Run: `pnpm vitest run src/lib/components/MonthlyBars.test.ts`

Expected: FAIL because the summary, roving tabindex, container-query CSS, and no-tooltip contract do not yet exist.

- [ ] **Step 3: Implement the monthly presentation**

In `DashboardPage.svelte`, pass the existing `$momDeltas.saved` and localized `dashboard.vsPrevMonth` suffix into `MonthlyBars`. Increase only the Monthly savings card toggle's hit target to `36 × 36px` using a monthly-card-specific class; leave the category and last-scan toggles unchanged.

In `MonthlyBars.svelte`:

- accept the delta and suffix props;
- render a stable active-month summary above the plot;
- remove the permanent `ChartTooltip` import and instance;
- use `nextChartIndex` for Arrow Left/Right/Home/End navigation;
- give only the active month control `tabindex="0"`, with all others at `-1`;
- preserve all twelve controls and the screen-reader table;
- replace JavaScript width-driven label hiding with a named inline-size container and component-scoped container queries at `480px` and `400px`;
- use existing color, radius, duration, and typography tokens only.

- [ ] **Step 4: Run focused tests and confirm GREEN**

Run: `pnpm vitest run src/lib/charts.test.ts src/lib/components/MonthlyBars.test.ts`

Expected: both test files pass with no warnings.

- [ ] **Step 5: Refactor while green**

Remove unused plot/tooltip width state and imports, keep derived state names explicit, and ensure a pointer preview restores the user's keyboard/click selection on pointer leave without changing the single roving tab stop.

### Task 3: Verification and delivery

**Files:**
- Modify if test counts change: `AGENTS.md`

- [ ] **Step 1: Run frontend quality gates**

Run:

```bash
pnpm test
pnpm check
pnpm build
```

Expected: all commands exit `0`, with the frontend test count increased by the exact number added in Tasks 1–2.

- [ ] **Step 2: Review the diff against the scope**

Confirm `CategoryLines.svelte`, `StorageDonut.svelte`, their Dashboard data plumbing, Rust files, and unrelated local files are unchanged.

- [ ] **Step 3: Update the testing baseline if needed**

If tests were added, update only the frontend and total counts in `AGENTS.md` using the fresh test output. Do not alter the Rust count without running Rust tests.

- [ ] **Step 4: Commit the isolated package**

Stage only the design/spec, plan, Monthly savings implementation, tests, and the count-only `AGENTS.md` update. Commit with:

```bash
git commit -m "feat(dashboard): refine monthly savings chart"
```

- [ ] **Step 5: Push and open a pull request**

Push `codex/dashboard-monthly-responsive` and open a ready PR against `main` summarizing the visual behavior, responsive contract, accessibility improvement, excluded cards, and verification evidence.
