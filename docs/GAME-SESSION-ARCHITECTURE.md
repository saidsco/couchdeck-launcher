# GameSessionManager – Architektur, Nachweise und Roadmap

Stand: Phase III, 27. November 2026
Status: **Research/Architektur, keine Implementierung.**
Zielplattform: Bazzite KDE/Wayland; reale Tests bisher auf
Manjaro/KDE/Wayland.

Begleitdokumente:

- [`SESSION-PHASE-II-SUMMARY.md`](SESSION-PHASE-II-SUMMARY.md) – Phase-II-Überblick
- [`SESSION-SUPERVISOR-SPIKE-RESULTS.md`](SESSION-SUPERVISOR-SPIKE-RESULTS.md) – Phase-III-Messungen
- [`SESSION-SUPERVISOR-DECISION.md`](SESSION-SUPERVISOR-DECISION.md) – Supervisor-Entscheidung (Variante C)
- [`SESSION-IDENTITY-AND-LIFECYCLE.md`](SESSION-IDENTITY-AND-LIFECYCLE.md) – Identität, Zustände, Recovery
- [`TIME-ACCOUNTING.md`](TIME-ACCOUNTING.md) – sessionbasierte Zeitkonten
- [`TIME-LIMIT-ENFORCEMENT.md`](TIME-LIMIT-ENFORCEMENT.md) – Warnungen, Stop, Notfall
- [`STEAM-SPIKE-RESULTS.md`](STEAM-SPIKE-RESULTS.md)
- [`PRISM-SPIKE-RESULTS.md`](PRISM-SPIKE-RESULTS.md)
- [`RYUJINX-SPIKE-RESULTS.md`](RYUJINX-SPIKE-RESULTS.md)
- [`GENERIC-GIO-SPIKE-RESULTS.md`](GENERIC-GIO-SPIKE-RESULTS.md)
- [`BAZZITE-SESSION-TEST-MATRIX.md`](BAZZITE-SESSION-TEST-MATRIX.md)
- [`RYUJINX_SPIKE.md`](RYUJINX_SPIKE.md) – ursprünglicher Arbeitsauftrag

## 1. Ziel

CapyDeck soll Spielsitzungen nicht nur als „Startbefehl wurde
angenommen“ kennen, sondern abhängig vom Backend:

- den tatsächlichen Start erkennen,
- eine laufende Sitzung einem Profil und einer Startmethode zuordnen,
- das Ende erkennen,
- das Spiel bei Bedarf wieder in den Vordergrund holen,
- es regulär beenden,
- im Notfall ausschließlich die eigene Sitzung gezielt terminieren,
- die tatsächliche Spielzeit für Kinder-Zeitkontingente messen.

Nicht jedes Backend kann alle Punkte. Fähigkeiten werden pro Backend **und pro
konkreter Sitzung** bestätigt und können zur Laufzeit zurückgenommen werden.
Tracking, Fokus, reguläres Beenden und Notfall-Terminierung bleiben getrennte
Fähigkeiten.

## 2. Qualitätsstufen

| Stufe | Bedeutung |
|---|---|
| **Unbekannt** | nicht untersucht |
| **Experimentell** | Einzelspike, eine Umgebung, offene Fälle |
| **Verifiziert** | mehrfach reproduziert, Zielumgebung bestätigt, Restfälle dokumentiert |
| **Produktiv unterstützt** | dauerhaft getestet, Fehlerpfade definiert, für Kinderprofile freigegeben |

Regeln:

- Ein erfolgreicher Einzelspike ist **experimentell**, nicht „unterstützt“.
- Manjaro-Ergebnisse ersetzen keine Bazzite-Abnahme.
- Eine Capability wird erst produktiv, wenn die Bazzite-Testmatrix für das
  Backend bestanden ist.

## 3. Nachgewiesene Backend-Fähigkeiten (Phase II)

Legende: ✅ nachgewiesen, ⚠️ eingeschränkt/offen, ❌ nicht möglich.
Stufe: U/E/V/P.

