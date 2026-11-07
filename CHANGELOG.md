# Changelog

## 0.4.0 – 2026-11-07

### Desktopunabhängigkeit

- KDE-spezifische Laufzeitannahmen aus Anwendung, Autostart, Installer und Dokumentation entfernt.
- Unterstützte Desktop-Ziele: KDE Plasma, GNOME und XFCE unter Wayland oder X11; Bazzite und SteamOS Desktop Mode als Zielplattformen, Gamescope/Gaming Mode nicht zugesichert.
- „Zu KDE minimieren“ → „Zum Desktop minimieren“ in Deutsch und Englisch.
- Autostart ist nicht mehr KDE-exklusiv, sondern an die erkannte KDE-/GNOME-/XFCE-Sitzung gebunden.

### Autostart und Single Instance

- Sitzungserkennung aus `XDG_CURRENT_DESKTOP`, `XDG_SESSION_DESKTOP` und `DESKTOP_SESSION`, inklusive mehrteiliger Werte.
- `OnlyShowIn=<erkannte Sitzung>;`, `NotShowIn=gamescope;` und `X-CouchDeck-Desktop=<Sitzung>;`.
- Unbekannte Sitzungen und Gamescope/Steam Gaming Mode erhalten keinen Autostart; bestehende fremde Einträge werden als inaktiv angezeigt und können entfernt werden.
- Installer `--autostart` erkennt die Sitzung und verweigert riskante Annahmen.
- Benutzerspezifischer `flock`-Instanzschutz vor parallelen UIs; automatische Freigabe bei Abstürzen.

### Desktop-Discovery

- Quellen: `$XDG_DATA_HOME/applications`, `$XDG_DATA_DIRS`, Flatpak-Benutzer-/Systemexports, tatsächlicher XDG-Desktopordner aus `user-dirs.dirs`, CouchDeck-verwaltete Dateien und zusätzliche Admin-Suchpfade.
- XDG-Überlagerung und stabile Desktop-IDs, inklusive Unterverzeichnissen.
- Hintergrund-Scan ohne Blockieren des UI-Threads; begrenzte Rekursion; keine Symlink-Verzeichnis-Loops.
- `NoDisplay`, `Hidden`, `OnlyShowIn`, `NotShowIn`, `TryExec`, lokalisierte Namen, Symlinks, Leerzeichen und fehlende/unlesbare Verzeichnisse werden behandelt.
- Neue Admin-Ansicht „Suchpfade & Discovery“ mit automatischen/manuellen Quellen, Hinzufügen per Ordnerdialog, Entfernen, Rescan und Warnungen.

### Import und Spielemodell

- Importübersicht gruppiert nach Herkunft und zeigt Quelle, Quellverzeichnis, Desktop-ID, Pfad, `Exec` und Diagnostik.
- `NoDisplay`-Einträge bleiben bewusst importierbar.
- Neue Importe werden bei vorhandenen Kinderprofilen nicht automatisch für Kinder sichtbar.
- `GameEntry` um `desktop_id`, `source` und `launch_method` erweitert; `library.json` aus 0.3.1 bleibt ohne Migration lesbar.
- Discovery-Läufe verändern keine bestehenden Bibliothekseinträge; fehlende Quelldateien werden nicht automatisch gelöscht.

### Tests / Doku

- Neue Tests für XDG-/Flatpak-/Desktopordner-/Custom-Discovery, Prioritäten, Desktop-IDs, `NoDisplay`/`Hidden`/`OnlyShowIn`/`NotShowIn`/`TryExec`, Symlinks, 0.3.1-Kompatibilität, kindersichere Import-Sichtbarkeit, Autostart-Sitzungserkennung und Single-Instance-Lock.
- Desktop-Testmatrix für KDE/Wayland, KDE/X11, GNOME/Wayland, XFCE/X11 und SteamOS Desktop Mode.
- AGENTS, README, README-de, Datenformat, Release-/Testdokumentation und Desktop-Architektur aktualisiert.
- Kein Git-Tag, GitHub-Release oder Push ohne ausdrückliche Freigabe.


