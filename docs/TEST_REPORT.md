# Test- und Messbericht

**Stand:** 6. November 2026 · **Version:** CouchDeck 0.3.1

**Getestete Umgebung:** Manjaro Linux (KDE Plasma, Wayland-Session), Kernel `6.18.55-1-MANJARO`, NVIDIA RTX 4070 Ti SUPER
**Release-Buildumgebung:** Fedora 42 Container, glibc 2.41, Rust 1.89.0, x86_64
**Nicht die Zielhardware:** Bazzite KDE Plasma auf Wayland wurde in dieser Umgebung nicht ausgeführt.

## Toolchain und Abhängigkeiten

| Komponente | Version |
|---|---|
| Rust / Cargo | `1.89.0-beta.3` |
| eframe / egui | `0.32.3` / `0.32.3` |
| gilrs | `0.11.2` |
| image | `0.25.10` |
| argon2 | `0.5.3` |
| fluent-bundle | `0.16.0` |
| unic-langid | `0.9.6` |
| chrono | `0.4.45` |
| serde_json | `1.0.154` |
| toml | `0.8.23` |
| GIO | `2.88.3` (Host) / `2.84.4` (Fedora-42-Release-Container) |
| Release-Build | Fedora 42, glibc 2.41, Rust 1.89.0 |

`Cargo.lock` ist eingecheckt.

## Automatisierte Prüfungen

```text
$ cargo fmt --check
(keine Ausgabe, Exit 0)

$ cargo clippy --all-targets -- -D warnings
Finished `dev` profile ... (Exit 0)

$ cargo test
running 62 tests
test result: ok. 62 passed; 0 failed
running 2 tests
test result: ok. 2 passed; 0 failed
(running 0 tests: main.rs, Doc-Tests)

$ cargo build --release
Finished `release` profile [optimized]
```

Das sind **64 Tests insgesamt** (62 Unit-Tests + 2 GIO-Integrationstests).

Zusätzlich wurde die Release-Paketierung im Fedora-42-Container geprüft:

```text
scripts/check-runtime.sh target/release/couchdeck ...   # ldd ohne "not found"
scripts/package-release.sh                              # tar.gz + AppImage + SHA256SUMS
APPIMAGE_EXTRACT_AND_RUN=1 CouchDeck-<version>.AppImage --version
xvfb-run ... CouchDeck-<version>.AppImage --windowed    # Start bis "app: Start:"
sha256sum -c SHA256SUMS
```

### Abgedeckte Unit-Tests (Auszug)

- Desktop-Parsing: lokalisierte Namen, Escapes, ungültige Gruppen, Nicht-Application-Typen.
- XDG-Discovery: User-Eintrag gewinnt gegen System-Eintrag; `Hidden`, `NoDisplay`, fehlerhafte Dateien werden übersprungen.
- Steam-Shortcut-Erkennung, Prism-`--launch`-Erkennung, sichere Diagnose-Tokenisierung.
- Verwaltete Steam-/Prism-Desktop-Dateien: Original bleibt unverändert, Quoting mit Leerzeichen, Pfad-Traversal wird abgelehnt.
- Icon-Auflösung im User-Theme.
- Bildimport: PNG kopieren, defekte Bilder ablehnen, 3:4-Center-Crop, Icon-Fit.
- Persistenz: atomisches Schreiben, keine Temp-Reste, Roundtrip, Schema-zu-neu wird abgelehnt, beschädigte Library/Settings/Secrets werden gesichert und nicht überschrieben, Reset mit Backup.
- PIN: nur Ziffern 4–12, Argon2id-Hash enthält nicht die Klartext-PIN, Verifikation, `0600`.
- Controller: Button-Mapping (inkl. LB/RB = Schultertasten), Deadzone/Hysterese, Initial- und Hold-Repeat, Rückkehr von D-Pad zu Stick.
- Fokus-Gating: Controller-Navigation wird nur bei fokussiertem CouchDeck-Fenster akzeptiert; beim Fokuswechsel wird gepufferte Navigation verworfen.
- Admin-/Controller-Fokus: zyklische Fokusreihenfolge, Fokus-Start, und injizierter Enter-Event aktiviert den fokussierten Button.
- Autostart v0.2.5: KDE-only-Desktop-Entry, Quoting eines Binary-Pfads mit Leerzeichen, `--autostart`, Deaktivierung/Entfernen.
- Power v0.2.5: feste, shellfreie `systemctl --no-block --no-wall reboot|poweroff`-Argumente und Labels.
- i18n v0.3.0: Locale-Erkennung (`de_DE`, `de:en`, `en_US`, `fr_FR` → Englisch), Live-Wechsel ohne Neustart, Englisch-Fallback für eine nur in `en.ftl` vorhandene Nachricht.
- Zeitkontingent v0.3.0: Zusatzminuten addieren sich, Nutzung wird am Budget gedeckelt, Tageswechsel setzt Nutzung/Zusatz/Warnflags zurück, Warnflags re-armieren nach Zusatzzeit, Reset der Zusatzzeit lässt Nutzung stehen, Speichern/Laden (Neustart), beschädigte `timekeeping.json` wird gesichert.
- Eltern-/Kindersperren v0.3.0: beliebige Nicht-Kind-Admin-PIN wird akzeptiert (Spieler-PIN nicht), Profilwechsel aus Kinderprofil erfordert ParentAuth, erschöpftes Budget blockiert neue Starts.
- Datenkompatibilität v0.3.0: Profile/Settings aus v0.2.5 ohne neue Felder werden mit Defaults geladen (`background=null`, `is_child=false`, `daily_limit_minutes=null`, `language=auto`, `alarm_sound=true`).
- GIO-Integrationstest: Desktop-Datei mit Leerzeichen im Pfad und gequotetem `Exec` wird über `gio launch` gestartet; fehlende Datei liefert einen Fehler.

