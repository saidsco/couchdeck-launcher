# Changelog

## 0.6.0 – 2026-11-28

### CapyDeck Rebrand

- Produkt- und Repository-Name: **CouchDeck → CapyDeck**.
- Binary, App-ID, Fenstertitel, UI-Texte und I18n auf `capydeck`/`CapyDeck` umgestellt.
- Neue XDG-Pfade: `~/.config/capydeck`, `~/.local/share/capydeck`, `~/.cache/capydeck`, `~/.local/state/capydeck`.
- Desktop-, Autostart- und Home-Launcher-Dateien heißen jetzt `capydeck.desktop` bzw. `capydeck-home.desktop`.
- Instanz-Lock, Aktivierungs-Socket, Log und verwaltete Desktop-Dateien verwenden `capydeck`-Namen.
- Neues CapyDeck-Logo und PNG-Icons (`capydeck.png`, `capydeck-512.png`, `capydeck-1024.png`); das alte SVG-Icon entfällt.
- Release-Artefakte heißen `capydeck-<version>-linux-x86_64.tar.gz` und `CapyDeck-<version>-x86_64.AppImage`.
- Repository: `https://github.com/saidsco/capydeck`.
- Keine Datenmigration und keine Legacy-Aliase; es gibt keine bestehenden Benutzerdaten. Historische Einträge unter 0.5.x und früher beschreiben den früheren Namen CouchDeck.

### Tests / Doku

- README, README-de, AGENTS, Release-/Installationsdokumentation und Testmatrizen auf CapyDeck aktualisiert.
- Installer/Uninstaller und AppImage-Paketierung auf die neuen Pfade und PNG-Icons umgestellt.
- Kein Release-/Tag-Schritt ohne gesonderte Freigabe.


## 0.5.2 – 2026-11-25

### Automatische Controller-Home-Berechtigung

- Beim Start erkennt CouchDeck, ob Lesezugriff auf Gamepad-Geräte vorhanden ist.
- Fehlt der Zugriff, erscheint ein Dialog „Jetzt einrichten / Später“.
- „Jetzt einrichten“ startet `pkexec usermod -aG input <benutzer>` mit ausdrücklicher Polkit-Bestätigung; keine stille Root-Aktion.
- „Später“ wird gespeichert und verhindert wiederholte Start-Prompts.
- Status und Einrichtungshinweis bleiben unter Admin → Einstellungen sichtbar.
- Gilt für AppImage und tar.gz; der Installer-Prompt bleibt zusätzlich bestehen.
- Nach erfolgreicher Einrichtung ist eine Neuanmeldung erforderlich.
- Der native Controller-Home-Chord (Start+Select 2 s) bleibt standardmäßig aktiv.

### Tests / Doku

- 88 Unit-Tests + 2 GIO-Integrationstests (90 gesamt).
- Doku und Bazzite-Testmatrix aktualisiert.
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.5.1 – 2026-11-25

### Controller Home

- Standardmäßig aktiver globaler Controller-Home-Chord: **Start + Select 2 Sekunden halten**.
- Funktioniert, solange CouchDeck läuft, auch minimiert im Hintergrund; keine Steam-/Steam-Input-Abhängigkeit.
- Alternative Kombinationen in Admin → Einstellungen: Start + linker Stickklick, Start + rechter Stickklick.
- Chord-Erkennung im bestehenden gilrs-Thread, read-only und ohne exklusiven Geräte-Grab; normale Bibliotheksnavigation bleibt auf fokussiertes CouchDeck beschränkt.
- Home-Hinweis in Bibliotheks-Footer und Einstellungen.
- Vorbereitung für weitere Home-Aktionen (z. B. spätere Gamesession-Beendigung), ohne jetzt einen GameSessionManager einzuführen.

### Geräteberechtigungen

- Installer fragt nach der Installation, ob der Benutzer zur Gruppe `input` hinzugefügt werden soll; danach Neuanmeldung.
- `couchdeck --setup-input-permissions` für AppImage und direkte Installationen; nutzt `pkexec` oder – bei Terminal – `sudo`, nie eine Shell.
- Admin → Einstellungen zeigt den `/dev/input`-Berechtigungsstatus.
- Kein Root-Daemon, kein `uinput`, kein exklusiver Input-Grab.

### Tests / Doku

- Unit-Tests für 2-Sekunden-Chord, Alternativkombinationen und Berechtigungsstatus.
- 88 Unit-Tests + 2 GIO-Integrationstests (90 gesamt).
- Controller-/Steam-Input-Dokumentation und Bazzite-Testmatrix aktualisiert.
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.5.0 – 2026-11-24

### Controller Home & Return

