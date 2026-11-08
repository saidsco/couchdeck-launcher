#!/usr/bin/env bash
# Build the portable release archive and AppImage for Linux x86_64.
#
# Required at build time: cargo release binary, tar/gzip, sha256sum,
# appimagetool (APPIMAGETOOL=...), rsvg-convert or ImageMagick.
# `scripts/fetch-release-tools.sh` provides the pinned appimagetool build helper.
# The produced packages do not need a Rust toolchain.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="${VERSION:-$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)}"
BINARY="${BINARY:-target/release/couchdeck}"
OUT_DIR="${OUT_DIR:-dist}"
APPIMAGETOOL="${APPIMAGETOOL:-}"
APPIMAGE_RUNTIME_FILE="${APPIMAGE_RUNTIME_FILE:-}"
DIST_NAME="couchdeck-${VERSION}-linux-x86_64"
STAGE="${OUT_DIR}/${DIST_NAME}"
APPIMAGE="${OUT_DIR}/CouchDeck-${VERSION}-x86_64.AppImage"
RUNTIME_INFO="${OUT_DIR}/couchdeck-${VERSION}-runtime-info.txt"

if [[ -z "$VERSION" ]]; then
  echo "Fehler: Version konnte nicht aus Cargo.toml gelesen werden." >&2
  exit 1
fi
if [[ ! -x "$BINARY" ]]; then
  echo "Fehler: Release-Binary fehlt: $BINARY" >&2
  exit 1
fi
if [[ -z "$APPIMAGETOOL" || ! -x "$APPIMAGETOOL" ]]; then
  echo "Fehler: APPIMAGETOOL muss auf ein ausführbares appimagetool zeigen." >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
rm -rf "$STAGE" "$OUT_DIR/AppDir" "$APPIMAGE" "$RUNTIME_INFO" "$OUT_DIR/${DIST_NAME}.tar.gz" "$OUT_DIR/SHA256SUMS" "$OUT_DIR/RELEASE_NOTES.md"
mkdir -p "$STAGE/scripts"

