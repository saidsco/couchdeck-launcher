# Definition of Done — Status

Legende:
- `[x]` implementiert und in dieser Umgebung geprüft
- `[~]` implementiert; Prüfung auf Bazzite-Zielhardware steht aus
- `[ ]` nicht umgesetzt oder nicht geprüft

## Technisch

- [ ] Reproduzierbarer Build in Distrobox auf passendem Fedora-Release — Befehle dokumentiert, aber nicht auf Bazzite ausgeführt (Entwicklungsumgebung: Manjaro).
- [x] `cargo fmt --check` erfolgreich.
- [x] `cargo clippy --all-targets -- -D warnings` erfolgreich.
- [x] `cargo test` erfolgreich: 77 Unit-Tests + 2 GIO-Integrationstests.
- [x] `cargo build --release` erfolgreich.
- [~] Fullscreen unter KDE/GNOME/XFCE (Wayland/X11) + `--windowed` — Fensterbetrieb auf KDE/Xwayland geprüft; andere Sitzungen offen.
- [x] Install/Uninstall ohne sudo, kein `rpm-ostree`-Layer — in temporärem `$HOME` getestet.
- [x] Installationsskript installiert Binary, XDG-Desktop-Entry und Icon.
- [x] Optionaler desktopbezogener XDG-Autostart (KDE/GNOME/XFCE) über `install-user.sh --autostart` oder Admin → Einstellungen; Gamescope/unbekannte Sitzungen ausgeschlossen.
- [x] Uninstall entfernt Binary/Entry/Icon/Autostart; `--purge` löscht Benutzerdaten nur nach expliziter Bestätigung.
- [x] Fehlermeldung bei fehlender Desktop-Datei, fehlendem `Exec`, ungültigem Bild, fehlerhaftem JSON, fehlendem GIO (Code-/UI-Pfad).
- [x] Beschädigte Bibliothek: Fatal-Screen, Backup, keine stille Überschreibung.
- [x] Atomare Schreibvorgänge für Bibliothek, Settings und Secrets.

## Bibliothek

- [x] Start direkt in der Bibliothek des letzten Profils, kein Shop.
- [x] XDG-Discovery und Import beliebiger `.desktop`-Dateien.
- [x] Admin-Vorschau mit Name, Pfad und rohem `Exec`-Inhalt vor der Freigabe.
- [x] Quell-Desktop-Dateien werden nie verändert; verwaltete Dateien liegen separat.
- [x] Start ausschließlich via `gio launch /absoluter/pfad.desktop`; kein eigener Exec-Parser.
- [x] GIO-Erfolg wird nicht als „Spiel läuft“ interpretiert.
- [x] Name, Desktop-Icon, eigenes Cover, optionaler Spiel-/Global-/Profil-Hintergrund.
- [x] Profil-Hintergrund mit Fallback auf globales Hintergrundbild; vorhandene Spiel-/Global-Hintergründe bleiben erhalten.
- [x] Profil-Hintergrund importieren/entfernen ohne Änderung der Desktop-Datei.
- [x] Fallback: eigenes Cover → Desktop-Icon → neutrale Kachel.
- [x] Fehlende Desktop-Datei bleibt als Eintrag sichtbar und zeigt „Datei fehlt“.
- [x] Grid per Tastatur und Maus navigierbar; Gamepad-Logik unit-getestet.
- [~] Gamepad-Navigation auf realer Hardware.

## Controller/UX

