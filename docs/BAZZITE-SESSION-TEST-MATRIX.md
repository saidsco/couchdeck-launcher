# Bazzite KDE/Wayland – Session-Testmatrix (manuell)

Stand: Phase III, 27. November 2026

Ziel: Die in Phase II und III auf Manjaro gewonnenen Ergebnisse auf der primären
Zielplattform **Bazzite KDE Plasma/Wayland** reproduzieren und die
Qualitätsstufe von „Experimentell“ auf „Verifiziert“ heben.

**Wichtig:** Tests unter Manjaro sind wertvoll, ersetzen aber nicht die Abnahme
auf Bazzite. Die Spalte „Manjaro“ dokumentiert den bisherigen Stand; die Spalte
„Bazzite“ ist bis zur Durchführung offen.

## Umgebung und Voraussetzungen

### Zielsystem

- Bazzite KDE Plasma, Wayland-Sitzung
- Natives Steam (Bazzite-Standard)
- Prism Launcher als Flatpak (`org.prismlauncher.PrismLauncher`)
- Ryujinx (falls Direct/Managed getestet wird)
- Mindestens ein Controller (Xbox oder PlayStation), idealerweise per USB und
  Bluetooth
- 1080p und 4K (skalierte) Anzeige
- Ein Benutzerkonto ohne bestehende CapyDeck-Daten

### Software-Versionen dokumentieren

```bash
cat /etc/os-release
plasmashell --version
kwin_wayland --version
systemctl --version | head -1
flatpak --version
flatpak list --app --columns=application,version | grep -Ei 'steam|prism'
steam --version 2>/dev/null || true
```

### Evidenz sammeln

Für jeden Test mindestens erfassen:

```bash
date --iso-8601=seconds
cat /proc/sys/kernel/random/boot_id
ps -eo pid,ppid,cgroup:200,args --sort=pid
systemctl --user list-units --type=scope --all --no-pager
systemctl --user list-units --all --no-pager | grep -Ei 'capydeck|steam|prism|ryujinx'
journalctl --user --since '-10 min' --no-pager
```

Fensterliste (KDE/KWin):

```javascript
// /tmp/kwin-probe.js
var wins = workspace.windowList();
for (var i = 0; i < wins.length; ++i) {
    var w = wins[i];
    console.info('WIN|' + w.pid + '|' + w.caption + '|' +
        w.desktopFileName + '|' + w.resourceClass +
        '|active=' + w.active + '|min=' + w.minimized +
        '|fs=' + w.fullScreen);
}
```

```bash
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript /tmp/kwin-probe.js probe
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.start
sleep 1
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.unloadScript probe
journalctl --user --since '-5 seconds' --no-pager | grep 'WIN|'
```

Fenster schließen (nur exakte PID):

```javascript
// /tmp/kwin-close.js – TARGETPID ersetzen
var target = 12345;
var wins = workspace.windowList();
for (var i = 0; i < wins.length; ++i) {
    var w = wins[i];
    if (w.pid === target) {
        console.info('CLOSE|' + w.pid + '|' + w.caption);
        w.closeWindow();
    }
}
```

### Legende

- `[ ]` offen
- `[x]` auf dieser Plattform bestanden
- `[~]` teilweise bestanden / eingeschränkt
- `[!]` fehlgeschlagen
- `[n/a]` nicht anwendbar

## A. Launcher-Grundlagen

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| A1 | Start ohne Daten | Erststart/Profilanlage, keine Crashs | offen | `[ ]` |
| A2 | Start mit gespeichertem Profil | direkte Bibliothek, fullscreen | offen | `[ ]` |
| A3 | `--windowed` | Fenstermodus, bedienbar | offen | `[ ]` |
| A4 | Controller-Navigation | D-Pad/Stick erreicht alle Kacheln, A/B/Menü | offen | `[ ]` |
| A5 | Tastatur/Maus parallel | Bedienung möglich | offen | `[ ]` |
| A6 | Cover-Fallback | fehlendes/kaputtes Cover → Icon/Kachel, kein Crash | offen | `[ ]` |
| A7 | Single Instance | zweiter Start beendet sich mit Hinweis | offen | `[ ]` |
| A8 | `--home` | aktiviert laufende Instanz oder startet | offen | `[ ]` |
| A9 | Controller-Home-Chord | Start+Select 2 s aktiviert | offen | `[ ]` |