| Backend | Start | Start erkennen | Session zuordnen | Ende erkennen | Fokus | Regulär beenden | Notfall-Stop | Stufe |
|---|---|---|---|---|---|---|---|---|
| **Direct/Managed** (Ryujinx) | ✅ | ✅ Unit/MainPID/cgroup/Fenster | ✅ Unit + PID-Startzeit | ✅ Unit inactive (Nachlauf) | ⚠️ KDE/KWin | ✅ nur mit managed Config | ✅ Unit-Stop + SIGKILL-Fallback | **E** |
| **Steam nativ** | ✅ | ✅ Scope + Fenster-PID | ✅ App-ID + cgroup + PID | ✅ Scope-Ende (1–2 s) | ⚠️ KDE/KWin | ⚠️ pro Spiel (Close kann ignoriert werden) | ✅ eigene Spiel-Scope | **E** |
| **Prism/Minecraft** | ✅ | ✅ Java-PID + Instanzpfad | ✅ Instanz-ID + Java-PID | ✅ Java-Ende (2 s) | ⚠️ KDE/KWin | ✅ Fenster exakte Java-PID | ⚠️ nur gesamte Flatpak-Instanz | **E** |
| **Steam Flatpak** | ✅ (POC) | ⚠️ Fenster/Spiel-PID | ⚠️ gemeinsame Flatpak-Scope | ⚠️ Fenster weg | ⚠️ | ✅ exakte PID (POC) | ❌ nur gesamte Instanz | **E/zurückgestellt** |
| **Generisches GIO** | ✅ | ⚠️ nur Fenster/App-ID | ❌ kein PID/Scope | ❌/⚠️ | ❌ | ⚠️ nur eindeutiges Fenster | ❌ | **E/Kompatibilität** |

### 3.1 Direct/Managed

Nachgewiesen mit Ryujinx 1.3.3 und Mario Kart 8 Deluxe:

- Start über transiente Service-Unit (`Type=simple`).
- `MainPID` = Ryujinx-PID = KWin-Fenster-PID.
- `ControlGroup` ist der stabile Anker.
- Vollbild über verwaltete Konfiguration
  (`start_fullscreen=true`, `remember_window_state=false`); `--fullscreen`
  allein genügte im Test nicht.
- Reguläres Schließen nur mit `show_confirm_exit=false`; sonst öffnet Ryujinx
  einen Bestätigungsdialog, der ohne Eingabeinjektion nicht bestätigt werden
  kann.
- Notfall-Stop über `systemctl --user stop` oder – bei Blockade –
  `systemctl --user kill --signal=KILL` auf die eigene Unit.
- Kindprozesse werden über `KillMode=control-group` mit beendet.
- Fensterneuerstellung beendet die Session nicht.
- Scope-Variante: kein `MainPID`; Stop nur mit `.scope`-Suffix.

Bewertung: **stärkstes Backend** für Ownership, Tracking und Notfall-Stop.
Voraussetzung ist eine verwaltete Konfiguration und eine getestete
Fensterschließstrategie (KDE/KWin bisher).

### 3.2 Steam nativ

Nachgewiesen mit einem nativen Linux-Spiel (LEGO Bricktales) und einem
Proton-Spiel (Disco Elysium, Proton 8.0):

- Steam legt pro Spiel eine Scope
  `app-steam-app<appid>-<pid>.scope` an – auch unter Proton.
- Der Spielprozess liegt in dieser Scope; der Steam-Client nicht.
- Fenster-PID = Spiel-PID (nativ) bzw. `disco.exe`-PID (Proton).
- Reguläres Schließen über die exakte Fenster-PID: Scope endet in ca. 1–2 s.
- Absturz (`SIGKILL` auf die Spiel-PID): Scope endet in ca. 1 s.
- Zwei Spiele gleichzeitig: getrennte Scopes, unabhängiger Stop.
- Kaltstart über `gio launch`: Spiel-Scope erscheint auch bei vorher beendetem
  Steam (ca. 10–13 s).
- `MainPID` der Scope ist leer; `ControlGroup`/`cgroup.procs` verwenden.
- Der Steam-Client erbt beim Kaltstart die cgroup des Launchers; nur die
  Spiel-Scope ist ein sicherer Stop-Anker.
- Die Fenster-PID kann bei Proton ein Kindprozess sein
  (`Stray-Win64-Shipping.exe`), nicht der erste `.exe`-Prozess.
- Fensterschließen ist spielabhängig: Bei Stray (Proton 9.0) wurden zwei
  `closeWindow()`-Aufrufe ignoriert; ein gezieltes `SIGTERM` auf die exakte
  Spiel-PID beendete die Scope in ca. 5 s. `can_graceful_stop` muss daher pro
  Spiel/Session bestätigt werden.

