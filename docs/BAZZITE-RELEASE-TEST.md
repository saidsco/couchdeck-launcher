# Bazzite-Release-Test (manuell)

Ziel: AppImage **und** tar.gz auf einer sauberen Bazzite-KDE-Plasma-Installation
testen. Der Test benötigt **keinen Rust-Compiler** und **keine Root-Rechte**.

## Vorbereitung

1. Frisches Bazzite KDE Plasma (Wayland) starten, möglichst neues Benutzerkonto
   ohne vorhandene CouchDeck-Daten.
2. Release-Assets und `SHA256SUMS` von GitHub herunterladen.
3. Im Download-Ordner:

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

Nur die heruntergeladenen Dateien werden geprüft; `--ignore-missing` erlaubt
einen Test von AppImage und Archiv getrennt.

## 0.4.1 Fix-Retest (offen)

- [ ] Menü-Icon wird nach Installation/Update korrekt angezeigt (PNG + absoluter Pfad)
- [ ] Fullscreen → „Zum Desktop minimieren“ → über Taskleiste zurück → Fullscreen wieder aktiv
- [ ] Mit `--windowed` gestartet: Minimieren/Wiederherstellen bleibt im Windowed-Modus

## A. AppImage

```bash
chmod +x CouchDeck-<version>-x86_64.AppImage
./CouchDeck-<version>-x86_64.AppImage --version
./CouchDeck-<version>-x86_64.AppImage
```

Falls FUSE fehlt:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./CouchDeck-<version>-x86_64.AppImage
```

Prüfen:

- [X] Startet direkt fullscreen in der Bibliothek
- [X] D-Pad/Stick erreicht alle Kacheln
- [X] A startet, B zurück, Start öffnet Menü
- [X] LB/RB öffnet die Profilübersicht
- [X] Controller-Reconnect/Hotplug funktioniert
- [X] Import einer `.desktop`-Datei über Admin → Spiele (Quelle, Desktop-ID und Exec prüfen)
- [ ] Admin → Suchpfade: zusätzliches Verzeichnis hinzufügen/entfernen, Rescan
- [ ] Autostart nur nach Opt-in; `OnlyShowIn` entspricht der aktuellen Sitzung
- [X] Spielstart über `gio launch` (Test-Desktop-Datei mit Leerzeichen im Pfad)
- [X] Admin-PIN mit On-Screen-Pad
- [X] Zum Desktop minimieren
- [ ] Systemaktionen: Bestätigungsdialog und systemctl/Polkit (Neustart/
      Herunterfahren nur auf einem Wegwerf-Testsystem wirklich ausführen)

## B. tar.gz / Benutzerinstallation

```bash
tar -xzf couchdeck-<version>-linux-x86_64.tar.gz
./couchdeck-<version>-linux-x86_64/scripts/install-user.sh
~/.local/bin/couchdeck --version
```

Danach dieselben UI-/Controller-/GIO-Tests wie oben. Zusätzlich:

- [X] `~/.local/bin/couchdeck` existiert und ist ausführbar
- [ ] Desktop-Eintrag und Icon erscheinen in der Desktop-Anwendungsliste — 0.4.0-Defect: kein Icon; 0.4.1 liefert PNG + absoluten Icon-Pfad, Retest offen
- [ ] `ldd ~/.local/bin/couchdeck` enthält kein `not found`
- [ ] `ldd --version` erfüllt die dokumentierte glibc-Anforderung
- [ ] `~/.local/share/couchdeck/` wird bei einem Update nicht überschrieben
- [ ] Einstellungen/Profile/Artwork bleiben nach Update erhalten
- [ ] `scripts/uninstall-user.sh` entfernt nur Binary/Menü/Icon/Autostart
- [ ] `--purge` verlangt die explizite Löschbestätigung

## C. Update-Test

Vor dem Update ein Testprofil, ein importiertes Spiel, ein eigenes Cover und
eine geänderte Einstellung anlegen. Danach das neue Archiv installieren und
prüfen:

- [ ] gleiche Profile und Spiele
- [ ] gleiches Cover/Artwork
- [ ] gleiche Einstellungen und Admin-PIN
- [ ] neue CouchDeck-Version wird gestartet

## D. Ergebnis dokumentieren

Datum, Bazzite-Version, Kernel, Desktop-Sitzung, GPU/Treiber, Controller
(Modell und USB/Bluetooth/Dongle-Modus) sowie alle Abweichungen in
`docs/TEST_REPORT.md` ergänzen. Nicht ausgeführte Punkte bleiben ausdrücklich
offen. Weitere Desktop-Kombinationen: [`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md).
