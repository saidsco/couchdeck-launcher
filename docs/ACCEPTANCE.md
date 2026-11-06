# Definition of Done — Status

Legende:
- `[x]` implementiert und in dieser Umgebung geprüft
- `[~]` implementiert; Prüfung auf Bazzite-Zielhardware steht aus
- `[ ]` nicht umgesetzt oder nicht geprüft

## Technisch

- [ ] Reproduzierbarer Build in Distrobox auf passendem Fedora-Release — Befehle dokumentiert, aber nicht auf Bazzite ausgeführt (Entwicklungsumgebung: Manjaro).
- [x] `cargo fmt --check` erfolgreich.
- [x] `cargo clippy --all-targets -- -D warnings` erfolgreich.
- [x] `cargo test` erfolgreich: 45 Unit-Tests + 2 GIO-Integrationstests.
- [x] `cargo build --release` erfolgreich.
- [~] Wayland/KDE Fullscreen + `--windowed` — Fensterbetrieb geprüft; Wayland-Fullscreen und KDE-Fokus auf Zielhardware offen.
- [x] Install/Uninstall ohne sudo, kein `rpm-ostree`-Layer — in temporärem `$HOME` getestet.
- [x] Installationsskript installiert Binary, XDG-Desktop-Entry und Icon.
- [x] Optionaler KDE-Autostart über `install-user.sh --autostart` oder Admin → Einstellungen.
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
- [x] Name, Desktop-Icon, eigenes Cover, optionaler Spiel-/Global-Hintergrund.
- [x] Fallback: eigenes Cover → Desktop-Icon → neutrale Kachel.
- [x] Fehlende Desktop-Datei bleibt als Eintrag sichtbar und zeigt „Datei fehlt“.
- [x] Grid per Tastatur und Maus navigierbar; Gamepad-Logik unit-getestet.
- [~] Gamepad-Navigation auf realer Hardware.

## Controller/UX

- [x] D-Pad + linker Stick mit Deadzone/Hysterese und Hold-Repeat implementiert und unit-getestet.
- [x] A/South = bestätigen/starten, B/East = zurück, Start = Menü, LB/RB = Profilübersicht.
- [x] Hotplug-Events (Connected/Disconnected) implementiert.
- [x] Doppelstart-Schutz: Start-Overlay, Cooldown, laufender Start blockiert weitere Starts.
- [x] Tastatur und Maus parallel bedienbar.
- [x] Lesbarkeit bei 1600×900/1080p-Klasse geprüft.
- [~] 4K/DPI-Skalierung: Code vorhanden, Zielhardware-Test offen.
- [x] Fokusanimation ca. 180 ms; keine Repaint-Schleife im Idle.
- [x] Idle-CPU-Messung: 0 Ticks über 5 s im Release-Build.
- [~] Controller-Hotplug/Reconnect mit echtem Gerät.
- [x] „Zu KDE minimieren“: Fullscreen verlassen, Dekorationen wiederherstellen, Fenster minimieren; Wayland-Verhalten offen.
- [~] Autostart als KDE-only XDG-Eintrag; Entry-Erzeugung/Entfernen unit-getestet, echte Anmeldesitzung offen.
- [x] Beenden über Menü/Strg+Q beendet nur den Launcher; laufende Spiele werden nicht beendet.
- [~] System neu starten/herunterfahren mit Bestätigung und festen `systemctl`-Aufrufen; Ausführung auf Zielhardware offen.

## Verwaltung

- [x] Lokale Profile mit Anzeigename, Rolle und Avatar.
- [x] Profil-spezifische Sichtbarkeit von Spielen.
- [x] Admin-PIN als Argon2id-Hash, separate Datei mit `0600`.
- [x] Controllerbedienbares On-Screen-Ziffernfeld für PIN-Eingabe.
- [x] Nicht-Admin-Profile sehen keine Admin-/Import-/Bearbeitungsfunktionen.
- [x] Spiele importieren, ausblenden, entfernen, Titel ändern, Cover/Hintergrund ändern.
- [x] Profil hinzufügen/entfernen, Rolle ändern (letzter Admin geschützt).
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
- [ ] Rückkehr und Fokus nach Spielstart/-ende auf echtem KDE/Wayland geprüft.

## Nicht Teil des MVP

- Kein eigener Store, keine Steam-API, keine automatische Cover-Suche/Web-Scraping.
- Keine Accounts, kein Sync, keine Cloud.
- Keine eigene Gamescope-/Login-Session.
- Kein systemweiter Autostart ohne Opt-in.
- Kein Flatpak-Paket für CouchDeck selbst.
- Favoriten-UI nicht implementiert (Datenfeld reserviert).
- SVG-Desktop-Icons werden nicht rasterisiert.
