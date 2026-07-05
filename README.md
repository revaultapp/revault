# Revault

Ultra-lightweight desktop app for image compression, conversion, analysis, and organization.

**Open source. Offline-first. Cross-platform.**

> **Status:** Shipped and actively maintained. 195 Rust unit tests + 139 frontend tests.

## Features

- **Compress** — JPEG (mozjpeg), PNG (oxipng), WebP, AVIF with quality control and exact-size targeting
- **Convert** — HEIC (native decode), PNG, JPEG, WebP format conversion with batch processing
- **Resize** — Batch image resize with anti-upscaling safeguards
- **Duplicates** — Find exact duplicates (SHA256) or perceptually similar images (pHash) in folders
- **Privacy** — Strip EXIF, GPS, camera info, and metadata from images
- **Video** — Compress video with CRF presets, privacy modes, MOV→MP4 remux
- **GIF Export** — Create animated GIFs from video clips via gifski
- **PDF Tools** — Strip metadata and compress streams in PDF documents

## Installing

Download the latest release for your platform below. Since these builds are not yet signed, you may see security warnings on first launch — these are safe to bypass:

- **macOS:** On first launch, macOS will block the app (Gatekeeper). Go to **System Settings → Privacy & Security** and click the **Open Anyway** button next to ReVault. Alternatively, remove the quarantine flag in Terminal:
  ```bash
  xattr -dr com.apple.quarantine /Applications/ReVault.app
  ```

- **Windows:** When SmartScreen appears, click **More info** → **Run anyway**.

Signed and notarized builds are planned for a future release.

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
