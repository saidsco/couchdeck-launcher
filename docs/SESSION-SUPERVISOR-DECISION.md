# Session Supervisor – Architekturentscheidung

Stand: Phase III, 27. November 2026
Status: **Research-Entscheidung, keine produktive Integration.**

## Entscheidung

**Variante C: systemd-Unit + minimaler Child-Subreaper.**

Eine eigene transiente systemd-User-Unit ist der dauerhafte Anchor für
Prozessgruppe und Lebensdauer. In dieser Unit läuft ein kleiner
Subreaper-Supervisor, der den Spielprozess startet und Anwendungsereignisse
liefert.

```text
GameSessionManager
      │  startet transiente Unit
      ▼
capydeck-session-<id>.service        (Type=exec, ExitType=cgroup,
      │                               KillMode=control-group)
      ▼
session-supervisor                    (Subreaper, Ereignisse)
      │
      ▼
Spiel / Emulator / Managed-GIO-Helper
```

Verantwortlichkeiten:

| Ebene | Verantwortung |
|---|---|
| **systemd** | Prozessgruppierung, Unit-Identität (`InvocationID`, `ControlGroup`), Unit-Lebensdauer, Stop der gesamten Sitzung |
| **Supervisor** | Start des Spiels, Subreaper, Kindprozessüberwachung, Exit-Status/Signale, Sitzungsereignisse |
| **GameSessionManager** | Backend-Auswahl, Session-Zustand, Capabilities, Persistenz, Recovery |
| **Timekeeping** | Profilbezogene Spielzeit, unabhängig von UI-Fokus und Home |

## Begründung aus den Messungen

### Warum nicht A (systemd allein)?

systemd mit `Type=exec` + `ExitType=cgroup` ist ein ausgezeichneter Anchor:

- Unit bleibt aktiv, wenn der Hauptprozess endet und Kinder weiterlaufen.
- `MainPID=0`, `ControlGroup` bleibt gesetzt.
- `systemctl stop` beendet die gesamte cgroup.
- Supervisor-/UI-Crash kann die Unit überleben.

Aber systemd allein:

- liefert keine Ereignisse für einzelne Kindprozesse,
- unterscheidet nicht „Hauptspiel“ und „Hilfsprozess“,
- kennt keine Fenster-PID oder Spielidentität,
- verliert Prozesse, die in eine andere cgroup wechseln,
- kann bei einem dauerhaft laufenden Hilfsprozess die Spielsitzung nicht
  beenden, obwohl das Spiel schon weg ist.

Für Zeitkonten und Session-Zustände reicht das nicht.

### Warum nicht B (Subreaper allein)?

Der Subreaper liefert genau die fehlende Anwendungsebene:

- Adoption von Early-Exit-, Double-Fork-, `setsid`- und cgroup-escaped
  Nachfahren,
- Exit-Codes/Signale für direkte und adoptierte Kinder,
- Fenster-/Prozessidentität über PID + Startzeit,
- sehr geringer Overhead (ca. 2,7 MB RSS, 0 % CPU im Leerlauf).

Aber:

- stirbt der Supervisor, sind Ereignisse und Adoption weg,
- das Spiel lebt zwar weiter, aber es gibt keinen dauerhaften Unit-Anchor,
- ein UI-Crash kann den Supervisor verwaisen lassen.

Allein ist der Subreaper kein belastbarer Recovery-Anker.

### Warum C?

C kombiniert die Stärken:

- `ExitType=cgroup` hält die Unit am Leben, wenn der Supervisor stirbt.
- Der Supervisor liefert die Anwendungsebene.
- CouchDeck kann nach einem Supervisor-Crash über Unit/cgroup/Fenster
  wieder anknüpfen und die Sitzung stoppen.
- Ein UI-Crash beeinflusst die Unit nicht.
- Der Stop der Unit beendet die gesamte Sitzung.

### Warum nicht D (bisherige Backend-Architektur beibehalten)?

