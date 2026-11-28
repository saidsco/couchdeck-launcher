# Game Session Research Phase II – Zusammenfassung

Stand: 26. November 2026
Status: Research abgeschlossen für die geprüften Backends; **keine
produktive Session-Implementierung, keine Zeitkonten-Ablösung**.

Produktstatus: Der aktuelle Release ist **CapyDeck 0.6.0**. Die Umbenennung
von CouchDeck zu CapyDeck ist abgeschlossen; die Session-Foundation folgt als
0.6.1. Dieses Dokument beschreibt die technische Grundlage unabhängig vom
Rebranding.

Diese Phase hatte das Ziel, eine belastbare technische Grundlage für den
GameSessionManager und sessionbasierte Kinder-Zeitkontingente zu schaffen.
Bazzite KDE/Wayland bleibt die primäre Zielplattform; die realen Tests fanden
auf Manjaro/KDE/Wayland statt.

## Durchgeführte Tests

| Bereich | Umgebung | Ergebnis |
|---|---|---|
| Steam nativ, natives Linux-Spiel (LEGO Bricktales) | Manjaro/KDE/Wayland | Scope, Fenster-PID, Close, Crash, Notfall-Stop |
| Steam nativ, Proton-Spiel (Disco Elysium, Proton 8.0) | Manjaro/KDE/Wayland | Scope, Proton/Wine, Close, Crash |
| Steam nativ, weiteres Proton-Spiel (Stray, Proton 9.0) | Manjaro/KDE/Wayland | Scope, Kindprozess-Fenster-PID, Close ignoriert, SIGTERM |
| Steam kalt vs. warm | Manjaro/KDE/Wayland | Kaltstart über `gio launch` funktioniert |
| Zwei Steam-Spiele gleichzeitig | Manjaro/KDE/Wayland | getrennte Scopes, unabhängiger Stop |
| Prism/Minecraft Flatpak | Manjaro/KDE/Wayland | UI offen/zu, ein/zwei Instanzen, Close, Crash |
| Ryujinx Direct/Managed mit echtem Spiel | Manjaro/KDE/Wayland | Service-Unit, Vollbild, Close-Dialog, Notfall-Stop |
| Native Test-App (GTK4) | Manjaro/KDE/Wayland | Kindprozess, Crash, Fenster-Neuerstellung |
| Generic GIO API + Desktop-Dateien | Manjaro/KDE/Wayland | PID-Callback, Wrapper, D-Bus, Early-Exit, Fehlerfälle |

Nicht getestet: Bazzite, GNOME, XFCE, SteamOS Gaming Mode, Steam Flatpak,
Controller-Hardware, vollständige interaktive Spielverläufe mit Speichern/Laden,
Steam Cloud, Steam Input.

Wiederholungen der relevanten Start-/Close-Pfade: LEGO Bricktales 3×,
Disco Elysium 2×, Stray 1×, Prism/Minecraft 3×, Ryujinx 3× (plus
Direct/Managed-Test-App). Stray und einzelne Sonderfälle sind noch nicht
mehrfach bestätigt.

## Wichtigste Erkenntnisse

### 1. Steam: pro Spiel eine eigene Scope – auch unter Proton

Sowohl native Linux-Spiele als auch Proton-Spiele erhalten:

```text
app-steam-app<appid>-<pid>.scope
```

Die Scope enthält den Spielprozess (nativ) bzw. Proton/Wine und den
Windows-Spielprozess. Der Steam-Client bleibt außerhalb. Ein Stop der eigenen
Spiel-Scope beendet nur das Spiel.

**Aber:** `MainPID` ist bei einer Scope leer. Die Identität muss über
`ControlGroup`/`cgroup.procs` und die Fenster-PID verifiziert werden. Der
Scope-Name allein ist kein Beweis.

### 2. Die LEGO-Bricktales-Abweichung ist geklärt

Der Phase-I-Bericht nannte `LEGO_Bricktales.x86_64`. Das ist der **native
Linux-Build**. Es gab kein Proton-Prefix (`compatdata/1898290` leer), keinen
`CompatToolMapping`-Eintrag und keine Proton-Prozesse. Die Annahme „LEGO lief
über Proton“ trifft für diese Installation nicht zu. Proton wurde separat mit
Disco Elysium getestet.

### 3. Close-Request ist nicht Ende

