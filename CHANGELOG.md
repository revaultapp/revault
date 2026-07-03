# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Image Compression**: JPEG (mozjpeg SIMD), PNG (oxipng+zopfli), WebP, AVIF (ravif) with quality control and exact-size targeting; JXL decode support (thumbnails)
- **Convert/HEIC**: Native HEIC decoding (macOS ImageIO, Windows WIC), format conversion (PNG/JPEG/WebP), auto-detect iPhone photos
- **Resize**: Batch image resize engine with anti-upscaling warnings and default output folder
- **Duplicates**: Exact duplicate detection (SHA256) and Similar mode for perceptual duplicates (pHash DoubleGradient)
- **Privacy**: EXIF/GPS/camera metadata stripping for JPEG, PNG, HEIC images with selective privacy controls
- **Video Compression**: FFmpeg backend with CRF presets (Smallest/Balanced/HighQuality), MOV→MP4 remux, privacy modes (Off/Smart/GPS-only/Full), size prediction
- **Video Trim**: Cut a video to a start/end range with fast, lossless stream-copy (ffmpeg -c copy); non-clobbering output, range validated against real media duration
- **GIF Export**: Create animated GIFs from video via gifski sidecar with streaming pipeline and cancellation
- **PDF Tools**: PDF metadata stripping and stream compression with embedded image re-encoding
- **Dashboard**: Storage analysis, compression savings tracker, quick actions, recent activity
- **UI/UX**: Locked-dark chrome, cross-platform window controls, Svelte 5 runes reactivity, animated progress indicators, before/after slider with keyboard navigation
- **Security**: Input path validation across all command entry points, memory limits on HEIC/JXL decode, gifski archive SHA-256 verification, dependency auditing

### Changed
- Removed Organize/Rename feature (out of scope)
- Removed Watermark feature (pending UX redesign)
- Deferred Cloud sync (offline-first design decision)
- Deferred OCR (complex scope, no timeline)
