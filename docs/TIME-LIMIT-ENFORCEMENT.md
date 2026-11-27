# Durchsetzung des Zeitlimits

Stand: Phase II, 26. November 2026
Status: Architektur/Research, keine Implementierung.

Dieses Dokument beschreibt, was passieren soll, wenn das Zeitbudget eines
Kinderprofils abläuft. Es trennt strikt:

- **Warnen**
- **Neue Starts verhindern**
- **Regulär beenden**
- **Notfall-terminieren**

Automatische harte Terminierung bleibt eine gesonderte, ausdrücklich zu
genehmigende Funktion.

## Grundsätze

1. Eine gesendete Schließanforderung ist **kein** erfolgreiches Beenden.
2. Wenn eine Sitzung nicht zuverlässig beendet werden kann, muss CouchDeck das
   transparent anzeigen.
3. Notfall-Terminierung nur für die eigene, verifizierte Session.
4. Kein `pkill`, keine Prozessnamen-Suche, keine fremden Prozesse.
5. Die bestehende Elternfreigabe (PIN, Zusatzminuten) bleibt erhalten.
6. Die UI blockiert nicht; Stop-Aufträge laufen im Session-Manager.

## Konfigurierbare Aktionen

```rust
enum ExpiryAction {
    WarnOnly,
    PreventNewStarts,
    GracefulStop,
    GracefulThenEmergency,
}
```

Empfohlene Standardwerte:

| Profil | Standard |
|---|---|
| Kinderprofil | `PreventNewStarts` + Warnungen |
| Kinderprofil, Eltern konfigurieren | optional `GracefulStop` |
| Kinderprofil, ausdrücklich freigegeben | `GracefulThenEmergency` |
| Admin/Erwachsene | keine Begrenzung |

Zusätzlich:

- Warnschwellen: 5 Minuten, 1 Minute, erschöpft (bestehend).
- Optionaler Alarmton (`alarm_sound`, bestehend).
- Schonfrist zum Speichern: konfigurierbar, z. B. 60 s, Standard 0 s nur bei
  expliziter Aktivierung.
- „Nur eine Spielsitzung gleichzeitig“ pro Kinderprofil (optional, siehe
  [`TIME-ACCOUNTING.md`](TIME-ACCOUNTING.md)).

## Ablauf bei Ablauf des Budgets

```text
Restzeit > 0
  └─ Warnungen bei 5 min / 1 min (einmalig, mit optionalem Ton)

Restzeit == 0
  ├─ PreventNewStarts: neue Starts blockieren, verständliche Meldung
  ├─ Laufende Sessions: je nach ExpiryAction
  │    ├─ WarnOnly: weiterlaufen lassen, nur anzeigen
  │    ├─ GracefulStop: Close anfordern, auf Ende warten
  │    └─ GracefulThenEmergency: Close, dann nach Timeout Notfall-Stop
  └─ Elternfreigabe: PIN → Zusatzminuten → Restzeit wieder positiv
```

## Reguläres Beenden

### Drei getrennte Ebenen

1. **Reguläre Anwendungsschließung**
   - Fenster über die exakte PID schließen (KWin).
   - Nur wenn die Anwendung Close unterstützt.
   - Ryujinx: `show_confirm_exit=false` in der managed Config.
   - Steam: Close kann ignoriert werden (Stray).
2. **SIGTERM / Unit-Stop**
   - `systemctl stop` sendet SIGTERM an die cgroup; das ist **nicht**
     automatisch graceful.
   - Nur nach verifizierter Session-Identität.
   - Timeout und `StopUnconfirmed` beachten.
3. **SIGKILL / Notfall**
   - `systemctl kill --signal=KILL` auf die eigene Unit oder pidfd-SIGKILL.
   - Nur ausdrücklich konfiguriert/freigegeben.

Der Supervisor beendet **keine** Spiele aufgrund eines Zeitlimits oder eines
verschwundenen Fensters. Die Beendigung wird vom GameSessionManager nach
verifizierter Identität und ausdrücklicher Aktion/Policy ausgelöst.

### Ablauf

1. Capability prüfen: `can_graceful_stop`.
2. Close-Request an das Backend:
   - Direct/Managed: KWin `closeWindow` auf die exakte Fenster-PID; nur wenn
     die verwaltete Konfiguration den Bestätigungsdialog deaktiviert hat.
   - Steam: KWin `closeWindow` auf die exakte Spiel-PID. Wird der Close
     ignoriert, ist ein gezieltes `SIGTERM` auf dieselbe, verifizierte Spiel-PID
     der zweite Versuch.
   - Prism: KWin `closeWindow` auf die exakte Java-PID.
   - Generic GIO: nur wenn ein eindeutiges Fenster zugeordnet ist; sonst nicht.