- Neuer Befehl `couchdeck --home`: aktiviert eine laufende CouchDeck-Instanz oder startet CouchDeck, wenn keine läuft.
- `--home` startet niemals eine zweite UI, beendet keine Spiele/Launcher und umgeht keine PIN-/Kinder-/Zeitbudget-Prüfung.
- `--activate` bleibt der reine Aktivierungsbefehl und meldet bei fehlender Instanz einen Fehler.
- Optionaler, ausschließlich benutzerseitig ausgelöster Menüeintrag „CouchDeck Home“ über `--install-home-launcher`; Entfernen mit `--remove-home-launcher`.
- AppImage-Pfad wird über `APPIMAGE` erkannt, damit der Launcher nicht auf ein temporäres Extraktionsverzeichnis zeigt.
- Keine automatische Registrierung globaler Tastenkombinationen, keine systemweiten Änderungen.
- Steam-Input-Anleitung für Controller-Kombination → Desktop-Shortcut → `couchdeck --home`; kein eigener Input-Treiber, keine globale Controllerüberwachung.

### Tests / Doku

- Unit-Tests für Home-Entscheidung, Desktop-Entry-Quoting, atomisches Installieren/Entfernen des Launchers.
- 85 Unit-Tests + 2 GIO-Integrationstests (87 gesamt).
- README/README-de, Aktivierungsdokumentation, Steam-Input-Dokumentation, Test-/Abnahmematrix aktualisiert.
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.4.7 – 2026-11-08

### Bugfix: Taskleisten-/Fullscreen-Minimierung

- Ursache: Bei minimiertem eframe-Fenster erhält die UI teilweise keine Frames mehr. Der Aktivierungsauftrag lag dann im IPC-Kanal, bis Maus-Hover einen Frame auslöste.
- Der IPC-Server startet unter KDE/Wayland den begrenzten KWin-Fallback jetzt direkt aus dem Hintergrundthread, sofern kein beobachteter Fokus vorliegt. Die Wiederherstellung hängt nicht mehr von einem UI-Frame ab.
- `RequestUserAttention` wird unter KDE/Wayland bei fehlendem Token nicht mehr vor dem KWin-Fallback gesendet; ein früher Fokus auf ein noch verborgenes Fenster wird vermieden.
- Der KWin-Fallback wiederholt die Aktivierung nach 180 ms einmalig, damit KWin die zuvor minimierte Fläche tatsächlich mapped.
- Fullscreen wird nach bestätigtem Fokus erst nach einer kurzen Stabilisierungszeit wiederhergestellt; der zusätzliche Repaint nach 300 ms bleibt begrenzt.
- `--windowed` bleibt windowed; externe Fullscreen-Minimierung erzeugt keinen unnötigen Fullscreen-Toggle.
- Kein Spiel-/Launcher-Prozess wird beendet, keine PIN-/Kinderrechte-Umgehung.

### Diagnose / Tests

- Neue Logzeile für den KWin-Fallback aus dem IPC-Thread.
- Manuell auf Manjaro/KDE 6.7.5/Wayland: interne Fullscreen-Minimierung → `--activate` → KWin-Zustand `minimized=false`, `active=true`, `fullScreen=true`; kein Maus-Hover nötig.
- 81 Unit-Tests + 2 GIO-Integrationstests (83 gesamt).
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.4.6 – 2026-11-08

### Bugfix: Fensteraktivierung unter KDE/Wayland

- Ursache: Winit/Wayland kann ein bestehendes Fenster weder per `Focus` fokussieren noch per `Minimized(false)` entminimieren; `is_minimized()` liefert auf Wayland keine verlässliche Information. Die bisherige 0.4.5-Logik sandte daher wirkungslose ViewportCommands.
- `couchdeck --activate` liest jetzt ein vom Aufrufer bereitgestelltes `XDG_ACTIVATION_TOKEN` und übermittelt es über den lokalen IPC-Socket an die laufende Instanz.
- Die Instanz wendet den Token über einen minimalen `xdg-activation-v1`-Guest-Client auf ihre bereits bestehende Wayland-Oberfläche an. Es wird kein neues Fenster erzeugt und kein Token erfunden.
- KDE/Wayland: Begrenzter, einmaliger KWin-Scripting-Fallback für Terminalaufrufe ohne Token oder wenn ein Token nicht fokussiert. Das Skript aktiviert ausschließlich das CouchDeck-Fenster, wiederholt die Aktivierung nach 180 ms einmalig (falls KWin die Minimierung erst verzögert mapped), fordert danach einen zusätzlichen Repaint an und wird entfernt; keine Polling-Schleife, keine Manipulation fremder Fenster.
- Fullscreen wird nur nach tatsächlich beobachtetem Fokus wiederhergestellt, wenn CouchDeck zuvor für das Minimieren aus dem Fullscreen gewechselt wurde. Ein extern minimiertes Fullscreen-Fenster wird nicht unnötig umgeschaltet.
- Kein automatischer Fokusgewinn beim Spielstart; laufende Spiele/Launcher werden nicht beendet.

