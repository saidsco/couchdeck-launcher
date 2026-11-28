# Session Supervisor – Spike-Ergebnisse

Stand: Phase III, 27. November 2026
Status: **Research, keine produktive Integration.**
Getestet auf Manjaro Linux / KDE Plasma 6.8.2 / Wayland, Kernel 6.18.63,
systemd 262.

## 1. Fragestellung

Gesucht ist ein schlanker Supervisor nach dem Vorbild des Steam Reapers, der
den Lebenszyklus gestarteter Anwendungen zuverlässig erkennt, ohne die
Laufzeitumgebung der Spiele unnötig zu verändern. Drei Varianten wurden
untersucht:

- **A:** systemd allein (`Type=exec`, `ExitType=cgroup`, Service/Scope)
- **B:** eigener Linux Child Subreaper (Rust-Prototyp)
- **C:** Subreaper innerhalb einer eigenen systemd-Unit

Zusätzlich: Managed GIO, Ryujinx-Managed-Config, Kompatibilität, Performance und
Lebenszyklus/Zeitkonten.

Testwerkzeuge:

- [`spikes/session-supervisor/`](spikes/session-supervisor/) – Rust-Subreaper
- [`spikes/gio-managed-spawn.c`](spikes/gio-managed-spawn.c) – GIO-Launchmanager
- [`spikes/env-probe.c`](spikes/env-probe.c) – Umgebungs-/FD-Vergleich
- [`spikes/capydeck-testapp.py`](spikes/capydeck-testapp.py) – GTK4-Test-App

## 2. Testumgebung

| Komponente | Version |
|---|---|
| Distribution | Manjaro Linux, Kernel 6.18.63-3 |
| Desktop | KDE Plasma 6.8.2, KWin 6.8.2, Wayland |
| systemd | 262.2 |
| Rust / Cargo | 1.89.0-beta.3 |
| Supervisor-Abhängigkeit | libc 0.2.191 |
| Ryujinx | 1.3.3, .NET 9.0.9 |
| Steam | nativ |
| Prism Launcher | 11.1.1 Flatpak |
| Controller | 8BitDo Ultimate Wireless / Pro 2 (`/dev/input/js0`) |

Wiederholungen: Wesnoth-Prozessstart 3× je Variante, `vkcube` 3× je Variante,
Ryujinx 2× (normaler Close + Supervisor-Crash), Supervisor-Crash/Recovery 2×,
systemd-Lebenszyklus A1–A3 je 1–2×. `setsid` und cgroup-Escape wurden nach
einem ersten Race mit längerer Laufzeit wiederholt.

## 3. Variante A – systemd ohne eigenen Subreaper

### A1: Default `ExitType=main`

Test:

```bash
systemd-run --user --unit=cd-sup-a1 --collect \
  --property=Type=exec --property=KillMode=control-group \
  -- bash -c 'sleep 311 & echo $! > /tmp/child; exit 0'
```

Ergebnis:

- Unit sofort `inactive`/`dead`.
- `MainPID=0`, `ControlGroup` leer.
- Das Kind `sleep` wurde beendet (`alive=no`).

**Bedeutung:** Ein normaler Service betrachtet die Sitzung als beendet, sobald
der Hauptprozess endet. `KillMode=control-group` beendet verbleibende
Kindprozesse.

### A2: `ExitType=cgroup`

Gleicher Test mit `--property=ExitType=cgroup`:

- Unit blieb `active`/`running`.
- `MainPID=0`.
- `ControlGroup` zeigte weiterhin auf die Unit-cgroup.
- Das Kind `sleep` lief weiter.
- `systemctl --user stop` beendete das Kind und die Unit.

**Bedeutung:** systemd kann die Unit-Lebensdauer an die gesamte cgroup binden,
auch wenn der Hauptprozess bereits weg ist.

### A3: Hauptprozess stirbt, Kind bleibt

Test mit einem Hauptprozess, der auf ein Kind wartet; danach `SIGKILL` auf den
Hauptprozess:

- Unit blieb `active`/`running`.
- `MainPID=0`.
- Kind lief weiter.
- `systemctl stop` beendete das Kind.

Das ist die Grundlage für „Supervisor/UI weg, Unit und Spiel bleiben“.

### A4: Scope statt Service

```bash
systemd-run --user --scope --unit=cd-sup-a4 --collect \
  --property=KillMode=control-group \
  bash -c 'sleep 314 & exit 0'
```