3. Zustand `Stopping` setzen.
4. Auf Prozess-/Unit-/Scope-Ende warten (Timeout, z. B. 15–30 s).
5. Bei Erfolg: `Ended`.
6. Bei Timeout: `StopUnconfirmed` und UI-Transparenz.
7. Bei Supervisor-Verlust: `TrackingLost`; zuerst Unit/cgroup/Fenster prüfen,
   dann ggf. Unit-Stop.

### Wichtig: Bestätigungsdialoge

Ryujinx zeigt standardmäßig einen Bestätigungsdialog
(`ContentDialogOverlayWindow`). Ein Close-Request öffnet nur den Dialog; das
Spiel läuft weiter. Ohne Eingabeinjektion kann CouchDeck den Dialog nicht
bestätigen.

Deshalb:

- Direct/Managed startet Ryujinx mit einer **verwalteten Konfiguration**, in der
  `show_confirm_exit=false` gesetzt ist.
- Ist das nicht möglich, bleibt `can_graceful_stop = false`; die UI zeigt das
  an.
- Ein Dialog darf nicht als „Spiel beendet“ gewertet werden.

### Spiele, die Close ignorieren

Stray (Proton 9.0) ignorierte zwei KWin-`closeWindow()`-Aufrufe; erst ein
gezieltes `SIGTERM` auf die exakte Spiel-PID beendete die Scope. Deshalb:

- `can_graceful_stop` wird pro Spiel/Session bestätigt, nicht nur pro Backend.
- Nach einem ignorierten Close folgt ein gezieltes `SIGTERM` auf dieselbe
  verifizierte PID.
- Bleibt das Ende aus, folgt `StopUnconfirmed`; der Scope-Stop bleibt der
  Notfallweg.
- Vor einem Signal-Stop muss die PID eindeutig in der Spiel-Scope verifiziert
  sein.

## Notfall-Stop

### Voraussetzungen

- `can_emergency_stop` ist für die konkrete Session bestätigt.
- Der Handle ist eindeutig:
  - Direct/Managed: eigene Unit/Scope.
  - Steam: eigene Spiel-Scope.
  - Prism: nur wenn keine andere Session in derselben Flatpak-Instanz läuft.
  - Generic GIO: nie.
- Ausdrückliche Konfiguration/Freigabe durch Admin.
- Bei automatischer Ausführung: vorher konfiguriert und bestätigt; keine
  stillschweigende Aktivierung.

### Ablauf

1. Zustand `Stopping`.
2. Nicht blockierend stoppen:
   - Direct/Managed, Ebene 2 (SIGTERM):
     `systemctl --user stop --no-block <unit>`.
   - Direct/Managed, Ebene 3 (SIGKILL, nur Notfall):
     `systemctl --user kill --signal=KILL <unit>`.
   - Steam: `systemctl --user stop <game-scope>` (bzw. SIGTERM auf die exakte
     Spiel-PID, wenn die Scope nicht der richtige Anker ist).
   - Prism: `flatpak kill <instance>` **nur** bei nachweislich keiner anderen
     Session.
3. Auf Ende warten; bei Erfolg `Ended`.
4. Bei Misserfolg: `StopUnconfirmed`, UI-Meldung, Log.
5. Niemals `pkill`, Prozessnamen-Suche oder fremde Units.
6. Wo verfügbar, pidfd + Startzeit für Signalidentität nutzen; keine Signale
   an eine möglicherweise wiederverwendete PID.

### Blockierendes `systemctl stop`

Phase II zeigte: `systemctl --user stop` kann bei einem hängenden Prozess
blockieren. Der Session-Manager darf deshalb:

- `--no-block` verwenden,
- einen kurzen Timeout setzen,
- danach gezielt `SIGKILL` auf die eigene Unit anwenden,
- die UI nie blockieren.

## UI-Transparenz

Die UI muss unterscheiden:

| Zustand | Anzeige |
|---|---|
| `Stopping` | „Spiel wird beendet …“ |
| `StopUnconfirmed` | „Das Spiel reagiert nicht auf die Schließanforderung.“ |
| `TrackingLost` | „Sitzung wird noch über systemd/cgroup beobachtet.“ |
| `EmergencyRequired` | „Notfall-Stopp erforderlich“ (nur wenn freigegeben) |
| `EmergencyRunning` | „Notfall-Stopp läuft …“ |
| `Ended` | „Beendet“ |
| `Unknown` | „Status unklar“ mit Erklärung |

