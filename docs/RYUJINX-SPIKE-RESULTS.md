# Ryujinx-Spike – Ergebnisse

Stand: Phase II, 26. November 2026
Getestet auf Manjaro Linux / KDE Plasma 6.8.2 / Wayland.

Umgebung:

- Ryujinx **1.3.3**, .NET **9.0.9**
- Installation: `~/dat2/publish/Ryujinx`
- Spiel: **Mario Kart 8 Deluxe v1.0.0**
  (`0100152000022000`), NSP-Datei auf externem Datenträger
- Kein Controller angeschlossen
- Der ursprüngliche Arbeitsauftrag liegt unverändert in
  [`RYUJINX_SPIKE.md`](RYUJINX_SPIKE.md).

## Phase-I-Ergebnis (kurz)

Der erste Spike startete nur die Emulator-Oberfläche, weil keine
`.nsp`/`.xci`-Datei gefunden wurde. Beide Varianten – transiente Service-Unit
und Scope – starteten Ryujinx direkt. Die Unit lief in einer eigenen cgroup;
`MainPID` war die Ryujinx-PID; das KWin-Fenster hatte dieselbe PID. Ein
KWin-Script konnte das Fenster über die exakte PID schließen. Ein Scope-Stop
benötigt `<unit>.scope`; `systemctl --user stop <unit>` sucht eine `.service`
und schlug fehl.

## Phase-II-Test 1: Direktstart eines echten Spiels

Start über eine transiente systemd-User-Service-Unit:

```bash
systemd-run --user --unit=capydeck-ryujinx-test --collect \
  --property=KillMode=control-group \
  --working-directory=/home/chris/dat2/publish \
  /home/chris/dat2/publish/Ryujinx --fullscreen \
  "/run/media/chris/Ext1/ryu/Mario Kart 8 Deluxe ROM Base.NSP"
```

Beobachtung:

- Unit: `capydeck-ryujinx-test.service`
- Typ: `Type=simple`
- `MainPID` = `1499239` = Ryujinx-Prozess = KWin-Fenster-PID
- `ControlGroup`:
  `/user.slice/user-1000.slice/user@1000.service/app.slice/capydeck-ryujinx-test.service`
- Fenster nach ca. 13 s.
- Caption nach dem Laden:
  `Ryujinx 1.3.3 - Mario Kart 8 Deluxe v1.0.0 (0100152000022000) (32-bit)`
- Ryujinx-Log meldete wiederholt:
  `Hid Remap: No matching controllers found.`

Damit ist die Service-Unit ein stabiler Identitäts- und Stop-Anker:
`MainPID`, cgroup und Fenster-PID stimmen überein.

## Phase-II-Test 2: Vollbild

### `--fullscreen` allein

Bei `start_fullscreen=false` in der Ryujinx-Konfiguration führte
`--fullscreen` **nicht** zu einem KWin-Vollbildfenster:

```text
fs=false
geom=1732,1027 1232x720
```

Das Fenster war normal und nicht fokussiert. `--fullscreen` allein ist in
dieser Version/Sitzung kein verlässlicher Vollbild-Schalter.

### `start_fullscreen=true` + `remember_window_state=false`

Mit einer verwalteten Kopie der Konfiguration:

```json
{
  "start_fullscreen": true,
  "remember_window_state": false
}
```

wurde das Fenster tatsächlich fullscreen:

```text
fs=true
geom=0,0 1080x1920
```

Auffällig: Das Vollbild landete auf dem linken, **rotierten** Monitor
(DP-2, 1080x1920), nicht auf dem Hauptmonitor DP-3 (1920x1080). Die
Monitorwahl ist ein Compositor-/Fensterplatzierungsthema und sollte nicht als
CouchDeck-Aufgabe betrachtet werden. Auf einem typischen einzelnen
Wohnzimmerbildschirm ist das unkritisch; bei Multi-Monitor-Setups muss die
Zielhardware getestet werden.

## Phase-II-Test 3: Reguläres Beenden und Bestätigungsdialog

Ryujinx hat standardmäßig:

```json
"show_confirm_exit": true
```

Ein KWin-`closeWindow()` auf das Vollbildfenster öffnete statt des Endes einen
Dialog:

```text
ContentDialogOverlayWindow
```

- Die Unit blieb aktiv.
- Das Schließen des Dialogfensters über dieselbe PID **brach den Dialog ab**;
  das Spiel lief weiter.
- Ohne Eingabeinjektion (Tastatur/Controller) konnte der Dialog nicht bestätigt
  werden.

**Wichtige Erkenntnis:** Ein `closeWindow()` ist kein garantiertes reguläres
Beenden. Das Backend muss erkennen, dass eine Bestätigung aussteht, und darf
„Schließen angefordert“ nicht mit „Spiel beendet“ gleichsetzen.