install -m755 "$BINARY" "$STAGE/couchdeck"
install -m644 README.md README-de.md LICENSE NOTICE CHANGELOG.md "$STAGE/"
mkdir -p "$STAGE/docs"
install -m644 docs/*.md "$STAGE/docs/"
cp -a docs/screenshots "$STAGE/docs/screenshots"
install -m644 assets/couchdeck.svg assets/couchdeck.png assets/couchdeck-512.png "$STAGE/"
install -m755 scripts/install-user.sh scripts/uninstall-user.sh scripts/check-runtime.sh "$STAGE/scripts/"

"$ROOT/scripts/check-runtime.sh" "$BINARY" "$RUNTIME_INFO"
install -m644 "$RUNTIME_INFO" "$STAGE/runtime-info.txt"

# Validate the package binary before it is archived.
"$STAGE/couchdeck" --version >/dev/null

# Reproducible tar where possible.
epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 0)}"
tar --sort=name --mtime="@${epoch}" --owner=0 --group=0 --numeric-owner \
  -C "$OUT_DIR" -cf - "$DIST_NAME" | gzip -n -9 > "$OUT_DIR/${DIST_NAME}.tar.gz"

# AppImage: package the unmodified native binary. CouchDeck deliberately keeps
# host glibc, GPU drivers, Wayland/X11 and libudev instead of bundling them;
# this avoids driver/ABI conflicts and matches the native Linux desktop target.
ICON_PNG="${OUT_DIR}/couchdeck.png"
if command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w 256 -h 256 assets/couchdeck.svg -o "$ICON_PNG"
elif command -v convert >/dev/null 2>&1; then
  convert -background none assets/couchdeck.svg -resize 256x256 "$ICON_PNG"
elif command -v magick >/dev/null 2>&1; then
  magick -background none assets/couchdeck.svg -resize 256x256 "$ICON_PNG"
else
  echo "Fehler: rsvg-convert oder ImageMagick wird für das AppImage-Icon benötigt." >&2
  exit 1
fi

APPDIR="${OUT_DIR}/AppDir"
rm -rf "$APPDIR"
mkdir -p \
  "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m755 "$BINARY" "$APPDIR/usr/bin/couchdeck"
install -m644 packaging/couchdeck.desktop "$APPDIR/couchdeck.desktop"
install -m644 packaging/couchdeck.desktop "$APPDIR/usr/share/applications/couchdeck.desktop"
install -m644 "$ICON_PNG" "$APPDIR/couchdeck.png"
install -m644 "$ICON_PNG" "$APPDIR/usr/share/icons/hicolor/256x256/apps/couchdeck.png"
ln -s couchdeck.png "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
set -e
HERE="${APPDIR:-$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)}"
exec "$HERE/usr/bin/couchdeck" "$@"
APPRUN
chmod 0755 "$APPDIR/AppRun"

APPIMAGETOOL="$(realpath "$APPIMAGETOOL")"
APPIMAGETOOL_ARGS=("$APPDIR" "$APPIMAGE")
if [[ -n "$APPIMAGE_RUNTIME_FILE" ]]; then
  APPIMAGETOOL_ARGS=(--runtime-file "$APPIMAGE_RUNTIME_FILE" "${APPIMAGETOOL_ARGS[@]}")
fi
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 \
  "$APPIMAGETOOL" "${APPIMAGETOOL_ARGS[@]}"
rm -rf "$APPDIR" "$ICON_PNG"

if [[ ! -f "$APPIMAGE" ]]; then
  echo "Fehler: AppImage wurde nicht erzeugt: $APPIMAGE" >&2
  find "$OUT_DIR" -maxdepth 1 -name '*.AppImage' -print >&2 || true
  exit 1
fi
chmod 0755 "$APPIMAGE"

# Runtime smoke test without a Rust toolchain and without FUSE.
APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGE" --version >/dev/null

(
  cd "$OUT_DIR"
  sha256sum "${DIST_NAME}.tar.gz" "CouchDeck-${VERSION}-x86_64.AppImage" "couchdeck-${VERSION}-runtime-info.txt" > SHA256SUMS
)

cat > "$OUT_DIR/RELEASE_NOTES.md" <<NOTES
# CouchDeck ${VERSION}

Native Linux x86_64 release for KDE Plasma, GNOME and XFCE (Wayland or X11).

## Assets

- \`${DIST_NAME}.tar.gz\` – portable archive with binary, installer, docs and runtime report
- \`CouchDeck-${VERSION}-x86_64.AppImage\` – executable AppImage
- \`couchdeck-${VERSION}-runtime-info.txt\` – ldd/glibc runtime report
- \`SHA256SUMS\` – SHA-256 checksums

## Verify

\`\`\`bash
sha256sum -c SHA256SUMS
\`\`\`

## Install (archive)

\`\`\`bash
tar -xzf ${DIST_NAME}.tar.gz
./${DIST_NAME}/scripts/install-user.sh
\`\`\`

Update: run the same installer from the new archive. User library, profiles,
artwork and settings under the XDG directories are not touched.

## AppImage

\`\`\`bash
chmod +x CouchDeck-${VERSION}-x86_64.AppImage
./CouchDeck-${VERSION}-x86_64.AppImage
\`\`\`

If FUSE is unavailable:

\`\`\`bash
APPIMAGE_EXTRACT_AND_RUN=1 ./CouchDeck-${VERSION}-x86_64.AppImage
\`\`\`

No Rust compiler, root privileges or Flatpak sandbox are required.
See \`README.md\` and \`docs/RELEASE.md\` for details and known limitations.
NOTES

echo
echo "Release artifacts in $OUT_DIR:"
ls -lh "$OUT_DIR/${DIST_NAME}.tar.gz" "$APPIMAGE" "$RUNTIME_INFO" "$OUT_DIR/SHA256SUMS" "$OUT_DIR/RELEASE_NOTES.md"