Ergebnis:

- Scope blieb `active`, solange das Kind lief.
- Kein `MainPID`.
- Stop nur mit `cd-sup-a4.scope`; `systemctl stop cd-sup-a4` sucht eine
  `.service` und schlägt fehl.
- `systemd-run --scope` blockiert den Aufrufer und unterstützt `--pipe` nicht.

### A5: Umgebung von Service und Scope

`env-probe` zeigte:

- **Service (`Type=exec`):** Wayland, Display, DBus, `XDG_RUNTIME_DIR`, `PATH`
  und `HOME` bleiben erhalten. `cwd` ist `/home/chris` (nicht das
  Aufruferverzeichnis). Der Prozess bekommt eine eigene Session/Prozessgruppe.
- **Scope:** Wayland/DBus/PATH bleiben erhalten; `cwd` ist das
  Aufruferverzeichnis; Session/Prozessgruppe werden vom Aufrufer geerbt.
- `child_subreaper=0` in beiden Fällen.

**Konsequenz:** Ein Service muss `WorkingDirectory` explizit setzen, wenn das
Spiel ein bestimmtes Arbeitsverzeichnis erwartet. Die wichtigen GUI-Variablen
fehlen nicht.

### A6: Fehlgeschlagener Exec

- Fehlende Executable: `systemd-run` verweigert den Start bereits.
- Vorhandene Datei mit ungültigem Interpreter (`#!/nonexistent/...`):
  - `Type=exec`: Unit `failed`, `Result=exit-code`, `ExecMainStatus=203`.
  - `Type=simple`: Unit wird zunächst gestartet und fällt dann auf `failed`.
  - `Type=exec` liefert dem Aufrufer den Fehler synchron; `Type=simple` meldet
    möglicherweise fälschlich „gestartet“.

### A7: cgroup-Escape

Ein Prozess startete über `systemd-run --scope` eine neue Scope:

```text
cd-escape-sd2-inner.scope
```

- Die äußere Unit `cd-escape-sd2` wurde `inactive`, obwohl der Prozess in der
  inneren Scope weiterlief.
- `systemctl stop` der äußeren Unit beendete die innere Scope nicht.
- Der Prozess lief in der neuen Scope weiter, bis er selbst endete.

**Bedeutung:** `ExitType=cgroup` betrachtet nur die eigene cgroup. Prozesse,
die in eine andere cgroup wechseln, gehen für systemd verloren.

### A8: Bewertung Variante A

| Fähigkeit | Status |
|---|---|
| Unit-Lebensdauer an cgroup | ✅ |
| MainPID/InvocationID/ControlGroup | ✅ |
| ExitStatus des Hauptprozesses | ✅ |
| Erkennung „Hauptspiel vs. Hilfsprozess“ | ❌ |
| Ereignisse für einzelne Kindprozesse | ❌ |
| Erkennung von Fenster-PID/Spielidentität | ❌ |
| Tracking nach cgroup-Escape | ❌ |
| Supervisor-/UI-Crash ohne Spielabbruch | ✅ mit `ExitType=cgroup` |
| Stop der gesamten Sitzung | ✅ `systemctl stop` |
| Umgebungs-/GUI-Kompatibilität | ✅ (cwd beachten) |

**Fazit:** systemd allein ist ein sehr guter **durable anchor** für
Prozessgruppe und Unit-Lebensdauer, aber kein Anwendungssupervisor. Es liefert
keine spielbezogenen Ereignisse und kann Haupt- und Hilfsprozesse nicht
unterscheiden.

## 4. Variante B – eigener Child Subreaper

### B1: Aufbau

Der Rust-Prototyp:

- setzt `PR_SET_CHILD_SUBREAPER=1`,
- blockiert `SIGCHLD`/`SIGTERM`/`SIGINT` und nutzt `signalfd`,
- spawnt genau einen Befehl,
- entdeckt Nachfahren über `/proc/<pid>/task/<pid>/children`,
- reapt verwaiste Nachfahren über `waitpid(-1, WNOHANG)`,
- protokolliert `spawn`, `adopted`, `discovered`, `exit`, `disappeared`,
  `cgroup_changed`, `main_exit`, `supervisor_exit`,
- zeichnet PID **und** `/proc/<pid>/stat`-Startzeit auf,
- kann pidfds öffnen und Signale über `pidfd_send_signal` senden,
- beendet sich standardmäßig nicht mit Signalweitergabe an die Kinder.

