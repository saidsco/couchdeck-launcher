# Prism/Minecraft-Spike – Ergebnisse

Stand: Phase II, 26. November 2026
Getestet auf Manjaro Linux / KDE Plasma 6.8.2 / Wayland.

Umgebung:

- Prism Launcher **11.1.1** als System-Flatpak
  (`org.prismlauncher.PrismLauncher`)
- Instanz `26.3`, UUID `9a6ec2f8fa87498180bfcc30422fc14d`, Fabric,
  Minecraft 26.3
- Java-Runtime: Microsoft Java **25.0.1** (Prism-eigene Runtime)
- Für den Mehrinstanztest wurde eine temporäre Kopie
  `capydeck-test-2` (UUID `c114a4779c74402f9e8b35e27177c9e4`) angelegt und
  anschließend wieder entfernt.

## Phase-I-Ergebnis (kurz)

`flatpak run org.prismlauncher.PrismLauncher --launch <INSTANCE_ID>` startete
Prism und die Minecraft-Java-Instanz. `flatpak ps` lieferte die Flatpak-Instanz;
das Minecraft-Fenster hatte die Java-PID. Ein äußerer `systemd-run`-Scope
kontrollierte die Flatpak-Prozesse nicht, weil Flatpak eine eigene
`app-flatpak-…scope` anlegt. Reguläres Fensterschließen über die exakte
Java-PID funktionierte; `flatpak kill <instance>` beendete die gesamte
Flatpak-Instanz.

## Phase-II-Test 1: Prism-UI bereits geöffnet

### Start

Prism-UI wurde mit

```bash
flatpak run org.prismlauncher.PrismLauncher
```

gestartet. Beobachtung:

- Flatpak-Instanz: `3465285812`
- Flatpak-Client-PID: `1493515`
- Scope:
  ```text
  app-flatpak-org.prismlauncher.PrismLauncher-3465285812.scope
  ```
- Prism-Fenster-PID: `1493528` (`prismrun`)
- Fenster-Caption: `Prism Launcher 11.1.1`
- App-ID/Ressourcenklasse: `org.prismlauncher.PrismLauncher`

Hinweis aus Phase III: Ohne bereits laufende Prism-UI kann
`flatpak run ... --launch` im Vordergrund bis zum Spielende blockieren. Der
Launch-Client ist langlebig; CouchDeck darf ihn nicht synchron im UI-Thread
ausführen.

Danach wurde bei **bereits geöffneter UI** gestartet:

```bash
flatpak run org.prismlauncher.PrismLauncher \
  --launch 9a6ec2f8fa87498180bfcc30422fc14d
```

Ergebnis:

- `flatpak run` kehrte mit Exit-Code 0 zurück.
- Es wurde **keine zweite Flatpak-Instanz** dauerhaft angelegt; der Auftrag
  wurde an die bestehende Instanz weitergegeben.
- Neue Java-PID: `1493757`, Kind von `prismrun` (`1493528`).
- Minecraft-Fenster-PID: `1493757`.
- Caption: `Minecraft* 26.3`.
- App-ID: `org.prismlauncher.PrismLauncher`.
- Dieselbe Flatpak-Scope wie die UI.

### Reguläres Spielende bei geöffneter UI

KWin schloss nur das Minecraft-Fenster über die exakte Java-PID:

```javascript
if (window.pid === targetPid) {
    window.closeWindow();
}
```

Ergebnis:

- Schließen: `1795728895.409`
- Java-Prozess beendet: `1795728897.448` (**ca. 2,0 s**)
- Prism-UI blieb geöffnet.
- Flatpak-Instanz und Scope blieben aktiv.
- Die Logzeile `Stopping!` bestätigte das reguläre Spielende.

Das ist der gewünschte Fall: **Spielende ist unabhängig vom Launcher-Ende
erkennbar.**

## Phase-II-Test 2: Zweiter Start derselben Instanz

Bei bereits laufendem Minecraft wurde dieselbe Instanz erneut gestartet:

