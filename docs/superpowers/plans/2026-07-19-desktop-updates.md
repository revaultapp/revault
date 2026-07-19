# Desktop Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add signed Tauri updater plumbing, startup update offers, a 24-hour deferral policy, and a manual Settings update control.

**Architecture:** A single update store owns all Tauri updater interactions and local deferral state. The shell renders a global update dialog; Settings consumes the store without duplicating logic. Release configuration publishes signed per-platform artifacts through GitHub Releases.

**Tech Stack:** Tauri v2 updater and process plugins, Svelte 5, TypeScript, Vitest, GitHub Actions.

---

### Task 1: Define and test update state

**Files:**
- Create: `src/lib/stores/updates.ts`
- Create: `src/lib/stores/updates.test.ts`

- [ ] Write failing tests for 24-hour deferral, a newer-version reset, idle-only prompting, and state transitions.
- [ ] Implement a focused store with explicit update status, pending version, progress, manual check, download and install/restart actions.
- [ ] Run `pnpm vitest run src/lib/stores/updates.test.ts` red then green.

### Task 2: Add updater dependencies and signed release configuration

**Files:**
- Modify: `package.json`, `pnpm-lock.yaml`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `.github/workflows/release.yml`

- [ ] Add and initialise official Tauri updater/process plugins with least-privilege permissions.
- [ ] Configure signed updater artifacts and static GitHub Releases endpoint; document required GitHub secrets without committing private material.
- [ ] Add release generation/upload of `latest.json` after every platform artifact and signature exist.
- [ ] Run focused frontend and Rust checks that prove configuration compiles.

### Task 3: Render update UX

**Files:**
- Create: `src/lib/components/UpdateDialog.svelte`
- Modify: `src/routes/+layout.svelte`, `src/lib/components/SettingsPage.svelte`, all locale dictionaries
- Test: component/store tests

- [ ] Write failing tests for manual update action labels, deferred dialog behavior, and localized copy parity.
- [ ] Add shell dialog with explicit update/later/restart/retry states.
- [ ] Replace the About version metadata row with the status and manual check action.
- [ ] Run focused tests then `pnpm check`.

### Task 4: Review and delivery

**Files:**
- Modify: `AGENTS.md` only if frontend test count changes

- [ ] Run `pnpm test --run`, `pnpm check`, `pnpm build`, Rust tests, formatting and strict clippy.
- [ ] Review updater configuration and release workflow against the approved design and official Tauri security requirements.
- [ ] Commit the feature on `codex/update-experience`, push it, and create a PR to `main` with release-secret activation steps called out explicitly.