### B2: Synthetische Lebenszyklus-Matrix

| Szenario | Beobachtung |
|---|---|
| Direkter Prozess (`sleep 1`) | `spawn` → `exit` → `main_exit` → `cgroup_empty` |
| Shell-Wrapper mit wartendem Elternprozess | Wrapper und Kind werden getrackt |
| Früh endender Elternprozess (`sh -c 'sleep 3 & exit 0'`) | Eltern `exit`; Kind wird `adopted`; Supervisor wartet auf Kind |
| Mehrere Kindprozesse | Kinder werden `discovered`; Exit-Status ggf. nicht sichtbar, wenn der Wrapper sie reapt |
| Double-Fork (`(sleep 2 &); exit 0`) | Enkel wird `adopted` und getrackt |
| `setsid` (10 s) | Prozess wird trotz neuer Session getrackt; `sid`/`pgrp` ändern sich, die PID bleibt |
| Hauptprozess-Crash (`SIGKILL`) | `exit` mit `signal=9`; Supervisor bleibt bis cgroup leer |
| Kind läuft nach Hauptprozess weiter | `--wait main` beendet Supervisor und lässt Kind laufen; `--wait cgroup` wartet |
| Fenster zerstören/neu erstellen | Prozess/Unit bleiben; Fensterwechsel beendet die Session nicht |
| Dauerhafter Hilfsprozess | Supervisor kann Haupt-Exit und Hilfsprozess getrennt protokollieren |
| Fehlgeschlagener Start | `spawn_failed`, Exit 2, keine falsche Sitzung |
| Ignoriertes SIGTERM | `test_signal` gesendet; Hauptprozess lebt weiter |
| SIGKILL über pidfd | `signal=9`, Ende erkannt |
| Supervisor `SIGKILL` | Kind lebt weiter, reparentet zu systemd/init |
| Supervisor `SIGTERM` | Supervisor beendet sich sauber, Kind lebt weiter |
| cgroup-Escape über `systemd-run --scope` | Kind wird adoptiert und weiter getrackt, obwohl es die cgroup gewechselt hat |

**Wichtige Einschränkung:** Wenn ein Wrapper seine Kinder selbst reapt, sieht
der Subreaper für diese Kinder oft nur `disappeared` statt eines Exit-Status.
Das ist für die Sitzungsdauer ausreichend, aber nicht für detaillierte
Exit-Codes.

### B3: Supervisor-Crash

- `SIGKILL` auf den Supervisor: Kind lebt weiter (PPID wird 1144/systemd).
- `SIGTERM` auf den Supervisor: Supervisor protokolliert `supervisor_signal`,
  beendet sich, Kind lebt weiter.
- Es wurde kein `PR_SET_PDEATHSIG` gesetzt; der Supervisor reißt seine Kinder
  bei eigenem Ende nicht mit.

Das ist für „Supervisor-Crash darf das Spiel nicht beenden“ genau richtig.

### B4: pidfd und Signalidentität

- `pidfd_open` funktionierte für Haupt- und Kindprozesse.
- `pidfd_send_signal` sendete `SIGTERM` und `SIGKILL` gezielt.
- Der Supervisor protokolliert PID + Startzeit; die Startzeit ist die
  eigentliche Identität, nicht `comm` oder `argv0`.
- Ein `adopted`-Event kann `comm=sh` zeigen, obwohl der Prozess kurz darauf
  `sleep` wird. Das ist eine Lese-Race, keine Identitätsänderung.

### B5: Performance

| Messung | Ergebnis |
|---|---|
| Supervisor-CPU bei `sleep` (8 s) | 0,000 % |
| Supervisor-CPU bei `glxgears` (8 s) | ca. 0,125 % |
| Supervisor-RSS | ca. 2,7 MB |
| `vkcube` 60 Frames, Supervisor + `--settle-ms 0` | 0,565–0,615 s (wie direkt) |
| `vkcube` 60 Frames, Supervisor default (300 ms Settle) | 0,915–0,916 s |
| `glxgears` FPS | 143,9–144,4 FPS (vsync-limitiert) |
| `glxgears` Prozess-CPU direkt | 1,4–1,8 % (2 Läufe) |
| `glxgears` Prozess-CPU supervised | 2,0 % (2 Läufe) |
| Wesnoth-Prozessstart direkt | 0,063 s (3 Läufe) |
| Wesnoth-Prozessstart supervised | 0,158–0,161 s (3 Läufe) |
| Ryujinx-Fenster supervised | ca. 12,7 s (Phase II direkt: ca. 13 s) |
| Pillars of Eternity Fenster supervised | ca. 2,5 s (kein direkter Vergleich) |

