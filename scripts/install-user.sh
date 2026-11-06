#!/usr/bin/env bash
# Install CouchDeck for the current user. No sudo, no rpm-ostree layer.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [[ ${EUID} -eq 0 ]]; then
  echo "Fehler: CouchDeck wird als normaler Benutzer installiert, nicht als root." >&2
  exit 1
fi

BINARY="${COUCHDECK_BINARY:-target/release/couchdeck}"
if [[ ! -x "$BINARY" ]]; then
  cat >&2 <<EOF
Fehler: $BINARY wurde nicht gefunden oder ist nicht ausführbar.

Baue zuerst im passenden Distrobox-Container:
    cargo build --release

Danach dieses Skript auf dem Host (ohne sudo) ausführen.
EOF
  exit 1
fi

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

install -Dm755 "$BINARY" "$BIN_DIR/couchdeck"
install -Dm644 assets/couchdeck.svg "$ICON_DIR/couchdeck.svg"
install -d "$APP_DIR"

cat > "$APP_DIR/couchdeck.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=CouchDeck
Comment=Controllerfreundliche Fullscreen-Spielebibliothek
Exec="${HOME}/.local/bin/couchdeck"
Icon=couchdeck
Terminal=false
Categories=Game;
StartupNotify=true
DESKTOP
chmod 0644 "$APP_DIR/couchdeck.desktop"

# Best-effort desktop/icon cache refresh; harmless if the tools are absent.
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

cat <<EOF
CouchDeck installiert:
  Binary:  $BIN_DIR/couchdeck
  Eintrag: $APP_DIR/couchdeck.desktop
  Icon:    $ICON_DIR/couchdeck.svg

Start:  ~/.local/bin/couchdeck
Debug:  ~/.local/bin/couchdeck --windowed --no-controller
EOF