```bash
flatpak run org.prismlauncher.PrismLauncher \
  --launch 9a6ec2f8fa87498180bfcc30422fc14d
```

Ergebnis:

- Exit-Code 0.
- **Kein zweiter Java-Prozess.**
- Nur ein Java-Kind von `prismrun`.
- Prism meldete:
  ```text
  QLocalSocket::setServerName() called while not in unconnected state
  ```

Damit gilt: **Ein erfolgreicher `--launch`-Aufruf ist kein Beweis, dass eine
neue Spielsitzung gestartet wurde.** Prism verhindert (mindestens für dieselbe
Instanz) einen zweiten parallelen Start.

## Phase-II-Test 3: Zwei unterschiedliche Instanzen gleichzeitig

Die Instanz `26.3` wurde nach `capydeck-test-2` kopiert, mit neuer UUID und
neuem Namen. Prism-UI lief bereits.

Start beider Instanzen:

```bash
flatpak run org.prismlauncher.PrismLauncher --launch 9a6ec2f8...
flatpak run org.prismlauncher.PrismLauncher --launch c114a477...
```

Ergebnis:

- Java-Prozess 1: `1496974`, Instanz `26.3`
- Java-Prozess 2: `1497182`, Instanz `capydeck-test-2`
- Beide Java-Prozesse waren Kinder desselben `prismrun` (`1496846`).
- Beide lagen in **derselben Flatpak-Scope**
  `app-flatpak-org.prismlauncher.PrismLauncher-2649964544.scope`.
- Zwei Minecraft-Fenster, beide mit Caption `Minecraft* 26.3`.
- `flatpak ps` zeigte nur eine dauerhafte Flatpak-Instanz. Während des zweiten
  Starts erschien kurzzeitig eine zweite transiente Flatpak-Instanz
  (`3624498629`, PID `1497146`), die nach dem Weiterreichen wieder verschwand.

### Zuordnung Java-PID → Prism-Instanz

Die Fenster-Caption war **nicht** eindeutig (beide `Minecraft* 26.3`). Die
Zuordnung gelang über die Java-Kommandozeile:

```text
-Djava.library.path=.../instances/26.3/natives
-Djava.library.path=.../instances/capydeck-test-2/natives
```

Damit ist die Instance-ID bzw. der Instanzordner im Java-Prozess enthalten und
kann mit der bekannten Prism-Instanz abgeglichen werden.

### Unabhängiges Beenden

Das Fenster der Instanz `26.3` wurde über die exakte Java-PID `1496974`
geschlossen:

- Java `1496974` beendet nach ca. 2 s.
- Java `1497182` (`capydeck-test-2`) lief weiter.
- Prism-UI blieb geöffnet.
- Flatpak-Instanz und Scope blieben aktiv.

Damit sind mehrere unterschiedliche Minecraft-Instanzen parallel möglich und
unabhängig beendbar, obwohl sie sich eine Flatpak-Scope teilen.

## Phase-II-Test 4: Absturz eines Minecraft-Prozesses

Java `1497182` wurde mit `SIGKILL` auf die exakte PID beendet.

Ergebnis:

- Java-Prozess nach ca. 1 s tot.
- Prism-UI blieb geöffnet.
- Flatpak-Instanz und Scope blieben aktiv.
- Das Minecraft-Fenster blieb zunächst noch ca. 2 s in der KWin-Liste und
  verschwand erst später (ca. 10 s). **Fensterexistenz ist kein Prozessbeweis.**
- Kein automatischer Neustart, kein Launcher-Absturz.

## Phase-II-Test 5: Launcher bleibt nach Spielende geöffnet

Bei geöffneter Prism-UI und geschlossenem Minecraft-Fenster:

- `flatpak ps` zeigte weiterhin die Prism-Instanz.
- Die Prism-Fenster blieben vorhanden.
- Nur der Java-Prozess und das Minecraft-Fenster verschwanden.

Das ist ein klarer Vorteil gegenüber dem `--launch`-Start ohne UI, bei dem Prism
nach dem Spielende ebenfalls beendet wurde (Phase I).

