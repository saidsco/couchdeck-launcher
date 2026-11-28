# Sessionbasierte Zeitkonten

Stand: Phase II, 26. November 2026
Status: Modell/Research, **keine produktive Ablösung**. Die bestehende
Zeitmessung bleibt zunächst aktiv; die neue Messung läuft im Shadow Mode.

## Warum ein neues Modell?

Die aktuelle Messung in `src/app.rs` (`tick_time_budget`) zählt die Zeit des
aktiven Kinderprofils, **solange CapyDeck läuft** – unabhängig davon, ob ein
Spiel läuft. Bibliotheksnavigation, Einstellungen und Profilwechsel zählen
mit. Das ist für ein Zeitkontingent „Spielzeit“ zu grob.

Das neue Modell zählt nur **nachgewiesene Spielsitzungen** und behandelt
Home-Rückkehr, Minimieren und Fokuswechsel korrekt.

## Ist-Zustand (CapyDeck 0.6.0; Verhalten unverändert seit 0.5.2)

- `timekeeping.json`, Schema 1.
- `ProfileTime { date, used_seconds, bonus_minutes, warned_5, warned_1 }`.
- `tick_time_budget()` läuft, solange CapyDeck geöffnet ist und das aktive
  Profil ein Kind mit `daily_limit_minutes` ist.
- `add_usage()` addiert höchstens die Restzeit; `used_seconds` wird am
  Gesamtbudget gekappt.
- Tageswechsel: Beim ersten Zugriff an einem neuen lokalen Kalendertag werden
  `used_seconds` und `bonus_minutes` zurückgesetzt.
- Warnungen bei 5 Minuten, 1 Minute und erschöpft; optionaler Alarmton.
- Elternfreigabe fügt `bonus_minutes` hinzu; `reset_extra_minutes` entfernt sie.
- Startsperre: `check_start_allowed()` blockiert neue Starts bei erschöpftem
  Budget.
- Speicherung alle 30 s und bei Warnungen/Ereignissen, atomar.

**Wichtig:** `used_seconds` ist heute eine gekappte Größe, keine tatsächlich
gemessene Nutzungsdauer. Über das Budget hinaus wird nichts mehr erfasst.

## Zielmodell

### Grundregeln

1. **Zeit läuft nur während einer nachgewiesenen Spielsitzung.**
2. **Home-Rückkehr, Minimieren, Fokuswechsel und Profilwechsel pausieren die
   Spielzeit nicht.**
3. **Die Zeit gehört dem Profil, das die Sitzung gestartet hat**, nicht dem
   aktuell aktiven Profil.
4. **Parallele Sitzungen desselben Profils werden nicht doppelt gezählt.**
   Gezählt wird die Vereinigung der Zeitintervalle, nicht die Summe.
5. **Tatsächlich verbrauchte Zeit wird auch über das Tagesbudget hinaus
   erfasst.**
6. **Die Restzeit wird auf null begrenzt, nicht die gemessene Nutzungsdauer.**
7. **Elternfreigaben/Zusatzminuten werden mit der bereits verbrauchten Zeit
   verrechnet.**
8. **Suspend/Schlafzeit wird nicht angerechnet.**
9. **Unbekannte Zustände dürfen die Zeitmessung nicht stillschweigend
   aussetzen.**
10. **Die alte Messung bleibt im Shadow Mode parallel aktiv, bis die neue
    Messung validiert ist.**

### Datenmodell

Tagesaggregat (`timekeeping.json`, Schema 2):

```json
{
  "schema_version": 2,
  "profiles": {
    "mia": {
      "date": "2026-11-26",
      "used_seconds_actual": 3720,
      "used_seconds_legacy": 3540,
      "bonus_minutes": 15,
      "warned_5": false,
      "warned_1": false,
      "exhausted_notified": true
    }
  }
}
```

- `used_seconds_actual`: tatsächlich gemessene Session-Zeit, **nicht gekappt**.
- `used_seconds_legacy`: Shadow-Vergleichswert der alten Messung.
- `bonus_minutes`: wie bisher.
- Restzeit = `max(0, limit + bonus - used_actual)`.
- `used_actual` kann das Budget überschreiten; die Anzeige „verbraucht“ zeigt
  die echte Zeit, „Rest“ bleibt 0.

