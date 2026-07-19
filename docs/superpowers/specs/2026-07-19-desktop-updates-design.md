# ReVault Desktop Updates Design

**Date:** 2026-07-19
**Status:** Approved for implementation; production activation is gated on release secrets and signed release verification.

## Goal

Let installed desktop copies of ReVault discover a newer trusted release when the app starts with network access, offer a clear one-click update path, and retain a manual update control in Settings without interrupting active work.

## User experience

### Startup discovery

- On desktop startup, check the Tauri updater endpoint in the background after the shell is interactive.
- No network indicator, modal, or error is shown when offline, the endpoint is unavailable, or the installed version is current.
- If a newer release exists and the user has not postponed the same version in the last 24 hours, show a non-blocking application dialog with version, release notes, **Update now**, and **Later**.
- **Later** stores the available version and timestamp locally. The same version may be offered again after 24 hours; a newer version resets that delay.
- A pending update is never surfaced while ReVault is processing files. If discovery finishes during processing, defer the dialog until all known processing stores are idle.

### Installation

- **Update now** downloads the platform package with visible byte progress and verifies its updater signature.
- Once the verified package is ready, show **Restart to update**. That explicit, idle-guarded action installs the package and relaunches ReVault; on Windows the installer closes the app as part of installation.
- Download or installation failures remain recoverable: show the correct stage-specific error and retain **Try again** plus the release download fallback. A relaunch failure retries only the relaunch.

### Settings

The existing About/version row becomes an update status row:

- Current: `Version 0.1.0` with a secondary **Check for updates** action.
- Update found: `Version 0.2.0 available` with **Update now**.
- Checking/installing: show a compact status. While downloading, also show byte progress; do not permit duplicate checks or downloads.
- Up to date: announce a quiet confirmation only after a manual check.

## Architecture

- `tauri-plugin-updater` is registered in Rust and `@tauri-apps/plugin-updater` is used only through a focused frontend update store.
- The store owns updater state, once-per-launch startup checks, 24-hour local deferral, progress, retry and install/relaunch handoff. It exposes derived “safe to prompt” state; page components do not call updater APIs directly.
- A shell-level `UpdateDialog` renders the startup offer so it is independent of the active page. `SettingsPage` renders the persistent manual control.
- The updater endpoint is a static `latest.json` published with each GitHub Release. Tauri verifies the signed platform artifact before installation.
- The release workflow creates signed updater artifacts and publishes `latest.json` only as part of a fully assembled release.

## Security and release prerequisites

- Tauri updater signing has a dedicated keypair. The public key is embedded in `tauri.conf.json`; the private key and optional password live only in protected GitHub secrets and an offline owner backup.
- System distribution trust remains separate: macOS releases require Developer ID signing and notarization; Windows releases require code signing. These credentials are owner-managed and are not generated, printed, or committed by this branch.
- The feature must not be enabled for production until all updater artifacts have been signed and a release has passed install/update smoke tests on macOS, Windows, and Linux.

## Non-goals

- No silent background download.
- No automatic restart.
- No update check while a file-processing job is active.
- No telemetry, accounts, cloud service, or dynamic update backend in this first version.
- No downgrade channel. Recovery is a higher-version hotfix.
