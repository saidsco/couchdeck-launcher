# Changelog

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