Der Supervisor selbst kostet im Leerlauf praktisch keine CPU und ca. 2,7 MB
RSS. Der apparente `vkcube`-Unterschied von 300 ms ist der konfigurierbare
Settle-Delay **nach dem Spielende**, nicht eine Verlangsamung des Spiels. Mit
`--settle-ms 0` verschwindet er.

Die `glxgears`-Prozess-CPU lag direkt bei 1,4–1,8 %, supervised bei 2,0 %
(jeweils 2 Läufe). Die absolute Differenz ist klein und bei dieser Last nicht
signifikant; die FPS blieben vsync-limitiert bei ca. 144. Für echte Spiele
lässt sich daraus kein pauschaler Overhead ableiten; die Supervisor-CPU selbst
lag bei ca. 0,125 %.

Der Wesnoth-Prozessstart ist supervised ca. **95 ms** langsamer. Das kommt
durch den zusätzlichen GIO-Helper und den Supervisor-Start. Für den
Spielstart ist das unkritisch, aber messbar. Eine produktive Integration könnte
den Launcher direkt im Supervisor ausführen, um diesen Overhead zu reduzieren.

### B6: Kompatibilität

- `env-probe` unter Supervisor: gleiche Wayland-/Display-/DBus-/PATH-/HOME-
  Variablen wie direkt; `cwd` bleibt erhalten; Session/Prozessgruppe bleiben
  erhalten, sofern nicht `--new-session` gesetzt ist.
- `child_subreaper=0` im Kind: das Subreaper-Flag wird **nicht** vererbt.
- `vulkaninfo`, `vkcube`, `glxgears`, `pactl`, `jstest` funktionierten unter
  Supervisor.
- Der Controller (8BitDo) wurde in Ryujinx als ProController Player1 erkannt,
  wenn das verwaltete Root die Controllerprofile teilte.

### B7: Bewertung Variante B

| Fähigkeit | Status |
|---|---|
| Ereignisse für Hauptprozess | ✅ |
| Adoption verwaister Nachfahren | ✅ |
| Early-Exit / Double-Fork / setsid | ✅ |
| cgroup-Escape (solange Supervisor lebt) | ✅ |
| Exit-Codes/Signale für direkte/adoptierte Kinder | ✅ |
| Exit-Codes für von Wrappern gereapte Enkel | ⚠️ nur `disappeared` |
| D-Bus-/Single-Instance-Ownership | ❌ |
| Tracking nach Supervisor-Crash | ❌ |
| CPU/RSS-Overhead | ✅ sehr gering |
| Startlatenz | ⚠️ ca. +95 ms (Wesnoth) |

**Fazit:** Der Subreaper liefert genau die Anwendungsebene, die systemd fehlt.
Er ist aber allein kein dauerhafter Anchor: stirbt er, gehen Ereignisse und
Adoption verloren.

## 5. Variante C – systemd + Subreaper

### C1: Supervisor-Crash mit `ExitType=cgroup`

Aufbau:

```bash
systemd-run --user --unit=cd-sup-c1 --collect \
  --property=Type=exec --property=ExitType=cgroup \
  --property=KillMode=control-group \
  -- session-supervisor --wait cgroup -- sleep 30
```

Dann `SIGKILL` auf den Supervisor:

- Unit blieb `active`/`running`.
- `MainPID=0`.
- `ControlGroup` blieb gesetzt.
- Das Kind `sleep` lief weiter.
- `systemctl stop` beendete das Kind und die Unit.

### C2: Default `ExitType=main`

Gleicher Test ohne `ExitType=cgroup`:

- Unit `inactive`/`dead`.
- Kind wurde mitbeendet.

**Konsequenz:** `ExitType=cgroup` ist für die Supervisor-Crash-Sicherheit
zwingend.

Zusatz: Auch ein **normaler** Supervisor-Exit (`--wait main`) lässt die Unit
mit `ExitType=cgroup` aktiv, solange das Kind in der cgroup läuft
(`MainPID=0`). Für den produktiven Betrieb bleibt `--wait cgroup` dennoch
sinnvoll, weil nur dann der Supervisor weiter Ereignisse liefert.

### C3: Ryujinx unter Supervisor + systemd

