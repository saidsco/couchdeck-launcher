# Fensteraktivierung und Rückkehr (0.4.5)

CouchDeck fordert den Fokus **nicht** automatisch zurück, wenn ein Spiel, Steam,
ein vorgeschalteter Launcher (z. B. Larian), Heroic, Lutris, Wine, Prism oder ein
Emulator ein Fenster öffnet. Der Startauftrag bleibt ein Startauftrag; ein
erfolgreiches `gio launch` bedeutet nicht, dass ein Spiel läuft oder beendet ist.

## Architektur

Die laufende UI besitzt einen lokalen Server:

- Unix-Domain-Socket in `$XDG_RUNTIME_DIR/couchdeck-activate.sock`
  (Fallback: `$XDG_STATE_HOME/couchdeck/activate.sock`).
- Socket-Modus `0600`; zusätzlich prüft der Server `SO_PEERCRED` und akzeptiert
  nur die eigene lokale UID.
- Der Server nimmt maximal kleine Anfragen an, sendet ein Ereignis an den
  UI-Thread und ruft `egui::Context::request_repaint()` auf. Die UI blockiert
  dadurch nicht.
- Veraltete Sockets nach einem Absturz werden beim nächsten Start ersetzt.
- Der bestehende Single-Instance-`flock` bleibt die Autorität dafür, ob eine
  Instanz läuft.

`couchdeck --activate` ist ein kurzer Client:

1. verbindet sich mit dem Socket,
2. sendet `ACTIVATE 1`,
3. wartet auf `OK REQUESTED`,
4. beendet sich ohne zweite UI.

Die Antwort bedeutet nur **„Anforderung angenommen“**. Ob der Compositor den
Fokus tatsächlich gewährt, kann der Client nicht garantieren.

## Aktivierungsverhalten

`request_activation()` in der laufenden UI:

- Ist CouchDeck bereits fokussiert und sichtbar: keine Aktion.
- Ist es minimiert: `Minimized(false)`, danach `Focus` und ggf. Fullscreen.
- Ist es sichtbar, aber unfokussiert: `Focus` und `RequestUserAttention`
  (Best-Effort, insbesondere unter Wayland).
- Lief CouchDeck vor dem Minimieren im Fullscreen: Fullscreen wird
  wiederhergestellt.
- Wurde CouchDeck mit `--windowed` gestartet oder hat der Benutzer Fullscreen
  manuell verlassen: Es bleibt im Windowed-Modus.
- Keine wiederholten aggressiven Fokusversuche, kein Beenden oder Verschieben
  anderer Fenster, keine Prozessmanipulation.
- Die Wiederherstellung wartet höchstens kurz (2 s) auf einen Minimierungswechsel
  und verwirft die Anforderung dann mit Logeintrag, statt dauerhaft zu pollen.
- Keine Menü-, PIN- oder Profilaktionen.

## X11 und Wayland

| Umgebung | Aktivierung |
|---|---|
| X11 | `ViewportCommand::Focus` funktioniert in der Regel, wenn der Window-Manager dies erlaubt. |
| Wayland | Der Compositor entscheidet. `Focus` kann ignoriert werden; `RequestUserAttention` kann die Taskleiste hervorheben, aber nicht garantieren, dass das Fenster den Fokus erhält. |
| KDE Plasma Wayland | Compositor-Regeln und die aktuelle Sitzung entscheiden; eine manuelle Taskleisten-/Shortcut-Aktivierung ist Best-Effort. |
| GNOME Wayland | Fokusübergabe an fremde Prozesse ist stark eingeschränkt. Shortcut/Taskleiste bleiben der zuverlässigste Weg. |
| XFCE/X11 | Fokus funktioniert üblicherweise über den Window-Manager. |

Es werden keine KDE-spezifischen APIs zur allgemeinen Voraussetzung gemacht.
`--diagnose-window` protokolliert Fokus-, Minimierungs- und Fullscreen-Wechsel
in `$XDG_STATE_HOME/couchdeck/couchdeck.log`.

## Desktop-Shortcuts

Der Shortcut ist eine **manuelle Benutzerentscheidung**. CouchDeck registriert
keine globale Tastenkombination automatisch.

### KDE Plasma

Systemeinstellungen → Kurzbefehle → Benutzerdefinierte Kurzbefehle → Neuer
globaler Kurzbefehl:

```text
/home/USER/.local/bin/couchdeck --activate
```

Alternativ kann ein benutzerdefinierter KDE-Kurzbefehl über die
Systemeinstellungen angelegt werden. Der vollständige Pfad ist zuverlässiger
als ein PATH-Aufruf.

### GNOME

Einstellungen → Tastatur → Tastenkürzel → Benutzerdefiniert:

```text
Name:    CouchDeck aktivieren
Befehl:  /home/USER/.local/bin/couchdeck --activate
Tasten:  z. B. Super+Alt+C
```

### XFCE

Einstellungen → Tastatur → Tastenkürzel → Hinzufügen:

```text
Befehl: /home/USER/.local/bin/couchdeck --activate
```

Für AppImage-Nutzer zeigt der Pfad auf die AppImage-Datei, nicht auf einen
temporären Mount:

```text
/home/USER/Applications/CouchDeck-0.4.5-x86_64.AppImage --activate
```

## Steam Input (Vorbereitung für 0.5)

Steam Input kann eine Controller-Taste auf eine Desktop-Aktion abbilden, z. B.
über ein Steam-Input-Layout mit einem „Guide“-/Home-Button und einem
Desktop-Shortcut. In 0.4.5 gibt es **keine** globale Controller-Home-Funktion
und keine zusätzliche Controllerüberwachung im Hintergrund. Die Tastenkombination
wird ausschließlich vom Benutzer in Steam Input eingerichtet.

## Diagnose

```bash
couchdeck --diagnose-window
couchdeck --activate
```

Relevante Logzeilen:

- `activation: Anforderung empfangen ...`
- `activation: Minimierten Zustand erkannt ...`
- `activation: Fokus angefordert (Compositor kann die Anforderung ablehnen).`
- `window: Fokus gewonnen/verloren ...`
- `launch: gio launch ...`

Fehlerfälle:

- keine laufende Instanz → Exit-Code ungleich 0, verständliche Meldung
- Socket vorhanden, aber nicht erreichbar → keine falsche Erfolgsmeldung
- veralteter Socket nach Crash → wird beim nächsten Start/Client-Aufruf ersetzt
- anderer lokaler Benutzer → Socket-Permissions/`SO_PEERCRED` verweigern die
  Verbindung

## Manueller Testablauf

1. CouchDeck starten, `--activate` aufrufen (sollte keine sichtbare Störung
   erzeugen).
2. CouchDeck minimieren, `--activate` aufrufen; Fenster soll zurückkehren.
3. CouchDeck im Fullscreen minimieren, `--activate`; Fullscreen soll
   zurückkehren.
4. CouchDeck mit `--windowed` starten, minimieren, `--activate`; Windowed soll
   bleiben.
5. Zweimal schnell `--activate` aufrufen; es darf keine zweite Instanz und
   keinen instabilen Zustand geben.
6. CouchDeck beenden, `--activate`; Exit-Code ungleich 0 mit verständlicher
   Meldung.
7. Steam-Spiel starten, Steam-Fenster/Launcher zulassen, danach manuell
   `--activate`; CouchDeck darf den Fokus nur auf Anforderung zurückholen.

Die Desktop-Matrix und offene Hardwaretests stehen in
[`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md).
