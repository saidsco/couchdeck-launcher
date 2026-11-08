# CouchDeck — Coding-Agent-Auftrag

## Rolle und Arbeitsauftrag
Du bist ein erfahrener Rust-/Linux-Desktop-Entwicklungsagent. Implementiere ein quelloffenes, controller-first, fullscreenfähiges Game-Launcher-MVP für **Linux-Wohnzimmer-PCs**. Der Launcher startet direkt in eine persönliche Spielebibliothek. Kein Shop, keine News, keine Accounts von Spieleplattformen, kein Overlay. Arbeite inkrementell, führe Tests aus und dokumentiere reale Testergebnisse. Frage nur bei tatsächlich blockierenden Unklarheiten; verwende sonst die Vorgaben in diesem Paket.

**Produktziel:** „CouchDeck is a family-friendly, controller-first game launcher for Linux living-room PCs, independent of a specific distribution or desktop environment.“

**Projektname:** CouchDeck (Arbeitstitel; Namenskonflikte vor Veröffentlichung prüfen).
**Technologie:** Rust stable, eframe/egui, gilrs, serde + JSON, toml, standard/XDG-Pfade, GLib/GIO-Desktop-Launching über `gio launch`. Wähle gepflegte kompatible Versionen, lege `Cargo.lock` ab. Bevorzuge möglichst wenig Abhängigkeiten. Kein Web-Frontend, Electron, Python, Datenbankserver, sudo im Produkt oder Netzwerkzwang.

**Zielumgebungen:** KDE Plasma, GNOME und XFCE unter Wayland oder X11, soweit die jeweilige Sitzung und eframe/egui es unterstützen. Bazzite und SteamOS im Desktop Mode sind Zielplattformen, aber keine technischen Voraussetzungen. Steam Gaming Mode/Gamescope ist derzeit **nicht** zugesichert.