### Diagnose

- Getrennte Logs für IPC-Empfang, Token vorhanden/nicht vorhanden, Token angewendet/fehlgeschlagen, Fokus angefordert, Fokus tatsächlich beobachtet und Timeout.
- Wayland-Minimierungszustand wird ausdrücklich als nicht zuverlässig protokolliert.
- KWin-Fallback meldet fehlenden Fokus nach seinem Lauf; KDE-spezifische Aktionen bleiben auf KDE beschränkt.

### Doku / Tests

- `docs/WINDOW-ACTIVATION.md` um Token-Übergabe, KWin-Fallback, Terminal-vs-Shortcut-Verhalten und Grenzen erweitert.
- Manuelle Tests auf Manjaro/KDE 6.7.5/Wayland: Token-Aktivierung aus einem KDE/KIO-gestarteten Prozess, Terminal ohne Token mit KWin-Fallback, minimiertes Windowed- und Fullscreen-Fenster, Fake-Token-Fallback, X11-Regressionen.
- 81 Unit-Tests + 2 GIO-Integrationstests (83 gesamt).
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.4.5 – 2026-11-08

### Fensteraktivierung

- Neuer CLI-Befehl `couchdeck --activate` kontaktiert die bereits laufende Instanz über einen lokalen Unix-Domain-Socket.
- Kein zweiter UI-Prozess; bestehender Single-Instance-Lock bleibt maßgeblich.
- Socket unter `$XDG_RUNTIME_DIR` (Fallback State-Verzeichnis), Modus `0600`, zusätzliche `SO_PEERCRED`-Prüfung auf die eigene lokale UID.
- Veraltete Sockets nach Abstürzen werden sicher ersetzt; keine falsche Erfolgsmeldung bei fehlender/defekter IPC.
- Aktivierung ist Best-Effort: IPC-Annahme und tatsächlicher Compositor-Fokus werden getrennt.
- Minimiertes CouchDeck wird wiederhergestellt; vorheriger Fullscreen wird wiederhergestellt, Windowed bleibt Windowed.
- Ist CouchDeck bereits fokussiert, erfolgt keine Fensteraktion.
- Kein automatischer Fokusgewinn beim Spielstart; Steam-/Proton-/Launcher-Dialoge bleiben ungestört.
- Keine Prozessmanipulation, keine PIN-/Kinderrechte-Umgehung, keine Aggression gegen andere Fenster.

### Diagnose

- Neue Fokus-, Minimierungs-, Fullscreen-, Aktivierungs- und IPC-Logzeilen.
- `--diagnose-window` für zusätzliche Zustandswechsel.
- Sitzungsinformationen (Wayland/X11, erkannte Desktopumgebung, Token) beim Start.

### Doku / Tests

- Neue Doku `docs/WINDOW-ACTIVATION.md` mit CLI, Architektur, Desktop-Shortcuts für KDE/GNOME/XFCE, Steam-Input-Vorbereitung und Wayland-/X11-Grenzen.
- Manuelle Aktivierungs-, Minimize-, Fullscreen- und No-Instance-Tests unter X11 durchgeführt; Wayland-/Bazzite-Tests bleiben offen.
- 77 Unit-Tests + 2 GIO-Integrationstests (79 gesamt).
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


## 0.4.1 – 2026-11-07

### Fixes

- Desktop-/Menü-Icon: Installation liefert jetzt zusätzlich 256×256- und 512×512-PNGs plus SVG; der Desktop- und Autostart-Eintrag referenziert das PNG per absolutem Pfad. Das umgeht KDE-/Icon-Cache-Probleme bei reinen SVG-Theme-Icons.
- „Zum Desktop minimieren“ merkt sich, ob CouchDeck im Fullscreen lief. Beim Wiederherstellen aus der Taskleiste wird Fullscreen erneut aktiviert, sofern der Compositor Minimieren oder Fokusverlust gemeldet hat. Bei einem Start mit `--windowed` oder nach manuellem Verlassen des Fullscreens bleibt der Windowed-Modus erhalten.
- Tests für die Fullscreen-Wiederherstellungslogik ergänzt.

### Tests / Doku

- 74 Unit-Tests + 2 GIO-Integrationstests (76 gesamt).
- Kein Git-Tag/GitHub-Release/Push ohne Freigabe.


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