## B. Steam nativ

Voraussetzung: natives Steam, ein natives Linux-Spiel und ein Proton-Spiel.

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| B1 | Steam bereits geöffnet, natives Spiel starten | Spiel-Scope + Fenster; Steam bleibt | `[x]` LEGO | `[ ]` |
| B2 | Steam vollständig beendet, Spiel starten | Kaltstart, Spiel-Scope + Fenster | `[x]` LEGO | `[ ]` |
| B3 | Proton-Spiel starten | Spiel-Scope; Proton/Wine in Scope; Fenster-PID | `[x]` Disco | `[ ]` |
| B4 | Scope-Identität prüfen | `ControlGroup` enthält Spiel-PID; `MainPID` leer | `[x]` | `[ ]` |
| B5 | Regulär schließen (nativ) | Scope endet ca. 1–3 s; Steam bleibt | `[x]` | `[ ]` |
| B6 | Regulär schließen (Proton) | Scope endet; Steam bleibt; Close kann ignoriert werden | `[~]` Disco ok, Stray ignoriert | `[ ]` |
| B7 | Absturz (exakte Spiel-PID `SIGKILL`) | Scope endet ca. 1–2 s; keine Restprozesse | `[x]` | `[ ]` |
| B8 | Zwei Spiele gleichzeitig | zwei Scopes; Stop eines Spiels lässt das andere laufen | `[x]` LEGO+Disco | `[ ]` |
| B9 | Notfall-Stop eigene Spiel-Scope | nur Spiel endet; Steam bleibt | `[x]` | `[ ]` |
| B10 | Overlay öffnen/schließen | Overlay funktioniert; Session unbeeinflusst | offen | `[ ]` |
| B11 | Steam Input mit Controller | Spiel reagiert; Session unbeeinflusst | offen | `[ ]` |
| B12 | Steam Cloud | Spiel speichert/lädt; keine hängenden Prozesse | offen | `[ ]` |
| B13 | Vollbild ↔ Fenster | Wechsel ohne Session-Ende | `[~]` KWin-Toggle | `[ ]` |
| B14 | Minimieren → Wiederherstellen | Prozess/Scope bleiben; Fullscreen wieder da | `[x]` | `[ ]` |
| B15 | Home-Rückkehr → Spiel fortsetzen | Session bleibt; Spiel wird wieder fokussiert | offen (nur KWin) | `[ ]` |
| B16 | Steam-Client während Spiel beenden | Verhalten dokumentieren; keine falsche Endekennung | offen | `[ ]` |
| B17 | Proton-Fenster-PID ist Kindprozess | z. B. `Stray-Win64-Shipping.exe`; PID in Scope verifizieren | `[x]` Stray | `[ ]` |
| B18 | Close wird vom Spiel ignoriert | zweiter Close, dann gezieltes `SIGTERM` auf exakte Spiel-PID | `[x]` Stray | `[ ]` |
| B19 | Proton 9.0 (Beta) | Scope/Fenster/Ende wie bei Proton 8.0 | `[x]` Stray | `[ ]` |

### Hinweise zu B1–B16

- Scope-Muster: `app-steam-app<APPID>-<PID>.scope`.
- `systemctl --user show <scope>` hat keinen `MainPID`; `ControlGroup`
  verwenden.
- Fenster-PID ist bei Proton der `*.exe`-Prozess.
- Keine Änderungen an Steam-Startoptionen ohne ausdrückliche Freigabe.
- Kein `pkill`; für den Crash-Test nur die exakte, bekannte Spiel-PID verwenden.