Managed Root mit `start_fullscreen=true`, `remember_window_state=false`,
`show_confirm_exit=false`:

- Unit `cd-ryu-sup.service`, `Type=exec`, `ExitType=cgroup`.
- Supervisor = `MainPID` 2096019.
- Ryujinx = Kind 2096020, Fenster-PID = 2096020, Vollbild.
- Minimieren/Wiederherstellen: Ryujinx blieb am Leben, Vollbild blieb.
- Fenster schließen: Ryujinx-Exit nach ca. 0,93 s; Supervisor-Exit nach ca.
  1,23 s; Unit inaktiv nach ca. 2,03 s.
- Supervisor-Events: `spawn`, `exit` (Code 0), `main_exit`, `supervisor_exit`.

### C4: Supervisor-Crash mit Ryujinx

- `SIGKILL` auf den Supervisor.
- Unit blieb `active`, `MainPID=0`.
- Ryujinx lief weiter, cgroup enthielt weiterhin die Ryujinx-PID.
- Fenster blieb vorhanden.
- `systemctl stop` beendete Ryujinx.

Das ist der zentrale Recovery-Fall: **Die Unit bleibt der durable Anchor; der
Supervisor ist ein Ereignislieferant, kein Single Point of Failure.**

### C5: Simulierter CapyDeck-UI-Crash

Eine Dummy-UI startete die Unit und wurde danach `SIGKILL`t:

- Unit blieb `active`.
- Supervisor und Spiel liefen weiter.
- `systemctl stop` beendete das Spiel.

Die UI ist nicht der Parent der Unit; ihr Tod beeinflusst die Sitzung nicht.
Der **echte** CapyDeck-Prozess mit `sessions.json`-Recovery wurde in dieser
Phase nicht gestartet; das bleibt ein offener Integrationstest.

### C6: Bewertung Variante C

| Fähigkeit | Status |
|---|---|
| Durable Unit-/cgroup-Anchor | ✅ |
| Anwendungsereignisse | ✅ solange Supervisor lebt |
| Supervisor-Crash ohne Spielabbruch | ✅ mit `ExitType=cgroup` |
| UI-Crash ohne Spielabbruch | ✅ |
| Recovery über Unit/cgroup/window | ✅ |
| Stop der Sitzung | ✅ `systemctl stop` |
| cgroup-Escape nach Supervisor-Crash | ⚠️ nicht mehr über die alte cgroup auffindbar |
| Komplexität | ⚠️ zwei Ebenen, aber klar getrennt |

**Fazit:** Variante C ist die beste Kombination aus Zuverlässigkeit,
Anwendungsebene und Recovery.

## 6. Managed GIO

### 6.1 Aufbau

`gio-managed-spawn` ruft
`g_desktop_app_info_launch_uris_as_manager()` auf. Der Supervisor wird **vor**
dem Start eingerichtet und ist der Subreaper. Nach dem GIO-Launch beendet sich
der Helper; verwaiste Nachfahren werden vom Supervisor adoptiert.

### 6.2 Ergebnisse

| Desktop-Fall | Callback-PID | Supervisor-Tracking | Bewertung |
|---|---|---|---|
| Direkter Prozess (`sleep 4`) | `sleep` | `adopted`, Exit 0 | ✅ managed |
| Shell-Wrapper (Wesnoth) | `sh` | `sh` adoptiert, `wesnoth` discovered, Fenster-PID = wesnoth | ✅ managed |
| Mehrere Kindprozesse | `sh` | Kinder discovered; `disappeared`, wenn `sh` sie reapt | ⚠️ Lifetime ja, Exit-Status nein |
| Early-Exit (`sh -c 'sleep & exit'`) | `sh` | `sh` adoptiert, `sleep` adoptiert | ✅ managed |
| Double-Fork | `sh` | Enkel adoptiert | ✅ managed |
| `setsid` | `sh`/`sleep` | gleiche PID wird `sleep`, neue Session, weiter getrackt | ✅ managed |
| cgroup-Escape (`systemd-run --scope`) | `sh` | `sleep` in neuer Scope wird adoptiert und weiter getrackt | ✅ solange Supervisor lebt |
| D-Bus (Spectacle) | **kein Callback** | kein neuer Nachfahre | ❌ nicht managed |
| Single-Instance (Spectacle läuft) | **kein Callback** | kein neuer Nachfahre | ❌ nicht managed |
| Fehlendes Executable | GIO `LOAD_FAILED` | Helper-Exit 3, kein Nachfahre | ✅ sauberer Fehler |
| `--do-not-reap` | wie default | gleiches Adoptionsverhalten | ✅ kein Unterschied im Test |

