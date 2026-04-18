# Third-Party Licenses

ReVault's own source code is distributed under the **MIT License** (see `LICENSE`).

This file documents third-party components that are distributed alongside ReVault
but are governed by their own licenses. Components are listed by integration type.

---

## Sidecar Binaries

These programs are invoked by ReVault via subprocess (CLI arguments + files on disk).
They are **not** compiled into or linked with the ReVault binary. Per the
[GNU GPL FAQ on aggregation](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation),
communication via pipes, sockets, and command-line arguments between separate programs
does not create a "combined work" — each program retains its own license.

---

### gifski

| Field       | Value |
|-------------|-------|
| Version     | 1.34.0 |
| Upstream    | https://github.com/ImageOptim/gifski |
| Author      | Kornel Lesiński (ImageOptim) |
| License     | **AGPL-3.0-or-later** |
| Integration | Sidecar binary — invoked via `std::process::Command`, never linked |

#### How gifski is distributed with ReVault

gifski is **not bundled in the ReVault installer**. On the first use of the
"Export as GIF" feature, ReVault downloads the appropriate pre-built binary from
ReVault's own GitHub Releases:

```
https://github.com/revaultapp/revault/releases/download/gifski-v1.34.0/gifski-1.34.0-{target}.tar.gz
```

Where `{target}` is one of:
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-apple-darwin` (macOS Intel)
- `x86_64-unknown-linux-gnu` (Linux x86_64)
- `x86_64-pc-windows-msvc` (Windows x86_64, `.zip` instead of `.tar.gz`)

These binaries are built from source by ReVault's CI pipeline
(`.github/workflows/gifski-release.yml`) using `cargo install gifski --locked`.
The build is reproducible and triggered by pushing a `gifski-v*` tag.

#### Compliance obligations (AGPL-3.0)

The AGPL-3.0 license requires that the following be made available to users who
receive the binary:

1. **License text** — a copy of the AGPL-3.0 license (`gifski-LICENSE.txt`) is
   packaged inside every binary archive distributed by ReVault
   (e.g., `gifski-1.34.0-aarch64-apple-darwin.tar.gz`). It is stored alongside
   the binary in ReVault's local data directory after download.

2. **Source code (AGPL §6 written offer)** — the complete, corresponding source
   code for each gifski version ReVault distributes is available upstream at a
   version-specific permalink. For gifski v1.34.0:
   https://github.com/ImageOptim/gifski/tree/1.34.0
   (tarball: https://github.com/ImageOptim/gifski/archive/refs/tags/1.34.0.tar.gz)

   ReVault also offers, valid for three years from distribution of each binary,
   to provide the corresponding source on physical media at no more than the
   cost of performing the copy. Contact: revaulthq@gmail.com.

3. **No modifications** — ReVault distributes gifski unmodified. If ReVault ever
   modifies gifski source, the modified source must be published under AGPL-3.0
   and distributed alongside the modified binary.

#### Why gifski instead of FFmpeg's built-in GIF encoder

FFmpeg's native GIF output uses a fixed 256-color global palette with no adaptive
dithering, producing visible banding in gradients and files 20–58% larger than
gifski for equivalent visual quality. gifski uses per-frame palette optimization
and error-diffusion dithering. The quality difference is the justification for the
additional sidecar complexity.

#### Commercial licensing

If ReVault ever introduces a proprietary tier that requires linking gifski
statically, a commercial license is available from Kornel Lesiński:
https://kornel.ski/contact

---

### FFmpeg

| Field       | Value |
|-------------|-------|
| Version     | Auto-detected at runtime (latest stable) |
| Upstream    | https://ffmpeg.org |
| License     | **LGPL-2.1+** (standard build; some features GPL) |
| Integration | Sidecar binary — invoked via `ffmpeg-sidecar` crate, never linked |

ReVault uses a standard LGPL build of FFmpeg (no GPL components enabled).
FFmpeg is downloaded on first use via the `ffmpeg-sidecar` crate's auto-download
mechanism. LGPL-2.1 does not require distributing a license copy alongside each
binary download, but FFmpeg's full license text is available at:
https://ffmpeg.org/legal.html

---

## Rust Crate Dependencies

All Rust crates compiled into the ReVault binary are MIT or Apache-2.0 licensed,
both of which are compatible with ReVault's MIT license. A complete machine-readable
list is available via:

```bash
cd src-tauri && cargo license
```

Notable crates with non-MIT licenses (all permissive and compatible):

| Crate | License | Notes |
|-------|---------|-------|
| `mozjpeg` | zlib/libjpeg | JPEG compression |
| `oxipng` | MIT | PNG optimization |
| `ravif` | MIT/Apache-2.0 | AVIF encoding |
| `rayon` | MIT/Apache-2.0 | CPU parallelism |

---

### Archive format handling (gifski download)

When the user triggers "Export as GIF" for the first time, ReVault downloads the
gifski sidecar binary (see above) from GitHub Releases and extracts it using
pure-Rust archive libraries rather than invoking system tools (`tar`, `Expand-Archive`).

**Reason:** System tool availability is not guaranteed — PowerShell's `Expand-Archive`
is restricted in some corporate Windows environments, and `/usr/bin/tar` flags differ
between macOS and GNU tar. Using library-level extraction gives controlled error
messages and identical behaviour across all three supported platforms.

Crates used for extraction:

| Crate | Version | License | Role |
|-------|---------|---------|------|
| `flate2` | 1.x | MIT OR Apache-2.0 | gzip decompression (`.tar.gz`) |
| `tar` | 0.4 | MIT OR Apache-2.0 | `.tar` entry iteration (Unix/macOS) |
| `zip` | 4.x | MIT | `.zip` entry iteration (Windows) |
| `ureq` | 3.x | MIT OR Apache-2.0 | HTTPS download with rustls (no OpenSSL dep) |

All four crates were already present as transitive dependencies in `Cargo.lock` before
this feature — adding them as direct dependencies does not introduce new code into the
binary. No AGPL, GPL, or otherwise copyleft crates are used for archive handling.

---

*Last updated: April 2026. Maintained by the ReVault project.*
*If you believe a license is missing or incorrectly stated, please open an issue.*
