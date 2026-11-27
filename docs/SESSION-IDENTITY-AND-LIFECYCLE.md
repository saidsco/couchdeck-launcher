# Session-Identität und Lifecycle

Stand: Phase II, 26. November 2026
Status: Architektur/Research, keine Implementierung.

Dieses Dokument beantwortet zwei Fragen:

1. **Woran erkennt CouchDeck zuverlässig, dass eine bestimmte Spielsitzung
   noch läuft?**
2. **Wann darf CouchDeck sagen, dass sie beendet ist?**

Es trennt ausdrücklich:

- **Fensterexistenz**
- **Prozesslebensdauer**
- **Spielsitzungsdauer**
- **Launcher-/Client-Lebensdauer**

## Grundsatz

Eine PID allein ist **keine** dauerhafte Prozessidentität. PIDs werden
wiederverwendet. Eine Fenster-PID allein ist kein Lebensdauerbeweis: Fenster
können nach einem Absturz kurzzeitig weiterleben oder von einem anderen Prozess
neu erstellt werden.

CouchDeck verwendet deshalb einen **Identitätstupel** und verifiziert ihn bei
jeder Zustandsänderung erneut.

## Identitätstupel

```rust
struct SessionIdentity {
    session_id: SessionId,          // von CouchDeck erzeugt (UUID)
    backend: BackendKind,
    game_key: String,               // stabile CouchDeck-Spiel-ID
    profile_id: String,             // startendes Profil
    started_at_unix: i64,
    started_at_boottime: u64,
    boot_id: String,                // /proc/sys/kernel/random/boot_id

    // Backend-Handle (nur die zutreffenden Felder sind gesetzt)
    systemd_unit: Option<SystemdUnit>,
    supervisor: Option<SupervisorHandle>,
    pid: Option<ProcessIdentity>,
    cgroup: Option<String>,
    flatpak_instance: Option<FlatpakInstance>,
    steam_app_id: Option<u32>,
    prism_instance_id: Option<String>,
    window: Option<WindowIdentity>,
    tracking: TrackingConfidence,   // Confirmed | Probable | Heuristic | Lost
}

struct ProcessIdentity {
    pid: u32,
    starttime_ticks: u64,           // /proc/<pid>/stat, Feld 22
}

struct SystemdUnit {
    name: String,                   // z. B. capydeck-session-<id>.service
    invocation_id: Option<String>,
    kind: UnitKind,                 // Service | Scope
}

struct SupervisorHandle {
    pid: u32,
    starttime_ticks: u64,
    event_channel: Option<String>,  // Socket/Datei/Pipe
}

struct FlatpakInstance {
    instance_id: String,            // flatpak ps
    app_id: String,                 // org.prismlauncher.PrismLauncher
    pid: Option<u32>,
}

struct WindowIdentity {
    pid: u32,
    app_id: Option<String>,
    caption: Option<String>,
}
```

### Warum diese Felder?

| Feld | Zweck |
|---|---|
| `boot_id` | Erkennt Neustarts; verhindert Verwechslung alter PIDs |
| `pid` + `starttime_ticks` | Eindeutige Prozessidentität trotz PID-Reuse |
| `cgroup` | Prüft, ob der Prozess noch in der erwarteten Unit liegt |
| `systemd_unit` | Stabile Unit-Identität, auch wenn MainPID wechselt |
| `supervisor` | Ereignisquelle; Verlust führt zu `TrackingLost` |
| `tracking` | Confidence der aktuellen Session-Erkennung |
| `flatpak_instance` | Trennt mehrere Flatpak-Instanzen derselben App |
| `steam_app_id` | Ordnet Scope und Fenster dem Spiel zu |
| `prism_instance_id` | Ordnet Java-Prozess der Prism-Instanz zu |
| `window` | Nur Beobachtung/Fokus, **kein** Lebensdauerbeweis |
| `session_id` | CouchDeck-eigene, unveränderliche Sitzungs-ID |

## Backend-Handles und Verifikation

### Direct/Managed (Ryujinx, native Spiele)

Handle:

- Service-Unit-Name (`capydeck-session-<id>.service`) oder Scope
  (`capydeck-session-<id>.scope`)
- `InvocationID`
- `MainPID` (nur bei Service)
- `ControlGroup`
- PID + Startzeit des Hauptprozesses
- Fenster-PID

Verifikation „läuft“:

1. `boot_id` stimmt.
2. Unit ist `active`.
3. `ControlGroup` existiert und enthält den erwarteten Prozess.
4. `/proc/<pid>/stat` Startzeit stimmt.
5. Optional: Fenster-PID stimmt (nicht erforderlich, wenn das Spiel kein
   Fenster hat).

Verifikation „beendet“:

- Unit ist `inactive`/`failed` **und** der Prozess ist tot.
- Oder Prozess tot und cgroup leer.
- **Nicht** ausreichend: Fenster verschwunden.
- Nach einem Close-Request: erst nach Unit-/Prozessende oder Timeout als
  beendet werten.

### Steam nativ (App-ID + Spiel-Scope)

Handle:

- `steam_app_id` aus der Desktop-Datei/URI
- Scope `app-steam-app<appid>-<pid>.scope`
- `ControlGroup`
- Spiel-PID (Fenster-PID bei nativ, `disco.exe`-PID bei Proton)
- PID + Startzeit

Verifikation „läuft“:

1. Scope existiert und ist `active`.
2. Die Spiel-PID liegt in der `ControlGroup` der Scope.
3. `/proc/<pid>/stat` Startzeit stimmt.
4. Fenster-PID entspricht der Spiel-PID (oder ist eine bekannte Kind-PID).

Verifikation „beendet“:

- Scope ist inaktiv **oder** Spiel-PID tot.
- Steam-Client lebt weiter; das ist **kein** Session-Ende.
- Ein anderes Spiel in einer anderen Scope darf nicht betroffen sein.

**Nicht ausreichend:** Scope-Name allein. Der Name enthält eine PID und ist ein
Steam-Implementierungsdetail. Die cgroup-/Prozessprüfung ist Pflicht.

### Steam Flatpak

Zurückgestellt. Die vorhandenen Ergebnisse bleiben dokumentiert. Eine
spielspezifische Scope existiert nicht; ein Notfall-Stop würde die gesamte
Flatpak-Steam-Instanz betreffen. Kein Release-Blocker für Bazzite.

### Prism/Minecraft (Flatpak)

Handle:

- `prism_instance_id` (UUID/Instanzordner)
- Flatpak-Instanz-ID
- Java-PID
- `-Djava.library.path=.../instances/<name>/natives` als Zuordnungsmerkmal
- Fenster-PID (= Java-PID)
- optional `prismrun`-PID

Verifikation „läuft“:

1. Flatpak-Instanz existiert (oder eine bekannte UI-Instanz).
2. Java-PID lebt und ist Kind von `prismrun`.
3. Java-Kommandozeile enthält den erwarteten Instanzpfad.
4. Startzeit stimmt.
5. Fenster-PID entspricht der Java-PID.

Verifikation „beendet“:

- Java-PID tot.
- Prism-UI darf weiterleben; das ist **kein** Spielende.
- Flatpak-Scope darf weiterleben, solange die UI läuft.

**Nicht ausreichend:** Fenstertitel (mehrere Instanzen können denselben Titel
haben), Flatpak-Scope-Ende (UI kann offen bleiben), `flatpak ps` allein.

### Generisches GIO (Kompatibilitätsbackend)

Handle:

- Desktop-Datei/App-ID
- optionale GIO-Callback-PID (nur bei tatsächlich gespawntem Prozess)
- beobachtete Fenster-PID/App-ID
- Zeitfenster

Verifikation „läuft“:

- Nur heuristisch. Der GIO-Callback ist kein Beweis.
- Fenster + Prozessbaum + Zeitfenster können auf eine laufende Anwendung
  hindeuten.
- Kein Prozessownership, kein zuverlässiges Ende.

Verifikation „beendet“:

- Nicht zuverlässig.
- Konservativer Kompatibilitätsmodus oder optionaler strikter Modus.

## Zustandsmaschine