### 6.3 cgroup-Escape im Detail

Ein Managed-GIO-Start mit

```ini
Exec=sh -c "systemd-run --user --scope --unit=... sh -c 'sleep 8 & exit 0'"
```

führte dazu, dass `sleep 8` in eine **neue Scope** wechselte. Der Supervisor
adoptierte den Prozess über den Subreaper und trackte ihn bis zum Ende. Ein
systemd-only-Ansatz mit `ExitType=cgroup` hätte die äußere Unit bereits nach
dem Ende des Wrappers als beendet betrachtet, obwohl das Spiel weiterlief.

**Aber:** Stirbt der Supervisor, bevor das escaped Spiel endet, ist der Prozess
nicht mehr über die alte Unit-cgroup auffindbar. Das bleibt ein Randfall.

### 6.4 Bewertung Managed GIO

- Managed GIO ist für Desktop-Dateien geeignet, die tatsächlich einen
  kontrollierbaren Prozessbaum spawnen.
- D-Bus-Aktivierung und Single-Instance-Anwendungen bleiben
  **nicht managed**; keine Ownership vortäuschen.
- Der Supervisor liefert Lifetime und (bei direkt/adoptierten Prozessen)
  Exit-Status; bei von Wrappern gereapten Enkeln nur `disappeared`.
- Kein automatischer zweiter Start nach unklarem Erststart.
- Managed GIO bleibt **opt-in**; der normale `gio launch`-Pfad bleibt
  unverändert.

## 7. Ryujinx Managed Config

Getestet wurde ein verwalteter Root:

```text
/tmp/ryu-shared/
  Config.json          (managed: start_fullscreen=true,
                        remember_window_state=false,
                        show_confirm_exit=false)
  bis        -> ~/.config/Ryujinx/bis
  system     -> ~/.config/Ryujinx/system
  games      -> ~/.config/Ryujinx/games
  profiles   -> ~/.config/Ryujinx/profiles
  mods       -> ~/.config/Ryujinx/mods
  sdcard     -> ~/.config/Ryujinx/sdcard
```

Ergebnis:

- Ryujinx startete mit dem managed `Config.json`.
- Vollbild war aktiv.
- Die Save-Dateien unter `~/.config/Ryujinx/bis/user/save` wurden aktualisiert
  (z. B. `1795786768`), also **geteilt**, nicht dupliziert.
- Firmware/System, Spiele-Cache, Mods, SD-Karte und Controllerprofile wurden
  über Symlinks geteilt.
- `prod.keys`/`title.keys` liegen in der Ryujinx-Installation und wurden nicht
  verändert.
- Der Controller wurde als **ProController Player1** erkannt.
- Ein separater Config-Root erzeugt damit **keine** zweite unabhängige
  Emulatorinstallation, solange die Zustandsverzeichnisse symlinken.
- Kein vollständiger interaktiver Save/Load-Test; die Save-Dateien wurden
  geladen/geschrieben, aber nicht durch menügesteuertes Speichern/Laden
  verifiziert.

**Empfehlung:** Managed Config nur für `Config.json`; `bis`, `system`, `games`,
`profiles`, `mods`, `sdcard` symlinken. `Logs` kann separat bleiben. Vor
produktiver Nutzung muss die Symlink-Strategie auf Bazzite und mit
Ryujinx-Updates erneut geprüft werden.

## 8. Steam-/Prism-Regression

Beide Backends wurden nach den Supervisor-Tests noch einmal als
unveränderte Struktur geprüft:

- Steam nativ (LEGO Bricktales): Spiel-Scope
  `app-steam-app1898290-2105142.scope`, Fenster-PID `2105296`, Fenster
  fullscreen; Schließen beendete die Scope; Steam blieb aktiv.
- Prism/Minecraft: Flatpak-Instanz `701840162`, Java-PID `2105786`,
  Minecraft-Fenster; Schließen beendete Java und die Flatpak-Instanz
  (keine Prism-UI geöffnet).

Wichtig für den Start: Ohne bereits laufende Prism-UI blockierte
`flatpak run org.prismlauncher.PrismLauncher --launch <id>` im Vordergrund bis
zum Spielende. Der Launch-Client ist langlebig; CapyDeck darf ihn nicht
synchron im UI-Thread ausführen. Mit geöffneter UI wurde der Auftrag dagegen
sofort weitergereicht (Phase II).

