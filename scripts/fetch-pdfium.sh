#!/usr/bin/env bash
# Fetches the pinned pdfium dynamic library into src-tauri/resources/pdfium/,
# verifying the archive SHA-256 before extraction. The library is bundled into
# the installer via tauri.conf.json's bundle.resources — it is NOT downloaded
# at app runtime.
#
# Usage: scripts/fetch-pdfium.sh [mac-arm64|mac-x64|win-x64|linux-x64]
#   (no argument: auto-detect the current host)
#
# Pinned release: bblanchon/pdfium-binaries chromium/7947 (immutable release).
# Hashes below were computed locally over the downloaded archive bytes on
# 2026-07-17 — never trust a checksum file published alongside the asset.
set -euo pipefail

RELEASE_TAG="chromium/7947"
BASE_URL="https://github.com/bblanchon/pdfium-binaries/releases/download/${RELEASE_TAG}"

sha_for() {
  case "$1" in
    mac-arm64) echo "aa9739354fc7bc8f200f3f3c9532bd5233298203051e094820272ccd9c997a77" ;;
    mac-x64)   echo "16d7a263b9e2f550d230ce81637697381b0ce898f2e3a22c7316594b15199d87" ;;
    win-x64)   echo "75df6802fc090ad7c76ccc29ed80c3fcb1a375c775bbf8e522189174647b101f" ;;
    linux-x64) echo "f73d69d309fe1f33cc7269dcc99be31ec44e1cf608e31d7e2fcc6545fc2f9323" ;;
    *) echo "unknown target: $1" >&2; exit 1 ;;
  esac
}

lib_member_for() {
  case "$1" in
    mac-arm64|mac-x64) echo "lib/libpdfium.dylib" ;;
    win-x64)           echo "bin/pdfium.dll" ;;
    linux-x64)         echo "lib/libpdfium.so" ;;
  esac
}

detect_target() {
  case "$(uname -s)" in
    Darwin)
      case "$(uname -m)" in
        arm64) echo "mac-arm64" ;;
        x86_64) echo "mac-x64" ;;
        *) echo "unsupported macOS arch: $(uname -m)" >&2; exit 1 ;;
      esac ;;
    Linux) echo "linux-x64" ;;
    MINGW*|MSYS*|CYGWIN*) echo "win-x64" ;;
    *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
}

TARGET="${1:-$(detect_target)}"
EXPECTED_SHA="$(sha_for "$TARGET")"
LIB_MEMBER="$(lib_member_for "$TARGET")"
ARCHIVE="pdfium-${TARGET}.tgz"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST_DIR="${SCRIPT_DIR}/../src-tauri/resources/pdfium"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Fetching ${ARCHIVE} (${RELEASE_TAG})..."
curl -sSfL -o "${TMP_DIR}/${ARCHIVE}" "${BASE_URL}/${ARCHIVE}"

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_SHA="$(sha256sum "${TMP_DIR}/${ARCHIVE}" | cut -d' ' -f1)"
else
  ACTUAL_SHA="$(shasum -a 256 "${TMP_DIR}/${ARCHIVE}" | cut -d' ' -f1)"
fi

if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  echo "SHA-256 mismatch for ${ARCHIVE}:" >&2
  echo "  expected: ${EXPECTED_SHA}" >&2
  echo "  actual:   ${ACTUAL_SHA}" >&2
  exit 1
fi
echo "SHA-256 verified."

mkdir -p "${DEST_DIR}"
# Extract only the library and the third-party license notices (shipped with
# the app for compliance) — headers/cmake stay out of the bundle.
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}" "${LIB_MEMBER}" licenses LICENSE
cp "${TMP_DIR}/${LIB_MEMBER}" "${DEST_DIR}/"
rm -rf "${DEST_DIR}/licenses"
cp -R "${TMP_DIR}/licenses" "${DEST_DIR}/licenses"
cp "${TMP_DIR}/LICENSE" "${DEST_DIR}/LICENSE"

echo "Installed $(basename "$LIB_MEMBER") + licenses into ${DEST_DIR}/"