Wichtige Klärung: Der Phase-I-Test mit LEGO Bricktales war **nativ**, nicht
Proton. `LEGO_Bricktales.x86_64` ist der Linux-Build; es gab kein
Proton-Prefix. Proton wurde separat getestet.

### 3.3 Prism/Minecraft

Nachgewiesen mit Prism 11.1.1 (Flatpak), Java 25.0.1, Minecraft 26.3:

- Start über `flatpak run org.prismlauncher.PrismLauncher --launch <id>`.
- Bei geöffneter UI wird der Auftrag an die bestehende Flatpak-Instanz
  weitergereicht; keine dauerhafte zweite Instanz.
- Java-PID ist Kind von `prismrun`; Fenster-PID = Java-PID.
- Schließen des Minecraft-Fensters beendet nur Java; Prism-UI und
  Flatpak-Instanz bleiben.
- Zwei unterschiedliche Instanzen laufen parallel in **derselben**
  Flatpak-Scope; die Zuordnung erfolgt über
  `-Djava.library.path=.../instances/<name>/natives`.
- Fenstertitel sind zur Unterscheidung unzuverlässig.
- Ein zweiter `--launch` derselben Instanz startet kein zweites Spiel
  (Exit-Code 0 trotzdem).
- `flatpak kill <instance>` beendet die gesamte Instanz – nur als bestätigter
  Notfallweg, wenn keine andere Session betroffen ist.

### 3.4 Generisches GIO

- `gio launch` bleibt für normale Desktop-Dateien unverändert.
- Die GIO-API `g_desktop_app_info_launch_uris_as_manager()` liefert über den
  PID-Callback nur den unmittelbar gespawnten Prozess:
  - direkt: korrekt,
  - Shell-Wrapper: Wrapper-PID,
  - Early-Exit: PID ggf. bereits tot,
  - D-Bus: kein Callback,
  - Single-Instance: ggf. kein neuer Prozess.
- Der Callback ist kein Identitäts-/Lebensdauerbeweis.
- Fenster-/App-ID-Beobachtung bleibt heuristisch.
- Kein Notfall-Stop, keine harte Zeitdurchsetzung.

### 3.5 Steam Flatpak

Bewusst zurückgestellt. Bazzite nutzt natives Steam. Die vorhandenen
POC-Ergebnisse bleiben dokumentiert, sind aber kein Release-Blocker.

### 3.6 Session Supervisor (Phase III)

**Entscheidung: Variante C – systemd-Unit + minimaler Child-Subreaper.**
Details in [`SESSION-SUPERVISOR-DECISION.md`](SESSION-SUPERVISOR-DECISION.md).

| Ebene | Verantwortung |
|---|---|
| systemd-Unit | Prozessgruppe, `InvocationID`, `ControlGroup`, Unit-Lebensdauer |
| Supervisor | Spielstart, Subreaper, Kindprozesse, Exit-Codes/Signale, Ereignisse |
| GameSessionManager | Backend, Zustand, Capabilities, Persistenz, Recovery |
| Timekeeping | Profilzeit, unabhängig von UI-Fokus/Home |

Empfohlene Unit:

```text
capydeck-session-<id>.service
  Type=exec
  ExitType=cgroup
  KillMode=control-group
  WorkingDirectory=<explizit>
```

Nachgewiesen:

- `ExitType=cgroup` hält die Unit am Leben, wenn der Hauptprozess endet
  (`MainPID=0`, cgroup bleibt).
- Supervisor-Crash: Unit bleibt aktiv, Spiel läuft weiter, `systemctl stop`
  beendet es.
- Default `ExitType=main`: Unit endet und `KillMode=control-group` beendet das
  Spiel (Negativkontrolle).
- Subreaper adoptiert Early-Exit-, Double-Fork-, `setsid`- und
  cgroup-escaped Nachfahren.
- Von Wrappern gereapte Enkel erscheinen als `disappeared` (kein Exit-Status).
- Managed GIO ist für kontrollierbare Prozessbäume brauchbar; D-Bus und
  Single-Instance bleiben nicht managed.
- Supervisor: ca. 2,7 MB RSS, 0 % CPU im Leerlauf; Wesnoth-Start ca. +95 ms;
  `glxgears`-Prozess-CPU 1,4–1,8 % direkt vs. 2,0 % supervised (klein, nicht
  signifikant); FPS vsync-limitiert unverändert.
