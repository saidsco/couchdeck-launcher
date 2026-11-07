#!/usr/bin/env bash
# Install CouchDeck for the current user. No sudo, no rpm-ostree layer.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

AUTOSTART=0
for arg in "$@"; do
  case "$arg" in
    --autostart)
      AUTOSTART=1
      ;;
    -h|--help)
      cat <<EOF
Verwendung: $0 [--autostart]

  --autostart   desktopbezogenen XDG-Autostart anlegen
                (nur KDE Plasma, GNOME oder XFCE; nicht im Steam Gaming Mode)
EOF
      exit 0
      ;;
    *)
      echo "Unbekannte Option: $arg" >&2
      echo "Verwendung: $0 [--autostart]" >&2
      exit 2
      ;;
  esac
done

if [[ ${EUID} -eq 0 ]]; then
  echo "Fehler: CouchDeck wird als normaler Benutzer installiert, nicht als root." >&2
  exit 1
fi

detect_autostart_desktop() {
  local raw="${XDG_CURRENT_DESKTOP:-}:${XDG_SESSION_DESKTOP:-}:${DESKTOP_SESSION:-}"
  local token upper
  IFS=':;' read -ra tokens <<<"$raw"
  for token in "${tokens[@]}"; do
    token="${token//[[:space:]]/}"
    upper="${token^^}"
    if [[ "$upper" == *GAMESCOPE* || "$upper" == *GAMINGMODE* ]]; then
      echo "GAMING"
      return 0
    fi
  done
  for token in "${tokens[@]}"; do
    token="${token//[[:space:]]/}"
    [[ -z "$token" ]] && continue
    upper="${token^^}"
    if [[ "$upper" == "KDE" || "$upper" == *PLASMA* ]]; then
      echo "KDE"
      return 0
    fi
    if [[ "$upper" == "GNOME" || "$upper" == GNOME* ]]; then
      echo "GNOME"
      return 0
    fi
    if [[ "$upper" == "XFCE" || "$upper" == XFCE* ]]; then
      echo "XFCE"
      return 0
    fi
  done
  return 1
}

AUTOSTART_DESKTOP=""
if [[ $AUTOSTART -eq 1 ]]; then
  if ! AUTOSTART_DESKTOP="$(detect_autostart_desktop)"; then
    echo "Fehler: Autostart wird nur für KDE Plasma, GNOME und XFCE angeboten;" >&2
    echo "die aktuelle Desktop-Sitzung wurde nicht erkannt. Es wurde nichts installiert." >&2
    exit 1
  fi
  if [[ "$AUTOSTART_DESKTOP" == "GAMING" ]]; then
    echo "Fehler: Autostart ist in Steam Gaming Mode/Gamescope nicht verfügbar." >&2
    echo "Bitte im Desktop-Modus installieren. Es wurde nichts installiert." >&2
    exit 1
  fi
fi

if [[ -n "${COUCHDECK_BINARY:-}" ]]; then
  BINARY="$COUCHDECK_BINARY"
elif [[ -x "$PACKAGE_ROOT/couchdeck" ]]; then
  # Release archive layout: couchdeck-<version>/couchdeck
  BINARY="$PACKAGE_ROOT/couchdeck"
else
  BINARY="$PACKAGE_ROOT/target/release/couchdeck"
fi

if [[ ! -x "$BINARY" ]]; then
  cat >&2 <<EOF
Fehler: $BINARY wurde nicht gefunden oder ist nicht ausführbar.

Baue zuerst im passenden Distrobox-Container:
    cargo build --release

Oder verwende ein vollständiges CouchDeck-Release-Archiv. Danach dieses Skript
auf dem Host (ohne sudo) ausführen.
EOF
  exit 1
fi

if [[ -n "${COUCHDECK_ICON:-}" ]]; then
  ICON_SOURCE="$COUCHDECK_ICON"
elif [[ -f "$PACKAGE_ROOT/assets/couchdeck.svg" ]]; then
  ICON_SOURCE="$PACKAGE_ROOT/assets/couchdeck.svg"
elif [[ -f "$PACKAGE_ROOT/couchdeck.svg" ]]; then
  # Release archive layout
  ICON_SOURCE="$PACKAGE_ROOT/couchdeck.svg"
else
  echo "Fehler: couchdeck.svg wurde nicht gefunden." >&2
  exit 1
fi

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

install -Dm755 "$BINARY" "$BIN_DIR/couchdeck"
install -Dm644 "$ICON_SOURCE" "$ICON_DIR/couchdeck.svg"
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

CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
AUTOSTART_FILE="$CONFIG_HOME/autostart/couchdeck.desktop"
if [[ $AUTOSTART -eq 1 ]]; then
  install -d "$CONFIG_HOME/autostart"
  cat > "$AUTOSTART_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=CouchDeck
Comment=Controllerfreundliche Fullscreen-Spielebibliothek
Exec="${HOME}/.local/bin/couchdeck" --autostart
Icon=couchdeck
Terminal=false
Categories=Game;
StartupNotify=false
OnlyShowIn=${AUTOSTART_DESKTOP};
NotShowIn=gamescope;
X-GNOME-Autostart-enabled=true
X-CouchDeck-Autostart=true
X-CouchDeck-Desktop=${AUTOSTART_DESKTOP};
DESKTOP
  chmod 0644 "$AUTOSTART_FILE"
fi

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
Autostart: $(if [[ $AUTOSTART -eq 1 ]]; then echo "aktiv für $AUTOSTART_DESKTOP ($AUTOSTART_FILE)"; else echo "inaktiv"; fi)

Der Autostart kann später auch in CouchDeck unter Admin → Einstellungen
ein- und ausgeschaltet werden.

Bibliothek, Profile, PIN, Artwork und Einstellungen werden durch dieses
Installations-/Update-Skript nicht angefasst.
EOF