## 0.3.1 – 2026-11-06

### Native Release-Distribution

- GitHub-Actions-Workflow für Tags `v*` und manuellen Dispatch eines vorhandenen Tags.
- Portables `couchdeck-<version>-linux-x86_64.tar.gz` mit Binary, Icon, Installer/Uninstaller, README, Docs und Runtime-Report.
- Ausführbares `CouchDeck-<version>-x86_64.AppImage` ohne Flatpak-Sandbox und ohne Root-Rechte.
- `SHA256SUMS` für alle veröffentlichten Dateien.
- `ldd`-Prüfung, Build-glibc und maximale GLIBC-Symbolversion als `runtime-info.txt`.
- Release-Build in Fedora 42 (glibc 2.41, Rust 1.89); Pakete benötigen keinen Rust-Compiler.
- AppImage/Archiv nutzen weiterhin `~/.local/bin/couchdeck` und die vorhandenen XDG-Datenordner; Profile, Artwork und Einstellungen werden bei Updates nicht überschrieben.
- Installer erkennt Repo- und Release-Archiv-Layout; Update ist derselbe `install-user.sh`-Aufruf.
- README/README-de mit Installation, Update, Deinstallation, Prüfsummen und glibc-Hinweisen für Bazzite erweitert.
- Dokumentierte Bazzite-Zielhardware-Checkliste für AppImage/Archiv, Controller, GIO-Spielstart und Systemaktionen.

### Tests / Doku

- Bestehende 62 Unit-Tests + 2 GIO-Integrationstests bleiben grün.
- Release-Paketierung lokal im Fedora-42-Container getestet: tar.gz, AppImage, SHA256SUMS, `ldd`, AppImage-Start unter Xvfb.
- Echter Test auf einer frischen Bazzite-KDE-Installation bleibt als manueller Zielhardware-Test offen.


## 0.3.0 – 2026-11-06

### Internationalisierung

- Deutsche und englische UI über Fluent (`fluent-bundle`, `unic-langid`).
- Automatische Spracherkennung aus `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG`; Englisch als Fallback.
- Sprache live unter Admin → Einstellungen umschaltbar (Automatisch/Deutsch/English), ohne Neustart.
- Neue Einstellungen `language` und `alarm_sound`; v0.2.5-Daten werden mit Defaults weiter geladen.

### Profil-Hintergründe

- Eigenes Hintergrundbild pro Profil, unabhängig von `Icon=` und Spielhintergründen.
- Import in `$XDG_DATA_HOME/couchdeck/assets/backgrounds/`; Originaldateien bleiben unverändert.
- Fallback: Profil → globales Hintergrundbild. Vorhandene Spiel-/Global-Hintergründe bleiben erhalten.
- Auswahl/Entfernen im Admin-Profil-Editor; Profilansicht zeigt den Hintergrund des fokussierten Profils.

### Kinderprofile und Zeitkontingente

- Optionales Kinderprofil mit täglichem Zeitbudget in Minuten.
- Messung aktiver Profilzeit, solange CouchDeck läuft; Persistenz in `timekeeping.json` (atomar, Backup bei Korruption).
- Restzeitanzeige im Header, Warnungen bei 5/1 Minuten, optionaler Best-Effort-Alarmton über System-Player.
- Zentrale Startprüfung für alle neuen Spiel-/Medienstarts; laufende Spiele werden nicht zwangsweise geschlossen.
- Tageswechsel setzt Nutzung, Zusatzzeit und Warnflags zurück.
- „Mehr Spielzeit anfragen“ im Startmenü und auf dem Bildschirm „Zeitkontingent verbraucht“.
- Admin-PIN, dann +5/+15/+30 oder benutzerdefinierte Minuten (5–240 in 5er-Schritten, Controller-bedienbar).
- Zusatzzeit gilt nur für den aktuellen Kalendertag, addiert sich, verändert das reguläre Budget nicht und bleibt über Neustarts erhalten.
- Admin kann vergebene Zusatzzeit einsehen und zurücksetzen.
- Kinderprofile erhalten keinen Admin-UI-Zugriff. Profilwechsel, Minimieren, Beenden und Systemaktionen erfordern bei aktiven Kinderprofilen eine einmalige Admin-PIN-Freigabe; ParentAuth entsperrt den Admin-Bereich nicht dauerhaft.
- Best-Effort-Schutz gegen WM-Minimize/Close; dokumentierte Grenzen: Prozess-Kill, TTY-Wechsel, Dateiänderungen im selben Linux-Konto und fehlende Spielprozess-Erkennung.