- Ryujinx-Managed-Root mit Symlinks teilt Saves/Firmware/Profile; keine zweite
  Emulatorinstallation.

Grenzen:

- Subreaper trackt nur Nachfahren; D-Bus/Single-Instance bleiben `Untracked`.
- Nach Supervisor-Crash ist ein cgroup-escaped Prozess nicht mehr über die
  alte Unit-cgroup auffindbar → `TrackingLost`.
- cgroup-Escape ist selten, aber ein bekannter Randfall.

## 4. Session-Modell und Identität

Kurzfassung; Details in
[`SESSION-IDENTITY-AND-LIFECYCLE.md`](SESSION-IDENTITY-AND-LIFECYCLE.md).

```rust
enum SessionState {
    Starting,
    RunningTracked,
    RunningUntracked,
    TrackingLost,
    Stopping,
    StopUnconfirmed,
    Ended,
    Failed,
    Unknown,
}

struct SessionIdentity {
    session_id: SessionId,
    backend: BackendKind,
    game_key: String,
    profile_id: String,
    started_at_unix: i64,
    started_at_boottime: u64,
    boot_id: String,
    systemd_unit: Option<SystemdUnit>,   // Name + InvocationID + kind
    supervisor: Option<SupervisorHandle>, // PID + Startzeit + Ereigniskanal
    pid: Option<ProcessIdentity>,   // pid + starttime_ticks
    cgroup: Option<String>,
    flatpak_instance: Option<FlatpakInstance>,
    steam_app_id: Option<u32>,
    prism_instance_id: Option<String>,
    window: Option<WindowIdentity>,
    tracking: TrackingConfidence,   // Confirmed | Probable | Heuristic | Lost
}
```

Regeln:

- Eine Session gehört dem Profil, das sie gestartet hat.
- Home/Minimieren/Fokuswechsel beenden keine Session und pausieren die
  Spielzeit nicht.
- `gio launch`/`--launch`-Erfolg ist kein „Spiel läuft“.
- Fensterexistenz ist kein Lebensdauerbeweis.
- `Stopping`/`CloseRequested` ist nicht `Ended`.
- `TrackingLost` ist nicht `Ended`; die Unit kann weiterleben.
- Ein Supervisor-Timeout ist kein bestätigtes Spielende.
- Ein verbleibender Hilfsprozess ist kein Beweis, dass das Spiel läuft.
- Supervisor-Ereignisse sind primär; bei Verlust Fallback über
  Unit/cgroup/PID-Startzeit/Fenster.
- Nach einem Neustart werden keine unbelegten „läuft noch“-Aussagen getroffen.
- Persistenz in `sessions.json` mit Schema-Version, atomar.

## 5. Start, Ende und Zustandsübergänge

### Start erkennen

| Backend | Beleg |
|---|---|
| Direct/Managed | Unit `active` + Supervisor `spawn` + MainPID/cgroup + PID-Startzeit (+ Fenster) |
| Steam | Spiel-Scope `active` + Spiel-PID in cgroup + Fenster-PID |
| Prism | Java-PID + Instanzpfad + Fenster-PID |
| Generic GIO | nur heuristisch → `RunningUntracked` |

### Ende erkennen

| Backend | Primär | Nicht verwenden |
|---|---|---|
| Direct/Managed | Supervisor `main_exit`/`exit` + Unit inactive/Prozess tot | Fenster allein |
| Steam | Scope inactive / Spiel-PID tot | Steam-Client-Ende |
| Prism | Java-PID tot | Flatpak-Scope-/UI-Ende |
| Generic GIO | keine | Callback allein |

Nachlauf:

- Steam: 1–2 s.
- Prism: ca. 2 s.
- Ryujinx: ca. 2 s mit managed Config; ca. 10 s bei Aufräumarbeiten.
- Fenster können nach einem Absturz kurzzeitig weiterleben.
- Der Supervisor-Settle (Default 300 ms) verzögert nur das Supervisor-Ende,
  nicht den Spielprozess.

CapyDeck wartet nach einem erkannten Ende eine kurze Settle-Frist, bevor es
`Ended` persistiert.

### TrackingLost und Supervisor-Ausfall