## C. Prism Launcher / Minecraft (Flatpak)

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| C1 | Prism-UI geschlossen, `--launch` | Prism + Java; Fenster-PID = Java | `[x]` | `[ ]` |
| C2 | Prism-UI geöffnet, `--launch` | bestehende Flatpak-Instanz wird genutzt | `[x]` | `[ ]` |
| C3 | Zweiter Start derselben Instanz | kein zweites Java; Exit 0 ≠ Start | `[x]` | `[ ]` |
| C4 | Zwei unterschiedliche Instanzen | zwei Java-PIDs, gleiche Flatpak-Scope | `[x]` | `[ ]` |
| C5 | Zuordnung Java → Instanz | `-Djava.library.path=.../instances/<name>/natives` | `[x]` | `[ ]` |
| C6 | Ein Minecraft schließen | nur dieses Java endet; anderes läuft weiter | `[x]` | `[ ]` |
| C7 | Prism-UI bleibt offen | UI und Flatpak-Instanz bleiben | `[x]` | `[ ]` |
| C8 | Minecraft-Absturz (`SIGKILL` Java) | Java tot; UI bleibt; Fenster ggf. kurz stale | `[x]` | `[ ]` |
| C9 | `flatpak kill` bei einer Session | gesamte Instanz endet (dokumentieren) | `[x]` | `[ ]` |
| C10 | `flatpak kill` bei zwei Sessions | beide enden → nur mit Freigabe | `[x]` Einschätzung | `[ ]` |
| C11 | Welt laden, speichern, erneut laden | vollständiger Spielverlauf | offen | `[ ]` |
| C12 | Launcher nach Spielende schließen | sauberes Ende ohne Java-Rest | `[x]` | `[ ]` |

### Hinweise zu C1–C12

- `flatpak kill <instance>` ist **kein** spielspezifischer Stop.
- Fenstertitel können bei mehreren Instanzen identisch sein.
- Die Flatpak-Scope bleibt bei offener UI bestehen; sie ist kein Spielende-
  Signal.

## D. Direct/Managed (Ryujinx und native Testanwendung)

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| D1 | Ryujinx-Service-Unit starten | `MainPID` = Fenster-PID, cgroup korrekt | `[x]` | `[ ]` |
| D2 | Echtes Spiel im Vollbild | Spiel läuft, Vollbild | `[x]` MK8D | `[ ]` |
| D3 | `--fullscreen` ohne Config | Verhalten dokumentieren | `[x]` wirkungslos | `[ ]` |
| D4 | `start_fullscreen=true` | KWin `fullScreen=true` | `[x]` | `[ ]` |
| D5 | `show_confirm_exit=true`, Fenster schließen | Bestätigungsdialog; kein automatisches Ende | `[x]` | `[ ]` |
| D6 | `show_confirm_exit=false`, Fenster schließen | reguläres Ende ca. 2 s | `[x]` | `[ ]` |
| D7 | Minimieren → Wiederherstellen | Prozess/Unit bleiben; Vollbild wieder da | `[x]` | `[ ]` |
| D8 | Notfall-Unit-Stop | Unit/Prozess weg; keine Reste | `[x]` | `[ ]` |
| D9 | Blockierender Stop | Timeout + `SIGKILL`-Fallback | `[x]` | `[ ]` |
| D10 | Scope statt Service | Stop nur mit `.scope`; kein `MainPID` | `[x]` | `[ ]` |
| D11 | Kindprozess-Testapp | Kind in cgroup; Crash killt Kind | `[x]` | `[ ]` |
| D12 | Fenster neu erstellen | Session bleibt; neue Fenster-PID | `[x]` | `[ ]` |
| D13 | Save/Load im Spiel | vollständiger Verlauf | offen | `[ ]` |
| D14 | Controller im Emulator | Bedienung, Hotplug | offen | `[ ]` |
| D15 | Weiteres natives Spiel (Pillars of Eternity) | Direktstart, Fenster-PID, Unit-Stop | `[x]` | `[ ]` |

