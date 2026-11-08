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

- [X] Menü-Icon wird nach Installation/Update korrekt angezeigt (PNG + absoluter Pfad)
- [X] Fullscreen → „Zum Desktop minimieren“ → über Taskleiste zurück → Fullscreen wieder aktiv
- [X] Mit `--windowed` gestartet: Minimieren/Wiederherstellen bleibt im Windowed-Modus

## 0.4.5 Fensteraktivierung (manuell)

Die Aktivierung ist Best-Effort. Unter Wayland kann der Compositor den Fokus
verweigern. `/home/USER/.local/bin/couchdeck` durch den tatsächlichen Pfad
ersetzen; bei AppImage den AppImage-Pfad verwenden.

- [X] `couchdeck --activate` bei fokussiertem CouchDeck: keine sichtbare Störung
- [ ] CouchDeck minimieren → `--activate` → Fenster wieder sichtbar - Defekt: da passiert nichts
- [ ] CouchDeck im Fullscreen minimieren → `--activate` → Fullscreen und Fokus wieder da - Defekt: da passiert nichts
- [ ] CouchDeck mit `--windowed` minimieren → `--activate` → bleibt Windowed - auch defekt
- [X] Zwei schnelle `--activate`-Aufrufe → keine zweite Instanz, stabiler Zustand
- [X] CouchDeck beenden → `--activate` → Exit-Code ungleich 0, verständliche Meldung
- [ ] Steam-Spiel starten, Steam-/Proton-/Launcher-Fenster zulassen, danach
      manuell `--activate`; CouchDeck darf den Fokus nur auf Anforderung holen
- [ ] `--diagnose-window` aktivieren; Fokus-/Minimierungs-/Fullscreen-Wechsel im Log prüfen
Fehlerlog:
[1794151262] INFO  couchdeck: Logdatei: /home/cr/.local/state/couchdeck/couchdeck.log
[1794151262] INFO  main: couchdeck 0.4.5 | Session=wayland | Wayland=wayland-1 | Desktop=Some(Kde) | DesktopTokens=["KDE", "/USR/SHARE/WAYLAND-SESSIONS/PLASMA.DESKTOP"] | XDG_DATA_HOME=/home/cr/.local/share | Autostart=false | Diagnose=true
[1794151262] INFO  launch: GIO: 2.90.1
[1794151262] INFO  app: Start: 3 Profile, 2 Spiele, Daten: /home/cr/.local/share/couchdeck
[1794151262] INFO  app: Hintergrund: None, anzeigen=true
[1794151262] INFO  window: Fokus verloren (Hintergrund); fullscreen=true; minimized=Some(false)
[1794151262] INFO  controller: Fensterfokus verloren; Gamepad-Navigation pausiert.
[1794151262] INFO  window: diagnose: focused=false, minimized=Some(false), fullscreen=true
[1794151262] INFO  window: Fokus gewonnen; fullscreen=true; minimized=Some(false)
[1794151262] INFO  controller: Fensterfokus gewonnen; Eingabepuffer wird verworfen.
[1794151262] INFO  window: diagnose: focused=true, minimized=Some(false), fullscreen=true
[1794151262] INFO  controller: Verbunden: 8BitDo Ultimate 2C
[1794151262] INFO  controller: 1 Controller bereit.
[1794151262] INFO  discovery: 221 Desktop-Einträge, 8 Quellen, 0 Warnungen
[1794151266] INFO  window: Fokus verloren (Hintergrund); fullscreen=true; minimized=Some(false)
[1794151266] INFO  controller: Fensterfokus verloren; Gamepad-Navigation pausiert.
[1794151266] INFO  window: diagnose: focused=false, minimized=Some(false), fullscreen=true
[1794151272] INFO  couchdeck: Logdatei: /home/cr/.local/state/couchdeck/couchdeck.log
[1794151272] INFO  main: couchdeck 0.4.5 | Session=wayland | Wayland=wayland-1 | Desktop=Some(Kde) | DesktopTokens=["KDE", "/USR/SHARE/WAYLAND-SESSIONS/PLASMA.DESKTOP"] | XDG_DATA_HOME=/home/cr/.local/share | Autostart=false | Diagnose=false
[1794151272] INFO  activation: Aktivierungsanforderung an die UI übermittelt.
[1794151272] INFO  activation: CLI --activate: Accepted - Aktivierungsanforderung übermittelt (Fensterfokus nicht bestätigt).
[1794151272] INFO  activation: Anforderung empfangen: focused=false, minimized=false, fullscreen=true, was_fullscreen=false
[1794151272] INFO  activation: Fullscreen wird wiederhergestellt.
[1794151272] INFO  activation: Fokus angefordert (Compositor kann die Anforderung ablehnen).

- [ ] Globalen Shortcut für `couchdeck --activate` einrichten und testen

## 0.4.6 Bugfix Fensteraktivierung – Bazzite Retest (offen)