- Supervisor-Ereignisse sind primär.
- Stirbt der Supervisor, bleibt die Unit mit `ExitType=cgroup` aktiv; die
  Session wechselt in `TrackingLost`.
- Fallback: `systemctl show` (ActiveState, MainPID, ControlGroup), cgroup.procs,
  PID + Startzeit, Fenster.
- Ein Beobachtungs-Timeout ist kein bestätigtes Ende.
- Ein verbleibender Hilfsprozess ist kein Beweis für ein laufendes Spiel.
- Die Zeitmessung zählt konservativ weiter oder blockiert im strikten Modus;
  sie pausiert nicht stillschweigend.

## 6. Zeitkonten

Kurzfassung; Details in [`TIME-ACCOUNTING.md`](TIME-ACCOUNTING.md).

- Zeit läuft nur während einer nachgewiesenen Spielsitzung.
- Home/Minimieren/Fokuswechsel pausieren nicht.
- Zeit gehört dem startenden Profil.
- Parallele Sessions desselben Profils: Vereinigung der Intervalle, keine
  Doppelzählung.
- Tatsächliche Nutzung wird auch über das Budget hinaus erfasst; die Restzeit
  wird auf 0 begrenzt, nicht die Messung.
- Zusatzminuten werden mit der bereits verbrauchten Zeit verrechnet.
- Suspend/Schlafzeit zählt nicht.
- Neustart/Absturz/Recovery werden über `boot_id`, Monotonic/Boottime und
  Session-Handles behandelt.
- Supervisor-Ereignisse (`main_exit`, `exit`, `cgroup_empty`) sind primär;
  bei `TrackingLost` Fallback über Unit/cgroup/PID/Fenster.
- Unbekannte Backends: konservativer Kompatibilitätsmodus; optional strikter
  Modus.
- Zunächst Shadow Mode parallel zur bestehenden Messung.

Wichtiger Unterschied zum Ist-Zustand: Die aktuelle Messung zählt die Zeit des
aktiven Kinderprofils, solange CapyDeck läuft (auch Bibliotheksnavigation),
und kappt `used_seconds` am Budget. Das neue Modell misst echte
Session-Zeit und speichert `used_seconds_actual` ungekappt.

## 7. Zeitlimit-Durchsetzung

Kurzfassung; Details in [`TIME-LIMIT-ENFORCEMENT.md`](TIME-LIMIT-ENFORCEMENT.md).

```rust
enum ExpiryAction {
    WarnOnly,
    PreventNewStarts,
    GracefulStop,
    GracefulThenEmergency,
}
```

- Standard: Warnungen + keine neuen Starts.
- Graceful Stop nur mit bestätigter Capability.
- Notfall-Stop nur nach ausdrücklicher Konfiguration/Bestätigung und nur für
  die eigene, verifizierte Session.
- Close-Request ≠ Ende; `StopUnconfirmed` muss sichtbar sein.
- Kein `pkill`, keine Prozessnamen-Suche, keine fremden Units.
- Elternfreigabe/Zusatzminuten bleiben der bestehende PIN-Flow.

## 8. Komponenten und Threading

```text
UI / HomeAction
      │
      ▼
GameSessionManager (eigener Thread/Event-Loop)
  ├─ Session-Registry
  ├─ Backend-Registry
  ├─ Capability-Confirmation
  ├─ Zeit-/Zustandsereignisse
  ├─ Persistenz (sessions.json)
  └─ Control-Aufträge (Start/Close/Stop)
      │
      ├─ systemd-Unit (Type=exec, ExitType=cgroup)
      │     └─ Session-Supervisor (Subreaper)
      │           └─ Direct/Managed-Spiel
      │
      └─ Steam-Scope | Prism-Java | GioCompat
```

- Der Manager blockiert die UI nicht.
- Start/Stop laufen asynchron mit Timeouts.
- Zustandsänderungen werden als Ereignisse an die UI publiziert.
- Logind/Suspend werden beobachtet.
- Die Zeitmessung abonniert Session-Ereignisse; sie läuft nicht im UI-Tick.
- Die systemd-Unit ist der durable Anchor; der Supervisor liefert
  Anwendungsebene.
- Bei Supervisor-Verlust: `TrackingLost`, Fallback über Unit/cgroup/Fenster.

## 9. Backend-Design

### 9.1 Direct/Managed