### Hinweise zu D1–D14

- Verwaltete Konfiguration/verwalteter Root, keine Änderung der
  Benutzerkonfiguration.
- `KillMode=control-group` setzen.
- `systemctl stop` nie im UI-Thread; `--no-block` + Timeout + Kill-Fallback.
- Ryujinx-Bestätigungsdialog nicht als Ende werten.

## E. Generisches GIO (Kompatibilitätsbackend)

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| E1 | `gio launch` direkte App | Start, Fenster | `[x]` | `[ ]` |
| E2 | Shell-Wrapper (Wesnoth) | Wrapper-PID, Spiel-PID im Kind | `[x]` | `[ ]` |
| E3 | GIO-PID-Callback direkt | Callback = Prozess-PID | `[x]` | `[ ]` |
| E4 | GIO-PID-Callback Wrapper | Callback = Wrapper-PID | `[x]` | `[ ]` |
| E5 | Mehrere Kindprozesse | nur direkter Startprozess im Callback | `[x]` | `[ ]` |
| E6 | Early-Exit-Launcher | Kind lebt weiter; Callback-PID tot | `[x]` | `[ ]` |
| E7 | D-Bus-Aktivierung | kein Callback, ggf. kein neuer Prozess | `[x]` | `[ ]` |
| E8 | Bereits laufende App | neuer Prozess oder Übernahme dokumentieren | `[x]` | `[ ]` |
| E9 | Mehrere Instanzen | getrennte Wrapper/Prozesse | `[x]` | `[ ]` |
| E10 | Fehlendes Exec/TryExec | Vorabfehler, verständliche Meldung | `[x]` | `[ ]` |
| E11 | Fenster schließen | nur bei eindeutiger PID | `[x]` | `[ ]` |
| E12 | Kein Notfall-Stop | Capability bleibt false | `[x]` | `[ ]` |

## F. Session-Identität, Neustart, Suspend

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| F1 | CouchDeck neu starten, Spiel läuft weiter | Session wird adoptiert oder `Unknown`, nie blind „läuft“ | offen | `[ ]` |
| F2 | CouchDeck-Absturz während Session | Recovery korrekt, keine erfundene Endzeit | offen | `[ ]` |
| F3 | Systemneustart während Session | `boot_id` anders → Session beendet | offen | `[ ]` |
| F4 | Suspend/Resume während Session | Session bleibt; Schlafzeit zählt nicht | offen | `[ ]` |
| F5 | PID-Reuse-Simulation | Startzeit verhindert Verwechslung | offen | `[ ]` |
| F6 | Fenster weg, Prozess lebt | Session bleibt `Running` | `[x]` Ryujinx | `[ ]` |
| F7 | Prozess tot, Fenster kurz sichtbar | Session endet am Prozess, nicht am Fenster | `[x]` | `[ ]` |
| F8 | Close-Request, aber Prozess lebt | `StopUnconfirmed`, kein `Ended` | `[x]` Ryujinx | `[ ]` |
| F9 | Scope-/Unit-Ende nach Close | Settle-Frist, dann `Ended` | `[x]` | `[ ]` |

