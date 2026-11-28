#!/usr/bin/env bash
# Remove the user-level installation. Settings, library and artwork are kept
# unless --purge is explicitly supplied.
set -euo pipefail

PURGE=0
if [[ ${1:-} == "--purge" ]]; then
  PURGE=1
elif [[ $# -gt 0 ]]; then
  echo "Verwendung: $0 [--purge]" >&2
  exit 2
fi

rm -f "$HOME/.local/bin/capydeck"
rm -f "$HOME/.local/share/applications/capydeck.desktop"
rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/applications/capydeck-home.desktop"
rm -f "$HOME/.local/share/icons/hicolor/256x256/apps/capydeck.png"
rm -f "$HOME/.local/share/icons/hicolor/512x512/apps/capydeck.png"
rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/capydeck.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
fi
if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

if [[ $PURGE -eq 1 ]]; then
  cat <<EOF
ACHTUNG: --purge löscht Bibliothek, Profile, PIN-Hash, importierte Bilder,
Einstellungen, Cache und Logs von CapyDeck.
Betroffene Ordner:
  ${XDG_CONFIG_HOME:-$HOME/.config}/capydeck
  ${XDG_DATA_HOME:-$HOME/.local/share}/capydeck
  ${XDG_CACHE_HOME:-$HOME/.cache}/capydeck
  ${XDG_STATE_HOME:-$HOME/.local/state}/capydeck
EOF
  read -r -p "Zum Bestätigen 'LOESCHEN' eingeben: " answer
  if [[ "$answer" != "LOESCHEN" ]]; then
    echo "Abbruch: Daten wurden nicht gelöscht." >&2
    exit 1
  fi
  rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/capydeck"
  rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/capydeck"
  rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/capydeck"
  rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/capydeck"
  echo "CapyDeck-Benutzerdaten gelöscht."
else
  echo "Binary, Menüeinträge, Icons und Autostart-Eintrag entfernt."
  echo "Bibliothek, Profile, Artwork und Einstellungen wurden bewusst nicht gelöscht."
  echo "Zum vollständigen Löschen: $0 --purge"
fi
