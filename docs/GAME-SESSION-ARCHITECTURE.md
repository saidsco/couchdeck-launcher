# GameSessionManager – Architektur und Backend-Fähigkeiten

Stand: 0.6-Vorbereitung, 26. November 2026

Dieses Dokument beschreibt die geplante Session-Architektur. Es ist noch keine
Implementierung. `docs/RYUJINX_SPIKE.md` bleibt der ursprüngliche
Ryujinx-Arbeitsauftrag; die Ergebnisse des ersten technischen Spikes stehen
separat in [`RYUJINX-SPIKE-RESULTS.md`](RYUJINX-SPIKE-RESULTS.md).

## Ziel

CouchDeck soll Spielsitzungen nicht nur als „Startbefehl wurde angenommen“
kennen, sondern abhängig vom Backend:

- den tatsächlichen Start erkennen,
- eine laufende Sitzung einem Profil und einer Startmethode zuordnen,
- das Ende erkennen,
- das Spiel bei Bedarf wieder in den Vordergrund holen,
- es regulär beenden,
- im Notfall ausschließlich die eigene Sitzung gezielt terminieren.

Nicht jedes Backend kann alle Punkte. Die Fähigkeiten müssen explizit modelliert
werden; Tracking und Beendigung sind getrennte Fähigkeiten.

## Komponenten

```text
UI / HomeAction
      │
      ▼
GameSessionManager
  ├─ Session-Registry
  ├─ Backend-Registry
  ├─ Zeit-/Zustandsereignisse
  ├─ Persistenz
  └─ Control-Aufträge
      │
      ▼
Direct/Managed | Steam | Prism | GioCompat
```

Der Manager läuft getrennt vom UI-Thread oder als klar gekapselter Dienst. Er
blockiert die UI nicht und publiziert Zustands- und Zeittereignisse.

## Backend-Fähigkeiten

```rust
struct BackendCapabilities {
    can_launch: bool,
    can_detect_start: bool,
    can_attach_session: bool,
    can_detect_end: bool,
    can_focus: bool,
    can_graceful_stop: bool,
    can_emergency_stop: bool,
    confidence: Confidence,
}
```

Damit sind beispielsweise folgende Kombinationen möglich:

- Starten und Start erkennen, aber nicht beenden.
- Fenster schließen, aber das Spielende nicht zuverlässig erkennen.
- Nur starten; Zeitmessung im konservativen Kompatibilitätsmodus.

## Session-Modell

```rust
enum SessionState {
    Starting,
    RunningTracked,
    RunningUntracked,
    Stopping,
    Ended,
    Failed,
    Unknown,
}

struct GameSession {
    id: SessionId,
    game_id: String,
    profile_id: String,
    backend: BackendKind,
    started_at: SystemTime,
    last_accounted_at: SystemTime,
    state: SessionState,
    backend_handle: BackendHandle,
    capabilities: BackendCapabilities,
}
```

Regeln:

- Eine Session gehört dem Profil, das sie gestartet hat, nicht dem aktuell
  aktiven Profil.
- Die Home-Rückkehr beendet keine Session und pausiert die Spielzeit nicht.
- `gio launch` allein darf nie als „Spiel läuft“ gelten.
- Nach einem CouchDeck-Neustart werden keine unbelegten „läuft noch“-Aussagen
  getroffen.
- Persistenz erfolgt rückwärtskompatibel, z. B. über eine separate
  `sessions.json` mit Schema-Version.

## Backend-Machbarkeit

