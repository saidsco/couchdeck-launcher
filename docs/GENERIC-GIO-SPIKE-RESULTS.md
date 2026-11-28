# Generischer GIO-Spike – Ergebnisse

Stand: Phase II, 26. November 2026
Getestet auf Manjaro Linux / KDE Plasma 6.8.2 / Wayland.

Testartefakte:

- `org.wesnoth.Wesnoth.desktop` (Battle for Wesnoth, nativ, Shell-Wrapper)
- temporäre Desktop-Dateien für direkte Prozesse, Mehrkindprozesse,
  Early-Exit-Launcher, D-Bus-Aktivierung und fehlende Executable
- C-Spike: [`spikes/gio-pid-spike.c`](spikes/gio-pid-spike.c)

## Phase-I-Ergebnis (kurz)

`gio launch /home/chris/Projekte/couchtest/org.wesnoth.Wesnoth.desktop`
startete Wesnoth über:

```ini
Exec=sh -c "wesnoth >/dev/null 2>&1"
```

Beobachtet wurden:

```text
sh -c wesnoth >/dev/null 2>&1
  └─ wesnoth
```

Das Fenster `The Battle for Wesnoth - 1.18.8` hatte die PID des
`wesnoth`-Prozesses; die App-ID/Ressourcenklasse war `wesnoth`. Es gab keine
spielspezifische systemd-Scope/cgroup. Ein KWin-Script konnte das Fenster über
die exakte PID schließen.

## Phase-II-Frage: PID-Callback der GIO-API

Untersucht wurde:

```c
g_desktop_app_info_launch_uris_as_manager(appinfo, uris, context,
    G_SPAWN_SEARCH_PATH | G_SPAWN_DO_NOT_REAP_CHILD |
    G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
    NULL, NULL, launch_cb, NULL, &error);
```

Der Callback erhält die PID des unmittelbar von GIO gestarteten Prozesses. Der
Spike misst zusätzlich die Zeit bis zum Callback und prüft anschließend
Prozessbaum, Fenster und Lebensdauer.

### Test 1: Direkter Prozess ohne Shell

```ini
Exec=sleep 300
```

Ergebnis:

```text
CALLBACK pid=<sleep-pid> dt_ms=1
RESULT ok=1 total_ms=1
```

Der Callback lieferte die PID des tatsächlichen Prozesses. Das ist der
günstigste Fall.

### Test 2: Shell-Wrapper (Wesnoth)

```ini
Exec=sh -c "wesnoth >/dev/null 2>&1"
```

Ergebnis:

```text
CALLBACK pid=<sh-pid> dt_ms=1
```

Prozessbaum:

```text
sh -c wesnoth >/dev/null 2>&1   (Callback-PID)
  └─ wesnoth                    (Fenster-PID)
```

Der Callback lieferte **nicht** die Spiel-PID, sondern den Wrapper. Die
Zuordnung zum Spiel war nur über den Prozessbaum und das Fenster möglich.

### Test 3: Mehrere Kindprozesse

```ini
Exec=sh -c "sleep 300 & sleep 301 & wait"
```

Ergebnis:

```text
CALLBACK pid=<sh-pid>
  ├─ sleep 300
  └─ sleep 301
```

Der Callback liefert nur den direkten Startprozess. Weitere Kinder sind nicht
Teil des Callbacks.

### Test 4: Startprozess endet frühzeitig

```ini
Exec=sh -c "sleep 300 >/dev/null 2>&1 & exit 0"
```

Ergebnis:

```text
CALLBACK pid=<sh-pid> dt_ms=1
RESULT ok=1 total_ms=1
```

Der Callback-Prozess (`sh`) war bereits beendet, als er untersucht wurde. Das
Kind `sleep 300` lief weiter (reparented). GIO meldete den Start trotzdem als
Erfolg.

Damit ist der Callback **kein Beweis** für die Lebensdauer des eigentlichen
Spiels.

### Test 5: D-Bus-aktivierte Anwendung (Spectacle)

`/usr/share/applications/org.kde.spectacle.desktop` hat
`DBusActivatable=true`. Spectacle lief bereits.

Ergebnis:

```text
LOADED id=org.kde.spectacle.desktop name=Spectacle hidden=0 dbusactivatable=1
RESULT ok=1 total_ms=0
```

- **Kein CALLBACK** wurde aufgerufen.
- Es wurde kein neuer Prozess gestartet.
- Die bestehende Instanz übernahm den Startauftrag über D-Bus.

Für D-Bus-Aktivierung liefert die GIO-Launch-API also keine PID. Das ist für
einen Kompatibilitätsmodus relevant, aber nicht für Prozessownership.

### Test 6: Bereits laufende Anwendung

- Wesnoth ist kein Single-Instance-Programm: Ein zweiter Start erzeugte einen
  neuen `sh`-Wrapper, einen neuen `wesnoth`-Prozess und ein zweites Fenster.
  Der Callback lieferte die neue Wrapper-PID.
- Spectacle (D-Bus) übernahm den Auftrag ohne neuen Prozess und ohne Callback.

„Bereits laufend“ kann also je nach Anwendung entweder einen neuen Prozess oder
gar keinen sichtbaren Prozess bedeuten.

### Test 7: Mehrere Instanzen derselben Anwendung

Zwei Wesnoth-Starts erzeugten zwei unabhängige Wrapper-/Spiel-Prozesse und zwei
Fenster. Jeder Start hatte seinen eigenen Callback. Eine Zuordnung „welcher
Callback gehört zu welchem Fenster“ ist nur über den Prozessbaum und die
Fenster-PID möglich.