- Steam: Scope-Ende ca. 1–2 s nach Close/Crash.
- Stray (Proton 9.0): Zwei `closeWindow()`-Aufrufe wurden ignoriert; erst ein
  gezieltes `SIGTERM` auf die exakte Spiel-PID beendete die Scope (ca. 5 s).
  Fensterschließen ist also spielabhängig.
- Prism: Java-Ende ca. 2 s nach Fensterschließen; Prism-UI bleibt.
- Ryujinx: Standard-Bestätigungsdialog verhindert automatisches Schließen;
  mit `show_confirm_exit=false` endet die Unit in ca. 2 s.
- Fenster können nach einem Absturz kurzzeitig weiterleben.

**Fensterexistenz ≠ Prozesslebendigkeit ≠ Spielsitzung.**

### 4. Ryujinx Direct/Managed braucht eine verwaltete Konfiguration

- `start_fullscreen=true` + `remember_window_state=false` erzeugt ein
  KWin-Vollbildfenster; `--fullscreen` allein genügte im Test nicht.
- `show_confirm_exit=false` ermöglicht reguläres Fensterschließen.
- `systemctl stop` kann blockieren; Notfall-Stop braucht `--no-block`,
  Timeout und `SIGKILL`-Fallback.
- Service-Unit: `MainPID`/cgroup/Fenster-PID sind stabil.
- Scope: kein `MainPID`; Stop nur mit `.scope`-Suffix.

### 5. Prism: Java-PID ist der Spielanker, die Flatpak-Scope nicht

- Bei geöffneter UI wird `--launch` an die bestehende Flatpak-Instanz
  weitergereicht.
- Java-PID ist Kind von `prismrun`; Fenster-PID = Java-PID.
- Die Prism-Instanz lässt sich über
  `-Djava.library.path=.../instances/<name>/natives` zuordnen.
- Zwei Instanzen teilen sich dieselbe Flatpak-Scope; ein `flatpak kill` würde
  beide beenden.
- Ein zweiter `--launch` derselben Instanz startet kein zweites Spiel
  (Exit-Code 0 trotzdem).

### 6. Generic GIO bleibt Kompatibilitätsbackend

Der PID-Callback von `g_desktop_app_info_launch_uris_as_manager()` hilft nur,
wenn GIO tatsächlich einen Prozess spawnt:

- Direkter Prozess: korrekte PID.
- Shell-Wrapper: Wrapper-PID, nicht Spiel-PID.
- Early-Exit-Launcher: PID kann bereits tot sein.
- D-Bus-Aktivierung: kein Callback.
- Single-Instance-Apps: möglicherweise kein neuer Prozess.

Der Callback ist kein Identitäts- oder Lebensdauerbeweis. `gio launch` bleibt
für normale Desktop-Dateien unverändert.

### 7. Parallele Sessions sind möglich, aber unterschiedlich stark trennbar

- Steam: getrennte Scopes pro Spiel; unabhängiger Stop möglich.
- Prism: mehrere Java-Prozesse in einer Flatpak-Scope; unabhängiger
  Fenster-Close möglich, Notfall-Stop nur für die ganze Instanz.
- Direct/Managed: eigene Unit pro Session; stärkste Trennung.

### 8. Zeitkonten müssen von der Bibliothekszeit entkoppelt werden

Die aktuelle Messung zählt die Zeit des aktiven Kinderprofils, solange
CapyDeck läuft – auch ohne Spiel. Das neue Modell zählt nur nachgewiesene
Spielsitzungen, pausiert nicht bei Home/Minimieren, zählt parallele Sessions
als Vereinigung und erfasst tatsächliche Nutzung auch über das Budget hinaus.

## Überarbeitete Capability-Matrix

Qualitätsstufen: **U** unbekannt, **E** experimentell, **V** verifiziert,
**P** produktiv unterstützt.