- [x] D-Pad + linker Stick mit Deadzone/Hysterese und Hold-Repeat implementiert und unit-getestet.
- [x] A/South = bestätigen/starten, B/East = zurück, Start = Menü, LB/RB = Profilübersicht.
- [x] LB/RB korrekt auf die Schultertasten (`Button::LeftTrigger`/`RightTrigger`) gemappt; Unit-Test vorhanden.
- [x] Navigation nur bei fokussiertem CouchDeck-Fenster; Connect/Disconnect weiterhin verarbeitet, Eingabepuffer bei Fokuswechsel verworfen.
- [x] Hotplug-Events (Connected/Disconnected) implementiert.
- [x] Doppelstart-Schutz: Start-Overlay, Cooldown, laufender Start blockiert weitere Starts.
- [x] Tastatur und Maus parallel bedienbar.
- [x] Lesbarkeit bei 1600×900/1080p-Klasse geprüft.
- [~] 4K/DPI-Skalierung: Code vorhanden, Zielhardware-Test offen.
- [x] Fokusanimation ca. 180 ms; keine Repaint-Schleife im Idle.
- [x] Idle-CPU-Messung: 0 Ticks über 5 s im Release-Build.
- [~] Controller-Hotplug/Reconnect mit echtem Gerät.
- [x] „Zum Desktop minimieren“: Fullscreen verlassen, Dekorationen wiederherstellen, Fenster minimieren; Wayland-Verhalten offen.
- [~] Autostart desktopbezogen; Sitzungserkennung und Entry-Erzeugung/Entfernen unit-getestet, echte GNOME/XFCE-Anmeldesitzungen offen.
- [x] Beenden über Menü/Strg+Q beendet nur den Launcher; laufende Spiele werden nicht beendet.
- [~] System neu starten/herunterfahren mit Bestätigung und festen `systemctl`-Aufrufen; Ausführung auf Zielhardware offen.

## Internationalisierung

- [x] Deutsche und englische UI über Fluent.
- [x] Automatische Spracherkennung aus `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG`, Englisch als Fallback.
- [x] Sprache live unter Admin → Einstellungen umschaltbar, ohne Neustart.
- [x] v0.2.5-Settings ohne Sprachfeld laden mit `auto`.
- [~] Vollständige Sichtprüfung aller Dialoge/Diagnosetexte auf Zielhardware offen; technische Diagnosetexte aus Desktop-Parsing bleiben teilweise deutsch.

## Kinderprofile und Zeitkontingente

- [x] Optionales Kinderprofil mit täglichem Zeitbudget.
- [x] Aktive Profilzeit wird gemessen, solange CouchDeck läuft; Persistenz in `timekeeping.json`.
- [x] Restzeitanzeige, 5-/1-Minuten-Warnungen, optionaler Best-Effort-Alarmton.
- [x] Zentrale Startprüfung für neue Spiele/Medien; laufende Spiele werden nicht beendet.
- [x] Tageswechsel setzt Nutzung/Zusatzzeit/Warnflags zurück.
- [x] „Mehr Spielzeit anfragen“ im Startmenü und auf dem Verbraucht-Screen.
- [x] Admin-PIN pro Freigabe, +5/+15/+30/benutzerdefiniert, Controller-bedienbar.
- [x] Zusatzzeit nur für den aktuellen Kalendertag, additiv, persistent, unabhängig vom regulären Budget.
- [x] Admin kann Zusatzzeit einsehen und zurücksetzen.
- [x] Kinderprofile ohne Admin-UI-Zugriff; Profilwechsel/Minimieren/Beenden/Systemaktionen mit ParentAuth.
- [x] ParentAuth entsperrt den Admin-Bereich nicht dauerhaft.
- [x] Tests für Zusatzzeit, Neustart, Tageswechsel, PIN-Prüfung, Profilwechsel und Start-Sperre.
- [~] Echte Zeitmessung über einen langen Zeitraum, Mitternachtswechsel, WM-Close/-Minimize und Alarmton auf Bazzite offen.
- [x] Grenzen dokumentiert: keine Spielprozess-Erkennung, kein Schutz gegen Prozess-Kill/TTY/Dateiänderungen im selben Linux-Konto.

## Verwaltung