Hintergrund: 0.4.5 sendete unter Wayland wirkungslose `Focus`-/
`Minimized(false)`-Befehle. 0.4.6 gibt `XDG_ACTIVATION_TOKEN` per IPC weiter und
nutzt unter KDE/Wayland zusätzlich einen begrenzten KWin-Scripting-Fallback.

Die folgenden Fälle sind auf **Bazzite/KDE/Wayland** noch nicht durchgeführt:

- [ ] Terminal ohne Token: `couchdeck --activate` bei minimiertem CouchDeck → Fenster sichtbar und fokussiert, **ohne** Maus-Hover über der Taskleiste
- [ ] Terminal ohne Token bei minimiertem Fullscreen-Fenster → Fullscreen/Fokus korrekt, kein unnötiger Toggle
- [ ] Taskleisten-Minimierung (Windowed und Fullscreen) → `--activate` → sofort sichtbar, kein Hover nötig
- [ ] `couchdeck --windowed`, minimieren, `--activate` → bleibt Windowed
- [ ] Benutzerdefinierter KDE-Shortcut auf `couchdeck --activate` → Verhalten dokumentieren (Token vorhanden/nicht vorhanden)
- [ ] KDE/KIO- oder Launcher-gestarteter Prozess mit `XDG_ACTIVATION_TOKEN` → Token wird angewendet, Fokus beobachtet
- [ ] Absichtlich ungültiger/leerer Token → KWin-Fallback oder verständlicher Timeout, keine falsche Erfolgsmeldung
- [ ] `--diagnose-window`: Logs für IPC, Token, KWin-Fallback, Fokus und Timeout prüfen
- [ ] Zwei schnelle `--activate`-Aufrufe → keine zweite Instanz, stabiler Zustand
- [ ] Laufendes Steam-/Proton-/Launcher-/Spiel-Szenario: CouchDeck kehrt nur auf Anforderung zurück, Spiel/Launcher läuft weiter

Auf dem Entwicklungsystem **Manjaro/KDE 6.7.5/Wayland** wurden Terminal-Fallback,
KIO-Token, minimiertes Windowed-/Fullscreen-Fenster, `--windowed` und ein
ungültiger Token praktisch geprüft. Diese Ergebnisse ersetzen **nicht** den
Bazzite-Zielhardware-Test.

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
- [X] Admin → Suchpfade: zusätzliches Verzeichnis hinzufügen/entfernen, Rescan
- [X] Autostart nur nach Opt-in; `OnlyShowIn` entspricht der aktuellen Sitzung
- [X] Spielstart über `gio launch` (Test-Desktop-Datei mit Leerzeichen im Pfad)
- [X] Admin-PIN mit On-Screen-Pad
- [X] Zum Desktop minimieren
- [X] Systemaktionen: Bestätigungsdialog und systemctl/Polkit (Neustart/
      Herunterfahren nur auf einem Wegwerf-Testsystem wirklich ausführen)

## B. tar.gz / Benutzerinstallation

```bash
tar -xzf couchdeck-<version>-linux-x86_64.tar.gz
./couchdeck-<version>-linux-x86_64/scripts/install-user.sh
~/.local/bin/couchdeck --version
```

Danach dieselben UI-/Controller-/GIO-Tests wie oben. Zusätzlich:

- [X] `~/.local/bin/couchdeck` existiert und ist ausführbar
- [X] Desktop-Eintrag und Icon erscheinen in der Desktop-Anwendungsliste — 0.4.0-Defect: kein Icon; 0.4.1 liefert PNG + absoluten Icon-Pfad, 0.4.1 Retest erfolgreich
- [X] `ldd ~/.local/bin/couchdeck` enthält kein `not found`
- [X] `ldd --version` erfüllt die dokumentierte glibc-Anforderung (ldd (GNU libc) 2.44)
- [X] `~/.local/share/couchdeck/` wird bei einem Update nicht überschrieben
- [X] Einstellungen/Profile/Artwork bleiben nach Update erhalten
- [X] `scripts/uninstall-user.sh` entfernt nur Binary/Menü/Icon/Autostart
- [X] `--purge` verlangt die explizite Löschbestätigung

## C. Update-Test

Vor dem Update ein Testprofil, ein importiertes Spiel, ein eigenes Cover und
eine geänderte Einstellung anlegen. Danach das neue Archiv installieren und
prüfen:

- [X] gleiche Profile und Spiele
- [X] gleiches Cover/Artwork
- [X] gleiche Einstellungen und Admin-PIN
- [X] neue CouchDeck-Version wird gestartet

## D. Ergebnis dokumentieren

Datum, Bazzite-Version, Kernel, Desktop-Sitzung, GPU/Treiber, Controller
(Modell und USB/Bluetooth/Dongle-Modus) sowie alle Abweichungen in
`docs/TEST_REPORT.md` ergänzen. Nicht ausgeführte Punkte bleiben ausdrücklich
offen. Weitere Desktop-Kombinationen: [`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md).