## Nicht verhandelbare Regeln
1. **Native Benutzeranwendung**. Kein Flatpak für CouchDeck selbst. Build in einem zur Ziel-Fedora-Version passenden Container; Installation nach `~/.local/bin/` und XDG Desktop Entry; keine `rpm-ostree`-Layer, kein root erforderlich. Release-Pakete: portables tar.gz und AppImage, ohne Rust-Toolchain auf dem Zielsystem.
2. **Desktop-Dateien sind die Startquelle**. Unterstütze `.desktop`-Dateien aus XDG-Anwendungsordnern, Flatpak-Exports, dem tatsächlichen XDG-Desktopordner, benutzerdefinierten Suchverzeichnissen und CouchDeck-verwalteten Dateien. Desktop-Entry-Spezifikation beachten (Name, Icon, Exec, NoDisplay, Hidden, TryExec, Path, DBusActivatable, OnlyShowIn, NotShowIn, Kategorien, lokalisierte Werte). Für das tatsächliche Starten `gio launch /absoluter/pfad/datei.desktop` benutzen; **niemals** `Exec=` per eigenem `split(' ')`, `eval`, `sh -c` o. ä. interpretieren. Rust-`Command` mit separaten Argumenten, nie dynamische Shell-Konkatenation. `gio launch`-Fehler anzeigen.
3. **Artwork unabhängig von `Icon=`!** Pro Bibliothekseintrag lassen sich eigenes Cover und optional eigener Hintergrund festlegen. Speichere diese Overrides in CouchDeck, **ohne die ursprüngliche `.desktop`-Datei zu verändern**. Bilder per Dateiauswahl importieren und in XDG_DATA_HOME/couchdeck/assets/ kopieren; PNG/JPEG/WebP, sichere Namen, skalieren/cachen, Fehler-Fallback. Bei keinem Cover: Desktop-Icon (absoluter Pfad oder Icon-Theme-Name), danach neutrale Kachel.
4. **Controller-first**. D-Pad und linker Stick für Fokus; A bestätigen/starten; B zurück; LB/RB Profilwechsel (nur in Profilübersicht oder wie im UI angezeigt); Start öffnet Menü; Tastatur und Maus parallel bedienbar. Deadzone, Hold-Repeat, Debouncing, Hotplug, Wechsel zwischen Controllern. Test mit einem realen Controller und Wayland/X11, falls möglich; andernfalls genaue Hardwaretest-Anleitung und Einschränkungen dokumentieren.
5. **Fullscreen ab App-Start** in der aktiven Desktop-Sitzung. Fensterrahmenlos, Focus State jederzeit klar, Overlay- oder Settings-Zustände ohne Browserfenster; optional Windowed-Debug-Schalter `--windowed`. Unter Wayland können Compositor und Fokusregeln das Verhalten beeinflussen; dokumentiere Einschränkungen statt sie zu verbergen.
6. **Bibliothek**: Cover-Kacheln im Grid, 16:9- bzw. 2:3/3:4-Artwork per center crop, sanfte Fokusvergrößerung/-umrandung (150–200 ms), 60 fps während Animationen, niedrige CPU-Auslastung im Idle, optionales Hintergrundbild. App startet direkt in Bibliothek des zuletzt gewählten Profils; Erststart darf Profilwahl/Setup anbieten.
7. **Lokale Profile**: Avatar, Anzeigename, eigene Sichtbarkeit/Favoriten und Reihenfolge optional. Mindestens 1 Admin und beliebig viele normale lokale Profile. Keine Linux-Accounts, keine Cloud-Accounts, keine Netzwerkabhängigkeit. Admin kann neue Desktop-Dateien importieren, Covers/Hintergründe ersetzen und Einträge entfernen/ausblenden. Lokale Admin-PIN (Argon2id-Hash statt Klartext) gegen versehentliches Ändern; PIN ist **keine echte OS-Sicherheitsgrenze**; bei Zugriff aufs gleiche Linux-Benutzerkonto kann man Daten ändern. Admin-PIN-Eingabe auch mit Controller/On-Screen-Numpad möglich.
8. **Robust**: korrupte Daten nicht still überschreiben, vor Migration Backup, atomisches Schreiben für Bibliothek/Settings, valide Fehlermeldungen, Logging in geeigneten XDG-Pfaden, kein Crash bei fehlendem Cover, fehlenden Discovery-Verzeichnissen oder entfernten Desktop-Dateien. Discovery-Scans laufen nicht blockierend im UI-Thread und folgen keinen Symlink-Schleifen.
9. **Kein Steam-Scraping und kein Store**. Steam, Prism Launcher und andere Flatpaks über echte Desktop-Dateien starten; Spiel-Shortcuts aus Steam nicht grundsätzlich voraussetzen. Optionalen Reparaturdialog anbieten, falls `Exec=steam steam://rungameid/ID` bei Flatpak-Steam auf dem Host nicht funktioniert: nach ausdrücklicher Bestätigung eine **separate verwaltete Desktop-Datei** mit `flatpak run com.valvesoftware.Steam steam://rungameid/ID` generieren. Original nicht editieren; Steam-App-ID vorher plausibilisieren und Flatpak-Installation prüfen. Keine generischen Shell-Overrides ohne ausdrückliche Adminfreigabe.
10. **Prism Launcher**: Es muss möglich sein, direkt eine Instanz über eine `.desktop`-Datei mit `Exec=flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID` zu starten. Beim Import kann die eigene Covergrafik zugewiesen werden. `INSTANCE_ID` = Ordnername der Prism-Instanz, nicht beliebig der sichtbare Name.
11. **Focus/Minimieren auf Wayland und X11**: Erwarte, dass die Desktop-Sitzung das gestartete Spielfenster in den Vordergrund nimmt. Behaupte nicht, ein Spiel allein anhand der Rückkehr von `gio launch` als beendet erkennen zu können; `gio launch` ist nur der Starter, Steam/Prism können weiterlaufen. Kein ungefragtes Beenden laufender Game-Launcher. „Zum Desktop minimieren“ muss desktopunabhängig funktionieren; falls ein Wayland-Compositor das Minimieren ignoriert, bleibt das Fenster im Windowed-Modus über Taskleiste/Alt+Tab erreichbar.
12. **Sicherheitsmodell**: Eine `.desktop`-Datei kann beliebige Befehle ausführen. Zeige Admin beim Import Name, Pfad, Quelle, Desktop-ID und `Exec`-Inhalt zur Prüfung. Nur lokal explizit vom Admin freigegebene Einträge ausführbar. Neu gefundene Einträge werden **nie automatisch** in die Bibliothek übernommen und **nie automatisch für Kinderprofile freigeschaltet**. Keine ungeprüften Downloads.
13. **Desktopunabhängiger Autostart**: XDG-Autostart ist opt-in und an die erkannte Sitzung (KDE, GNOME, XFCE) gebunden. Unbekannte Sitzungen und Gamescope/Steam Gaming Mode erhalten keinen Autostart. Kein blindes Entfernen von `OnlyShowIn`-Beschränkungen.
14. **Single Instance**: Ein benutzerspezifischer Lock verhindert parallele UIs gegen dieselbe Bibliothek und dasselbe Zeitbudget. Der Lock muss bei Abstürzen automatisch freigegeben werden.
15. **Manuelle Fensteraktivierung (0.4.5)**: `couchdeck --activate` kontaktiert die laufende Instanz über einen lokalen, benutzergebundenen Unix-Socket. Aktivierung ist Best-Effort; sie darf keinen zweiten Prozess starten, keine Spiele/Launcher beenden und keine PIN-/Kinderrechte umgehen. Ein Spielstart fordert niemals automatisch den Fokus zurück. Fullscreen/Windowing-Zustand wird respektiert; Wayland-Compositor-Grenzen werden dokumentiert.