## Manuelle UI-Smoke-Tests

Die UI wurde zusätzlich mit dem X11-Backend (Xwayland) gestartet, damit Fensterinhalte reproduzierbar per Window-Capture geprüft werden konnten. Die App wurde dabei mit isolierten XDG-Verzeichnissen unter `/tmp` betrieben; echte Benutzerdaten wurden nicht verändert.

| Szenario | Ergebnis |
|---|---|
| Start in der Bibliothek des aktiven Profils | OK |
| Cover-Grid, Fokusrahmen, fehlende Desktop-Datei (`Datei fehlt`-Badge) | OK |
| Profilübersicht, aktives Profil, Avatare, Rollen | OK |
| Menü mit Profilwechsel/Admin/Import/Vollbild/Minimieren/Beenden | OK |
| Admin-Menü, Fokusring, PIN-Warnung | OK |
| Spieleeditor: Titel-Override, Cover/Background, Sichtbarkeit je Profil, Sortierung, Entfernen | OK |
| Import-Dialog: Liste, Quelle, Pfad, rohes `Exec`, Feldcode-Hinweis, Import | OK |
| Dateibrowser: Navigation, Pfadeingabe, Bildimport | OK |
| Cover-/Hintergrundimport: Datei kopiert, Bibliothek/Settings gespeichert | OK |
| PIN setzen: On-Screen-Pad, Bestätigung, `secrets.json` mit `0600` und Argon2id | OK |
| Admin entsperren mit PIN | OK |
| `gio launch` eines Test-Desktop-Eintrags | OK, Exit 0; Overlay zeigt bewusst keinen „läuft“-Status |
| Start eines Eintrags mit fehlender Desktop-Datei | OK, Fehlerdialog |
| Beschädigte `library.json` | OK, Fatal-Screen, Backup-Datei angelegt, Original unverändert |
| Fehlendes `gio` (PATH ohne GIO) | OK, Start-Hinweis, App bleibt bedienbar, Start wird blockiert |
| Controller-Thread ohne angeschlossenen Controller | OK, Log „Kein Controller beim Start gefunden“, kein Crash |
| Profilfilterung: Start mit Profil `Mia` | OK, nur für Mia sichtbare Spiele, Header zeigt Mia/Spieler |
| Install-/Uninstall-Skript in temporärem `$HOME` | OK, Binary/Desktop/Icon installiert bzw. entfernt; Daten bleiben erhalten |
| Autostart-Datei erzeugen/entfernen (Unit-Test, v0.2.5) | OK; KDE-only, Pfad mit Leerzeichen korrekt gequotet |
| Autostart-Schalter in Admin → Einstellungen (UI, v0.2.5) | OK; Status, Pfad und Entwicklungs-Binary-Warnung sichtbar |
| Power-Menü mit Neustart/Herunterfahren (UI, v0.2.5) | OK; Bestätigungsdialoge vorhanden, keine echte Power-Aktion ausgeführt |
| Power-Flow mit Fake-`systemctl` (v0.2.5) | OK; Argumente `--no-block --no-wall poweroff`, App beendet sich nach Erfolg |
| Minimize mit `--windowed` unter X11/Xwayland (v0.2.5) | OK; Fensterzustand `_NET_WM_STATE_HIDDEN` |
| Controller-Fokus-Gating / LB/RB-Mapping (v0.2.5) | Unit-Tests grün; Fokusverlust/-gewinn unter X11/Xwayland geloggt; Hardwaretest mit echtem Controller offen |
| Power-Kommandokonstruktion (Unit-Test, v0.2.5) | OK; feste `systemctl`-Argumente, keine Shell |
| Autostart/Minimize/Power in echter KDE-Anmeldesitzung | Nicht ausgeführt (Zielhardware-Test offen) |
| Englische UI per `language = "en"` (v0.3.0) | OK; Header, Bibliothek, Zeitbudget-Dialog und Hinweise auf Englisch |
| Zeitkontingent verbraucht (v0.3.0) | OK; Overlay „Time budget exhausted“, Request-/Back-Buttons, Profilname und Restzeitstatus |
| Profil-Hintergrund-Rendering (v0.3.0) | OK; generiertes Hintergrundbild über `library.json` geladen und in der deutschen UI angezeigt; Dateiauswahl-UI noch nicht durchgeklickt |
| Zusatzzeit-Dialog mit Admin-PIN (v0.3.0) | UI/Logik implementiert; im Screenshot-Test war absichtlich keine Admin-PIN gesetzt, daher nicht bis zur Freigabe durchgeklickt |
| 8BitDo Ultimate unter Manjaro/Xwayland (v0.3.0) | Erkannt (`Controller: 8BitDo Ultimate`); reale Navigation/Modi auf Bazzite noch nicht dokumentiert |
| tar.gz-Release (v0.3.1) | OK im Fedora-42-Container; Layout, Installer, `--version`, SHA256SUMS geprüft |
| AppImage-Release (v0.3.1) | OK; `--version` und Start unter Xvfb bis zur Bibliothek, keine fehlenden Bibliotheken |
| ldd/glibc-Report (v0.3.1) | OK; Build-glibc 2.41, max. referenziertes Symbol `GLIBC_2.39`, keine `not found`-Einträge |
| Installation/Update in temporärem `$HOME` (v0.3.1) | Installer installiert Binary/Entry/Icon; XDG-Datenordner werden nicht angefasst |
| AppImage/Archiv auf frischer Bazzite-KDE-Installation (v0.3.1) | **Nicht ausgeführt**; manuelle Checkliste in `docs/BAZZITE-RELEASE-TEST.md` |