| Backend | Start | Start erkennen | Session zuordnen | Ende erkennen | Fokus | Regulär beenden | Notfall-Stop | Stufe |
|---|---|---|---|---|---|---|---|---|
| **Direct/Managed** (Ryujinx) | ✅ | ✅ Unit/MainPID/cgroup/Fenster | ✅ Unit + PID-Startzeit | ✅ Unit inactive | ⚠️ KDE/KWin | ✅ nur mit managed Config | ✅ Unit-Stop + SIGKILL-Fallback | **E** |
| **Steam nativ** | ✅ | ✅ Scope + Fenster-PID | ✅ App-ID + cgroup + PID | ✅ Scope-Ende | ⚠️ KDE/KWin | ⚠️ pro Spiel (Close kann ignoriert werden) | ✅ eigene Spiel-Scope | **E** |
| **Prism/Minecraft** | ✅ | ✅ Java-PID + Instanzpfad | ✅ Instanz-ID + Java-PID | ✅ Java-Ende | ⚠️ KDE/KWin | ✅ Fenster exakte Java-PID | ⚠️ nur gesamte Flatpak-Instanz | **E** |
| **Steam Flatpak** | ✅ (POC) | ⚠️ Fenster/Spiel-PID | ⚠️ gemeinsame Flatpak-Scope | ⚠️ Fenster weg | ⚠️ | ✅ exakte PID (POC) | ❌ nur gesamte Instanz | **E/zurückgestellt** |
| **Generisches GIO** | ✅ | ⚠️ nur Fenster/App-ID | ❌ kein PID/Scope | ❌/⚠️ | ❌ | ⚠️ nur eindeutiges Fenster | ❌ | **E/Kompatibilität** |

Legende: ✅ nachgewiesen, ⚠️ eingeschränkt/offen, ❌ nicht möglich.

## Was für die Implementierung noch fehlt

### Session Foundation (0.6.1)

- `GameSession`-/`SessionIdentity`-Modell und `sessions.json`.
- `GameSessionManager` mit eigenem Event-Loop, Backend-Registry und
  Capability-Bestätigung.
- Direct/Managed-Backend mit verwalteter Ryujinx-Konfiguration
  (`start_fullscreen`, `show_confirm_exit=false`, eigener Root).
- Home-Menü: „Nur zurück“, „Spiel fortsetzen“ (Best-Effort), „Sitzung
  beenden“ nur bei Capability.
- Shadow-Zeitmessung parallel zur bestehenden Messung.
- Keine automatische Terminierung.

### Zeitkonten-Härtung (0.6.2)

- Session-Zeit für Direct/Managed aktivieren.
- Kompatibilitätsmodus für nicht owning-fähige Backends.
- Neustart-, Absturz-, Suspend-Behandlung.
- Warnungen, Grace Period, Startsperre.
- Shadow-Vergleich dokumentieren.
- Datenmodell Schema 2 (`used_seconds_actual` statt gekapptem Wert).

### Kontrollierte Beendigung (0.6.3)

- Steam-/Prism-Graceful-Stop für verifizierte Fälle.
- Notfall-Stop optional und bestätigt.
- `GracefulThenEmergency`-Policy.
- UI-Transparenz bei `StopUnconfirmed`.
- Ryujinx-Realtest mit Controller und vollständigem Spielverlauf.

### Weitere Backends (0.7.0)

- Steam Flatpak nach eigenem Spike.
- GNOME/XFCE-Fensterschließung und Fokus.
- Weitere Emulatoren/Launcher.
- Optional: Heroic/Lutris, Kodi (nicht priorisiert).

## Offene Forschungsfragen

- Bazzite KDE/Wayland-Abnahme (alle Backends).
- Controller-Hardware, Hotplug, Steam Input.
- Vollständige Spielverläufe inkl. Speichern/Laden.
- Steam Cloud, Overlay-Interaktion.
- Weitere Proton-Versionen und Anti-Cheat-/Launcher-Spiele.
- Suspend/Resume und CapyDeck-Absturz während einer Session.
- PID-Reuse über echte Neustarts.
- Prism/Minecraft mit mehr als zwei Instanzen.
- Multi-Monitor-Vollbild.
- GNOME/XFCE.

## Messwerte (Manjaro/KDE/Wayland)

| Vorgang | Beobachtung |
|---|---|
| Steam LEGO: Scope sichtbar | ca. 4 s nach `gio launch` |
| Steam LEGO: Fenster sichtbar | ca. 7–11 s |
| Steam LEGO: Close → Scope inaktiv | ca. 2,0 s |
| Steam Disco (Proton): Scope sichtbar | ca. 68 s (Erststart/Prefix) |
| Steam Disco: Fenster sichtbar | ca. 79 s |
| Steam Disco: Crash (`SIGKILL`) → Scope inaktiv | ca. 1,0 s |
| Steam Stray (Proton 9.0): Scope sichtbar | ca. 16 s |
| Steam Stray: Fenster sichtbar | ca. 26 s |
| Steam Stray: Close ignoriert | 2 Versuche ohne Ende |
| Steam Stray: `SIGTERM` exakte Spiel-PID → Scope inaktiv | ca. 5 s |
| Steam Kaltstart: Scope sichtbar | ca. 10 s |
| Steam Kaltstart: Fenster sichtbar | ca. 12,6 s |
| Prism UI: Fenster sichtbar | ca. 2,6 s |
| Prism: `--launch` → Minecraft-Fenster | ca. 10,7 s |
| Prism: Minecraft-Close → Java tot | ca. 2,0 s |
| Prism: Java-Crash (`SIGKILL`) → Java tot | ca. 1,0 s |
| Ryujinx: Fenster sichtbar | ca. 13 s |
| Ryujinx: managed Close → Unit inaktiv | ca. 2,0 s |
| Ryujinx: Fenster-Close → Unit inaktiv | ca. 10,1 s (Aufräumen) |
| Test-App: Crash (`SIGKILL`) → Unit inaktiv | ca. 1 s |

