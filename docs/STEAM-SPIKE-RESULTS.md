# Steam-Spike – Ergebnisse

Stand: Phase II, 26. November 2026
Getestet auf Manjaro Linux / KDE Plasma 6.8.2 / Wayland, Kernel 6.18.63, nativem
Steam (kein Flatpak-Steam im Test) und NVIDIA RTX 4070 Ti SUPER.

Verwendete Spiele:

| Spiel | App-ID | Runtime | Zweck |
|---|---|---|---|
| LEGO Bricktales | `1898290` | native Linux (SteamLinuxRuntime Soldier) | natives Linux-Spiel |
| Disco Elysium | `632470` | Proton 8.0 (`8.0-5d`, compatdata `8.0-103`) | Windows-/Proton-Spiel |
| Stray | `1332010` | Proton 9.0 (Beta) | weiteres Proton-Spiel, Close-Verhalten |

## Phase-I-Ergebnis (kurz)

Der erste Spike startete LEGO Bricktales über die vorhandene Desktop-Datei
(`Exec=steam steam://rungameid/1898290`) und beobachtete eine spielspezifische
Scope `app-steam-app1898290-<pid>.scope`. Das Fenster ließ sich über die exakte
PID schließen; die Scope verschwand, der Steam-Client blieb aktiv.

## Klärung „nativ oder Proton?“

Der Phase-I-Bericht nannte als Spielprozess `LEGO_Bricktales.x86_64`. Das ist
der **native Linux-Build**, nicht ein Windows-Binary unter Proton. Die
Behauptung, LEGO Bricktales sei in diesem Test über Proton gelaufen, ist für
diese Installation nicht belegt und nach den vorliegenden Daten falsch.

Belege aus der aktuellen Installation:

- Im Spielverzeichnis liegt `LEGO_Bricktales.x86_64` mit `UnityPlayer.so`;
  es gibt kein `LEGO_Bricktales.exe`.
- `steamapps/compatdata/1898290` ist leer (kein Proton-Prefix).
- In `config/config.vdf` gibt es keinen `CompatToolMapping`-Eintrag für
  `1898290`.
- Die Startkette verwendet
  `SteamLinuxRuntime_soldier/_v2-entry-point` und
  `scout-on-soldier-entry-point-v2`, nicht `Proton`.
- `STEAM_COMPAT_*`-Umgebungsvariablen sind Runtime-Metadaten; sie beweisen
  allein keinen Proton-Einsatz.

Damit gilt: Der Phase-I-Test war ein **natives Linux-Spiel**. Für Proton wurde
in Phase II separat Disco Elysium getestet.

## Phase-II-Test 1: Natives Steam-Spiel (LEGO Bricktales)

### Start und Scope

`gio launch "$HOME/.local/share/applications/LEGO Bricktales.desktop"` wurde als
normaler CapyDeck-GIO-Pfad verwendet.

Beobachtung:

- `gio launch` kehrte sofort mit Exit-Code 0 zurück.
- Nach ca. 4 s erschien:
  ```text
  app-steam-app1898290-1487529.scope
  ```
- Nach ca. 7–11 s erschien das Spielfenster.
- Fenster-PID: `1487640` (`LEGO_Bricktales.x86_64`).
- Fenster-App-ID/Ressourcenklasse: `LEGO_Bricktales.x86_64`.

Prozesskette (gekürzt):

```text
steam-runtime-launch-client
  └─ steam-runtime-supervisor
      └─ reaper SteamLaunch AppId=1898290
          └─ SteamLinuxRuntime_soldier/_v2-entry-point
              └─ scout-on-soldier-entry-point-v2
                  └─ LEGO_Bricktales.x86_64   (PID 1487640)
```

Die Scope enthielt den Reaper, die Soldier-Runtime und den Spielprozess. Der
Steam-Client lag außerhalb der Spiel-Scope.

### Scope-Identität

`systemctl --user show app-steam-app1898290-1487529.scope` lieferte:

```text
Id=app-steam-app1898290-1487529.scope
ActiveState=active
SubState=running
ControlGroup=/user.slice/user-1000.slice/user@1000.service/app.slice/app-steam-app1898290-1487529.scope
```

Wichtig:

- `MainPID` war **leer**. Bei einer Scope ist `MainPID` kein verlässlicher
  Anker.
- Die PID im Scope-Namen (`1487529`) ist der erste Prozess (Reaper), **nicht**
  das Spiel.
- Die Spiel-PID muss über `ControlGroup`/`cgroup.procs` und das Fenster
  ermittelt werden.