### Test 8: Fehlendes Executable / TryExec

Desktop-Dateien mit

```ini
Exec=capydeck-nope-xyz --flag
```

oder

```ini
TryExec=/nonexistent/capydeck-tryexec
Exec=sleep 300
```

wurden von `g_desktop_app_info_new_from_filename()` **nicht geladen**
(`LOAD_FAILED`, Rückgabewert `NULL`). Das ist eine brauchbare Vorabprüfung:
offensichtlich fehlende Programme können vor dem Start erkannt werden.

Das ersetzt keine Laufzeitfehlerdiagnose: Ein vorhandenes Programm kann trotzdem
sofort abbrechen oder den Startauftrag an eine andere Instanz weiterreichen.

## Bewertung der GIO-API

### Was der PID-Callback leistet

- Bei direkt gespawnten Prozessen: sofortige, korrekte PID.
- Bei Wrapper-Skripten: PID des Wrappers, nicht des Spiels.
- Bei Early-Exit-Startern: PID kann bereits tot sein.
- Bei D-Bus-Aktivierung: kein Callback.
- Bei Single-Instance-Anwendungen: möglicherweise kein neuer Prozess.
- Bei mehreren Kindern: nur der direkte Startprozess.

### Was daraus für CapyDeck folgt

- Der externe `gio launch`-Pfad bleibt für generische Desktop-Dateien
  unverändert. Die bestehende GIO-Kompatibilität darf nicht eingeschränkt
  werden.
- Die GIO-API kann optional für Diagnose/Startattribute genutzt werden, ist aber
  **kein Ersatz** für Session-Ownership.
- Eine Session darf niemals allein aufgrund von `RESULT ok=1` als „läuft“
  gelten.
- Für den Kompatibilitätsmodus ist eine heuristische Beobachtung möglich:
  - Fenster-App-ID/Ressourcenklasse,
  - Fenster-PID,
  - Prozessbaum,
  - Zeitfenster nach dem Start.
- Diese Heuristik bleibt ausdrücklich unzuverlässig. Sie darf nicht für
  Notfall-Terminierung oder harte Zeitdurchsetzung verwendet werden.

## Nachgewiesene Erkenntnisse

1. `g_desktop_app_info_launch_uris_as_manager()` liefert über den
   `GDesktopAppLaunchCallback` die PID des unmittelbar gespawnten Prozesses.
2. Der Callback ist bei direkten Prozessen korrekt, bei Shell-Wrappern aber nur
   die Wrapper-PID.
3. Bei Early-Exit-Startern kann die Callback-PID bereits beendet sein, während
   das eigentliche Spiel weiterläuft.
4. Bei D-Bus-Aktivierung wird kein Callback aufgerufen.
5. Bereits laufende Anwendungen können den Startauftrag ohne neuen Prozess
   übernehmen.
6. Fehlende Executables/TryExec werden beim Laden der Desktop-Datei erkannt.
7. Der Callback allein ist kein Beweis für Identität, Lebensdauer oder
   Spielende.

## Technische Annahmen

- GIO-/GLib-Version und Anwendungstyp beeinflussen das Startverhalten. Die
  Ergebnisse wurden mit GLib 2.90.1 und den genannten Testprogrammen erhoben.
- D-Bus-Aktivierung ist nicht grundsätzlich unzuverlässig; sie ist nur für
  Prozessownership ungeeignet.
- Die Fenster-Heuristik kann bei Spielen mit mehreren Fenstern, Splash-Screens
  oder Launcher-Fenstern fehlschlagen.

## Offene Forschungsfragen

- Spiele mit mehreren Fenstern oder Launcher-vorgeschaltetem Start.
- Anwendungen, die den Startauftrag an eine bereits laufende Instanz übergeben
  und dabei ein neues Fenster im bestehenden Prozess öffnen.
- GIO-API mit Portalen/Flatpak-Exports.
- GNOME/XFCE-Fensterbeobachtung.
- Bazzite-Abnahme.

## Capability-Bewertung

| Fähigkeit | Status | Begründung |
|---|---|---|
| Start | ✅ | `gio launch` funktioniert |
| Start erkennen | ⚠️ nur heuristisch | Callback nur bei gespawntem Prozess |
| Session zuordnen | ❌/⚠️ | kein Prozessownership |
| Ende erkennen | ❌/⚠️ | kein zuverlässiger Anker |
| Fokus | ❌ | nicht backendgestützt |
| Regulär beenden | ⚠️ | nur bei eindeutigem Fenster |
| Notfall-Stop | ❌ | kein eigener Prozess-/Scope-Handle |

**Qualitätsstufe: Kompatibilitätsbackend (experimentell).** Geeignet für
normale Desktop-Dateien, nicht für Managed Sessions, harte Zeitdurchsetzung
oder Notfall-Terminierung.

## Reproduktion

```bash
# C-Spike bauen
gcc -O2 -Wall -Wextra -o /tmp/gio-pid-spike docs/spikes/gio-pid-spike.c \
  $(pkg-config --cflags --libs gio-unix-2.0)

# Direkter Prozess
/tmp/gio-pid-spike /tmp/gio-direct.desktop

# Shell-Wrapper (Wesnoth)
/tmp/gio-pid-spike /tmp/gio-wesnoth.desktop
```

Die temporären Desktop-Dateien sind in [`spikes/README.md`](spikes/README.md)
beschrieben.