Live-Sitzungen (`sessions.json`, Schema 1): siehe
[`SESSION-IDENTITY-AND-LIFECYCLE.md`](SESSION-IDENTITY-AND-LIFECYCLE.md).

### Zeitquellen

- `CLOCK_MONOTONIC` für Spielzeit (pausiert im Suspend).
- `CLOCK_BOOTTIME` zusätzlich für Session-Lebensdauer/Recovery.
- `boot_id` zur Erkennung eines Neustarts.
- `SystemTime`/Unix-Zeit für Tageszuordnung und Anzeige.
- logind `PrepareForSleep`/`PrepareForShutdown` (falls verfügbar) für
  Suspend-/Shutdown-Markierungen.
- Alle Zeitdifferenzen werden als `Duration`/Sekunden gespeichert, nicht als
  Fließkomma.

### Tagesgrenze

- Lokale Mitternacht (`chrono::Local`) wie bisher.
- Eine Sitzung, die über Mitternacht läuft, wird **an der Tagesgrenze
  aufgeteilt**: Der Anteil vor Mitternacht gehört zum alten Tag, der Rest zum
  neuen Tag.
- DST/Zeitzonenwechsel: Die Aufteilung erfolgt anhand lokaler Zeitpunkte; die
  Dauer wird aus Monotonic-Differenzen berechnet, nicht aus Wanduhr-Sprüngen.
- Beim Tageswechsel werden `used_actual`, `used_legacy`, `bonus_minutes` und
  Warnflags für das neue Tagesaggregat zurückgesetzt. Laufende Sessions bleiben
  bestehen und werden dem neuen Tag ab Mitternacht gutgeschrieben.

### Parallele Sitzungen

Beispiel: Profil `mia` startet Spiel A um 10:00 und Spiel B um 10:10; A endet
10:30, B endet 10:40.

- Falsch (Summe): 60 Minuten.
- Richtig (Vereinigung): 40 Minuten.

Umsetzung:

- Pro Profil werden die Intervalle `[start, end)` der Sessions gesammelt.
- Beim Aggregieren werden überlappende Intervalle verschmolzen.
- Die laufende Session zählt bis `now` (bzw. bis zum letzten bestätigten
  Lebenszeichen).
- Bei `Unknown` wird das Intervall konservativ fortgeführt, bis eine
  Entscheidung getroffen ist (Policy, siehe unten).

### Budget und Zusatzminuten

- Gesamtbudget = `daily_limit_minutes + bonus_minutes`.
- `used_actual` wird nicht gekappt.
- `remaining = max(0, total - used_actual)`.
- Eine Elternfreigabe erhöht `bonus_minutes`; dadurch steigt `total`, die
  bereits verbrauchte Zeit bleibt erhalten.
- `reset_extra_minutes` setzt `bonus_minutes` auf 0; `used_actual` bleibt.
- Bei einem Bonus nach Überschreitung kann die Restzeit wieder positiv werden;
  Warnflags werden neu bewaffnet.

### Suspend/Resume

- Während des Suspend wird keine Spielzeit gezählt.
- `CLOCK_MONOTONIC` pausiert; die Differenz nach Resume ist daher korrekt.
- Zusätzlich wird das Suspend-Intervall explizit geloggt (logind oder
  Boottime/Monotonic-Differenz).
- Wenn eine Session während des Suspend endet (z. B. Prozess stirbt), wird das
  Ende beim Resume erkannt; die Schlafzeit wird nicht angerechnet.

### Neustart, Absturz, Wiederherstellung

| Fall | Verhalten |
|---|---|
| CapyDeck-Neustart, Session läuft weiter | Handle verifizieren; Zeit ab `last_accounted_at` weiterzählen |
| CapyDeck-Absturz, Session läuft weiter | Beim Start Handle verifizieren; Lücke zwischen letztem Stand und Start als „uncertain“ markieren |
| Systemneustart (`boot_id` anders) | Keine Session kann weiterlaufen; Ende beim letzten bestätigten Stand oder Shutdown-Zeitpunkt |
| Session-Handle nicht verifizierbar | `Unknown`/`TrackingLost`; Policy anwenden, nicht stillschweigend pausieren |
| Prozess tot, Unit/Scope tot | `Ended`; Endzeit aus letztem bestätigten Lebenszeichen + Settle-Frist |
| Supervisor weg, Unit aktiv | `TrackingLost`; über Unit/cgroup/Fenster weiter beobachten |
| unit cgroup leer, Spiel tot | `Ended`; Supervisor-/Unit-Endzeit + Settle |

