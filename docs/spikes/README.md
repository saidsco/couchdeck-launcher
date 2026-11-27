# Session-Spike-Werkzeuge

Stand: Phase III, 27. November 2026

Kleine, nicht produktive Testwerkzeuge für die GameSessionManager-Forschung.
Sie gehören nicht zum CapyDeck-Laufzeitcode und werden nicht installiert.

## `gio-pid-spike.c`

Untersucht den PID-Callback von
`g_desktop_app_info_launch_uris_as_manager()`.

```bash
gcc -O2 -Wall -Wextra -o /tmp/gio-pid-spike docs/spikes/gio-pid-spike.c \
  $(pkg-config --cflags --libs gio-unix-2.0)
```

### Test-Desktop-Dateien

```ini
# /tmp/gio-direct.desktop
[Desktop Entry]
Type=Application
Name=GIO direct sleep
Exec=sleep 300
Terminal=false
```

```ini
# /tmp/gio-earlyexit.desktop
[Desktop Entry]
Type=Application
Name=GIO early-exit launcher
Exec=sh -c "sleep 300 >/dev/null 2>&1 & exit 0"
Terminal=false
```

```ini
# /tmp/gio-multichild.desktop
[Desktop Entry]
Type=Application
Name=GIO multi-child launcher
Exec=sh -c "sleep 300 & sleep 301 & wait"
Terminal=false
```

```ini
# /tmp/gio-wesnoth.desktop
[Desktop Entry]
Type=Application
Name=GIO Wesnoth wrapper
Exec=sh -c "wesnoth >/dev/null 2>&1"
Icon=wesnoth
Terminal=false
```

```ini
# /tmp/gio-missing.desktop
[Desktop Entry]
Type=Application
Name=GIO missing executable
Exec=/nonexistent/capydeck-missing-binary --flag
Terminal=false
```

### Ausführen

```bash
/tmp/gio-pid-spike /tmp/gio-direct.desktop
/tmp/gio-pid-spike /tmp/gio-earlyexit.desktop
/tmp/gio-pid-spike /tmp/gio-multichild.desktop
/tmp/gio-pid-spike /tmp/gio-wesnoth.desktop
/tmp/gio-pid-spike /tmp/gio-missing.desktop
```

Beobachtung danach:

```bash
ps -eo pid,ppid,comm,args | grep -E '[s]leep 30[01]|[w]esnoth'
```

Die erzeugten `sleep`-Prozesse nach dem Test über ihre exakten PIDs beenden.
Kein `pkill`.

## `capydeck-testapp.py`

GTK4-Testanwendung für Direct/Managed-Robustheit.

Voraussetzungen: Python 3, PyGObject, GTK4.

```bash
# Fenster bleibt offen, Kindprozess 300 s
systemd-run --user --unit=capydeck-testapp --collect \
  --property=KillMode=control-group \
  --working-directory=/tmp \
  python3 docs/spikes/capydeck-testapp.py stay

# Fenster wird nach 3 s zerstört und nach 2 s neu erstellt;
# Kindprozess läuft nur 5 s, damit der Supervisor schnell endet
systemd-run --user --unit=capydeck-testapp-recreate --collect \
  --property=KillMode=control-group \
  --working-directory=/tmp \
  python3 docs/spikes/capydeck-testapp.py recreate 5
```

Prüfen:

```bash
systemctl --user show capydeck-testapp \
  -p Id -p ActiveState -p MainPID -p ControlGroup
ps -eo pid,ppid,cgroup:180,comm,args | grep -E '[c]apydeck-testapp|[s]leep 300'
```

Crash-Test (nur die exakte MainPID):

```bash
mp=$(systemctl --user show capydeck-testapp -p MainPID --value)
kill -KILL "$mp"
```

Erwartung:

- Unit wird in ca. 1 s inaktiv.
- Kindprozess wird durch `KillMode=control-group` beendet.
- Das Fenster kann kurzzeitig in der KWin-Liste verbleiben.

## `session-supervisor` (Rust)

Minimaler Linux-Subreaper für Phase III. Kein Produktivcode.

```bash
CARGO_TARGET_DIR=/tmp/capydeck-supervisor-target \
  cargo build --release --offline \
  --manifest-path docs/spikes/session-supervisor/Cargo.toml
```

Binary:

```text
/tmp/capydeck-supervisor-target/release/session-supervisor
```

Beispiel:

```bash
S=/tmp/capydeck-supervisor-target/release/session-supervisor
$S --wait cgroup --child-stdio null \
  --events /tmp/session.jsonl -- sleep 5
```

Wichtige Optionen:

- `--wait main|cgroup` – Ende nach Hauptprozess oder leerer bekannter Gruppe.
- `--events PATH` – JSONL-Ereignisse in Datei (sonst stdout).
- `--child-stdio inherit|null` – Stdio des Kindes.
- `--cwd PATH` – Arbeitsverzeichnis des Kindes.
- `--new-session` – `setsid()` im Kind.
- `--pidfd` – pidfds für getrackte Prozesse öffnen.
- `--settle-ms N` – Settle-Frist vor `cgroup_empty` (Default 300).
- `--discover-ms N` – `/proc`-Discovery-Intervall (Default 100).
- `--timeout SECS` – Test-Timeout.
- `--stop-after SECS --stop-signal TERM|KILL` – **nur Testzwecke**; sendet
  einmalig ein Signal an den Hauptprozess (pidfd, falls verfügbar).