### Tests / Doku

- 62 Unit-Tests + 2 GIO-Integrationstests (64 gesamt): Zeitbudget, Tageswechsel, Persistenz/Neustart, Zusatzzeit, PIN-Prüfung, Profilwechsel-Sperre, Start-Sperre, i18n/Fallback und v0.2.5-Datenkompatibilität.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, Release-Build.
- README/README-de, Datenformat, Testbericht und Abnahmematrix auf v0.3.0 aktualisiert.


## 0.2.5 – 2026-11-06

### Review-Fixes

- Gamepad-Navigation wird nur bei aktivem CouchDeck-Fenster verarbeitet.
- Connect/Disconnect-Ereignisse werden auch ohne Fokus verarbeitet.
- Beim Fokusverlust werden gehaltene D-Pad-/Stick-Zustände zurückgesetzt; beim Fokusgewinn wird gepufferte Navigation verworfen.
- Admin-Fokus/Confirm kann bei Fensterfokusverlust nicht mehr in die UI feuern.
- LB/RB sind korrekt auf die Schultertasten (`Button::LeftTrigger`/`RightTrigger`) gemappt, nicht auf die analogen LT/RT-Trigger.
- Systemaktionen in eigenes Modul `system.rs` verschoben (`app.rs` entlastet).
- Tests für Fokus-Gating, Schultertasten-Mapping und SystemController ergänzt.

### Tests / Doku

- 47 Unit-Tests + 2 GIO-Integrationstests.
- Deutsche und englische README auf v0.2.5 aktualisiert.

## 0.2.0 – 2026-11-06

### Autostart / Kiosk

- Opt-in-XDG-Autostart für KDE (`OnlyShowIn=KDE;`, `X-KDE-autostart-after=panel`).
- Autostart-Eintrag unter `$XDG_CONFIG_HOME/autostart/couchdeck.desktop`.
- `scripts/install-user.sh --autostart` und Schalter unter Admin → Einstellungen.
- Steam Gaming Mode/Gamescope wird durch `OnlyShowIn=KDE;` nicht automatisch gestartet.

### Beenden / KDE / System

- „Zu KDE minimieren“: verlässt zuerst den borderlosen Fullscreen und minimiert danach das Fenster.
- Vollbild-Toggle setzt die Fensterdekorationen beim Verlassen des Fullscreens zurück.
- „CouchDeck beenden“ beendet weiterhin nur den Launcher, keine laufenden Spiele.
- Power-Menü mit Bestätigung für System-Neustart und Herunterfahren.
- Power-Aktionen verwenden feste `systemctl --no-block --no-wall reboot|poweroff`-Aufrufe ohne Shell.
- Fehler von `systemctl`/Polkit werden im UI angezeigt.
- Der Launcher beendet sich nach erfolgreich übergebenem Systembefehl.

### Tests / Doku

- Tests für Autostart-Erzeugung, KDE-only-Markierung, Quoting und Entfernen.
- Tests für feste Power-Kommandoargumente.
- README, Datenformat, Testbericht und Abnahmematrix auf v0.2 erweitert.
- Deutsche und englische README.

## 0.1.0 – 2026-11-05

- Erster MVP: Fullscreen-Bibliothek, XDG-Desktop-Import, GIO-Launch, Cover/Artwork, lokale Profile, Admin-PIN, Steam-/Prism-Integration.
- Persistenz mit atomarem Schreiben und Backups.
- Gamepad-Navigation über gilrs.