**Lückenpolitik:** Wenn CapyDeck nicht lief, aber das Spiel nachweislich
weiterlief (z. B. Direct-Unit noch aktiv), wird die Zeit ab
`last_accounted_at` bis zur Wiederaufnahme als Spielzeit gezählt. Ist der
Beweis nicht möglich, wird die Lücke als `uncertain` markiert und je nach
Policy konservativ (weiterzählen mit Obergrenze) oder gar nicht gezählt. Die
Entscheidung muss konfigurierbar und geloggt sein.

### Supervisor-Ereignisse und TrackingLost

Der Supervisor liefert die primären Zeitanker:

- `main_exit`/`exit` für den Spielprozess,
- `cgroup_empty` für die bekannte Gruppe,
- `disappeared` für von Wrappern gereapte Enkel (ohne Exit-Status).

Regeln:

- Die Zeitmessung darf **nicht** ausschließlich an der Supervisor-Lebensdauer
  hängen.
- Stirbt der Supervisor, bleibt die systemd-Unit mit `ExitType=cgroup` aktiv.
  Die Session wird `TrackingLost`; die Zeit läuft konservativ weiter, bis
  Unit/cgroup/Fenster einen belastbaren Endebeleg liefern.
- Ein Supervisor-Timeout ist kein bestätigtes Spielende.
- Ein verbleibender Hilfsprozess ist kein Beweis, dass das Spiel weiterläuft;
  `main_exit` und `cgroup_empty` getrennt bewerten.
- Nach einem UI-Neustart wird über `sessions.json` und die Unit
  reattachiert; die Lücke wird als `uncertain` markiert, nicht stillschweigend
  als Spielzeit oder Pause verbucht.
- cgroup-Escape nach Supervisor-Crash: nicht über die alte Unit auffindbar →
  `TrackingLost`; konservative Frist/Policy, klar loggen.

### Kompatibilitätsmodus (nicht zuverlässig überwachbare Spiele)

Für Generic GIO und andere nicht owning-fähige Backends:

- Session wird als `RunningUntracked` geführt.
- Zeit wird konservativ gezählt, solange ein plausibles Lebenszeichen
  (Fenster/Prozess) existiert.
- Es gibt eine dokumentierte Obergrenze/Grace-Policy, damit ein verlorenes
  Fenster nicht zu einer unbegrenzten Gutschrift führt.
- Optionaler **strikter Modus**: Kinderprofile dürfen nicht ausreichend
  überwachbare Spiele nicht starten. Der Modus ist standardmäßig aus.
- `Unknown`/`TrackingLost` dürfen nicht zu „Zeit pausiert“ führen. Entweder
  weiterzählen mit Warnung oder Start blockieren.

### Shadow Mode

Phase 1 der Einführung:

- Alte und neue Messung laufen parallel.
- `used_seconds_legacy` und `used_seconds_actual` werden getrennt gespeichert.
- Die UI zeigt weiterhin die alte Messung (oder einen klar gekennzeichneten
  Vergleich).
- Es werden Differenzen, Ausreißer und unklare Fälle geloggt.
- Erst nach einer definierten Beobachtungsphase und Bazzite-Abnahme wird die
  neue Messung führend.

### Ein oder mehrere Spiele pro Kinderprofil?

Empfehlung: **optional**, nicht Standard.

- Standard: parallele Sessions sind erlaubt; die Zeit wird als Vereinigung
  gezählt.
- Option „nur eine Spielsitzung gleichzeitig“ pro Kinderprofil:
  - Zweiter Start wird mit verständlicher Meldung blockiert.
  - Sinnvoll für jüngere Kinder und einfachere Aufsicht.
  - Keine Auswirkung auf Admin-/Erwachsenenprofile.