### Testumgebung

| Komponente | Version |
|---|---|
| Distribution | Manjaro Linux, Kernel 6.18.63-3 |
| Desktop | KDE Plasma 6.8.2, KWin 6.8.2, Wayland |
| GPU | NVIDIA RTX 4070 Ti SUPER |
| systemd | 262 |
| GLib/GIO | 2.90.1 |
| Steam | nativ, Client-Build 1794536281 |
| Proton | 8.0-5d (`8.0-103`), 9.0 (Beta) |
| Flatpak | 1.18.4 |
| Prism Launcher | 11.1.1 (Flatpak) |
| Java | Microsoft Java 25.0.1 |
| Ryujinx | 1.3.3, .NET 9.0.9 |

## Dokumente

- [`GAME-SESSION-ARCHITECTURE.md`](GAME-SESSION-ARCHITECTURE.md) – Gesamtarchitektur, Matrix, Roadmap
- [`SESSION-IDENTITY-AND-LIFECYCLE.md`](SESSION-IDENTITY-AND-LIFECYCLE.md) – Identität, Zustände, Recovery
- [`TIME-ACCOUNTING.md`](TIME-ACCOUNTING.md) – sessionbasierte Zeitkonten
- [`TIME-LIMIT-ENFORCEMENT.md`](TIME-LIMIT-ENFORCEMENT.md) – Warnungen, Stop, Notfall
- [`STEAM-SPIKE-RESULTS.md`](STEAM-SPIKE-RESULTS.md) – native/Proton-Steam
- [`PRISM-SPIKE-RESULTS.md`](PRISM-SPIKE-RESULTS.md) – Prism/Minecraft
- [`RYUJINX-SPIKE-RESULTS.md`](RYUJINX-SPIKE-RESULTS.md) – Direct/Managed
- [`GENERIC-GIO-SPIKE-RESULTS.md`](GENERIC-GIO-SPIKE-RESULTS.md) – GIO-Kompatibilität
- [`BAZZITE-SESSION-TEST-MATRIX.md`](BAZZITE-SESSION-TEST-MATRIX.md) – reproduzierbare Abnahme
- [`spikes/`](spikes/) – C-/Python-Testwerkzeuge

## Phase III

Die anschließende Phase III untersucht einen Session Supervisor
(systemd + Child-Subreaper) und Managed GIO. Ergebnisse:

- [`SESSION-SUPERVISOR-SPIKE-RESULTS.md`](SESSION-SUPERVISOR-SPIKE-RESULTS.md)
- [`SESSION-SUPERVISOR-DECISION.md`](SESSION-SUPERVISOR-DECISION.md)

Entscheidung: **Variante C** – systemd-Unit mit `ExitType=cgroup` als durable
Anchor, darin ein minimaler Subreaper-Supervisor. Steam und Prism bleiben ohne
zusätzlichen Supervisor. Managed GIO bleibt opt-in.

## Änderungsumfang

- Keine Änderungen am CapyDeck-/CapyDeck-Quellcode.
- Keine absichtlichen Änderungen an Steam-/Prism-Konfigurationen.
- Ryujinx-Tests mit verwalteter Konfiguration nutzten einen temporären Root.
  Der erste Realtest nutzte die bestehende Ryujinx-Konfiguration und
  aktualisierte normale Laufzeitdaten (Fensterzustand, Logs, Save-Daten); der
  ursprüngliche Fensterzustand wurde wiederhergestellt.
- Steam und Prism können normale Laufzeitdaten (Logs, Spielzeit, Caches)
  aktualisiert haben.
- Die temporäre Prism-Testinstanz `capydeck-test-2` wurde entfernt.
- Kein Commit, kein Tag, kein Release.
- Die Testwerkzeuge unter `docs/spikes/` sind reine Research-Artefakte.
