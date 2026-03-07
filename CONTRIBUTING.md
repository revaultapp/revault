# Contributing to ReVault

Thank you for your interest in contributing to ReVault!

## Development Setup

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- [pnpm](https://pnpm.io/) (v9+)
- Platform-specific Tauri dependencies: [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/revault.git
cd revault

# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Run Rust tests
cd src-tauri && cargo test
```

## Project Structure

- `src/` — Frontend (Svelte 5 + TypeScript)
- `src-tauri/src/commands/` — Tauri IPC commands (thin layer)
- `src-tauri/src/core/` — Core Rust logic (framework-independent, testable)

## Guidelines

- All code, comments, commits, and docs in **English**
- Rust code: `cargo fmt` + `cargo clippy`
- Frontend code: TypeScript strict mode
- Test your changes on macOS/Windows/Linux when possible