## Implementationsphasen
**Phase 0 – Probe**: Minimal-Binary fullscreen in der aktiven Desktop-Sitzung, Controller-Input mit gilrs auf echtem Gerät, `gio launch` für Test-Desktop-Datei. Dokumentiere potenzielle Steam-Input-Konflikte. Trenne Probleme der Game-Session (Gamescope/Steam Gaming Mode) vom Desktop.

**Phase 1 – Kern-MVP**: XDG-Desktop-Discovery, Import, Bibliotheksdatenmodell, Rendering mit Fallback-Icons, Keyboard/Controller-Navigation, Starten via GIO, Fehlerdialoge, persistente Konfiguration, Covers und Hintergründe manuell setzen, User-Installation.

**Phase 2 – Verwaltung**: Profile + lokale Admin-PIN, Admin-Menüs für Import/Ausblenden/Artwork, On-Screen-PIN-Pad, Steam-Flatpak-Shortcuts reparieren nach Bestätigung, eigene Prism-Instanz-Desktop-Datei generieren, Tests/Dokumentation.

**Phase 3 – Qualität**: Animationen, Performance-/Idle-Prüfung, Auflösungen 1080p und 4K, Xbox-/PlayStation-Controller, lokaler XDG-Autostart, Hintergrundänderung, Tastatur-/Mausbedienung, Release-Packaging.

**Phase 4 – Desktop-/Discovery-Härtung (0.4)**: generalisierter Autostart, XDG-/Flatpak-Discovery, konfigurierbare Suchpfade, erweiterte Importübersicht, zukunftsfähiges Spielemodell, Single-Instance-Schutz, Desktop-Testmatrix.

**Phase 5 – Fensteraktivierung (0.4.5)**: lokaler Aktivierungs-Socket, `--activate`, manuelle Desktop-Shortcuts, Fokusdiagnose, Wiederherstellung aus Minimierung/Fullscreen, keine automatische Fokusübernahme beim Spielstart.

## Abnahmefälle (müssen beschrieben und getestet werden)
- KDE Plasma, GNOME und XFCE unter Wayland/X11: Launcher startet fullscreen direkt in eigene Bibliothek; `--windowed` für Entwicklung; tatsächlich getestete Umgebungen werden klar von nur unterstützten getrennt.
- D-Pad/Stick kann beliebige Kacheln erreichen; A startet, B geht zurück, Menü funktioniert; Reconnect eines Controllers.
- Ein `.desktop`-Import erscheint sofort, nach Neustart weiterhin vorhanden, Launch auch mit Leerzeichen/Anführungszeichen in Exec korrekt durch GIO.
- XDG-Datenverzeichnis-Prioritäten, Flatpak-Exports, XDG-Desktopordner, verwaltete Dateien und zusätzliche Suchpfade werden gefunden; doppelte Desktop-IDs befolgen die XDG-Überlagerung.
- `NoDisplay`-Einträge sind nicht im normalen Desktop-Menü, aber nach bewusstem Admin-Import verwendbar. `Hidden`, `OnlyShowIn`, `NotShowIn` und fehlendes `TryExec` erzeugen verständliche Diagnosen.
- Neu gefundene Spiele werden nicht automatisch importiert oder für Kinderprofile freigeschaltet.
- Ausgewählte Cover-Datei ersetzt eine schlechte/fehlende `Icon=`-Darstellung ohne die importierte Datei zu editieren; Persistenz nach Neustart.
- Prism Flatpak kann eine spezifische Minecraft-Instanz via Desktop-Entry starten; bei fehlender/ungültiger Instanz erscheint eine verständliche Diagnose.
- Steam Flatpak wird als eigenständiger Eintrag gestartet. Ein bestehender fehlerhafter Steam-Game-Shortcut wird erkannt, **nicht still geändert**, und kann nach expliziter Zustimmung als verwaltete Variante aufgenommen werden.
- User ohne Admin kann keine Edit-Aktionen in der UI ausführen. Admin-PIN ist gehasht und mit Controller eingebbar. Profilwechsel zeigt unterschiedliche Auswahl/Cover wenn konfiguriert.
- Autostart wird nur für erkannte KDE-/GNOME-/XFCE-Sitzungen und nur nach expliziter Entscheidung erzeugt; Gamescope/unbekannte Sitzungen bleiben ausgeschlossen.
- Ein zweiter gleichzeitiger Start beendet sich mit Hinweis, ohne Bibliothek, Zeitbudget oder PIN zu beschädigen.
- `couchdeck --activate` erreicht eine laufende Instanz über lokale IPC; bei fehlender Instanz, veraltetem Socket oder verweigerter Aktivierung gibt es einen verständlichen Fehler und keine falsche Erfolgsmeldung.
- Ein Spielstart löst keine automatische Fokusübernahme aus; manuelle Aktivierung stellt Fullscreen nur wieder her, wenn CouchDeck zuvor fullscreen war.
- Entfernte `.desktop`-Datei, unlesbares Bild, fehlendes GIO, fehlendes Controllergerät und fehlerhaftes JSON erzeugen keine Abstürze.
- Benutzerinstallation und -deinstallation ohne root; `cargo test`, `cargo fmt --check`, `cargo clippy` laufen im passenden Build-Container.