Es wurden **keine** Supervisor- oder Unit-Änderungen an Steam/Prism
vorgenommen.

### 8.1 Weiteres natives Spiel: Pillars of Eternity

Zusätzlich wurde **Pillars of Eternity** (nativer Linux-ELF,
Unity) direkt unter der Unit + Supervisor gestartet:

```text
Unit: cd-pillars.service (Type=exec, ExitType=cgroup,
      KillMode=control-group)
Supervisor: 2111751
Spiel: 2111752 (PillarsOfEternity)
Fenster: 2111752, Caption "Pillars of Eternity", fullscreen
```

- Fenster nach ca. 2,5 s.
- Fenster-PID = Spiel-PID = Supervisor-Kind.
- `systemctl stop` beendete das Spiel; keine Restprozesse.
- Der Start erfolgte ohne zusätzliche Steam-Runtime direkt aus dem
  Spielverzeichnis.

Das bestätigt den Direct/Managed-Pfad für ein weiteres natives Spiel.

## 9. Kompatibilität und minimale Eingriffe

| Variante | cwd | pgrp/sid | stdio | Wayland/DBus | Subreaper im Kind |
|---|---|---|---|---|---|
| Direkt | Aufrufer | Aufrufer | geerbt | vorhanden | 0 |
| systemd Service | `/home/chris` | eigene Session | konfiguriert/`/dev/null` | vorhanden | 0 |
| systemd Scope | Aufrufer | Aufrufer | geerbt | vorhanden | 0 |
| Supervisor direkt | Aufrufer | Aufrufer | geerbt | vorhanden | 0 |
| Supervisor + systemd | `/home/chris` | eigene Session | konfiguriert | vorhanden | 0 |
| Supervisor `--new-session` | Aufrufer | eigene Session | geerbt | vorhanden | 0 |

Zusätzlich geprüft:

- **Vulkan/OpenGL:** `vulkaninfo`, `vkcube`, `glxgears` funktionierten unter
  systemd und Supervisor.
- **Audio:** `pactl info` funktionierte unter systemd und Supervisor
  (PipeWire 1.6.11).
- **Controller:** `jstest` öffnete `/dev/input/js0` unter systemd und
  Supervisor; Ryujinx erkannte den 8BitDo-Controller als ProController.
- **Save/Config:** Ryujinx-Shared-Root teilt Saves/Firmware/Profile; keine
  Duplikate.
- **Keine** LD_PRELOAD-, ptrace-, Root-, pkill- oder Prozessnamen-Ownership-
  Mechanismen.
- **Keine** vollständigen Umgebungsdumps: `env-probe` protokolliert nur
  ausgewählte Schlüssel; der Supervisor protokolliert `argv0`/`comm`, nicht die
  gesamte Umgebung. Für Produktion müssen Argumente redigiert werden.

## 9.5 Eignung für Shadow-Timekeeping

| Variante | Session-Start | Session-Ende | Haupt-/Hilfsprozess trennbar | Crash-Fallback | Eignung |
|---|---|---|---|---|---|
| A systemd allein | Unit active | cgroup leer | ❌ | Unit | mittel |
| B Subreaper allein | `spawn` | `main_exit`/`exit`/`cgroup_empty` | ✅ | ❌ | gut, aber fragil |
| C systemd + Subreaper | `spawn` | `main_exit`/`exit`/`cgroup_empty` | ✅ | Unit/cgroup/Fenster | **am besten** |

Für das Shadow-Timekeeping:

- **C** liefert die präzisesten Start-/Endeanchor und einen Fallback, wenn der
  Supervisor stirbt.
- `main_exit` und `cgroup_empty` müssen getrennt bewertet werden: Ein
  Hilfsprozess kann die cgroup offen halten, obwohl das Spiel beendet ist.
- `disappeared` (von Wrappern gereapt) liefert keinen Exit-Code, aber einen
  Lebenszyklus-Hinweis.
- Ein Supervisor-Timeout ist kein Spielende; `TrackingLost` konservativ
  behandeln.
- Ein Fenster ist weder Start- noch Endebeleg.
- Steam/Prism bleiben bei ihren eigenen Ankern (Scope/Java-PID), nicht beim
  Direct/Managed-Supervisor.