Im ersten Test (Fenstermodus, nicht Vollbild) wurde kein Dialog beobachtet; die
Unit endete nach ca. 10 s. Der Dialog trat im Vollbild auf. Ob das ausschließlich
am Vollbildmodus oder am damaligen Spielzustand lag, ist nicht abschließend
geklärt. Deshalb bleibt die verwaltete Konfiguration mit
`show_confirm_exit=false` für automatisierte Direct/Managed-Sessions Pflicht.

### Mit verwalteter Konfiguration: `show_confirm_exit=false`

In einer verwalteten Ryujinx-Konfiguration wurde gesetzt:

```json
"show_confirm_exit": false
```

Danach beendete `closeWindow()` das Spiel regulär:

- Schließen: `1795729686.051`
- Unit inaktiv: `1795729688.098`
- **ca. 2,0 s**
- Fenster und Prozess verschwanden; keine Rückfrage.

Das ist der empfohlene Weg für ein Direct/Managed-Backend: Ryujinx mit einem
CouchDeck-verwalteten Root/Config starten, in dem `show_confirm_exit=false`
gesetzt ist. Die Benutzerkonfiguration muss dabei nicht verändert werden.

### Fenstermodus (erster Test)

Der erste Test im Fenstermodus beendete die Unit nach dem Schließen in ca.
10 s. Ein Bestätigungsdialog wurde dabei nicht beobachtet. Die längere Dauer
ist plausibel durch Ryujinx-eigene Aufräumarbeiten (Audio, NV-Kanäle) und zeigt,
dass das Unit-Ende dem Fensterschließen nachlaufen kann.

## Phase-II-Test 4: Minimieren und Wiederherstellen

Bei laufendem Vollbildspiel:

- KWin-Script setzte `minimized=true`.
- Der Ryujinx-Prozess blieb am Leben.
- `minimized=false` + `workspace.activeWindow` stellte das Fenster wieder her.
- `fullScreen=true` blieb erhalten.
- Die Service-Unit blieb aktiv.

Das ist die Grundlage für „Home-Rückkehr und Spielfortsetzung“ bei
Direct/Managed: Minimieren beendet die Session nicht.

## Phase-II-Test 5: Notfall-/Unit-Stop

### Service-Unit

`systemctl --user stop capydeck-ryujinx-test` beendete die Unit in ca. 1 s;
`MainPID` war tot, keine Restprozesse.

In einem anderen Lauf blockierte `systemctl --user stop` länger als 60 s, weil
Ryujinx auf `SIGTERM` nicht zeitnah reagierte (der Test-Prozess war durch eine
vorherige Manipulation des temporären Roots zusätzlich gestört). Erst

```bash
systemctl --user kill --signal=KILL capydeck-ryujinx-test
```

beendete den Prozess zuverlässig.

Konsequenz:

- Ein Notfall-Stop muss **nicht blockierend** und mit kurzem Timeout laufen.
- Nach Timeout ist ein `SIGKILL` auf die eigene, verifizierte Unit zulässig.
- `KillMode=control-group` sorgt dafür, dass Kindprozesse mit beendet werden.
- `systemctl stop` darf niemals den UI-Thread blockieren.

### Scope-Unit

Ein separater Scope-Test mit einem einfachen Prozessbaum bestätigte:

```text
systemctl --user stop capydeck-scope-test          → Fehler (sucht .service)
systemctl --user stop capydeck-scope-test.scope    → Erfolg
```

Eine Scope hat **keinen `MainPID`** in `systemctl show`; die Prozesse müssen
über `ControlGroup`/`cgroup.procs` ermittelt werden.

## Phase-II-Test 6: Prozess-, Unit- und Fensterende

- `MainPID` der Service-Unit war die Ryujinx-PID.
- Das Fenster hatte dieselbe PID.
- Nach regulärem Close (mit `show_confirm_exit=false`) wurde die Unit in ca.
  2 s inaktiv.
- Nach Notfall-Stop wurde die Unit in ca. 1 s inaktiv.
- Ein Fenster kann nach einem Absturz kurzzeitig in der KWin-Liste verbleiben;
  Prozess-/Unit-Ende ist der primäre Endanker.

## Phase-II-Test 7: Save-Daten

Mario Kart 8 Deluxe lud vorhandene Save-Daten und schrieb sie beim Start/Menü
(ca. `1795729313`, `1795729396`). Ein vollständiger interaktiver Spielverlauf
mit Streckenauswahl, Speichern und erneutem Laden wurde **nicht** durchgeführt,
weil keine Eingabeinjektion zur Verfügung stand.

Damit ist belegt:

- Ryujinx liest und schreibt Save-Daten im Spielstart-/Menüpfad.
- Ein vollständiger Save/Load-Zyklus bleibt offen.

## Phase-II-Test 8: Native Testanwendung (Direct/Managed-Robustheit)

Zusätzlich wurde eine kleine GTK4-Testanwendung
([`spikes/capydeck-testapp.py`](spikes/capydeck-testapp.py)) als
Direct/Managed-Fall verwendet.

### Kindprozess und Absturz