- [x] Lokale Profile mit Anzeigename, Rolle, Avatar und optionalem Profil-Hintergrund.
- [x] Profil-spezifische Sichtbarkeit von Spielen.
- [x] Admin-PIN als Argon2id-Hash, separate Datei mit `0600`.
- [x] Controllerbedienbares On-Screen-Ziffernfeld für PIN-Eingabe.
- [x] Nicht-Admin-Profile sehen keine Admin-/Import-/Bearbeitungsfunktionen.
- [x] Spiele importieren, ausblenden, entfernen, Titel ändern, Cover/Hintergrund ändern.
- [x] Profil hinzufügen/entfernen, Rolle ändern (letzter Admin geschützt).
- [x] Kinderprofil-Schalter, Tageslimit, heutige Nutzung/Zusatzzeit, Zusatzzeit-Reset.
- [x] Letzte Admin-PIN kann nicht entfernt werden, solange Kinderprofile mit Budget existieren.
- [x] Speicherung über Neustart, atomar.
- [x] Admin-PIN ist klar als lokale UI-Sperre dokumentiert, nicht als OS-Sicherheitsgrenze.

## Integration

- [x] Prism-`--launch INSTANCE_ID`-Erkennung, Instanzordner-Prüfung und verwaltete Datei getestet.
- [x] Eigene Covergrafik für Prism-Einträge ohne Änderung der Desktop-Datei.
- [~] Prism-Flatpak tatsächlich starten (Flatpak in Testumgebung nicht installiert).
- [x] Steam-Flatpak-Shortcut-Erkennung und verwaltete Reparaturdatei; Original unverändert.
- [~] Steam-Flatpak tatsächlich starten (nicht installiert).
- [x] Manuell importiertes Nicht-Steam-Spiel startet über GIO (Integrationstest).
- [~] Fehlerhafter Steam-Exec wird erkannt, nicht still geändert, nach Zustimmung separat korrigiert; echter Steam-Flatpak-Test offen.
- [~] Manuelle Rückkehr via `--activate` unter X11 geprüft; Wayland-/GNOME-/XFCE-Fokus und echte Steam-/Launcher-Rückkehr offen.

## Desktop-/Discovery-Härtung (v0.4.0)

- [x] KDE-spezifische Laufzeitvoraussetzungen entfernt; KDE/GNOME/XFCE als unterstützte Desktop-Ziele dokumentiert.
- [x] „Zum Desktop minimieren“ in UI und Übersetzungen.
- [x] Sichere Sitzungserkennung aus mehrteiligen `XDG_CURRENT_DESKTOP`-Werten; Gamescope/Gaming Mode ausgeschlossen.
- [x] Autostart an erkannte KDE-/GNOME-/XFCE-Sitzung gebunden, inklusive `NotShowIn=gamescope`.
- [x] Installer `--autostart` verweigert unbekannte Sitzungen und Gamescope.
- [x] Single-Instance-`flock` mit automatischer Freigabe bei Abstürzen.
- [x] Discovery aus XDG_DATA_HOME, XDG_DATA_DIRS, Flatpak-Benutzer/System, XDG-Desktopordner, Managed und Custom-Pfaden.
- [x] XDG-Prioritäten/Desktop-IDs, Symlinks, Leerzeichen, `Hidden`, `NoDisplay`, `OnlyShowIn`, `NotShowIn`, `TryExec` behandelt.
- [x] Admin-Ansicht für zusätzliche Suchpfade mit Hinzufügen/Entfernen/Rescan und Fehleranzeige.
- [x] Importübersicht mit Quelle, Quellverzeichnis, Desktop-ID, Gruppierung und Diagnostik.
- [x] Neue Importe standardmäßig nicht für Kinderprofile sichtbar.
- [x] `GameEntry` um Herkunft/Startmethode/Desktop-ID erweitert; 0.3.1-Daten bleiben lesbar.
- [~] GNOME/Wayland, XFCE/X11 und SteamOS Desktop Mode nicht real getestet; Matrix in `docs/DESKTOP-TEST-MATRIX.md`.
- [~] Admin-Discovery-UI nicht vollständig manuell durchgeklickt.

## Bugfixes v0.4.1