Screenshots: [Zeitkontingent verbraucht](screenshots/time-budget.png), [Bibliothek](screenshots/library.png), [Bibliothek als Mia](screenshots/library-profile-mia.png), [Profile](screenshots/profiles.png), [Admin](screenshots/admin.png), [Spieleeditor](screenshots/game-editor.png), [Import](screenshots/import.png), [PIN](screenshots/pinpad.png), [Start-Overlay](screenshots/launch.png), [Power-Menü](screenshots/menu.png), [Autostart-Einstellungen](screenshots/autostart-settings.png), [Fatal](screenshots/corrupt-data.png).

Die Screenshots verwenden fiktive Demo-Spiele und Demo-Cover aus der Testumgebung. Sie zeigen keinen Shop und keine echten Spieleplattform-Daten.

## Release-Distribution (v0.3.1)

- GitHub-Actions-Workflow `.github/workflows/release.yml` für Tags `v*` und Workflow-Dispatch.
- Build und Tests laufen in einem frischen Fedora-42-Container; das Release-Binary wird ohne Rust-Toolchain weiterverpackt.
- `scripts/fetch-release-tools.sh` lädt ein gepinntes linuxdeploy-Build-Werkzeug mit SHA256-Prüfung und stellt daraus `appimagetool` bereit.
- Das AppImage enthält die unveränderte native Binary sowie Desktop-/Icon-Metadaten; glibc, GPU-Treiber, Wayland/X11 und `libudev` kommen von Bazzite. Dadurch keine gebündelten System-ABI-Konflikte.
- `SHA256SUMS` wird nach dem Packaging aus den finalen Dateien erzeugt.
- AppImage und Archiv-Binary wurden mit `env -i PATH=/usr/bin:/bin` (ohne Cargo/Toolchain im Suchpfad) per `--version` gestartet.
- Der echte Bazzite-KDE-Zielhardware-Test (Controller, GIO-Spielstart, Systemaktionen, AppImage/Archiv) ist noch offen und in `docs/BAZZITE-RELEASE-TEST.md` beschrieben.

## Idle-/Performance-Messung

Release-Build, sichtbares Fenster, 11 Bibliothekseinträge, 10 Cover, globales Hintergrundbild, keine Eingaben:

```text
PID     ELAPSED  %CPU  %MEM   RSS    STAT
2475074 01:37    0.4   0.1    256192 Sl

CPU-Ticks über 5 Sekunden: 0
```

- Keine Repaint-Schleife im Idle; `request_repaint_after` wird nur bei Animation, Toast, laufendem Startvorgang oder Controller-Event gesetzt. Bei aktivem Kinderprofil mit laufendem Budget wird bewusst alle 5 Sekunden neu gezeichnet, damit Restzeit/Warnungen/Tageswechsel aktuell bleiben.
- Der Gamepad-Thread pollt `gilrs` alle 10 ms, weckt egui aber nur bei tatsächlichen Events.
- RSS ca. 256 MB mit 10 geladenen Covern und einem 2560×1440-Hintergrund (Release). Das ist kein optimiertes Speicherziel, aber für den MVP unkritisch.

## Noch nicht getestete Punkte

Diese Punkte sind **nicht** als bestanden zu werten:

1. **Bazzite-Installation und -Start**: Build/Test erfolgte auf Manjaro, nicht in einer Distrobox auf Bazzite.
2. **Wayland-Fullscreen auf KDE Plasma**: Der Code setzt `with_fullscreen(true)` und rahmenlose Dekorationen; die App startete in einer Wayland-Session, aber Fullscreen-Verhalten, Fokus und Rückkehr wurden nicht auf der Zielhardware verifiziert.
3. **Reale Controller-Hardware**: Ein 8BitDo Ultimate wurde unter Manjaro erkannt; USB-/Bluetooth-/XInput-Modus, D-Pad/Stick, LB/RB, Hotplug, Reconnect und mehrere Controller auf Bazzite bleiben offen.
4. **Kinderprofil-/Zeitbudget-Hardwaretest**: TimeExpired-Screen und englische UI wurden per Xwayland-Screenshot geprüft. Echte Admin-PIN-Freigabe, Alarmton, Tageswechsel über Mitternacht, Minimize-/Close-Sperren und 4K/DPI sind auf Bazzite offen.
5. **Profilhintergründe**: Persistenz/Fallback sind implementiert und durch Deserialisierung/Code abgesichert; ein manueller Bildimport mit echtem Hintergrund wurde noch nicht durchgeklickt.
4. **4K/DPI-Skalierung**: Skalierung ist implementiert (Faktor aus Fensterhöhe), aber nicht auf einem 4K-TV getestet.
5. **Steam- und Prism-Flatpak-Start**: Flatpaks waren in der Testumgebung nicht installiert. Erkennung, Diagnose und Erzeugung der verwalteten Dateien sind getestet; der echte Flatpak-Start ist offen.
6. **Spiel-Rückkehr/Fokus**: Ob Plasma das gestartete Spiel zuverlässig in den Vordergrund holt und CouchDeck danach wieder erreichbar ist, muss auf Bazzite geprüft werden.
7. **Steam Input**: Konflikte mit aktivem Steam Input wurden nicht getestet.
8. **Minimieren unter Wayland**: Menüpunkt und `ViewportCommand::Minimized` sind implementiert, aber nicht auf Plasma/Wayland verifiziert.

## Hardware-Testanleitung für Bazzite

1. Auf dem Host die Fedora-Version ermitteln und eine passende Distrobox verwenden.
2. Im Container `cargo build --release`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` ausführen.
3. Auf dem Host `./scripts/install-user.sh` ausführen (ohne sudo).
4. CouchDeck aus der KDE-Anwendungsliste starten. Erwartung: rahmenloses Vollbild, Bibliothek sichtbar, Profil des letzten Starts aktiv.
5. Mit `--windowed` gegenprüfen, ob ein normales Fenster mit Dekoration erscheint.
6. Controller anschließen: D-Pad und linken Stick durch alle Kacheln bewegen; A startet; B schließt Dialoge; Start öffnet das Menü; LB/RB in der Profilübersicht.
7. Controller im Betrieb trennen und wieder verbinden. Erwartung: keine Crashs, erneute Eingabe möglich.
8. Ein Spiel starten. Prüfen: Spielfenster erhält Fokus, CouchDeck bleibt erreichbar/minimierbar, kein „Spiel beendet“-Status.
9. Steam-Flatpak-Spiel und Prism-Instanz über echte `.desktop`-Dateien importieren und starten.
10. Einen fehlerhaften Steam-Shortcut importieren, Reparatur bestätigen, prüfen, dass das Original unverändert bleibt und der neue Eintrag startet.
11. 4K-Auflösung/DPI testen: Lesbarkeit, Grid-Spalten, Fokusvergrößerung.
12. Idle-CPU mit `pidstat`/`ps` beobachten.