- Strukturierte, validierte Argumente; kein Parsen beliebiger `Exec=`-Werte.
- Eigene transiente Service-Unit:
  - `Type=exec`
  - `ExitType=cgroup`
  - `KillMode=control-group`
  - `WorkingDirectory` explizit
- In der Unit läuft der Session-Supervisor (Subreaper); er startet das Spiel
  und liefert Ereignisse.
- Die Unit ist der durable Anchor; bei Supervisor-Crash bleibt sie aktiv
  (`MainPID=0`), die Session wird `TrackingLost`.
- Verwalteter Config-/Datenroot für den Emulator, z. B. Ryujinx:
  - eigenes `Config.json` mit `start_fullscreen=true`,
    `remember_window_state=false`, `show_confirm_exit=false`
  - `bis`, `system`, `games`, `profiles`, `mods`, `sdcard` symlinken
  - keine Änderung der Benutzerkonfiguration
  - keine zweite unabhängige Emulatorinstallation
- Regulär zuerst: gezieltes Fensterschließen über die exakte PID.
- SIGTERM/Unit-Stop ist die zweite Ebene; SIGKILL nur als bestätigter
  Notfallweg.
- Andere Desktops als KDE brauchen eine eigene getestete
  Fensterschließstrategie; sonst bleibt `can_graceful_stop = false`.

### 9.2 Steam

- Start bleibt GIO/Steam.
- Session-Anker: App-ID + Spiel-Scope + cgroup + Spiel-PID.
- Regulär: Fenster über exakte Spiel-PID schließen. Wird der Close ignoriert
  (z. B. Stray), ist ein gezieltes `SIGTERM` auf dieselbe verifizierte PID der
  zweite Versuch.
- Notfall: eigene Spiel-Scope stoppen, nicht den Steam-Client.
- `MainPID` der Scope nicht verwenden.
- Kaltstart: Steam-Client erbt die Launcher-cgroup; niemals die eigene cgroup
  stoppen.
- Keine Änderungen an Steam-Startoptionen ohne ausdrückliche Freigabe.
- Kein zusätzlicher Supervisor in der Steam Runtime.

### 9.3 Prism

- Start über Flatpak/GIO bzw. den verwalteten Instanz-Pfad.
- Session-Anker: Prism-Instanz-ID + Java-PID + Flatpak-Instanz +
  Instanzpfad im Java-Kommando.
- Regulär: Minecraft-Fenster über exakte Java-PID schließen.
- Notfall: `flatpak kill <instance>` nur, wenn keine andere Session in
  derselben Flatpak-Instanz läuft.
- Kein äußerer systemd-run-Scope als alleinige Kontrollinstanz.
- Kein zusätzlicher Supervisor.

### 9.4 Generic GIO

- `gio launch` unverändert und vollständig kompatibel.
- Optionaler **Managed-GIO-Modus** mit Supervisor + Unit für Desktop-Dateien,
  die einen kontrollierbaren Prozessbaum spawnen.
- D-Bus-Aktivierung und Single-Instance-Anwendungen bleiben
  `RunningUntracked`; keine Ownership vortäuschen.
- Kein Prozessownership im normalen GIO-Pfad, keine harte Durchsetzung.
- Kompatibilitätsmodus; optional strikter Modus für Kinderprofile.

## 10. Persistenz und Recovery

- `timekeeping.json`: Tagesaggregate, Schema 2 (siehe TIME-ACCOUNTING).
- `sessions.json`: Live-Sessions, Schema 1 (siehe SESSION-IDENTITY).
- Zusätzlich: `systemd_unit` (Name, `InvocationID`, `ControlGroup`) und
  `supervisor` (PID, Startzeit, Ereigniskanal).
- Atomare Schreibvorgänge; Backup bei Korruption/Migration.
- Beim Start: `boot_id`, Unit und Backend-Handles prüfen.
- Supervisor weg, Unit aktiv → `TrackingLost`, nicht `Ended`.
- Unit weg/Spiel tot → `Ended`.
- Nicht verifizierte Sessions → `Unknown`/`Ended`, nie blind „läuft“.
- Alte CapyDeck-Daten werden beim CapyDeck-Rename nicht migriert (keine
  echten Benutzer); die neuen Schemata starten sauber.

## 11. Nachgewiesen / Technische Annahmen / Offen

### Nachgewiesen (Phase II, Manjaro/KDE/Wayland)