- [x] Installer liefert PNG-Icons (256/512) und SVG; Desktop-/Autostart-Eintrag nutzt absoluten PNG-Pfad.
- [x] Uninstaller entfernt SVG- und PNG-Icons.
- [x] Fullscreen-Wiederherstellung nach „Zum Desktop minimieren“ implementiert und per Unit-Test für die Zustandslogik abgesichert.
- [~] Sichtprüfung des Icons und echtes Taskleisten-Minimieren/Wiederherstellen auf Bazzite/GNOME/XFCE offen.

## Fensteraktivierung (v0.4.5)

- [x] `couchdeck --activate` existiert und kontaktiert die laufende Instanz über lokalen Unix-Socket.
- [x] Kein zweiter UI-Prozess; Single-Instance-Lock bleibt maßgeblich.
- [x] Socket `0600` und `SO_PEERCRED`-Prüfung auf gleiche lokale UID.
- [x] Veralteter Socket wird ersetzt; fehlende Instanz liefert Exit-Code ungleich 0.
- [x] Minimiertes Fenster wird wiederhergestellt; vorheriger Fullscreen wird wiederhergestellt; Windowed bleibt Windowed.
- [x] Fokussiertes CouchDeck erhält keine störende Aktion.
- [x] Spielstart fordert keinen Fokus automatisch zurück.
- [x] Fokus-/Minimierungs-/Fullscreen-/IPC-Logging und `--diagnose-window`.
- [x] X11-Smoke-Tests für fokussiert, minimiert, Fullscreen-minimiert, mehrfache Aufrufe und No-Instance.
- [~] Wayland-/Bazzite-/GNOME-/XFCE-Fokusverhalten und echte Steam-/Proton-/Launcher-Tests offen.

## Release-Distribution (v0.4.5-Vorbereitung)

- [x] GitHub-Actions-Workflow für Tags `v*` und manuellen Dispatch.
- [x] Portables x86_64-`.tar.gz` mit Binary, Installer, Doku und Runtime-Report.
- [x] Ausführbares x86_64-AppImage ohne Flatpak-Sandbox und ohne Root-Rechte.
- [x] `SHA256SUMS` für tar.gz, AppImage und Runtime-Report.
- [x] `ldd`-Prüfung und Dokumentation von Build-glibc sowie maximaler GLIBC-Symbolversion.
- [x] Release-Pakete laufen ohne installierten Rust-Compiler; Build-Tools nur im CI-Container.
- [x] Installation über `~/.local/bin/couchdeck` und XDG-Desktop-Entry weiterhin unterstützt.
- [x] Installer überschreibt keine Profile, Artwork oder Einstellungen; Update ist derselbe Aufruf.
- [x] AppImage/Archiv im Fedora-42-Container gebaut, per `--version` und Xvfb-Start geprüft.
- [x] GitHub Release `v0.3.1` mit allen Assets veröffentlicht; Download und SHA256-Prüfung erneut verifiziert.
- [~] AppImage und Archiv auf einer frischen Bazzite-KDE-Installation inkl. Controller, GIO-Spielstart und Systemaktionen getestet — manuelle Checkliste offen.
- [x] Release-Metadaten, Installer, AppImage-Skripte und Workflow auf 0.4.5-Kompatibilität geprüft; lokales 0.4.5-Paket im Fedora-42-Container gebaut und geprüft.
- [~] Kein v0.4.5-Tag/Release/Push ohne ausdrückliche Freigabe erzeugt.

## Nicht Teil des MVP

- Kein eigener Store, keine Steam-API, keine automatische Cover-Suche/Web-Scraping.
- Keine Accounts, kein Sync, keine Cloud.
- Keine eigene Gamescope-/Login-Session.
- Kein systemweiter Autostart ohne Opt-in.
- Kein Flatpak-Paket für CouchDeck selbst; stattdessen portables tar.gz und natives AppImage.
- Favoriten-UI nicht implementiert (Datenfeld reserviert).
- SVG-Desktop-Icons werden nicht rasterisiert.