- Service-Unit `capydeck-testapp.service`, `MainPID` = Python-/Fenster-PID.
- Die App startete ein Kind `sleep 300` in derselben cgroup.
- `SIGKILL` auf die `MainPID`:
  - Unit inaktiv nach ca. 1 s.
  - Kindprozess wurde durch `KillMode=control-group` mit beendet.
  - Keine Restprozesse.
  - Das Fenster blieb kurzzeitig in der KWin-Liste (stale window).

### Fenster neu erstellen

- Modus `recreate`: Fenster #1 wurde nach 3 s zerstört, Fenster #2 nach 2 s
  neu erstellt.
- Prozess-PID und Unit blieben identisch.
- Die Session blieb aktiv, obwohl das Fenster gewechselt wurde.
- Schließen von Fenster #2 beendete die App und die Unit in ca. 2 s.

Das bestätigt: **Session-Identität darf nicht an einem einzelnen Fenster
hängen.** Prozess/Unit/cgroup sind die stabilen Anker; Fenster sind
Beobachtungsobjekte.

## Nebenwirkungen

Während der Direct/Managed-Tests wurden keine fremden Units/Scopes gestoppt.
Steam, Prism und andere Anwendungen blieben unberührt. Die Tests mit
verwalteter Konfiguration liefen in einem temporären Root; der erste Realtest
nutzte die bestehende Ryujinx-Konfiguration und aktualisierte normale
Laufzeitdaten (Fensterzustand, Save, Logs). Der ursprüngliche Fensterzustand
wurde anschließend wiederhergestellt.

## Nachgewiesene Erkenntnisse

1. Ryujinx lässt sich mit einem echten Spiel direkt über eine transiente
   Service-Unit starten.
2. `MainPID`, cgroup und Fenster-PID sind identisch; die Unit ist ein stabiler
   Stop-Anker.
3. `--fullscreen` allein genügt in dieser Konfiguration nicht;
   `start_fullscreen=true` + `remember_window_state=false` erzeugt ein
   KWin-Vollbildfenster.
4. Ryujinx' Standard-Bestätigungsdialog verhindert ein automatisches
   Fensterschließen. `show_confirm_exit=false` in einer verwalteten
   Konfiguration löst das.
5. Reguläres Schließen mit deaktivierter Rückfrage beendet die Unit in ca. 2 s.
6. `systemctl stop` kann bei blockierendem Prozess hängen; Notfall-Stop muss
   nicht blockierend mit Timeout + `SIGKILL`-Fallback erfolgen.
7. Eine Scope hat keinen `MainPID`; Stop nur mit `.scope`-Suffix.
8. Minimieren/Wiederherstellen beendet die Session nicht.
9. Kindprozesse werden bei Unit-Ende über `KillMode=control-group` mit beendet.
10. Fensterneuerstellung beendet die Session nicht; Fenster sind keine
    Identität.
11. Save-Daten werden geladen/geschrieben; ein vollständiger interaktiver
    Save/Load-Zyklus steht aus.

## Technische Annahmen

- Ryujinx-Konfigurationsschlüssel (`start_fullscreen`, `remember_window_state`,
  `show_confirm_exit`) sind versionsabhängig. CouchDeck sollte die verwaltete
  Konfiguration versioniert erzeugen und bei Ryujinx-Updates prüfen.
- Die Monitorwahl im Vollbild ist compositorabhängig.
- Die gemessenen Start-/Endzeiten sind keine festen Zusagen.
- `systemctl stop`-Blockaden können durch Prozesszustand, Signalbehandlung oder
  externen Dateizugriff entstehen; der Timeout-Fallback ist trotzdem Pflicht.

## Offene Forschungsfragen

- Vollständiger Spielverlauf mit Controller, Speichern und erneutem Laden.
- Ryujinx mit `Ryujinx.Headless.SDL2` (nicht in dieser Installation vorhanden).
- Weitere native Spiele/Emulatoren mit Kindprozessen und eigenen
  Bestätigungsdialogen.
- Verhalten auf Bazzite KDE/Wayland.
- GNOME/XFCE-Fensterschließung.
- Multi-Monitor-Vollbild und Monitorauswahl.
- Controller-Hotplug während einer Session.

## Capability-Bewertung

| Fähigkeit | Status | Begründung |
|---|---|---|
| Start | ✅ experimentell | Service-Unit, strukturierte Argumente |
| Start erkennen | ✅ experimentell | MainPID + cgroup + Fenster |
| Session zuordnen | ✅ experimentell | Unit + InvocationID + cgroup + PID |
| Ende erkennen | ✅ experimentell | Unit inactive (kurze Nachlaufzeit) |
| Fokus | ⚠️ nur KDE/KWin getestet | Compositor-abhängig |
| Regulär beenden | ✅ nur mit verwalteter Config | `show_confirm_exit=false` |
| Notfall-Stop | ✅ experimentell | Unit-Stop + SIGKILL-Fallback |

**Qualitätsstufe: Experimentell.** Ein Spiel, eine Ryujinx-Version, kein
Controller, keine Bazzite-Abnahme.