- Steam native/Proton-Scope, Fenster-PID, Close, Crash, Notfall-Stop,
  zwei Spiele parallel, Kaltstart.
- Prism-UI offen/zu, eine/zwei Instanzen, Java-PID-Zuordnung, Close, Crash.
- Ryujinx echte Spiel-Session, Service-Unit, Vollbild, Bestätigungsdialog,
  managed Config, Notfall-Stop, Minimieren/Wiederherstellen.
- Native Test-App: Kindprozess, Crash-Cleanup, Fenster-Neuerstellung.
- Generic GIO: PID-Callback-Grenzen, Wrapper, D-Bus, Early-Exit, Fehlerfälle.
- Ryujinx/Steam: Home-/Minimieren beendet die Session nicht.

### Nachgewiesen (Phase III, Manjaro/KDE/Wayland)

- `systemd Type=exec` + `ExitType=cgroup` hält die Unit am Leben, wenn der
  Hauptprozess endet (`MainPID=0`); `systemctl stop` beendet die cgroup.
- Default `ExitType=main` beendet die Unit und killt Kinder.
- Subreaper adoptiert Early-Exit-, Double-Fork-, `setsid`- und
  cgroup-escaped Nachfahren.
- Supervisor-Crash beendet das Spiel nicht; mit `ExitType=cgroup` bleibt die
  Unit aktiv und das Spiel läuft weiter.
- UI-Crash beeinflusst die Unit nicht.
- Managed GIO funktioniert für kontrollierbare Prozessbäume; D-Bus und
  Single-Instance bleiben nicht managed.
- Ryujinx-Managed-Root mit Symlinks teilt Saves/Firmware/Profile; Controller
  wird als ProController erkannt.
- Ein weiteres natives Spiel (Pillars of Eternity) startete direkt unter
  Unit + Supervisor; Fenster-PID = Kind-PID, Unit-Stop ohne Reste.
- Supervisor: ca. 2,7 MB RSS, 0 % CPU im Leerlauf; Wesnoth-Start ca. +95 ms;
  `glxgears`-Prozess-CPU 1,4–1,8 % direkt vs. 2,0 % supervised (klein, nicht
  signifikant); FPS vsync-limitiert unverändert.

### Technische Annahmen

- systemd-User-Units/cgroups und KWin stehen auf Bazzite/KDE zur Verfügung.
- systemd 262 auf Bazzite verhält sich wie auf Manjaro.
- Flatpak-Instanz-IDs und `flatpak ps` bleiben stabil genug.
- Steam behält das Scope-Schema; CapyDeck verifiziert es trotzdem.
- Prism behält den Instanzpfad im Java-Kommando.
- Ryujinx behält Config-Schlüssel und Root-Struktur.
- GIOs Launchmanager-Verhalten bleibt für die genannten Desktop-Typen stabil.
- Monotonic/Boottime und logind verhalten sich auf Bazzite wie erwartet.

### Offen

- Bazzite-Abnahme für alle Backends und Supervisor-Varianten.
- Vollständige Spielverläufe inkl. Controller, Home, Speichern/Laden.
- Steam Cloud/Overlay-Interaktion.
- Weitere Proton-Versionen, Anti-Cheat/Launcher-Spiele.
- Suspend/Resume, CapyDeck-Absturz, PID-Reuse.
- Supervisor-Crash, während das Spiel in einer anderen cgroup läuft.
- Langzeit-CPU/RSS über Stunden.
- FPS/Frametimes in echten Spielen (bisher synthetisch/vsync-limitiert).
- GNOME/XFCE, Multi-Monitor-Vollbild.
- SteamOS Gaming Mode (nicht zugesichert).

## 12. Roadmap

### 0.6.0 – CapyDeck Clean Rebrand

- Produkt-/Repository-/Paketname, XDG-Pfade, App-ID, Desktop-Dateien.
- Keine Datenmigration (keine echten Benutzer).
- Keine Session-Features.

### 0.6.1 – Session Foundation

- `GameSession`-/`SessionIdentity`-Modell und `sessions.json`.
- `GameSessionManager` mit Event-Loop, Backend-Registry,
  Capability-Bestätigung.
- Transiente systemd-Unit (`Type=exec`, `ExitType=cgroup`,
  `KillMode=control-group`) und Supervisor-Prototyp für Direct/Managed.
