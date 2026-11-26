# Steam-Spike – Ergebnisse

Stand: 26. November 2026  
Getestet auf Manjaro/KDE Plasma/Wayland mit nativem Steam (kein Flatpak-Steam).

Verwendetes Spiel: **LEGO Bricktales**, App-ID `1898290`, über die vorhandene
Desktop-Datei:

```ini
Exec=steam steam://rungameid/1898290
```

Steam lief bereits; zusätzlich war noch ein Steam-Kontext für Baldur's Gate 3
vorhanden. Dieser wurde nicht beendet.

## Start

`gio launch /home/chris/.local/share/applications/LEGO Bricktales.desktop`
wurde verwendet. Das entspricht dem normalen CouchDeck-GIO-Pfad.

Ergebnis:

- LEGO Bricktales startete.
- KWin-Fenster: `LEGO Bricktales`
- Fenster-PID: die PID des eigentlichen Spielprozesses
  `LEGO_Bricktales.x86_64`
- Steam erzeugte eine **eigene transienten Unit/Scope** für das Spiel:
  ```text
  app-steam-app1898290-<pid>.scope
  ```
- Der Spielprozess und die Steam-Runtime-Bestandteile lagen in dieser Scope.
- Der Steam-Client selbst blieb in einer anderen Unit und lief weiter.

Wichtige Prozesskette:

```text
steam (bereits laufend)
  └─ steam-runtime-launch-client
      └─ steam-runtime-supervisor
          └─ reaper / SteamLinuxRuntime
              └─ LEGO_Bricktales.x86_64
```

## Session-Zuordnung

Die Steam-App-ID ist im Scope-Namen enthalten:

```text
app-steam-app1898290-<pid>.scope
```

Die Fenster-PID entsprach dem Spielprozess in dieser Scope. Damit ist für
diesen nativen Steam-Fall eine Zuordnung möglich über:

- Steam-App-ID,
- Scope-Name,
- PID/Prozessbaum,
- KWin-Fenster-PID.

## Reguläres Beenden

Ein KWin-Script schloss das LEGO-Fenster über die exakte PID:

```javascript
if (window.pid === targetPid) {
    window.closeWindow();
}
```

Ergebnis:

- Das LEGO-Fenster und der Spielprozess wurden beendet.
- Die spielspezifische `app-steam-app1898290-<pid>.scope` verschwand.
- Der Steam-Client lief weiter.
- Kein Steam-Neustart, kein Kill des Steam-Prozesses, kein `pkill`.

## Notfall-/Scope-Stop

`systemctl --user stop app-steam-app1898290-<pid>.scope` beendete das Spiel
ebenfalls. Der Steam-Client blieb aktiv. Das ist ein gezielter Notfallweg für
die eigene Spiel-Scope, kein Prozessnamen-Matching.

## Wichtige Einschränkungen

- Getestet wurde **natives Steam**, nicht Flatpak-Steam.
- Steam lief bereits. Der Start bei vollständig beendetem Steam wurde nicht
  getestet.
- Bei Steam lief noch ein anderer App-Kontext (Baldur's Gate 3); dieser blieb
  aktiv. Das ist ein gutes Zeichen für die Trennung, ersetzt aber keinen
  Mehrspiel-/Mehrbenutzer-Test.
- Die Scope-Namen enthalten eine PID und müssen zur Laufzeit ermittelt werden.
- Das Verhalten mit Steam Cloud/Proton/Steam Overlay und laufenden Downloads
  wurde nicht vollständig getestet.
- Ein tatsächlicher Spielverlauf (Speichern/Beenden im Spiel) wurde nicht
  getestet; der Spike prüft Start, Prozess-/Fensterzuordnung und Beenden.
- KWin-Fensterschließung ist KDE-spezifisch; GNOME/XFCE brauchen eigene Tests.
- Flatpak-Steam kann eine andere Prozess-/Scope-Struktur haben und muss separat
  geprüft werden.

## Fazit

Für **natives Steam** ist die Lage besser als zunächst angenommen:

- Start über den normalen GIO-/Steam-Desktop-Pfad funktioniert.
- Steam erzeugt eine spielspezifische `app-steam-app<ID>-<pid>.scope`.
- Diese Scope enthält den Spielprozess und die Steam-Runtime.
- Reguläres Fensterschließen über die exakte PID beendet nur das Spiel.
- Die eigene Spiel-Scope kann als bestätigter Notfallweg gestoppt werden.
- Der Steam-Client bleibt erhalten.

Damit kann Steam als eigenes Backend mit hoher, aber noch nicht vollständig
verifizierter Capability eingestuft werden. Flatpak-Steam bleibt ein separater
Spike.
