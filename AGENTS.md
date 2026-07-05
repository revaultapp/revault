# ReVault Agent Guide

## Product

ReVault is an offline-first desktop app for image and video optimization: compression, conversion, resize, duplicate analysis, privacy metadata stripping, video compression, and GIF export.

Core positioning: open source, local-only processing, cross-platform desktop, no server dependency.

## Stack

- Tauri v2 desktop app.
- Frontend: Svelte 5 runes, TypeScript, SvelteKit SPA/static adapter, Vite 6.
- Backend: Rust. Business logic belongs in Rust, not TypeScript.
- Package manager: `pnpm` only.
- Font: Plus Jakarta Sans.
- Icons: `lucide-svelte`.

## Commands

```bash
pnpm dev
pnpm tauri dev
pnpm build
pnpm check
pnpm test

cd src-tauri && cargo test
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo clippy --workspace -- -D warnings
cd src-tauri && cargo audit
```

For local review, also run the stricter command when touching Rust tests:

```bash
cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings
```

## Architecture Rules

- `src-tauri/src/core/`: pure Rust business logic. No `tauri::` imports.
- `src-tauri/src/commands/`: thin Tauri IPC wrappers only. Validate/receive args, call `core`, return results.
- Frontend stores/components handle UI state and orchestration only.
- File path validation must go through `core::paths` before filesystem operations.
- Output paths must avoid clobbering existing files.

## Current Modules

Rust core modules:

- `compression`: JPEG/PNG/WebP/AVIF image optimization and output path resolution (JXL is decode-only, used for thumbnails — no JXL encoding).
- `dedupe`: SHA256 exact duplicate detection and pHash similar mode.
- `gif`: GIF export via gifski sidecar.
- `heic`: native HEIC decode helpers.
- `image_io`: format detection, thumbnails, dimensions.
- `paths`: shared input path and output suffix validation.
- `pdf`: PDF metadata stripping, stream compression, and merge/split.
- `privacy`: EXIF/GPS/metadata reading and stripping.
- `resize`: image resize engine.
- `scanner`: folder image scanning.
- `video`: FFmpeg sidecar video compression, trim, privacy modes, preview/size estimation.

Tauri command modules mirror the app features: `compress`, `convert`, `dedupe`, `delete`, `gif`, `pdf`, `privacy`, `resize`, `scanner`, `thumbnail`, `video`.

## Frontend Structure

- `src/routes/+layout.svelte`: shell with sidebar/topbar/content.
- `src/routes/+page.svelte`: manual page switch based on `activePage`.
- `src/lib/components/`: page and shared UI components.
- `src/lib/stores/`: shared state and store tests.
- Routing is manual via `src/lib/stores/nav.ts`, not SvelteKit file routes.
- Use Svelte 5 runes in components. Avoid legacy reactive patterns in new code.
- Use component-scoped CSS and tokens from `src/app.css`; no Tailwind.

Current sidebar pages:

- Dashboard
- Optimize
- Duplicates
- Privacy
- Video
- PDF
- Settings

## Feature Status

- Image compression: mature. JPEG via mozjpeg, PNG via oxipng, WebP, AVIF, screenshots warnings, output-folder support.
- Convert/resize: implemented, with default output folder fallback and anti-upscale UX for resize.
- Duplicate analysis: exact SHA mode plus Similar pHash mode for near duplicates.
- Privacy: image metadata scan/strip UI, selective GPS/device/date/author stripping.
- Video: FFmpeg sidecar, presets, cancellation, size prediction, privacy modes `off | smart | gps_only | full`, trim to start/end range (lossless stream-copy).
- GIF export: implemented via gifski sidecar from the Video flow.
- PDF Tools: metadata stripping, stream compression with embedded image re-encoding, and merge/split (combine PDFs, extract page ranges).
- Dashboard: implemented with savings, storage analysis, quick actions, and recent activity.
- Removed/deferred scope: organize/rename, collage, watermark, cloud, OCR.

## Design Tokens

Source of truth is `src/app.css`.

- Accent: `--accent: #10D87A`.
- Light background: `--bg-main: #f0f3f6`.
- Dark background: `--bg-main: #060808`.
- Chrome/sidebar background: `--chrome-bg: #0c0f0e`.
- Radius tokens: `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`.
- Animation tokens: `--duration-fast`, `--duration-normal`, `--duration-slow`, `--ease-out`, `--ease-spring`.

## Testing Baseline

Current test suite (verified 2026-07-05):

- Rust: 191 unit tests via `cd src-tauri && cargo test`.
- Frontend: 133 Vitest tests via `pnpm test`.
- Total: 324 passing tests.

Always verify counts after changing tests by running the commands above; this section should be updated when tests are added or removed.

## Git Notes

- Default branch: `main`.
- Commit style: conventional commits, for example `fix(security): ...`, `feat(video): ...`, `docs: ...`.
- Do not commit stale research docs as active product truth. If a research/spec doc is useful but old, clearly label it historical before committing.
- `AGENTS.md` is the maintained agent guide. Do not recreate a duplicate `CLAUDE.md` in this repo.

## Useful Docs

- `README.md`: public overview.
- `CONTRIBUTING.md`: contribution guidelines.
- `NEXT_STEPS.md`: current working backlog (if present).