## G. Zeitkonten im Shadow Mode

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| G1 | Alte und neue Messung parallel | beide Werte getrennt, keine UI-Blockade | offen | `[ ]` |
| G2 | Home/Minimieren | neue Zeit läuft weiter, alte wie definiert | offen | `[ ]` |
| G3 | Fokuswechsel/Profilwechsel | Spielzeit pausiert nicht | offen | `[ ]` |
| G4 | Parallele Sessions desselben Profils | Vereinigung, keine Doppelzählung | offen | `[ ]` |
| G5 | Über Budget hinaus | `used_actual` steigt, Rest bleibt 0 | offen | `[ ]` |
| G6 | Zusatzminuten nach Überschreitung | Restzeit positiv, Warnflags neu | offen | `[ ]` |
| G7 | Suspend/Resume | keine Schlafzeit | offen | `[ ]` |
| G8 | Tageswechsel | Split an Mitternacht, Bonus reset | offen | `[ ]` |
| G9 | Absturz/Neustart | Lücke geloggt, konservative Policy | offen | `[ ]` |
| G10 | Generic GIO | konservativer Modus, keine stille Pause | offen | `[ ]` |
| G11 | Strikter Modus | nicht überwachbare Spiele blockiert | offen | `[ ]` |
| G12 | Nur eine Session pro Kind (optional) | zweiter Start blockiert | offen | `[ ]` |

## H. Zeitlimit-Durchsetzung

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| H1 | Warnung 5 min | einmalig, Ton optional | offen | `[ ]` |
| H2 | Warnung 1 min | einmalig | offen | `[ ]` |
| H3 | Budget 0 | neue Starts blockiert | offen | `[ ]` |
| H4 | Graceful Stop Steam | Spiel endet, UI bestätigt | offen | `[ ]` |
| H5 | Graceful Stop Prism | Java endet, UI bleibt | offen | `[ ]` |
| H6 | Graceful Stop Ryujinx | Unit endet (managed config) | offen | `[ ]` |
| H7 | Bestätigungsdialog | `StopUnconfirmed`, keine falsche Meldung | offen | `[ ]` |
| H8 | GracefulThenEmergency | Close → Timeout → Notfall-Stop | offen | `[ ]` |
| H9 | Elternfreigabe | PIN, Zusatzminuten, Starts wieder erlaubt | offen | `[ ]` |
| H10 | Alarmton | hörbar, kein Crash | offen | `[ ]` |

## I. Abnahme-Regeln

Ein Test gilt als bestanden, wenn:

1. Die erwartete Beobachtung reproduzierbar ist (mindestens 2 Läufe).
2. Die Evidenz (Logs, Prozess-/Scope-Ausgaben, Fenster-PIDs) dokumentiert ist.
3. Andere Anwendungen/Sessions unberührt bleiben.
4. Kein `pkill`, keine Prozessnamen-Suche, keine fremden Units verwendet
   wurden.
5. Die UI keine falsche Erfolgsmeldung zeigt.

## J. Bekannte Einschränkungen dieser Matrix

- Manjaro-Ergebnisse sind keine Bazzite-Abnahme.
- Kein Controller in der bisherigen Testumgebung; B11, B14, D14 und A4 sind
  offen.
- Kein vollständiger interaktiver Spielverlauf (Speichern/Laden) bisher.
- Steam Flatpak ist bewusst zurückgestellt.
- GNOME/XFCE und SteamOS Gaming Mode sind nicht Teil dieser Phase.
- Multi-Monitor-Vollbild ist compositorabhängig; Bazzite mit einem
  Wohnzimmerbildschirm testen.

## L. Session Supervisor (Phase III)

Voraussetzung: Rust-Supervisor und GIO-Helper gebaut (siehe
`docs/spikes/README.md`). Alle Stops nur über exakte PIDs/Units, kein `pkill`.

