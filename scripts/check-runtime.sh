#!/usr/bin/env bash
# Build a runtime dependency report for a CouchDeck binary.
# Fails if ldd reports missing libraries. Safe to run on the target system.
set -euo pipefail

BINARY="${1:-target/release/couchdeck}"
REPORT="${2:-runtime-info.txt}"

if [[ ! -x "$BINARY" ]]; then
  echo "Fehler: Binary nicht gefunden oder nicht ausführbar: $BINARY" >&2
  exit 1
fi
if ! command -v ldd >/dev/null 2>&1; then
  echo "Fehler: ldd fehlt; Laufzeitabhängigkeiten können nicht geprüft werden." >&2
  exit 1
fi

os_name="unbekannt"
if [[ -r /etc/os-release ]]; then
  os_name="$(sed -n 's/^PRETTY_NAME=//p' /etc/os-release | tr -d '"' | head -n1)"
fi

check_report() {
  echo "CouchDeck runtime dependency report"
  echo "generated_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=${os_name}"
  echo "kernel=$(uname -sr)"
  echo "binary=$BINARY"
  if command -v file >/dev/null 2>&1; then
    file "$BINARY"
  fi
  if command -v rustc >/dev/null 2>&1; then
    echo "rustc=$(rustc --version)"
  fi
  echo "build_glibc=$(ldd --version | head -n1)"
  if command -v readelf >/dev/null 2>&1; then
    max_glibc="$(readelf --version-info "$BINARY" 2>/dev/null \
      | grep -oE 'GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?' \
      | sort -Vu | tail -n1 || true)"
    echo "max_required_glibc_symbol=${max_glibc:-unbekannt}"
  else
    echo "max_required_glibc_symbol=unbekannt (readelf fehlt)"
  fi
  echo
  echo "ldd:"
  ldd "$BINARY"
}

ldd_output="$(ldd "$BINARY")"
if grep -q 'not found' <<<"$ldd_output"; then
  echo "Fehler: ldd meldet fehlende Bibliotheken:" >&2
  grep 'not found' <<<"$ldd_output" >&2
  exit 1
fi

check_report | tee "$REPORT"