Keine falsche Erfolgsmeldung. Kein „beendet“, solange der Prozess/die Unit
noch lebt.

## Elternfreigabe

- Bestehender PIN-Flow bleibt.
- Zusatzminuten erhöhen `bonus_minutes`.
- Nach Freigabe:
  - Restzeit neu berechnen,
  - Warnflags neu bewaffnen,
  - laufende Sessions dürfen weiterlaufen (wenn sie nicht bereits gestoppt
    wurden),
  - Startsperre aufheben, wenn Restzeit > 0.

## Zeitlimit und Session-Manager

Der Session-Manager liefert:

- `SessionStarted`, `SessionEnded`, `SessionStateChanged`
- `SessionTimeTick` (oder Zeitabfrage)
- `SessionStopRequested`, `SessionStopConfirmed`, `SessionStopFailed`
- `SessionCapabilitiesChanged`

Das Zeitkonto abonniert diese Ereignisse. Die Durchsetzung läuft nicht im
UI-Thread.

## Nachgewiesene Erkenntnisse (Phase II/III)

- Close-Request ≠ Ende (Ryujinx-Bestätigungsdialog).
- Close-Request kann auch bei Proton-Spielen ignoriert werden (Stray); ein
  gezieltes `SIGTERM` auf die exakte Spiel-PID kann helfen.
- Scope-/Unit-Ende kann dem Close um Sekunden nachlaufen.
- `systemctl stop` kann blockieren; Notfall-Stop braucht Timeout/Fallback.
- `systemctl stop` ist SIGTERM-Ebene, nicht automatisch graceful.
- `systemctl kill --signal=KILL` ist die Notfall-Ebene.
- `ExitType=cgroup` hält die Unit nach Supervisor-Crash am Leben; der
  Notfall-Stop muss dann über die Unit erfolgen.
- Prism-`flatpak kill` betrifft die gesamte Flatpak-Instanz.
- Steam-Spiel-Scope-Stop betrifft nur das Spiel.
- Fenster können nach einem Absturz kurzzeitig weiterleben.
- Der Supervisor beendet keine Spiele aufgrund von Zeitlimit oder
  Fensterverlust; die Policy liegt beim GameSessionManager.

## Technische Annahmen

- KWin `closeWindow` ist unter Bazzite/KDE verfügbar.
- systemd-User-Units erlauben `KillMode=control-group` und gezielte Stops.
- Die verwaltete Ryujinx-Konfiguration kann `show_confirm_exit=false` setzen,
  ohne die Benutzerkonfiguration zu verändern.
- Eltern-PIN und Zeitkonto bleiben lokal und ohne Netzwerk.

## Offene Forschungsfragen

- Automatischer Graceful Stop für Steam/Prism auf Bazzite.
- Verhalten, wenn der Benutzer während eines Stop-Versuchs eingreift.
- Notfall-Stop bei mehreren Prism-Instanzen in derselben Flatpak-Instanz.
- Signalbehandlung von Proton-Spielen (SIGTERM vs. Fensterschließen).
- Alarmton unter Wayland/PipeWire.
- Barrierefreiheit/Controller-Bedienung der Stop-Dialoge.

## Testfälle (Mindestumfang)

1. Restzeit 5 min → Warnung, kein Stop.
2. Restzeit 1 min → Warnung, kein Stop.
3. Restzeit 0 → neue Starts blockiert.
4. `GracefulStop` bei Steam: Fenster schließt, Scope endet, UI bestätigt.
5. `GracefulStop` bei Ryujinx mit `show_confirm_exit=false`: Unit endet.
6. `GracefulStop` bei Ryujinx mit Bestätigungsdialog: `StopUnconfirmed`,
   keine falsche Erfolgsmeldung.
7. `GracefulThenEmergency`: Close, Timeout, Scope-/Unit-Stop, Ende.
8. Notfall-Stop bei zwei Steam-Spielen: nur das Zielspiel endet.
9. Notfall-Stop bei zwei Prism-Spielen: nur wenn freigegeben und eindeutig;
   sonst verweigern.
10. Elternfreigabe nach Ablauf: Restzeit positiv, Starts wieder erlaubt.
11. Close-Request während des Ladens: Timeout, kein falsches „beendet“.
12. CouchDeck-Absturz während `Stopping`: Recovery zeigt `Unknown`, keine
    erfundene Endzeit.