### Reguläres Beenden

Ein KWin-Script schloss nur das Fenster mit der exakten Spiel-PID:

```javascript
if (window.pid === targetPid) {
    window.closeWindow();
}
```

Ergebnis:

- Schließen angefordert: `1795728595.437`
- Scope inaktiv: `1795728597.478`
- Verzögerung: **ca. 2,0 s**
- Spielprozess beendet, Steam-Client lief weiter.

### Notfall-/Scope-Stop

`systemctl --user stop app-steam-app1898290-<pid>.scope` beendete das Spiel
gezielt. Die Scope war nach ca. 2 s inaktiv; der Steam-Client blieb aktiv. Es
wurden keine fremden Prozesse beendet.

## Phase-II-Test 2: Proton-Spiel (Disco Elysium)

### Start und Scope

Start über eine temporäre Desktop-Datei mit
`Exec=steam steam://rungameid/632470`.

Beobachtung:

- Scope nach ca. 68 s:
  ```text
  app-steam-app632470-1489573.scope
  ```
- Fenster nach ca. 79 s (Proton-Prefix-/Erststart).
- Fenster-PID: `1489795`.
- Fensterprozess: `Z:\home\chris\steam\SteamLibrary\steamapps\common\Disco Elysium\disco.exe`
- Fenster-App-ID: `steam_app_632470`.

Prozesskette (gekürzt):

```text
reaper SteamLaunch AppId=632470
  └─ SteamLinuxRuntime_sniper/_v2-entry-point
      └─ Proton 8.0/proton waitforexitandrun
          └─ python3 .../proton
              ├─ wine64 c:\windows\system32\steam.exe ...
              ├─ wineserver
              ├─ winedevice.exe
              └─ disco.exe   (PID 1489795)
```

Die Spiel-Scope enthielt also die Sniper-Runtime, Proton, Wine und das
Windows-Spiel. Der Steam-Client blieb außerhalb.

### Scope-Ende und Absturz

- Reguläres Schließen des Fensters über die exakte PID beendete das Spiel; die
  Scope verschwand innerhalb weniger Sekunden; der Steam-Client blieb aktiv.
- **Absturztest:** `SIGKILL` auf die exakte Spiel-PID `1491693`.
  - Scope inaktiv nach **ca. 1,0 s**.
  - Keine verbleibenden `disco.exe`-, Proton- oder Wine-Prozesse.
  - Steam-Client blieb aktiv.

Damit ist die Scope für diesen Proton-Fall ein brauchbarer Lebensdauer-Anker,
aber erst nach Prüfung, dass der Fenster-/Spielprozess tatsächlich in dieser
Scope liegt.

## Phase-II-Test 2b: Weiteres Proton-Spiel (Stray, Proton 9.0)

Zusätzlich wurde **Stray** (App-ID `1332010`) unter **Proton 9.0 (Beta)**
getestet, um eine zweite Proton-Version und ein zweites Spiel zu prüfen.

Beobachtung:

- Start über eine temporäre Desktop-Datei mit
  `Exec=steam steam://rungameid/1332010`.
- Scope nach ca. 16 s:
  ```text
  app-steam-app1332010-1513108.scope
  ```
- Fenster nach ca. 26 s:
  `[Stray]`, App-ID `steam_app_1332010`, Vollbild
  (`1080,667 1920x1080`).
- Fenster-PID: `1513333` = `Stray-Win64-Shipping.exe`.
- Zusätzlich lief `Stray.exe` (PID `1513330`) als übergeordneter
  Spielprozess. **Beide** lagen in derselben Spiel-Scope.
- Prozesskette: reaper → Sniper-Runtime → Proton 9.0 → `python3` →
  `steam.exe`/Wine → `Stray.exe` → `Stray-Win64-Shipping.exe`.

Wichtig: Die Fenster-PID war hier **nicht** der erste `.exe`-Prozess, sondern
ein Kindprozess. Die Session-Identität muss die Fenster-PID in der Scope
verifizieren, nicht blind die erste `.exe` verwenden.

### Schließen

- Zwei KWin-`closeWindow()`-Aufrufe auf die Fenster-PID `1513333` wurden vom
  Spiel **ignoriert**: Fenster, Prozess und Scope blieben aktiv.
- Ein gezieltes `SIGTERM` auf die exakte Spiel-PID `1513333` beendete das
  Spiel: Scope inaktiv innerhalb von ca. 5 s, keine Restprozesse, Steam-Client
  blieb aktiv.