## Architektur / Modulstruktur (Vorschlag)
```
Cargo.toml
Cargo.lock
src/
  main.rs             # CLI, Start, Logging, Runtime, Single-Instance
  app.rs              # App-Zustandsmaschine/Navigation
  model.rs            # GameEntry, Profile, Settings
  storage.rs          # XDG, atomare Speicherung/Migration
  paths.rs            # XDG-Pfade, Desktopordner, Flatpak-Exports
  desktop.rs          # Discovery, Import, Validierung, Icon resolution
  session.rs          # Desktop-Erkennung, XDG-Autostart
  instance.rs         # benutzerspezifischer Instanz-Lock
  activation.rs       # Unix-Socket, `--activate`-Client und Aktivierungs-Server
  launch.rs           # GIO-Launch + Diagnose, Steam/Prism optional
  controller.rs       # gilrs -> normalisierte Navigationsereignisse
  artwork.rs          # Bildimport, Thumbnail-Cache, Fallback
  ui/
    mod.rs
    library.rs
    profiles.rs
    admin.rs
    dialogs.rs
assets/
scripts/install-user.sh
scripts/uninstall-user.sh
tests/
```
Keine unnötige Microservice-/Backend-Struktur. `UI -> AppState -> Library/Profiles -> Desktop Launcher (GIO)`.

## Datei-/Datenspeichervertrag
- `~/.config/couchdeck/settings.toml`: UI-Einstellungen, Animationen, Sprache, zusätzliche Desktop-Suchverzeichnisse (keine Geheimnisse).
- `~/.local/share/couchdeck/library.json`: Profil- und Spielemetadaten, Referenzen auf `.desktop`-Dateien, Herkunft/Startmethode, Artwork-Overrides, Sortierung/Sichtbarkeit.
- `~/.local/share/couchdeck/assets/`: importierte Covers, Hintergründe, Avatare.
- `~/.local/share/couchdeck/managed-desktop/`: optionale *eigene* verwaltete Launcher-Dateien; GIO muss absolute Pfade akzeptieren. Originaldateien bleiben unangetastet.
- `~/.local/share/couchdeck/timekeeping.json`: Tagesnutzung und Zusatzzeit der Kinderprofile.
- PIN-Hash lokal getrennt gespeichert (XDG_DATA_HOME, private Dateirechte), kein Anspruch auf OS-Sicherheitsisolation.
- `~/.local/state/couchdeck/` bzw. `$XDG_RUNTIME_DIR`: Logs, Instanz-Lock und Aktivierungs-Socket (`couchdeck-activate.sock`).
- `~/.cache/couchdeck/`: decodierte/skalierte Thumbnails nach Bedarf.
Nutze XDG-Variablen und `HOME`, niemals feste Benutzernamen, niemals blindes Parsen/Expansion aus importierten `Exec=`.

## Ergonomie
Die Bibliothek zeigt nur Cover, Spielname, bei Fokus eine reduzierte Aktionsleiste. Keine Store-/Download-/News-UI. Für die Admin-Bedienung darf Maus/Tastatur als Komfortweg dienen, essentielle Aktionen jedoch auch mit Controller erreichbar. Kein unnötiges Suchfeld beim Start. Leicht lesbare Typografie im TV-Abstand. UI skaliert nach Bildschirmgröße/DPI.

## Arbeitsweise und Ergebnisse
1. Prüfe Verfügbarkeit und API aktueller Crates/Abhängigkeiten vor Verwendung; halte Versionen im Lockfile fest.
2. Implementiere **funktionierende** Inkremente, nicht nur leere Stubs; führe die Tests aus.
3. Bei Umgebungsproblemen dokumentiere den genauen Fehler und baue soweit möglich weiter.
4. Liefere README (Linux-Desktop-Setup, Build/Run/Install/Uninstall), Screenshots, Testausgaben, known limitations und Akzeptanz-Checkliste. Kennzeichne ungetestete Desktop-/Hardwarefälle als ungetestet.
5. Kein Autostart, kein Ersetzen der Desktop-/Gaming-Sitzung und keine Schreibzugriffe auf Systemdateien ohne explizite Benutzeraktion.
