# ReVault

Ultra-lightweight desktop app for image compression, conversion, analysis, and organization.

**Open source. Offline-first. Cross-platform.**

> **Status:** Early development. Not yet ready for general use.

## Features (planned)

- **Compress** — JPEG, PNG, WebP with quality control and exact-size targeting
- **Convert** — HEIC/PNG/JPEG/WebP format conversion with batch processing
- **Analyze** — Scan folders to find duplicates, blurry photos, and wasted space
- **Privacy** — Strip GPS, camera info, and other metadata from images
- **Organize** — Batch rename, sort by date, custom pipelines
- **Cloud** — Optimize images in Google Photos, Drive, OneDrive, Dropbox

## Tech Stack

- **Backend:** Rust
- **Frontend:** Svelte 5 + TypeScript
- **Framework:** [Tauri v2](https://v2.tauri.app/)
- **Build:** Vite

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- [pnpm](https://pnpm.io/)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Setup

```bash
pnpm install
pnpm tauri dev
```

### Run Rust tests

```bash
cd src-tauri
cargo test
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