Damit ist das Fensterschließen **spielabhängig**. `can_graceful_stop` muss pro
Spiel/Session bestätigt werden. Ein gezieltes `SIGTERM` auf die exakte,
verifizierte Spiel-PID ist ein sinnvoller zweiter Graceful-Versuch vor dem
Scope-Stop – aber kein Ersatz für die Capability-Prüfung.

## Phase-II-Test 3: Zwei Spiele gleichzeitig

LEGO Bricktales (nativ) und Disco Elysium (Proton) liefen gleichzeitig:

```text
app-steam-app1898290-1490313.scope   (LEGO)
app-steam-app632470-1489573.scope    (Disco)
```

- Beide Spiele hatten getrennte Scopes.
- Disco wurde über sein Fenster geschlossen.
- Die Disco-Scope verschwand.
- LEGO lief weiter, Fenster und Scope blieben aktiv.
- Danach wurde LEGO über seine Scope als Notfalltest gestoppt.

Das ist ein direkter Nachweis, dass Steam pro Spiel eine eigene Scope anlegt und
dass ein gezielter Stop einer Spiel-Scope den Steam-Client und andere Spiele
nicht beendet.

## Phase-II-Test 4: Steam bereits geöffnet vs. vollständig beendet

### Steam bereits geöffnet

Die Tests 1–3 liefen bei bereits laufendem Steam-Client. Die Spiel-Scope wurde
in beiden Fällen neu angelegt.

### Steam vollständig beendet

`steam -shutdown` beendete den Client. Danach wurde LEGO erneut über
`gio launch` gestartet.

Beobachtung:

- Steam startete aus dem kalten Zustand.
- Spiel-Scope nach ca. 10 s.
- Spielfenster nach ca. 13 s.
- Fenster im Vollbild (`fullScreen=true`, Geometrie `1080,667 1920x1080`).
- Es gab nur die Spiel-Scope; der frisch gestartete Steam-Client lag in der
  cgroup des aufrufenden Prozesses (im Test: Terminal-Scope), nicht in einer
  eigenen Steam-App-Scope.
- Nach dem Schließen des Spiels verschwand die Spiel-Scope; Steam blieb laufen.

Konsequenz für CapyDeck: Beim Kaltstart erbt der Steam-Client die cgroup des
Prozesses, der `gio launch` ausführt. Die Spiel-Scope bleibt trotzdem eindeutig.
CapyDeck darf niemals seine eigene cgroup oder die seines Launcher-Prozesses
beenden, um ein Spiel zu stoppen.

## Phase-II-Test 5: Fokus, Minimieren, Vollbild

Mit LEGO im Vollbild:

- Minimieren: Prozess blieb am Leben, Scope blieb aktiv.
- Wiederherstellen: Fenster wurde wieder aktiv, `fullScreen=true` blieb.
- KWin-seitiges Umschalten `fullScreen=false` und wieder `true`: Prozess und
  Scope blieben stabil.
- Schließen über die exakte PID beendete die Scope; Steam blieb aktiv.

Das ist ein Compositor-/Fensterzustandstest, kein Test der Spiel-eigenen
Vollbildoptionen.

## Steam Overlay, Steam Input, Steam Cloud

- **Overlay:** Die Startkette lud `gameoverlayrenderer.so` (32/64 Bit) in die
  Spiel-Scope. Das belegt, dass das Overlay injiziert war. Eine interaktive
  Overlay-Bedienung wurde nicht durchgeführt.
- **Steam Input:** Es war kein Controller angeschlossen (`/dev/input/js*`
  fehlte; Ryujinx meldete ebenfalls keine passenden Controller). Steam Input
  wurde daher nicht getestet.
- **Steam Cloud:** Es wurde kein vollständiger Spielverlauf mit Speichern,
  Cloud-Sync und anschließendem Beenden durchgeführt. Cloud-spezifische
  Restprozesse sind offen.

## Nachgewiesene Erkenntnisse

1. Steam legt für native Linux- und Proton-Spiele eine **spielspezifische
   Scope** `app-steam-app<appid>-<pid>.scope` an.
2. Die Spiel-Scope enthält den Spielprozess (nativ) bzw. Proton/Wine und den
   Windows-Spielprozess. Der Steam-Client bleibt außerhalb.
3. Die Fenster-PID entspricht dem Spielprozess; bei Proton dem `disco.exe`-
   Prozess.
