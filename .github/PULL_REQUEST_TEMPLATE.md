## Type of Change

- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to change)
- [ ] Refactoring (code cleanup with no functional change)
- [ ] Documentation
- [ ] Chore (dependency updates, config, CI/CD)

## Description

<!-- Explain what changed and why. Be specific about the "why" — it helps reviewers understand intent. -->

## Security & Architecture Checklist

- [ ] **No new `unwrap()`/`expect()` outside tests** — Production code uses `?`, `match`, or `.unwrap_or_default()`. If added, include `// SAFETY:` comment justifying the panic.
- [ ] **Filesystem I/O validated via `core::paths`** — All new filesystem operations pass through `core/paths` validation before touching the system.
- [ ] **No secrets in diff** — No API keys, credentials, private keys, or sensitive data committed.
- [ ] **`core/` has zero `use tauri::` imports** — Business logic in `core/` remains pure Rust, never tied to Tauri.
- [ ] **New dependencies reviewed for license** — If adding a crate, license is compatible (MIT, Apache 2.0, BSD, etc.) and cross-platform. Run `cargo license` to verify.
- [ ] **`unsafe` blocks justified** — Every `unsafe` has a `// SAFETY:` comment explaining why it's sound.

## Testing

- [ ] Tests added or updated for new logic (unit tests in `src-tauri/src/core/**/*.rs` or Vitest in `src/lib/stores/**/*.test.ts`)
- [ ] **OR** N/A — no testable logic added (e.g., pure documentation, config-only changes)
- [ ] `cargo nextest run --workspace` passes (Rust, 195+ tests)
- [ ] `pnpm test` passes (Frontend, 142+ tests)

## Platform Testing

- [ ] Linux ✓ (CI ubuntu-22.04)
- [ ] macOS ✓ (CI macos-latest)
- [ ] Windows ✓ (CI windows-latest)

---

**No CI required:** If this is a doc-only or config-only change, you may check "N/A" on tests above. All other changes must pass the full test suite.