- `TrackingLost`-Zustand und Recovery über Unit/cgroup/Fenster.
- Managed Ryujinx-Root mit symlinkten Zustandsverzeichnissen.
- Home-Menü: „Nur zurück“, „Spiel fortsetzen“ (Best-Effort), „Sitzung
  beenden“ nur bei Capability.
- Shadow-Zeitmessung aus Supervisor-Ereignissen + Fallback; keine Ablösung
  der bestehenden Messung.
- Keine automatische Terminierung.

### 0.6.2 – Zeitkonten-Härtung

- Session-Zeitmessung für Direct/Managed aktivieren.
- Supervisor-Crash-/UI-Crash-Recovery und `TrackingLost`-Policy.
- Kompatibilitätsmodus für nicht owning-fähige Backends.
- Neustart-, Absturz-, Suspend-Behandlung.
- Warnungen, Grace Period, Startsperre.
- Datenmodell Schema 2 (`used_seconds_actual`).
- Shadow-Vergleich dokumentieren.
- Optional: nur eine Session pro Kinderprofil.

### 0.6.3 – Kontrollierte Beendigung

- Reguläres Schließen für verifizierte Steam-/Prism-Fälle.
- SIGTERM/Unit-Stop als zweite Ebene; SIGKILL/Notfall optional und bestätigt.
- `GracefulThenEmergency`-Policy.
- UI-Transparenz bei `StopUnconfirmed`.
- Ryujinx-Realtest mit Controller und vollständigem Spielverlauf.

### 0.7.0 – Weitere Backends

- Managed GIO opt-in für geeignete Desktop-Dateien.
- Steam Flatpak nach eigenem Spike.
- GNOME/XFCE-Fensterschließung und Fokus.
- Weitere Emulatoren/Launcher.
- Optional Heroic/Lutris, Kodi (nicht priorisiert).

## 13. Leitplanken

- GIO für normale Desktop-Dateien bleibt unverändert.
- Kein `pkill`, keine Prozessnamen-Suche, keine fremden Prozesse.
- Direct/Managed nur mit strukturierten, validierten Argumenten.
- Tracking, Fokus, reguläres Beenden und Notfall-Terminierung sind getrennt.
- Fensterschließen vor Scope-/Unit-Stop.
- Scope-/Unit-Stop nur für die eigene, verifizierte Session.
- Supervisor nur für die eigene Sitzung; kein globaler Subreaper für die UI.
- Supervisor-Crash darf das Spiel nicht beenden (`ExitType=cgroup`).
- Keine Signal-Eskalation ohne ausdrückliche Konfiguration.
- Keine Ownership-Vortäuschung bei D-Bus/Single-Instance.
- Keine vollständigen Umgebungsdumps oder sensiblen Argumente in Logs.
- Keine „läuft“-Behauptung nach einem erfolgreichen Startbefehl.
- Keine automatische Fokussierung während des Spiels.
- Close-Request ≠ Ende.
- Keine automatische harte Terminierung ohne ausdrückliche Freigabe.
- Kinderprofile, PIN und bestehende Zeitmessung bleiben bis zur nachgewiesenen
  Ablösung unangetastet.

## 14. Abnahmekriterien

Die Session-Architektur gilt erst als produktiv, wenn:

1. Die Bazzite-Testmatrix für das jeweilige Backend bestanden ist.
2. Start/Ende/Recovery mindestens einen vollständigen Spielverlauf korrekt
   abbilden.
3. Parallele Sessions und gezielte Stops andere Sessions nicht beeinflussen.
4. Close-Request, Timeout und Notfall-Stop sauber getrennt sind.
5. Die Zeitmessung Home/Minimieren/Suspend/Neustart korrekt behandelt.
6. Unbekannte Backends nicht stillschweigend die Zeit aussetzen.
7. Keine falsche Erfolgsmeldung bei nicht bestätigtem Ende.
8. Supervisor-Crash und UI-Crash das Spiel nicht beenden und die Unit
   wiederaufnehmbar bleibt.
9. `TrackingLost` nicht als `Ended` behandelt wird.
10. Managed GIO D-Bus/Single-Instance nicht als managed meldet.
11. Ryujinx-Managed-Root keine doppelten Spielstände erzeugt.
12. Kein messbarer FPS-Nachteil gegenüber dem direkten Start nachgewiesen wird;
    echte Frame-Time-Messungen bleiben offen.
13. `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`
    und `cargo test --locked` grün sind.