Die bisherige Architektur kann Direct/Managed-Sessions starten und über
Unit/PID beenden, aber sie hat keinen zuverlässigen Ereignisstrom für
Early-Exit-, Double-Fork- und cgroup-escaped Fälle. Für sessionbasierte
Zeitkontingente ist der Subreaper ein messbarer Mehrwert, insbesondere für
nicht-Steam-/nicht-Prism-Anwendungen. D wäre nur dann richtig, wenn der
Subreaper keinen praktischen Vorteil brächte; die Tests zeigen das Gegenteil.

## Unit-Konfiguration

Empfohlene transiente Unit:

```text
Unit:        capydeck-session-<session-id>.service
Type:        exec
ExitType:    cgroup
KillMode:    control-group
WorkingDirectory: explizit setzen
CollectMode: inactive-or-failed (bzw. --collect)
```

Wichtig:

- `Type=exec` liefert Exec-Fehler synchron.
- `ExitType=cgroup` ist für Supervisor-Crash-Sicherheit zwingend; mit
  `ExitType=main` würde die Unit beim Supervisor-Ende beendet und die cgroup
  gekillt.
- `KillMode=control-group` sorgt dafür, dass ein bewusster Stop alle
  Sitzungsprozesse beendet.
- `WorkingDirectory` muss gesetzt werden; der systemd-Service erbt nicht das
  cwd des Aufrufers.
- Die Unit-Namen müssen eindeutig und aus der Session-ID ableitbar sein.
- `InvocationID` und `ControlGroup` werden in `sessions.json` gespeichert.

## Supervisor-Aufgaben

Der Supervisor:

- setzt `PR_SET_CHILD_SUBREAPER=1`,
- startet genau den Spielprozess mit strukturierten Argumenten,
- blockiert `SIGCHLD` und nutzt `signalfd`,
- entdeckt Nachfahren über `/proc/<pid>/task/<pid>/children`,
- reapt verwaiste Nachfahren,
- protokolliert `spawn`, `adopted`, `discovered`, `exit`, `disappeared`,
  `cgroup_changed`, `main_exit`, `supervisor_exit`,
- zeichnet PID + Startzeit + Boot-ID auf,
- optional pidfd für sichere Signale,
- sendet **keine** Signale an Kinder, wenn er selbst beendet wird,
- beendet sich bei `--wait cgroup` erst, wenn die bekannte Prozessgruppe leer
  ist (mit Settle-Frist).

Der Supervisor ist **nicht** der Ort für:

- Zeitbudget-Entscheidungen,
- automatische Terminierung,
- Backend-Auswahl,
- Fensterfokus,
- Steam-/Prism-Steuerung.

## GameSessionManager

Der Manager:

- erzeugt die Session-ID und die Unit,
- wählt das Backend,
- startet die Unit über systemd (nicht als direkten Kindprozess der UI),
- liest Supervisor-Ereignisse (Socket/Datei/Pipe),
- hält den Session-Zustand,
- bestätigt Capabilities pro Session,
- persistiert `sessions.json`,
- erkennt Supervisor-Ausfall und wechselt in `TrackingLost`,
- reattachiert nach UI-Neustart an bestehende Units,
- stoppt Sessions nur nach verifizierter Identität und ausdrücklicher Aktion.

## Fehlerfälle

| Ereignis | Verhalten |
|---|---|
| Spiel endet regulär | Supervisor meldet `main_exit`/`cgroup_empty`; Unit endet; Session `Ended` |
| Spiel-Crash | Supervisor meldet Exit-Signal; Unit endet; Session `Ended`/`Failed` |
| Supervisor-Crash | Unit bleibt `active` (`ExitType=cgroup`); Spiel läuft; Session `TrackingLost`; Fallback über Unit/cgroup/Fenster |
| UI-Crash | Unit und Supervisor laufen weiter; beim Neustart Reattach über `sessions.json`/Unit |
| Unit-Stop | `systemctl stop` beendet die cgroup; Session `Stopping` → `Ended` |
| Unit hängt | `--no-block` + Timeout + `SIGKILL` auf die eigene Unit; `StopUnconfirmed` |
| cgroup-Escape | Subreaper trackt, solange er lebt; nach Supervisor-Crash ggf. nicht mehr über die alte cgroup auffindbar → `TrackingLost` |
| D-Bus/Single-Instance | kein Supervisor-Ownership; `RunningUntracked`/nicht managed |
| Fehlender Start | `spawn_failed`/GIO `LOAD_FAILED`; `Failed`; kein Retry |