## Notfall-/App-Stop

`flatpak kill <instance>` beendet die gesamte Flatpak-Instanz. Bei zwei
parallelen Minecraft-Instanzen in derselben Flatpak-Instanz würden **beide**
Spiele und die Prism-UI beendet.

Damit gilt:

- `flatpak kill` ist nur als bestätigter Notfallweg zulässig, wenn
  nachweislich keine andere Spielsitzung und keine fremde Launcher-Sitzung in
  derselben Flatpak-Instanz läuft.
- Ein spielspezifischer Notfall-Stop existiert für Prism/Minecraft nicht.
- Der reguläre Weg bleibt das Schließen des Minecraft-Fensters über die exakte
  Java-PID.

## Nachgewiesene Erkenntnisse

1. Bei geöffneter Prism-UI wird `--launch` an die bestehende Flatpak-Instanz
   weitergereicht; es entsteht keine dauerhafte zweite Instanz.
2. Das Minecraft-Fenster hat die Java-PID; die Java-PID ist Kind von
   `prismrun`.
3. Schließen des Minecraft-Fensters beendet nur Java, nicht Prism.
4. Ein zweiter `--launch` derselben Instanz startet kein zweites Spiel
   (Exit-Code 0 trotzdem).
5. Zwei unterschiedliche Instanzen können parallel laufen; sie teilen sich
   dieselbe Flatpak-Scope.
6. Die Java-PID lässt sich über `-Djava.library.path=.../instances/<name>/natives`
   der Prism-Instanz zuordnen.
7. Fenstertitel sind zur Instanzunterscheidung unzuverlässig.
8. Ein Java-Absturz beendet nicht Prism; Fenster können kurzzeitig verwaist
   weiterleben.
9. `flatpak kill` ist ein Instanz-Stop, kein spielspezifischer Stop.

## Technische Annahmen

- Prism-Version, Java-Runtime und Startargumente können sich ändern. Die
  Instance-ID im Java-Kommando ist ein Implementierungsdetail, aber derzeit ein
  belastbarer Zuordnungsweg.
- `flatpak ps` zeigt dauerhafte Instanzen; transiente Weiterreichungsinstanzen
  können kurzzeitig erscheinen.
- Die Scope `app-flatpak-…` bleibt bestehen, solange die Prism-UI läuft. Das
  Scope-Ende ist daher **kein** Spielende-Signal, wenn die UI offen bleibt.

## Offene Forschungsfragen

- Vollständiger Spielverlauf mit Welt laden, speichern und erneut laden
  (interaktive Eingabe war im Spike nicht verfügbar).
- Minecraft-Absturz durch Spiel-/Mod-Fehler statt externem `SIGKILL`.
- Verhalten bei mehr als zwei Instanzen.
- Prism-UI geschlossen, aber Flatpak-Instanz läuft weiter (Hintergrundprozess).
- Flatpak-Berechtigungen und Portal-Zugriff auf Spielstände.
- GNOME/XFCE-Fensterschließung.
- Bazzite-Abnahme.

## Capability-Bewertung

| Fähigkeit | Status | Begründung |
|---|---|---|
| Start | ✅ experimentell | `flatpak run --launch` |
| Start erkennen | ✅ experimentell | Java-PID + Instanzpfad + Fenster |
| Session zuordnen | ✅ experimentell | Instanz-ID + Java-PID + Flatpak-Instanz |
| Ende erkennen | ✅ experimentell | Java-Ende, unabhängig von UI |
| Fokus | ⚠️ nur KDE/KWin getestet | Compositor-abhängig |
| Regulär beenden | ✅ experimentell | KWin `closeWindow` exakte Java-PID |
| Notfall-Stop | ⚠️ nur gesamte Flatpak-Instanz | keine spielspezifische Scope |

**Qualitätsstufe: Experimentell.** Eine Prism-Version, eine Java-Runtime, kein
vollständiger Spielverlauf, keine Bazzite-Abnahme.