- Die Option muss mit dem strikten Modus kombinierbar sein.

### Datenspeicherung

- `timekeeping.json`: Tagesaggregate, Schema 2, atomar.
- `sessions.json`: Live-Sitzungen, Schema 1, atomar.
- Beim Laden:
  - korrupte Datei → Backup, Fehlerdialog, keine stille Überschreibung.
  - höhere Schema-Version → nicht überschreiben.
  - Migration von Schema 1 → 2: `used_seconds` als `used_seconds_legacy`
    übernehmen, `used_seconds_actual` zunächst gleich setzen oder aus
    Sessions rekonstruieren; Backup anlegen.
- Da es aktuell keine echten Benutzer gibt, ist die Migration unkritisch,
  muss aber trotzdem korrekt und getestet sein.

## Nachgewiesene Erkenntnisse (Phase II/III)

- Session-Ende ist für Steam/Prism/Ryujinx mit den beschriebenen Handles
  erkennbar; die Zeit kann an die Prozess-/Unit-Lebensdauer gekoppelt werden.
- Home/Minimieren beendet die Session nicht (Steam, Ryujinx).
- Fenster können nach einem Absturz kurzzeitig weiterleben; Zeit darf nicht am
  Fenster allein hängen.
- Ryujinx-Bestätigungsdialog, ignorierte Close-Requests (Stray) und
  blockierendes `systemctl stop` können ein sofortiges Ende verhindern; die
  Zeitmessung braucht einen `Stopping`/`StopUnconfirmed`-Zustand.
- Prism-Java-Ende ist unabhängig vom Launcher-Ende erkennbar.
- Generic GIO hat keinen zuverlässigen Lebensdauer-Anker.
- Supervisor-Ereignisse liefern `main_exit`/`exit`/`cgroup_empty`; nach
  Supervisor-Crash bleibt die Unit mit `ExitType=cgroup` aktiv und die Zeit
  muss über `TrackingLost` konservativ weiterlaufen.
- Von Wrappern gereapte Enkel liefern `disappeared` statt Exit-Status; die
  Sitzungsdauer ist trotzdem erkennbar.

## Technische Annahmen

- Monotonic/Boottime verhalten sich auf Bazzite wie auf Manjaro.
- logind ist verfügbar; falls nicht, wird die Suspend-Erkennung über
  Boottime/Monotonic-Differenz als Fallback genutzt.
- Die lokale Zeitzone ist während einer Sitzung stabil; Zeitzonenwechsel werden
  trotzdem defensiv behandelt.
- Die Session-Handles bleiben über kurze Lücken (CapyDeck-Neustart) gültig.

## Offene Forschungsfragen

- Genaues Suspend-/Resume-Verhalten auf Bazzite/KDE/Wayland.
- Verhalten bei CapyDeck-Absturz während einer Steam-/Prism-Session.
- Zeitmessung bei einem Spiel, das ohne Fenster läuft (z. B. Server/Headless).
- Sehr lange Sessions über mehrere Tage.
- Zeitänderung durch manuelle Uhrumstellung.
- Zusammenspiel mit dem bestehenden 30-s-Speicherintervall und atomaren
  Schreibvorgängen.
- Shadow-Mode-Vergleich über echte Spielverläufe.

## Abnahmekriterien für die Ablösung

Die neue Messung darf die alte erst ersetzen, wenn:

1. Direct/Managed, Steam nativ und Prism auf Bazzite/KDE/Wayland mindestens
   einen vollständigen Spielverlauf korrekt messen.
2. Home/Minimieren/Fokuswechsel die Zeit nicht pausieren.
3. Parallele Sessions desselben Profils nicht doppelt zählen.
4. Suspend/Resume keine Schlafzeit anrechnet.
5. Neustart/Absturz zu einem nachvollziehbaren, geloggten Zustand führen.
6. Unbekannte Backends nicht stillschweigend die Zeit aussetzen.
7. Shadow-Vergleich dokumentiert und akzeptiert ist.
8. `cargo test`, `cargo clippy`, `cargo fmt` und die Migrationstests grün sind.
