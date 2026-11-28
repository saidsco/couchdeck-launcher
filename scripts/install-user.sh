#!/usr/bin/env bash
# Install CapyDeck for the current user. No sudo, no rpm-ostree layer.
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

Für Controller-Home wird zusätzlich Lesezugriff auf /dev/input benötigt.
Das Skript fragt nach der Installation, ob der Benutzer zur Gruppe input
hinzugefügt werden soll (einmalige Admin-/Polkit-Bestätigung, danach Neuanmeldung).
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
  echo "Fehler: CapyDeck wird als normaler Benutzer installiert, nicht als root." >&2
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

if [[ -n "${CAPYDECK_BINARY:-}" ]]; then
  BINARY="$CAPYDECK_BINARY"
elif [[ -x "$PACKAGE_ROOT/capydeck" ]]; then
  # Release archive layout: capydeck-<version>/capydeck
  BINARY="$PACKAGE_ROOT/capydeck"
else
  BINARY="$PACKAGE_ROOT/target/release/capydeck"
fi

if [[ ! -x "$BINARY" ]]; then
  cat >&2 <<EOF
Fehler: $BINARY wurde nicht gefunden oder ist nicht ausführbar.

Baue zuerst im passenden Distrobox-Container:
    cargo build --release

Oder verwende ein vollständiges CapyDeck-Release-Archiv. Danach dieses Skript
auf dem Host (ohne sudo) ausführen.
EOF
  exit 1
fi

if [[ -n "${CAPYDECK_ICON_PNG:-}" ]]; then
  ICON_PNG="$CAPYDECK_ICON_PNG"
elif [[ -f "$PACKAGE_ROOT/assets/capydeck.png" ]]; then
  ICON_PNG="$PACKAGE_ROOT/assets/capydeck.png"
elif [[ -f "$PACKAGE_ROOT/capydeck.png" ]]; then
  # Release archive layout
  ICON_PNG="$PACKAGE_ROOT/capydeck.png"
else
  echo "Fehler: capydeck.png wurde nicht gefunden." >&2
  exit 1
fi

if [[ -n "${CAPYDECK_ICON_PNG_512:-}" ]]; then
  ICON_PNG_512="$CAPYDECK_ICON_PNG_512"
elif [[ -f "$PACKAGE_ROOT/assets/capydeck-512.png" ]]; then
  ICON_PNG_512="$PACKAGE_ROOT/assets/capydeck-512.png"
elif [[ -f "$PACKAGE_ROOT/capydeck-512.png" ]]; then
  ICON_PNG_512="$PACKAGE_ROOT/capydeck-512.png"
else
  ICON_PNG_512=""
fi

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_256_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
ICON_512_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"
ICON_ABS="$ICON_256_DIR/capydeck.png"

install -Dm755 "$BINARY" "$BIN_DIR/capydeck"
install -Dm644 "$ICON_PNG" "$ICON_ABS"
if [[ -n "$ICON_PNG_512" ]]; then
  install -Dm644 "$ICON_PNG_512" "$ICON_512_DIR/capydeck.png"
fi
install -d "$APP_DIR"

cat > "$APP_DIR/capydeck.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=CapyDeck
Comment=Controllerfreundliche Fullscreen-Spielebibliothek
Exec="${HOME}/.local/bin/capydeck"
Icon=$ICON_ABS
Terminal=false
Categories=Game;
StartupNotify=true
DESKTOP
chmod 0644 "$APP_DIR/capydeck.desktop"

CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
AUTOSTART_FILE="$CONFIG_HOME/autostart/capydeck.desktop"
if [[ $AUTOSTART -eq 1 ]]; then
  install -d "$CONFIG_HOME/autostart"
  cat > "$AUTOSTART_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=CapyDeck
Comment=Controllerfreundliche Fullscreen-Spielebibliothek
Exec="${HOME}/.local/bin/capydeck" --autostart
Icon=$ICON_ABS
Terminal=false
Categories=Game;
StartupNotify=false
OnlyShowIn=${AUTOSTART_DESKTOP};
NotShowIn=gamescope;
X-GNOME-Autostart-enabled=true
X-CapyDeck-Autostart=true
X-CapyDeck-Desktop=${AUTOSTART_DESKTOP};
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
if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

INPUT_USER="$(id -un)"
if ! id -nG "$INPUT_USER" 2>/dev/null | tr ' ' '\n' | grep -qx input; then
  echo
  echo "Controller-Home benötigt Lesezugriff auf Gamepad-Geräte (/dev/input)."
  echo "Der Benutzer '$INPUT_USER' ist noch nicht in der Gruppe 'input'."
  if [[ -t 0 ]]; then
    read -r -p "Jetzt zur Gruppe input hinzufügen? [y/N] " INPUT_ANSWER
    case "${INPUT_ANSWER,,}" in
      y|yes|j|ja)
        if "$BIN_DIR/capydeck" --setup-input-permissions --yes; then
          echo "Bitte einmal ab- und wieder anmelden, damit die Gruppe aktiv wird."
        else
          echo "Die Berechtigung konnte nicht eingerichtet werden." >&2
          echo "Später manuell möglich: $BIN_DIR/capydeck --setup-input-permissions" >&2
        fi
        ;;
      *)
        echo "Übersprungen. Später möglich: $BIN_DIR/capydeck --setup-input-permissions"
        ;;
    esac
  else
    echo "Später manuell möglich: $BIN_DIR/capydeck --setup-input-permissions"
  fi
else
  echo "Controller-Home: Gruppe input ist bereits zugewiesen (ggf. Neuanmeldung nötig)."
fi

cat <<EOF
CapyDeck installiert:
  Binary:  $BIN_DIR/capydeck
  Eintrag: $APP_DIR/capydeck.desktop
  Icon:    $ICON_ABS

Start:  ~/.local/bin/capydeck
Debug:  ~/.local/bin/capydeck --windowed --no-controller
Autostart: $(if [[ $AUTOSTART -eq 1 ]]; then echo "aktiv für $AUTOSTART_DESKTOP ($AUTOSTART_FILE)"; else echo "inaktiv"; fi)

Der Autostart kann später auch in CapyDeck unter Admin → Einstellungen
ein- und ausgeschaltet werden.

Bibliothek, Profile, PIN, Artwork und Einstellungen werden durch dieses
Installations-/Update-Skript nicht angefasst.
EOF