4. Scope-Ende korreliert mit dem Spielende:
   - reguläres Fensterschließen: ca. 2 s (LEGO/Disco),
   - Absturz (`SIGKILL` auf die Spiel-PID): ca. 1 s,
   - Stray (Proton 9.0): Close wurde ignoriert; `SIGTERM` auf die exakte
     Spiel-PID beendete die Scope in ca. 5 s.
5. Zwei Spiele gleichzeitig erhalten getrennte Scopes; ein Stop betrifft nur
   die eigene Spiel-Scope.
6. Ein Stop der eigenen Spiel-Scope beendet nicht den Steam-Client.
7. Der Kaltstart funktioniert über `gio launch`; die Spiel-Scope erscheint auch,
   wenn Steam vorher vollständig beendet war.
8. `MainPID` ist bei einer Scope leer; die Scope-Identität muss über
   `ControlGroup`/`cgroup.procs` und die Fenster-PID verifiziert werden.
9. Der Steam-Client erbt beim Kaltstart die cgroup des Launchers; nur die
   Spiel-Scope ist ein sicherer Stop-Anker.
10. Die Fenster-PID kann bei Proton ein Kindprozess sein
    (`Stray-Win64-Shipping.exe`), nicht der erste `.exe`-Prozess.
11. Fensterschließen ist spielabhängig; ein zweiter gezielter `SIGTERM` auf
    die exakte Spiel-PID ist ein sinnvoller Graceful-Fallback.

## Technische Annahmen

- Das Scope-Namensschema ist ein Implementierungsdetail von Steam und kann sich
  ändern. CapyDeck darf sich nicht ausschließlich darauf verlassen, sondern
  muss die Scope zusätzlich über cgroup/Prozess und Fenster verifizieren.
- Die gemessenen Startzeiten sind von Steam-Client, Proton-Prefix, Shader-Cache
  und Spiel abhängig. Sie sind keine festen Zusagen.
- Die Scope-Ende-Zeit ist ein brauchbarer, aber kein exakter Zeitpunkt für das
  „Spielende“. Für die Zeitmessung sollte die Prozess-/Scope-Lebensdauer mit
  einer kurzen Toleranz und einem definierten Endzustand verwendet werden.
- Der Kaltstart-Client in der Launcher-cgroup ist ein Risiko für unsaubere
  Notfall-Stops, wenn CapyDeck seine eigene cgroup beenden würde. Die
  Architektur verbietet das.

## Offene Forschungsfragen

- Weitere Proton-Versionen (10.0, 11.0, Experimental) und weitere Spiele.
- Spiele, die `closeWindow()` ignorieren: SIGTERM-Verträglichkeit,
  Speichern vor dem Beenden, Datenverlust-Risiko.
- Spiele mit eigenem Launcher, Anti-Cheat, EA App, Ubisoft Connect usw.
- Steam-Overlay-Interaktion, Steam Input mit realem Controller.
- Steam Cloud und verbleibende Runtime-Prozesse nach einem vollständigen
  Spielverlauf.
- Spielabsturz durch Spiel-/Proton-Fehler (nicht durch externes `SIGKILL`).
- Mehr als zwei gleichzeitige Spiele, mehrere Steam-Benutzerkonten.
- Verhalten auf Bazzite KDE/Wayland (Abnahme steht aus).
- GNOME/XFCE: Fensterschließen und Fokus.
- Steam Flatpak: bewusst zurückgestellt (Bazzite nutzt natives Steam).

## Capability-Bewertung

| Fähigkeit | Status | Begründung |
|---|---|---|
| Start | ✅ experimentell | `gio launch` + Steam-Kaltstart/warm |
| Start erkennen | ✅ experimentell | Scope + Fenster-PID, cgroup-verifiziert |
| Session zuordnen | ✅ experimentell | App-ID + Scope + cgroup + Fenster-PID |
| Ende erkennen | ✅ experimentell | Scope-Ende nach Close/Crash |
| Fokus | ⚠️ nur KDE/KWin getestet | Compositor-abhängig |
| Regulär beenden | ⚠️ pro Spiel | LEGO/Disco: Close ok; Stray: Close ignoriert, SIGTERM-Fallback |
| Notfall-Stop | ✅ experimentell | Stop der eigenen Spiel-Scope |
| Overlay/Input/Cloud | ❓ offen | nicht interaktiv getestet |

**Qualitätsstufe: Experimentell.** Einzelspikes auf einer Maschine; keine
Bazzite-Abnahme, kein Controller, kein vollständiger Spielverlauf.
