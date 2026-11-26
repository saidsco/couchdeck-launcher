# Generischer GIO-Spike – Ergebnisse

Stand: 26. November 2026  
Getestet auf Manjaro/KDE Plasma/Wayland.

Testartefakt aus dem Projektordner: `org.wesnoth.Wesnoth.desktop`

Wesentliche Felder:

```ini
Type=Application
Name=Battle for Wesnoth
Exec=sh -c "wesnoth >/dev/null 2>&1"
Icon=wesnoth-icon
```

Installiertes Programm: `/usr/bin/wesnoth` (Battle for Wesnoth 1.18.8), keine
Flatpak-Installation.

## Start

```bash
gio launch /home/chris/Projekte/couchtest/org.wesnoth.Wesnoth.desktop
```

Ergebnis:

- GIO akzeptierte den Startauftrag.
- Prozesskette:
  ```text
  sh -c wesnoth >/dev/null 2>&1
    └─ wesnoth
  ```
- Fenster: `The Battle for Wesnoth - 1.18.8`
- Fenster-PID: PID des `wesnoth`-Prozesses.
- App-ID/Ressourcenklasse: `wesnoth`

## Session-/Prozesszuordnung

- `gio launch` liefert **keine PID** an CouchDeck.
- Es gibt keine spielspezifische systemd-Unit oder cgroup; die Prozesse liegen
  im allgemeinen User-Slice.
- Eine Zuordnung wäre nur nachträglich über Fenster/App-ID/Titel und Prozess-
  beobachtung möglich.
- Das ist für einen Kompatibilitätsmodus ausreichend, aber nicht für eine
  zuverlässige Managed-Session.

## Reguläres Beenden

Ein KWin-Script schloss das Fenster über die exakte Wesnoth-PID:

```javascript
if (window.pid === targetPid) {
    window.closeWindow();
}
```

Ergebnis:

- Wesnoth wurde beendet.
- Der Shell-Wrapper-Prozess verschwand ebenfalls.
- Kein `pkill`, keine Prozessnamen-Suche.

Das bestätigt nur, dass ein erkanntes Fenster schließbar ist. Es bedeutet
**nicht**, dass CouchDeck den Prozess über `gio launch` zuverlässig besitzt oder
das Spielende sicher erkennen kann.

## Was daraus für das Generic-GIO-Backend folgt

- Start: ja, über `gio launch`.
- Start-/Fenstererkennung: bestenfalls heuristisch (App-ID, Titel, Zeitfenster).
- Session-Zuordnung: **nicht zuverlässig**.
- Spielende: **nicht zuverlässig**.
- Fokus: nur über die bestehende Fensteraktivierung, nicht backendgestützt.
- Reguläres Beenden: nur wenn ein Fenster eindeutig zugeordnet wurde.
- Notfall-Stop: **nein**, kein eigener Prozess-/Scope-Handle.
- Zeitmessung: konservativer Kompatibilitätsmodus.

## Fazit

`org.wesnoth.Wesnoth.desktop` ist als generischer GIO-Testfall geeignet:

- Der GIO-Startpfad wurde real geprüft.
- Das Fenster und die PID konnten beobachtet werden.
- Ein erkanntes Fenster ließ sich regulär schließen.

Für Managed Sessions bleibt Generic GIO aber ein **Kompatibilitätsbackend**.
Die Bereitstellung einer Desktop-Datei allein reicht nicht für Prozessownership,
zuverlässige Endekennung oder spielspezifischen Notfall-Stop.