Ereignisse: `supervisor_start`, `spawn`, `spawn_failed`, `adopted`,
`discovered`, `cgroup_changed`, `exit`, `disappeared`, `main_exit`,
`children_remaining`, `supervisor_signal`, `supervisor_exit`.

Sicherheit:

- Der Supervisor sendet ohne `--stop-after` keine Signale an Kinder.
- `SIGKILL`/`SIGTERM` auf den Supervisor lassen die Kinder weiterlaufen.
- Es wird nur die eigene Nachfahrenschaft beobachtet.
- Keine Prozessnamen als Ownership-Beweis; Identität ist PID + Startzeit.

### Synthetische Szenarien

```bash
# Früh endender Elternprozess; Kind wird adoptiert
$S --wait cgroup --child-stdio null --events /tmp/early.jsonl \
  -- sh -c 'sleep 3 & exit 0'

# Double-Fork
$S --wait cgroup --child-stdio null --events /tmp/double.jsonl \
  -- sh -c '(sleep 2 &); exit 0'

# setsid
$S --wait cgroup --child-stdio null --events /tmp/setsid.jsonl \
  -- sh -c 'setsid sleep 4'

# Hauptprozess beendet sich, Kind läuft weiter
$S --wait main --child-stdio null --events /tmp/main.jsonl \
  -- sh -c 'sleep 5 & exit 0'

# Supervisor-Crash simulieren: Supervisor-PID aus der Shell killen
$S --wait cgroup --child-stdio null --events /tmp/crash.jsonl -- sleep 30 &
kill -KILL $!
```

### Variante C (systemd + Supervisor)

```bash
systemd-run --user --unit=capydeck-session-test --collect \
  --property=Type=exec \
  --property=ExitType=cgroup \
  --property=KillMode=control-group \
  -- $S --wait cgroup --child-stdio null \
     --events /tmp/c-session.jsonl -- sleep 30
```

Dann Supervisor-PID killen und prüfen:

```bash
systemctl --user show capydeck-session-test \
  -p ActiveState -p MainPID -p ControlGroup -p ExitType
cat /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/capydeck-session-test.service/cgroup.procs
```

Erwartung: Unit bleibt `active`, `MainPID=0`, Kind lebt; `systemctl stop`
beendet es.

### Variante A (systemd allein)

```bash
# Default ExitType=main: Kind wird beim Haupt-Exit gekillt
systemd-run --user --unit=capydeck-a1 --collect \
  --property=Type=exec --property=KillMode=control-group \
  -- bash -c 'sleep 10 & exit 0'

# ExitType=cgroup: Unit bleibt, bis die cgroup leer ist
systemd-run --user --unit=capydeck-a2 --collect \
  --property=Type=exec --property=ExitType=cgroup \
  --property=KillMode=control-group \
  -- bash -c 'sleep 10 & exit 0'
```

## `gio-managed-spawn.c`

Managed-GIO-Launcher für den Supervisor.

```bash
gcc -O2 -Wall -Wextra -o /tmp/gio-managed-spawn \
  docs/spikes/gio-managed-spawn.c \
  $(pkg-config --cflags --libs gio-unix-2.0)
```

```bash
$S --wait cgroup --child-stdio inherit \
  --events /tmp/gio.jsonl \
  -- /tmp/gio-managed-spawn --stdout-null /tmp/app.desktop
```

Optionen:

- `--do-not-reap` – setzt `G_SPAWN_DO_NOT_REAP_CHILD`.
- `--stdout-null` – leitet Stdout/Stderr des gestarteten Programms nach
  `/dev/null`.

Der Supervisor muss **vor** dem GIO-Launch eingerichtet sein. Nach dem
GIO-Launch beendet sich der Helper; verwaiste Nachfahren werden adoptiert.

Wichtige Fälle:

- Direkter Prozess, Shell-Wrapper, mehrere Kinder, Early-Exit, Double-Fork,
  `setsid`: trackbar.
- D-Bus/Single-Instance: kein Callback, kein neuer Nachfahre → **nicht
  managed**.
- Fehlendes Executable: `gio_load_failed`, kein Nachfahre.

## `env-probe.c`

Vergleicht ausgewählte Umgebungs- und FD-Eigenschaften.

```bash
gcc -O2 -Wall -Wextra -o /tmp/env-probe docs/spikes/env-probe.c
```

```bash
/tmp/env-probe
systemd-run --user --unit=cd-env-svc --collect \
  --property=Type=exec --wait --pipe /tmp/env-probe
systemd-run --user --scope --unit=cd-env-scope --collect -- /tmp/env-probe
$S --wait cgroup --child-stdio inherit \
  --events /tmp/env-sup.jsonl -- /tmp/env-probe
```

Es werden nur ausgewählte Schlüssel ausgegeben (kein vollständiger
Umgebungsdump).



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

Fenster nur über die exakte PID schließen:

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

## Aufräumen

- Erzeugte `sleep`-Prozesse über ihre exakten PIDs beenden.
- Test-Desktop-Dateien aus `/tmp` löschen.
- Keine Benutzerkonfigurationen verändern; für Ryujinx einen temporären Root
  verwenden.