```text
Starting
  ├─ Startbefehl abgelehnt/fehlgeschlagen → Failed
  ├─ Start erkannt, Ownership bestätigt → RunningTracked
  └─ Start erkannt, Ownership unklar     → RunningUntracked

RunningTracked
  ├─ Prozess/Unit/Scope tot              → Ended
  ├─ Close angefordert                   → Stopping
  ├─ Supervisor weg, Unit lebt           → TrackingLost
  └─ Identität nicht mehr verifizierbar  → Unknown

RunningUntracked
  ├─ Heuristik sagt beendet              → Ended (niedrige Confidence)
  ├─ Strikter Modus / Budget erreicht    → Stopping (nur wenn möglich)
  └─ Unklar                              → Unknown

TrackingLost
  ├─ Unit/cgroup/Fenster bestätigt Spiel → RunningTracked
  ├─ Unit/Prozess tot                    → Ended
  ├─ Strikter Modus / Budget erreicht    → Stopping (nur wenn möglich)
  └─ Frist abgelaufen                    → Ended (konservativ, geloggt)

Stopping
  ├─ Prozess/Unit/Scope tot              → Ended
  ├─ Timeout, Prozess lebt               → StopUnconfirmed
  └─ Notfall-Stop erfolgreich            → Ended

StopUnconfirmed
  ├─ Später tot                          → Ended
  └─ Eltern/Admin entscheidet            → EmergencyStop | KeepRunning

Unknown
  ├─ Neubewertung ergibt Running         → RunningTracked/RunningUntracked
  ├─ Neubewertung ergibt Ended           → Ended
  └─ Frist abgelaufen                    → Ended (konservativ, geloggt)
```

Regeln:

- `Stopping` ist **kein** `Ended`.
- `CloseRequested` ist **kein** `Ended`.
- `TrackingLost` ist **kein** `Ended`; die Unit kann weiterleben.
- Ein Beobachtungs-Timeout ist **kein** bestätigtes Spielende.
- Ein verbleibender Hilfsprozess ist **kein** Beweis für ein laufendes Spiel.
- `Unknown`/`TrackingLost` dürfen die Zeitmessung nicht stillschweigend
  pausieren. Es braucht eine dokumentierte Policy (weiterzählen mit Warnung,
  Grace-Frist, oder strikter Modus).
- `RunningUntracked` hat eine niedrigere Confidence und keine
  Notfall-Stop-Capability.

## Fähigkeiten zur Laufzeit bestätigen

Capabilities werden nicht nur statisch pro Backend deklariert, sondern pro
Session bestätigt:

```rust
struct SessionCapabilities {
    can_detect_start: bool,
    can_attach: bool,
    can_detect_end: bool,
    can_focus: bool,
    can_graceful_stop: bool,
    can_emergency_stop: bool,
    confidence: Confidence, // Confirmed | Probable | Heuristic | Unknown
}
```

Beispiele:

- Direct/Managed: `can_graceful_stop` nur, wenn die verwaltete Konfiguration
  den Bestätigungsdialog deaktiviert hat (`show_confirm_exit=false`).
- Steam: `can_graceful_stop` nur, wenn die Spiel-PID in der Scope verifiziert
  wurde, ein Fenster existiert und das Close-Verhalten für dieses Spiel
  bestätigt ist (Close kann ignoriert werden; SIGTERM-Fallback).
- Prism: `can_graceful_stop` nur, wenn die Java-PID der Instanz eindeutig
  zugeordnet ist. `can_emergency_stop` nur, wenn keine andere Session in
  derselben Flatpak-Instanz läuft.
- Generic GIO: `can_emergency_stop = false`, `can_detect_end = false`.

Wird eine Fähigkeit später nicht mehr bestätigt (z. B. Fenster weg, Prozess
nicht mehr in der cgroup), wird sie **zurückgenommen** und die Session auf
`Unknown` gesetzt.

## Start erkennen

Ein Startbefehl ist nur ein Intent. Die Session bleibt `Starting`, bis
mindestens ein Beleg vorliegt:

- Direct/Managed: Unit `active` + `MainPID`/cgroup + PID-Startzeit.
- Steam: Spiel-Scope `active` + Spiel-PID in cgroup + Fenster-PID.
- Prism: Java-PID + Instanzpfad + Fenster-PID.
- Generic GIO: nur heuristisch; `RunningUntracked`.

Timeouts:

- `Starting` darf nicht unbegrenzt bleiben. Nach einem backendabhängigen
  Timeout (z. B. 120 s bei Proton/Prism, 60 s bei Direct) → `Failed` oder
  `Unknown`, mit Logeintrag.
- Ein verspäteter Start kann nachträglich adoptiert werden, wenn die Identität
  eindeutig ist.

## Ende erkennen

Primäre Anker:

| Backend | Primär | Sekundär | Nicht verwenden |
|---|---|---|---|
| Direct/Managed | Unit inactive + Prozess tot | cgroup leer | Fenster allein |
| Steam | Scope inactive / Spiel-PID tot | cgroup leer | Steam-Client-Ende |
| Prism | Java-PID tot | Java-Kindschaft weg | Flatpak-Scope-Ende, UI-Ende |
| Generic GIO | keine | Heuristik | Callback allein |

Nachlaufzeiten:

- Steam nativ: Scope-Ende ca. 1–2 s nach Close/Crash.
- Prism: Java-Ende ca. 2 s nach Fensterschließen.
- Ryujinx: Unit-Ende ca. 2 s mit `show_confirm_exit=false`; ca. 10 s bei
  Fenstermodus/Aufräumen.
- Fenster können nach einem Absturz kurzzeitig (Sekunden) weiterleben.

CouchDeck sollte nach einem erkannten Ende eine kurze **Settle-Frist** (z. B.
2–5 s) abwarten, bevor es die Session endgültig als `Ended` persistiert, um
Flackern bei Fenster-/Prozessneuaufbau zu vermeiden.

## Neustart, Absturz, Suspend

### CouchDeck-Neustart

Beim Start:

1. `boot_id` vergleichen.
2. `sessions.json` laden.
3. Für jede Session das Backend-Handle erneut prüfen:
   - Direct/Managed: Unit/Scope + PID-Startzeit.
   - Steam: Scope + Spiel-PID.
   - Prism: Flatpak-Instanz + Java-PID + Instanzpfad.
   - Generic GIO: nur Heuristik → `Unknown`.
4. Verifizierte Sessions werden adoptiert; nicht verifizierte werden
   `Ended`/`Unknown`, **nie** blind „läuft noch“.

### Supervisor-Verlust

Wenn der Supervisor nicht mehr läuft, die Unit aber noch aktiv ist:

- `systemctl show`: `ActiveState=active`, `MainPID=0`, `ControlGroup` gesetzt.
- cgroup.procs prüfen; erwartete Spiel-PID + Startzeit verifizieren.
- Fenster als zusätzliches Beobachtungsobjekt prüfen (kein Lebensdauerbeweis).
- Session wird `TrackingLost`, nicht `Ended`.
- Stop über `systemctl --user stop <unit>`; Notfall über
  `systemctl --user kill --signal=KILL <unit>`.
- Ist der Prozess in eine andere cgroup escaped und der Supervisor tot, ist er
  über die alte Unit nicht mehr auffindbar → `TrackingLost`, konservative
  Policy, keine erfundene Endzeit.

### Systemneustart

Wenn `boot_id` abweicht:

- Alte Sessions können nicht weiterlaufen.
- Endzeitpunkt: letzter bekannter `last_accounted_at` oder ein konservativer
  Shutdown-Zeitpunkt.
- Keine Zeit für die Zeit zwischen letztem Accounted-Stand und Neustart
  erfinden; die Lücke wird geloggt.

### Suspend/Resume

- `CLOCK_MONOTONIC` pausiert im Suspend; `CLOCK_BOOTTIME` läuft weiter.
- Für Spielzeit ist die **Monotonic**-Differenz maßgeblich, damit Schlafzeit
  nicht zählt.
- Zusätzlich `PrepareForSleep` von logind beobachten, um Suspend-Intervalle
  explizit zu markieren.
- Nach Resume: Session bleibt aktiv, aber die Suspend-Dauer wird nicht
  angerechnet.
- Wenn die Zeitquelle unklar ist, konservativ entscheiden und loggen.

## Supervisor-Ereignisse

Der Supervisor liefert mindestens:

- `spawn` – Hauptprozess gestartet (PID + Startzeit + cgroup).
- `adopted` – verwaister Nachfahre übernommen.
- `discovered` – Nachfahre im Prozessbaum gefunden.
- `cgroup_changed` – Prozess hat die cgroup gewechselt.
- `exit` – Prozess beendet (Code/Signal, sofern reapbar).
- `disappeared` – Prozess verschwunden, ohne Exit-Status (z. B. von Wrapper
  gereapt).
- `main_exit` – Hauptprozess beendet.
- `cgroup_empty` – bekannte Gruppe leer (nach Settle).
- `supervisor_exit` – Supervisor beendet (Grund, verbleibende Prozesse).

Regeln:

- `main_exit` allein ist kein Spielende, wenn Hilfsprozesse weiterlaufen.
- `cgroup_empty` allein ist kein Beweis, dass das Spiel bis zuletzt lief; es
  kann ein reiner Helper-Prozess gewesen sein.