## 10. Nachgewiesene Erkenntnisse

1. `systemd` mit `Type=exec` + `ExitType=cgroup` hält die Unit am Leben,
   solange Prozesse in ihrer cgroup sind; `MainPID` wird 0.
2. Default `ExitType=main` beendet die Unit beim Hauptprozess-Ende und killt
   per `KillMode=control-group` die Kinder.
3. Ein Scope hat keinen `MainPID`; Stop nur mit `.scope`.
4. `systemd` allein liefert keine spielbezogenen Ereignisse und kann
   Haupt-/Hilfsprozesse nicht unterscheiden.
5. Ein Subreaper adoptiert Early-Exit-, Double-Fork-, `setsid`- und
   cgroup-escaped Nachfahren und liefert Exit-Codes/Signale für direkte bzw.
   adoptierte Kinder.
6. Von Wrappern gereapte Enkel erscheinen als `disappeared` (kein Exit-Status).
7. Supervisor-Crash beendet das Spiel nicht; das Kind reparentet zu
   systemd/init.
8. Supervisor + `ExitType=cgroup`: Unit bleibt nach Supervisor-Crash aktiv,
   `MainPID=0`, Spiel läuft weiter; Stop der Unit beendet das Spiel.
9. UI-Crash beeinflusst die systemd-Unit nicht.
10. Managed GIO ist für kontrollierbare Prozessbäume brauchbar; D-Bus und
    Single-Instance bleiben nicht managed.
11. cgroup-Escape wird vom Subreaper (solange er lebt) erkannt, von systemd
    allein nicht.
12. Ryujinx-Managed-Config mit symlinkten Zustandsverzeichnissen teilt Saves,
    Firmware und Profile; keine zweite Emulatorinstallation.
13. Der Supervisor kostet ca. 2,7 MB RSS und im Leerlauf 0 % CPU; der
    Spielstart wird bei Wesnoth um ca. 95 ms verzögert.
14. Der 300-ms-Settle verzögert nur das Supervisor-Ende, nicht das Spiel; mit
    `--settle-ms 0` ist die Gesamtdauer wie direkt.
15. `Type=exec` meldet Exec-Fehler synchron; `Type=simple` kann zunächst
    „gestartet“ melden.
16. Ein weiteres natives Spiel (Pillars of Eternity) startete direkt unter
    Unit + Supervisor; Fenster-PID = Kind-PID, Unit-Stop ohne Reste.

## 11. Technische Annahmen

- systemd 262 auf Bazzite verhält sich wie auf Manjaro.
- KWin bleibt unter Bazzite/KDE der Fenstermanager.
- Die cgroup-/Unit-Semantik bleibt in systemd-User-Sitzungen stabil.
- Ryujinx behält die Config-Schlüssel und die Root-Struktur.
- GIOs Launchmanager-Verhalten bleibt für die genannten Desktop-Typen stabil.
- Der Controller wird auf Bazzite ebenfalls über evdev/js erkannt.

## 12. Offene Forschungsfragen

- Bazzite KDE/Wayland-Abnahme für alle Varianten.
- Echter CapyDeck-Absturz und Recovery über `sessions.json` (bisher nur
  Dummy-UI).
- Vollständiger interaktiver Spielverlauf mit Controller, Home-Rückkehr,
  Speichern und erneutem Laden.
- Verhalten bei Suspend/Resume und Systemneustart.
- Supervisor-Crash, während das Spiel in einer anderen cgroup läuft.
- Sehr große Prozessbäume/Launcher (Steam/Proton, Prism) unter Supervisor.
- Weitere GIO-Desktop-Typen (Portale, Flatpak-Exports).
- Ryujinx-Updates und Änderungen an Config/Root.
- Langzeit-CPU/RSS über Stunden.
- FPS/Frametimes in echten Spielen (bisher nur synthetisch/vsync-limitiert).
- PID-Reuse über echte Neustarts.

## 13. Empfehlung

Die detaillierte Entscheidung steht in
[`SESSION-SUPERVISOR-DECISION.md`](SESSION-SUPERVISOR-DECISION.md).

Kurzfassung: **Variante C** – eine eigene systemd-Unit
(`Type=exec`, `ExitType=cgroup`, `KillMode=control-group`) als durable Anchor,
darin ein minimaler Subreaper-Supervisor für Anwendungsereignisse. Kein
zusätzlicher Supervisor für Steam oder Prism. Managed GIO bleibt opt-in.