## TrackingLost

`TrackingLost` ist ein eigener Zustand, nicht `Ended`:

- Die Unit kann weiterleben, aber der Supervisor liefert keine Ereignisse
  mehr.
- CouchDeck prüft Unit, cgroup, Fenster und PID/Startzeit.
- Die Zeitmessung darf nicht stillschweigend pausieren. Sie zählt
  konservativ weiter, bis ein belastbarer Endebeleg vorliegt, oder blockiert
  neue Starts im strikten Modus.
- Ein abgelaufener Beobachtungs-Timeout ist **kein** bestätigtes Spielende.
- Ein verbleibender Hilfsprozess ist **kein** Beweis, dass das Spiel läuft.

## Backend-Anwendung

### Direct/Managed (Ryujinx, native Spiele/Emulatoren)

- Variante C.
- Unit + Supervisor + managed Config.
- Nachgewiesen mit Ryujinx (Mario Kart 8 Deluxe) und dem nativen
  Pillars of Eternity: Fenster-PID = Kind-PID, Unit-Stop ohne Reste.
- Regulär: Fensterschließen über die exakte PID (nur wenn die Anwendung das
  unterstützt; Ryujinx mit `show_confirm_exit=false`).
- Fallback: `systemctl stop` (SIGTERM an die cgroup), dann Notfall-SIGKILL.
- Kein automatischer Stop aufgrund von Zeitlimit oder Fensterverlust.

### Steam nativ

- **Kein zusätzlicher Supervisor.**
- Steam-eigene Spiel-Scopes und Reaper-Struktur bleiben die Grundlage.
- App-ID + Scope + cgroup + Spiel-PID + Fenster-PID.
- Regulär: Fenster über exakte Spiel-PID; Fallback SIGTERM auf exakte PID;
  Notfall: eigene Spiel-Scope stoppen.
- Keine Änderungen an Steam-Startoptionen/Proton.

### Prism/Minecraft

- **Kein zusätzlicher Supervisor.**
- Instanz-ID + Java-PID + Flatpak-Instanz + Instanzpfad.
- Regulär: Minecraft-Fenster über exakte Java-PID.
- Notfall: `flatpak kill` nur, wenn keine andere Session in derselben
  Flatpak-Instanz läuft.

### Generic GIO

- Normaler `gio launch`-Pfad bleibt unverändert und vollständig kompatibel.
- Optionaler **Managed-GIO-Modus** mit Supervisor + Unit für Desktop-Dateien,
  die einen kontrollierbaren Prozessbaum spawnen.
- D-Bus-Aktivierung und Single-Instance-Anwendungen bleiben
  `RunningUntracked`; keine Ownership vortäuschen.
- Kein automatischer zweiter Start nach unklarem Erststart.

## Ryujinx Managed Config

- Verwalteter Root mit eigenem `Config.json`:
  - `start_fullscreen=true`
  - `remember_window_state=false`
  - `show_confirm_exit=false`
- Zustandsverzeichnisse symlinken:
  - `bis`, `system`, `games`, `profiles`, `mods`, `sdcard`
- Dadurch bleiben Saves, Firmware, Profile und Controllerkonfiguration
  gemeinsam; keine zweite Emulatorinstallation.
- Keine dauerhafte Änderung der Benutzerkonfiguration.
- Vollständiger Save/Load-Test bleibt für die produktive Freigabe offen.

## Beenden: drei getrennte Ebenen

1. **Reguläre Anwendungsschließung**
   - Fenster über exakte PID schließen (KWin).
   - Nur wenn die Anwendung Close unterstützt.
   - Ryujinx: `show_confirm_exit=false` nötig.
2. **SIGTERM / Unit-Stop**
   - `systemctl stop` sendet SIGTERM an die cgroup; je nach Anwendung ist das
     **nicht** graceful.
   - Nur nach verifizierter Session-Identität.
   - Timeout und `StopUnconfirmed` beachten.
3. **SIGKILL / Notfall**
   - `systemctl kill --signal=KILL` auf die eigene Unit oder pidfd-SIGKILL.
   - Nur ausdrücklich konfiguriert/freigegeben.
   - Nie `pkill`, nie Prozessnamen-Matching, nie fremde Units.

