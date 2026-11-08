#!/usr/bin/env bash
# Download and unpack the pinned AppImage build tool used by the release workflow.
set -euo pipefail

TOOLS_DIR="${1:-/tmp/couchdeck-release-tools}"
URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage"
SHA256="c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d"

mkdir -p "$TOOLS_DIR"
LINUXDEPLOY="$TOOLS_DIR/linuxdeploy-x86_64.AppImage"
if [[ ! -f "$LINUXDEPLOY" ]]; then
  curl -L --fail --retry 5 --retry-delay 3 --retry-all-errors -o "$LINUXDEPLOY" "$URL"
fi
echo "$SHA256  $LINUXDEPLOY" | sha256sum -c - >&2
chmod +x "$LINUXDEPLOY"

EXTRACT_DIR="$TOOLS_DIR/linuxdeploy-extract"
rm -rf "$EXTRACT_DIR"
mkdir -p "$EXTRACT_DIR"
(
  cd "$EXTRACT_DIR"
  APPIMAGE_EXTRACT_AND_RUN=1 "$LINUXDEPLOY" --appimage-extract >/dev/null
)
APPIMAGETOOL="$EXTRACT_DIR/squashfs-root/plugins/linuxdeploy-plugin-appimage/usr/bin/appimagetool"
if [[ ! -x "$APPIMAGETOOL" ]]; then
  echo "Fehler: appimagetool wurde im linuxdeploy-Paket nicht gefunden." >&2
  exit 1
fi
chmod +x "$APPIMAGETOOL"
printf '%s\n' "$APPIMAGETOOL"