- `disappeared` ist ein Lebenszyklus-Hinweis, kein Exit-Code.
- `supervisor_exit` beendet keine Session; die Unit kann weiterleben.

## Persistenz

`sessions.json` (separat von `timekeeping.json`):

```json
{
  "schema_version": 1,
  "sessions": [
    {
      "id": "uuid",
      "game_id": "game-...",
      "profile_id": "mia",
      "backend": "steam_native",
      "state": "running_tracked",
      "started_at_unix": 1795728579,
      "started_at_boottime": 123456789,
      "last_accounted_at_unix": 1795728600,
      "last_accounted_at_boottime": 123456810,
      "boot_id": "92aa...",
      "backend_handle": { "...": "..." },
      "capabilities": { "...": "..." }
    }
  ]
}
```

Regeln:

- Atomar schreiben.
- Schema-Version prüfen; bei unbekannter höherer Version nicht überschreiben.
- Korrupte Datei → Backup, Fehlerdialog, keine stille Löschung.
- Sessions dürfen nicht als „läuft“ wiederhergestellt werden, wenn das
  Backend-Handle nicht verifiziert ist.

## Logging/Evidenz

Pro Session sollten mindestens geloggt werden:

- `session_id`, Spiel, Profil, Backend
- Start-/Endzeit (Unix + Boottime)
- `boot_id`
- Unit/Scope + `InvocationID`
- cgroup
- PID + Startzeit
- Fenster-PID/App-ID (falls vorhanden)
- Flatpak-Instanz/App-ID (falls vorhanden)
- Steam-App-ID (falls vorhanden)
- Prism-Instanz-ID (falls vorhanden)
- Capability-Entscheidungen und Downgrades
- Close-/Stop-Versuche, Timeouts, Notfall-Stop

## Nachgewiesene Erkenntnisse

- Steam-Scope und Spiel-PID sind für native und Proton-Spiele ein brauchbarer
  Anker; `MainPID` der Scope ist leer.
- Prism-Java-PID und Instanzpfad sind ein brauchbarer Anker; die Flatpak-Scope
  allein reicht nicht.
- Direct/Managed-Service-Unit mit `MainPID`/cgroup ist der stärkste Anker.
- Fenster können nach einem Absturz kurzzeitig weiterleben.
- Fensterneuerstellung beendet eine Session nicht.
- Ein Bestätigungsdialog kann ein Fensterschließen aufhalten.
- `systemctl stop` kann blockieren; Notfall-Stop braucht Timeout/Fallback.
- D-Bus-/Single-Instance-Apps liefern keinen GIO-PID-Callback.
- `ExitType=cgroup` hält die Unit nach Supervisor-Crash aktiv (`MainPID=0`).
- Ein Subreaper adoptiert Early-Exit-, Double-Fork-, `setsid`- und
  cgroup-escaped Nachfahren.
- Von Wrappern gereapte Enkel erscheinen als `disappeared` (kein Exit-Status).
- D-Bus/Single-Instance bleiben ohne Subreaper-Ownership `RunningUntracked`.
- Fensterneuerstellung beendet die Session nicht; Fenster sind keine Identität.
- `TrackingLost` ist ein eigener Zustand, nicht `Ended`.

## Technische Annahmen

- systemd-User-Units und cgroups bleiben auf Bazzite/KDE/Wayland verfügbar.
- Flatpak-Instanz-IDs und `flatpak ps` bleiben stabil genug für die
  Zuordnung.
- Steam behält das Scope-Schema in absehbarer Zeit bei; die Architektur
  verifiziert es trotzdem zur Laufzeit.
- Prism behält den Instanzpfad im Java-Kommando.
- KWin ist unter Bazzite/KDE der dominante Fenstermanager.

## Offene Forschungsfragen

- PID-Reuse-Test über einen echten Neustart.
- Suspend/Resume auf Bazzite (logind, Zeitquellen).
- CouchDeck-Absturz während einer Session und Wiederaufnahme.
- Mehrere Benutzer gleichzeitig (nicht Ziel, aber Datenmodell sollte nicht
  brechen).
- Steam-Client-Neustart während einer laufenden Spiel-Scope.
- Prism-UI wird während einer laufenden Java-Session beendet.
- Ryujinx-Crash ohne Unit-Ende (Hänger).
- Generic-GIO-Heuristik mit mehreren Fenstern/Launchern.