| Backend | Start | Start erkennen | Session zuordnen | Ende erkennen | Fokus | Regulär beenden | Notfall-Stop |
|---|---|---|---|---|---|---|---|
| **Direct/Managed** (Ryujinx POC) | ✅ | ✅ über Scope/MainPID/Fenster | ✅ PID/Cgroup/Unit | ✅ Unit-Ende | ⚠️ KDE KWin, andere offen | ✅ KWin `closeWindow` per PID | ✅ eigene `.scope` stoppen |
| **Prism/Minecraft** | ✅ via Flatpak | ✅ POC: `flatpak ps`/Java-PID/Fenster | ✅ POC: Instanz/Java-PID | ✅ POC: Fenster weg = Instanz weg | ⚠️ | ✅ POC: KWin exakte Java-PID | ✅ POC: `flatpak kill <instance>`; gesamte Instanz |
| **Steam (nativ, POC)** | ✅ via GIO/Steam | ✅ POC: App-ID + `app-steam-app<ID>-<pid>.scope` + Fenster-PID | ✅ POC: Spiel-Scope/SID | ✅ POC: Scope-Ende | ⚠️ | ✅ POC: KWin exakte PID | ✅ POC: eigene Spiel-Scope; Steam bleibt |
| **Steam Flatpak (POC)** | ✅ via Flatpak/GIO | ✅ POC: Fenster-/Spiel-PID | ⚠️ gemeinsame Flatpak-Scope | ✅ POC: Fenster weg | ⚠️ | ✅ POC: KWin exakte PID | ❌ nur gesamte Flatpak-Steam-Instanz |
| **Generisches GIO (POC)** | ✅ | ⚠️ nur Fenster/App-ID | ❌ kein PID/Scope | ❌/⚠️ | ❌ | ⚠️ nur bei eindeutigem Fenster | ❌ |

### Direct/Managed

- Strukturierte, validierte Argumente; kein Parsen beliebiger `Exec=`-Werte.
- Direktstart über einen systemd-User-Scope bzw. eine transiente Unit.
- Der eigene Scope wird nur für geeignete, explizit verwaltete Einträge genutzt.
- Regulärer Weg zuerst: gezielte Fensterschließung über die exakte PID.
- Notfallweg danach: ausschließlich die eigene Scope-Unit stoppen, nach
  Bestätigung.
- Andere Desktops als KDE benötigen eine eigene getestete Fensterschließstrategie;
  sonst bleibt `can_graceful_stop = false`.

### Steam (nativ) / Steam Flatpak

- Start bleibt GIO/Steam.
- Nativ erzeugt Steam für das Spiel eine eigene
  `app-steam-app<appid>-<pid>.scope`; der Spielprozess und die Steam-Runtime
  liegen darin, der Steam-Client bleibt separat.
- Regulär: Fenster über exakte Spiel-PID schließen (KDE-POC erfolgreich).
- Notfall: eigene Spiel-Scope stoppen, nicht den Steam-Client.
- Flatpak-Steam hat eine gemeinsame `app-flatpak-com.valvesoftware.Steam-…scope`
  für Steam und Spiel; ein spielspezifischer Scope-Stop ist nicht sauber
  möglich. Regulär funktioniert das Fensterschließen über die exakte PID.
- Noch offen: Steam vollständig beendet, mehrere Spiele, Proton-/Overlay-Fälle,
  GNOME/XFCE und ein Test ohne parallele native Steam-Instanz.
- Details: [`STEAM-SPIKE-RESULTS.md`](STEAM-SPIKE-RESULTS.md) und
  [`STEAM-FLATPAK-SPIKE-RESULTS.md`](STEAM-FLATPAK-SPIKE-RESULTS.md).

### Prism/Minecraft

- Start über Flatpak/GIO bzw. den verwalteten Instanz-Pfad.
- Spike-Ergebnis: `flatpak run --launch <instance>` startet Prism und Java;
  `flatpak ps` liefert die Instanz, das Minecraft-Fenster hat die Java-PID.
- Ein äußerer `systemd-run`-Scope kontrolliert die Flatpak-Prozesse **nicht**;
  Flatpak legt eine eigene `app-flatpak-…scope` an.
- Regulär: Minecraft-Fenster über exakte Java-PID schließen (KWin-POC
  erfolgreich). Notfall: `flatpak kill <instance>` für die eigene Instanz.
- Offen: Prism-UI gleichzeitig geöffnet, mehrere Instanzen, GNOME/XFCE.
- Details: [`PRISM-SPIKE-RESULTS.md`](PRISM-SPIKE-RESULTS.md).

### Generisches GIO

- Bleibt unverändert: `gio launch` für normale Desktop-Dateien.
- Keine Prozesskontrolle, keine sichere Session-Zuordnung.
- Fenster können nachträglich über App-ID/Titel beobachtet werden; das ist
  heuristisch, kein Prozessownership.