## Zeitkonten

- Primär: Supervisor-Ereignisse (`main_exit`, `exit`, `cgroup_empty`).
- Fallback bei Supervisor-Verlust: Unit/cgroup + PID-Startzeit + Fenster.
- Kein ausschließliches Abstellen auf Fensterexistenz oder
  Supervisor-Lebensdauer.
- `TrackingLost` konservativ behandeln, nicht stillschweigend pausieren.
- Parallele Sessions, Suspend, Tageswechsel und Elternfreigaben wie in
  [`TIME-ACCOUNTING.md`](TIME-ACCOUNTING.md).
- Shadow Mode bleibt bestehen; keine produktive Ablösung in dieser Phase.

## Implementierungsaufwand und Risiken

**Aufwand:**

- Kleiner Supervisor (Rust, libc) – überschaubar, aber sicherheitskritisch.
- Unit-/Session-Persistenz und Recovery – mittel.
- Managed GIO – opt-in, zusätzliche Komplexität.
- Ryujinx-Managed-Root – gering, aber versionsabhängig.
- Tests auf Bazzite – erforderlich.

**Risiken:**

- Supervisor-Crash: ohne `ExitType=cgroup` würde das Spiel sterben; die
  Property ist zwingend.
- cgroup-Escape nach Supervisor-Crash ist nicht vollständig rekonstruierbar.
- Falsche Ownership bei D-Bus/Single-Instance; muss `Untracked` bleiben.
- Signal-Eskalation kann Spielstände gefährden; standardmäßig aus.
- Ryujinx-Config-/Root-Änderungen können Saves duplizieren, wenn nicht
  symlinkt wird.
- Startlatenz +95 ms (Wesnoth) durch Helper/Supervisor; akzeptabel, aber
  messbar.

## Empfehlung für die Roadmap

### 0.6.1 – Session Foundation

- Session-Modell, `sessions.json`, `TrackingLost`.
- Transiente Unit (`Type=exec`, `ExitType=cgroup`, `KillMode=control-group`).
- Supervisor-Prototyp für Direct/Managed.
- Managed Ryujinx-Config mit Symlinks.
- Shadow-Zeitmessung aus Supervisor-Ereignissen + Fallback.
- Keine automatische Terminierung.

### 0.6.2 – Zeitkonten-Härtung

- Supervisor-Crash-/UI-Crash-Recovery.
- `TrackingLost`-Policy, konservative Zeitmessung.
- Suspend/Resume, Neustart, PID-Reuse.
- Warnungen, Grace Period, Startsperre.
- Shadow-Vergleich.

### 0.6.3 – Kontrollierte Beendigung

- Reguläres Fensterschließen für verifizierte Fälle.
- Unit-Stop/SIGTERM als zweite Ebene.
- Notfall-SIGKILL optional und bestätigt.
- `StopUnconfirmed`-UI.
- Vollständiger Spielverlauf mit Speichern/Laden auf Bazzite.

### 0.7.0 – Weitere Backends

- Managed GIO opt-in für geeignete Desktop-Dateien.
- Weitere Emulatoren/Launcher.
- Steam Flatpak, GNOME/XFCE.

## Abnahmekriterien

Die Entscheidung gilt als bestätigt, wenn auf Bazzite KDE/Wayland:

1. `ExitType=cgroup` den Supervisor-Crash überlebt und das Spiel weiterläuft.
2. `systemctl stop` die Sitzung zuverlässig beendet, ohne fremde Prozesse.
3. Der Supervisor Early-Exit/Double-Fork/setsid/cgroup-Escape trackt.
4. D-Bus/Single-Instance nicht fälschlich als managed gelten.
5. Die Zeitmessung bei Supervisor-Verlust nicht stillschweigend pausiert.
6. Ryujinx-Saves mit managed Root geteilt bleiben.
7. Ein vollständiger Spielverlauf inkl. Controller, Home, Speichern und Laden
   funktioniert.
8. Kein messbarer FPS-Nachteil gegenüber dem direkten Start nachweisbar ist;
   echte Frame-Time-Messungen in Spielen bleiben offen.