| ID | Test | Erwartung | Manjaro | Bazzite |
|---|---|---|---|---|
| L1 | systemd Service, `ExitType=main`, Hauptprozess endet | Unit inaktiv, Kind beendet | `[x]` | `[ ]` |
| L2 | systemd Service, `ExitType=cgroup`, Hauptprozess endet | Unit aktiv, `MainPID=0`, Kind lebt | `[x]` | `[ ]` |
| L3 | systemd Scope, Hauptprozess endet | Scope aktiv, kein `MainPID`, Stop nur `.scope` | `[x]` | `[ ]` |
| L4 | Service-Umgebung | Wayland/DBus/PATH erhalten, cwd gesetzt | `[x]` | `[ ]` |
| L5 | Subreaper: Early-Exit-Elternprozess | Kind wird adoptiert und getrackt | `[x]` | `[ ]` |
| L6 | Subreaper: Double-Fork | Enkel wird adoptiert und getrackt | `[x]` | `[ ]` |
| L7 | Subreaper: `setsid` | Prozess bleibt trotz neuer Session getrackt | `[x]` | `[ ]` |
| L8 | cgroup-Escape über `systemd-run --scope` | Supervisor trackt; systemd allein verliert | `[x]` | `[ ]` |
| L9 | D-Bus-Aktivierung | kein Callback, keine Ownership | `[x]` | `[ ]` |
| L10 | Single-Instance-Anwendung | kein neuer Prozess, keine Ownership | `[x]` | `[ ]` |
| L11 | Supervisor `SIGKILL` (direkt) | Kind lebt weiter | `[x]` | `[ ]` |
| L12 | Supervisor `SIGKILL` in `ExitType=cgroup`-Unit | Unit aktiv, `MainPID=0`, Spiel lebt | `[x]` | `[ ]` |
| L13 | Supervisor `SIGKILL` in `ExitType=main`-Unit | Unit inaktiv, Spiel beendet (Negativkontrolle) | `[x]` | `[ ]` |
| L14 | UI-/Launcher-Crash | Unit/Supervisor/Spiel laufen weiter | `[x]` | `[ ]` |
| L15 | `systemctl stop` der Unit | Spiel und cgroup beendet | `[x]` | `[ ]` |
| L16 | Ignoriertes SIGTERM, dann SIGKILL | SIGTERM nicht graceful; SIGKILL gezielt | `[x]` | `[ ]` |
| L17 | Managed GIO: direkt/Wrapper/multi/early/double | Prozessbaum wird getrackt | `[x]` | `[ ]` |
| L18 | Managed GIO: Wesnoth komplett | Fenster, Close, Supervisor-Ende | `[x]` | `[ ]` |
| L19 | Ryujinx Managed Root mit Symlinks | Saves/Firmware/Profile geteilt, kein Duplikat | `[x]` | `[ ]` |
| L20 | Performance | CPU/RSS/Startzeit/FPS dokumentiert | `[x]` | `[ ]` |
| L21 | Vollständiger Spielverlauf | Controller, Home, Speichern, Laden | offen | `[ ]` |

### Hinweise zu L1–L21

- `ExitType=cgroup` ist für Supervisor-Crash-Sicherheit zwingend.
- `Type=exec` meldet Exec-Fehler synchron; `Type=simple` kann zunächst
  „gestartet“ melden.
- Der Service erbt nicht das cwd des Aufrufers; `WorkingDirectory` setzen.
- `MainPID` einer Scope ist leer; `ControlGroup`/`cgroup.procs` verwenden.
- Nach einem Unit-Ende kann `systemctl show` `ExitType=main` als Default
  anzeigen; die Evidenz ist `ActiveState`/`MainPID`/`ControlGroup` während
  des Laufs.
- D-Bus/Single-Instance dürfen **nie** als managed gelten.
- Ein Supervisor-Timeout oder verschwundenes Fenster ist kein Spielende.
- Ein verbleibender Hilfsprozess ist kein Beweis, dass das Spiel läuft.
- Bei Ryujinx `show_confirm_exit=false` nur im managed Root setzen; die
  Benutzerkonfiguration nicht verändern.
- Controller: `/dev/input/js0` unter systemd und Supervisor prüfen;
  Ryujinx-Erkennung im Log (`Hid Configure`/`SetupNpad`).

## M. Ergebnisprotokoll (Vorlage)

```text
Test-ID:
Datum/Zeit:
Plattform/Version:
Desktop/Session:
Spiel/Backend:
Erwartet:
Beobachtet:
Prozess-/Scope-/Fenster-Evidenz:
Abweichungen:
Status: bestanden / teilweise / fehlgeschlagen
Offene Frage:
```