- Konservativer Kompatibilitätsmodus für Zeit und Status.
- Spike: [`GENERIC-GIO-SPIKE-RESULTS.md`](GENERIC-GIO-SPIKE-RESULTS.md).

## Zeitkonten

Die bisherige Messung bleibt zunächst als Fallback bestehen. Das neue Modell
führt Session-Zeit pro Session und startendem Profil:

- CouchDeck-Fokus ist irrelevant; Home/Minimieren/Profilwechsel pausieren die
  Spielzeit nicht.
- Parallele Sessions werden getrennt geführt; Zeit wird je Profil summiert.
- Standby/Suspend wird über Logind/Zeitquellen behandelt; Schlafzeit soll nicht
  als Spielzeit zählen.
- Direct/Managed-Sessions werden beim Neustart über Unit/PID wieder gesucht
  oder als beendet markiert.
- Unzuverlässige Backends nutzen einen dokumentierten konservativen Modus. Bei
  unbekanntem Status wird die Zeit nicht einfach pausiert; die konkrete
  Obergrenze/Grace-Policy wird konfigurierbar und geloggt.
- Die aktuelle `timekeeping.json`-Logik wird erst ersetzt, wenn die neue
  Messung stabil ist. Zunächst können beide Konten im Shadow Mode laufen.

## Zeitbudget-Aktionen

Konfigurierbar pro Kinderprofil:

```rust
enum ExpiryAction {
    WarnOnly,
    PreventNewStarts,
    GracefulStop,
    GracefulThenEmergency,
}
```

- Standard: Warnung + keine neuen Starts.
- Graceful Stop nur mit passender Backend-Fähigkeit.
- Emergency Stop nur nach ausdrücklicher Konfiguration und Bestätigung.
- Keine automatische harte Terminierung ohne Zustimmung.
- Elternfreigabe für Zusatzminuten bleibt der bestehende PIN-Flow.

## Schrittweise Implementierung

### 0.6.0 – Session Foundation

- Session-Modell und Persistenz.
- Backend-/Capability-Registry.
- GameSessionManager mit Event-Loop.
- Direct/Managed-Backend als POC für Ryujinx.
- Home-Menü: „Nur zurück“, „Spiel fortsetzen“ (Best-Effort), „Sitzung beenden“
  nur bei vorhandener Capability.
- Zeitkonten im Shadow Mode; keine Ablösung der bisherigen Messung.
- Keine automatische Terminierung.

### 0.6.1 – Zeitkonten-Härtung

- Session-Zeitmessung für Direct/Managed aktivieren.
- Kompatibilitätsmodus für unzuverlässige Backends.
- Neustart-, Absturz- und Suspend-Behandlung.
- Warnungen, Grace Period, Startsperre.
- Vergleich alter/neuer Messung dokumentieren.

### 0.6.2 – Kontrollierte Beendigung

- Reguläres Fensterschließen für verifizierte Backends.
- Notfall-Scope-Stop optional und bestätigt.
- Zeitbudget-Aktion „Graceful Then Emergency“.
- Ryujinx-Realtest mit echtem Spiel und Controller.

### 0.7.0 – Weitere Backends

- Steam-/Flatpak-Integration nach eigenem Spike.
- Prism/Minecraft-Integration nach Prozess-/Fenster-Spike.
- GNOME-/XFCE-Fensterschließung und Fokus.
- Weitere Emulator-Backends.

## Leitplanken

- GIO für normale Desktop-Dateien bleibt unverändert.
- Kein `pkill`, keine Prozessnamen-Suche.
- Direct/Managed nur mit strukturierten, validierten Argumenten.
- Tracking und Beendigung sind getrennte Fähigkeiten.
- Fensterschließen vor Scope-Stop.
- Scope-Stop nur für die eigene, verifizierte Unit.
- Keine „läuft“-Behauptung nach einem erfolgreichen Startbefehl.
- Keine automatische Fokussierung während des Spiels.
- Kinderprofile, PIN und bestehende Zeitmessung bleiben bis zur nachgewiesenen
  Migration unangetastet.
