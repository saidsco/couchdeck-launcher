#!/usr/bin/env bash
# Run inside a clean Fedora build container for GitHub Releases.
set -euo pipefail

: "${VERSION:?VERSION is required}"
: "${HOST_UID:?HOST_UID is required}"
: "${HOST_GID:?HOST_GID is required}"

dnf install -y --setopt=install_weak_deps=False \
  curl tar gzip findutils file binutils shadow-utils \
  gcc gcc-c++ pkgconf-pkg-config make \
  systemd-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel \
  mesa-libGL-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel \
  glib2 librsvg2-tools desktop-file-utils xorg-x11-server-Xvfb mesa-dri-drivers \
  dbus-daemon dbus-tools squashfs-tools

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain 1.89.0
. "$HOME/.cargo/env"
export PATH="/usr/sbin:/usr/local/sbin:$PATH"
rustup component add clippy rustfmt

export CARGO_TARGET_DIR=/tmp/couchdeck-target
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
scripts/check-runtime.sh \
  "$CARGO_TARGET_DIR/release/couchdeck" \
  /tmp/couchdeck-runtime-info.txt

APPIMAGETOOL="$(scripts/fetch-release-tools.sh /tmp/couchdeck-release-tools)"
APPIMAGE_RUNTIME_FILE="${APPIMAGE_RUNTIME_FILE:-/tmp/appimage-runtime-x86_64}"
if [[ ! -s "$APPIMAGE_RUNTIME_FILE" ]]; then
  rm -f "$APPIMAGE_RUNTIME_FILE"
  for attempt in 1 2 3 4 5; do
    if curl -L --fail --retry 5 --retry-delay 3 --retry-all-errors \
      -o "$APPIMAGE_RUNTIME_FILE" \
      https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64; then
      break
    fi
    sleep 5
  done
fi
if [[ ! -s "$APPIMAGE_RUNTIME_FILE" ]]; then
  echo "Fehler: AppImage-Runtime konnte nicht geladen werden." >&2
  exit 1
fi

VERSION="$VERSION" \
BINARY="$CARGO_TARGET_DIR/release/couchdeck" \
APPIMAGETOOL="$APPIMAGETOOL" \
APPIMAGE_RUNTIME_FILE="$APPIMAGE_RUNTIME_FILE" \
OUT_DIR=/work/dist \
scripts/package-release.sh

APPIMAGE="/work/dist/CouchDeck-${VERSION}-x86_64.AppImage"
SMOKE_LOG=/tmp/couchdeck-appimage-smoke.log
SMOKE_HOME=/tmp/couchdeck-smoke-home
rm -rf "$SMOKE_HOME"
mkdir -p "$SMOKE_HOME"/{config,data,cache,state}
set +e
XDG_CONFIG_HOME="$SMOKE_HOME/config" \
XDG_DATA_HOME="$SMOKE_HOME/data" \
XDG_CACHE_HOME="$SMOKE_HOME/cache" \
XDG_STATE_HOME="$SMOKE_HOME/state" \
LIBGL_ALWAYS_SOFTWARE=1 \
APPIMAGE_EXTRACT_AND_RUN=1 \
timeout 8s /usr/sbin/xvfb-run -a "$APPIMAGE" --windowed --no-controller \
  >"$SMOKE_LOG" 2>&1
smoke_status=$?
set -e
cat "$SMOKE_LOG"
if [[ $smoke_status -ne 0 && $smoke_status -ne 124 ]]; then
  echo "AppImage startup smoke test failed with status $smoke_status" >&2
  exit 1
fi
if ! grep -q 'Start: ' "$SMOKE_LOG"; then
  echo "AppImage startup smoke test did not reach the application start" >&2
  exit 1
fi
if grep -qi 'panicked\|not found' "$SMOKE_LOG"; then
  echo "AppImage startup smoke test reported a panic or missing library" >&2
  exit 1
fi

chown -R "$HOST_UID:$HOST_GID" /work/dist
