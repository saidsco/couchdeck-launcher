# Fensteraktivierung und Rückkehr (0.4.6)

CouchDeck fordert den Fokus **nicht** automatisch zurück, wenn ein Spiel, Steam,
ein vorgeschalteter Launcher (z. B. Larian), Heroic, Lutris, Wine, Prism oder ein
Emulator ein Fenster öffnet. Ein erfolgreicher `gio launch`-Aufruf ist nur ein
Startauftrag und keine Aussage darüber, ob ein Spiel läuft oder beendet ist.

## Architektur

### IPC

Die laufende UI besitzt einen lokalen Server:

- Unix-Domain-Socket in `$XDG_RUNTIME_DIR/couchdeck-activate.sock`
  (Fallback: `$XDG_STATE_HOME/couchdeck/activate.sock`).
- Socket-Modus `0600`; zusätzlich prüft der Server `SO_PEERCRED` und akzeptiert
  nur die eigene lokale UID.
- Der Server nimmt kleine, begrenzte Anfragen an, sendet ein Ereignis an den
  UI-Thread und ruft `egui::Context::request_repaint()` auf. Die UI blockiert
  dadurch nicht.
- Veraltete Sockets nach einem Absturz werden beim nächsten Start/Client-Aufruf
  ersetzt.
- Der bestehende Single-Instance-`flock` bleibt die Autorität dafür, ob eine
  Instanz läuft.

`couchdeck --activate` ist ein kurzer Client:

1. liest optional `XDG_ACTIVATION_TOKEN`,
2. verbindet sich mit dem Socket,
3. sendet `ACTIVATE 2` mit einem hex-kodierten Token, falls vorhanden, und
   `END`,
4. wartet auf `OK REQUESTED`,
5. beendet sich ohne zweite UI.

Die Antwort bedeutet nur **„Anforderung angenommen“**. Sie ist kein Beweis für
einen tatsächlich gesetzten Fensterfokus. Die CLI unterscheidet deshalb:

- `Activation-Token weitergegeben` (Token war vorhanden)
- `kein Activation-Token vorhanden`
- `CouchDeck läuft nicht` / `Aktivierung nicht möglich`

### xdg-activation-v1 auf Wayland

Winit/Wayland kann bei einem **bereits bestehenden** Fenster:

- `set_visible()` nicht umsetzen,
- `focus_window()` nicht umsetzen,
- `set_minimized(false)` nicht umsetzen (Winit ignoriert es),
- den Minimierungszustand nicht zuverlässig liefern (`is_minimized()`
  liefert auf Wayland nichts Verlässliches; egui zeigt deshalb ggf.
  `minimized=Some(false)`, obwohl das Fenster minimiert ist).

eframe 0.32 stellt keinen direkten Weg bereit, einen eingehenden
Activation-Token auf eine bestehende Oberfläche anzuwenden. CouchDeck nutzt
deshalb einen kleinen Wayland-Guest-Client:

- Die Raw-Handles von `wl_display` und `wl_surface` kommen aus dem
  eframe-/winit-Fenster.
- Auf derselben Verbindung wird `xdg_activation_v1` gebunden.
- `activate(token, surface)` wird für die bestehende Oberfläche gesendet.
- Es wird **kein neues Fenster** erzeugt und **kein Token erfunden**.
- Token sind kurzlebig und einmalig; die Weitergabe erfolgt sofort.
- Der Compositor entscheidet, ob er den Token akzeptiert.

Ein gültiger Token entsteht nur im Zusammenhang mit einer Benutzeraktion. Ein
normaler Terminalaufruf hat häufig keinen `XDG_ACTIVATION_TOKEN`; ein über
KDE/KIO gestarteter Prozess kann dagegen einen Token erhalten. CouchDeck
behauptet nie, dass ein Token ohne Benutzeraktion einen Fokus erzwingt.

### KDE/KWin-Fallback

Wenn unter KDE/Wayland kein Token vorhanden ist oder der Compositor den Token
nicht in einen beobachteten Fokus umsetzt, startet CouchDeck einen begrenzten
KWin-Scripting-Fallback:

- nur bei erkannter KDE-Sitzung und Wayland,
- nur nach einer expliziten `--activate`-Anforderung,
- ein einmaliger Versuch pro Anforderung, kein Polling,
- das Skript sucht ausschließlich das CouchDeck-Fenster anhand von
  `desktopFileName`, `resourceClass` oder dem Titel `CouchDeck`,
- es setzt `minimized = false` und `workspace.activeWindow`,
- es wiederholt die Aktivierung nach 180 ms einmalig, weil KWin die
  Minimierung in manchen Fällen erst verzögert mapped,
- es beendet oder verschiebt keine anderen Anwendungen,
- Skriptdatei und Plugin werden anschließend entfernt,
- benötigt `qdbus6`, `qdbus-qt6` oder `qdbus`; fehlt das Werkzeug, wird der
  Fehler geloggt.

Der Fallback ist bewusst KDE-spezifisch und keine allgemeine Voraussetzung.
GNOME/XFCE nutzen den Token-Pfad; X11 nutzt die normalen
Viewport-/Window-Manager-Fähigkeiten.

## Aktivierungsverhalten

- Ist CouchDeck bereits fokussiert und sichtbar: keine Aktion.
- Minimiert: unter X11 `Minimized(false)`, unter Wayland Token und ggf.
  KWin-Fallback. Der Minimalzustand wird nicht als verlässlich vorausgesetzt.
- Sichtbar, aber unfokussiert: Token, KWin-Fallback oder X11-`Focus`;
  `RequestUserAttention` ist nur ein Hinweis, falls kein Token vorliegt.
- Vorher Fullscreen: Fullscreen wird nur nach tatsächlich beobachtetem Fokus
  wiederhergestellt, wenn CouchDeck für das Minimieren aus dem Fullscreen
  gewechselt wurde.
- Windowed oder manuell verlassenes Fullscreen: bleibt windowed.
- Keine aggressiven Wiederholungen, kein Beenden oder Verschieben anderer
  Fenster, keine Prozessmanipulation.
- Eine ausstehende Minimierung wird bei einer Aktivierungsanforderung
  verworfen, damit nicht nachträglich wieder minimiert wird.
- Keine Menü-, PIN- oder Profilaktionen; die Aktivierung kann keine
  Kinderrechte erweitern oder Spiele neu starten.

## X11 und Wayland

| Umgebung | Aktivierung |
|---|---|
| X11 | `ViewportCommand::Focus` funktioniert in der Regel, wenn der Window-Manager dies erlaubt. Minimieren/Wiederherstellen über den WM ist üblich. |
| KDE Plasma Wayland | Token-Pfad plus begrenzter KWin-Fallback. Auf dem Entwicklungsystem (KDE 6.7.5) praktisch getestet. Bazzite-Zielhardware bleibt offen. |
| GNOME Wayland | Token-Pfad, falls der Aufrufer einen Token liefert. Kein GNOME-spezifischer Fallback; tokenlose Aktivierung kann verweigert werden. |
| XFCE/X11 | Normale X11-Fokuspfade; nicht auf Zielhardware getestet. |
| SteamOS Desktop Mode | Nicht getestet; Session-Erkennung schließt Gamescope aus. |

## Terminal versus KDE-Shortcut

- **Normales Terminal:** In der Regel kein `XDG_ACTIVATION_TOKEN`. Unter
  KDE/Wayland greift der KWin-Fallback; unter GNOME/Wayland ist die
  Aktivierung ohne Token Best-Effort und kann scheitern.
- **KDE/KIO-gestarteter Prozess oder Launcher:** KIO/KProcessRunner kann
  `XDG_ACTIVATION_TOKEN` setzen. Der Token wird über IPC weitergegeben und über
  `xdg-activation-v1` angewendet.
- **Vom Benutzer eingerichteter KDE-Shortcut:** Das Verhalten hängt davon ab,
  ob die Shortcut-Implementierung einen Token in die Umgebung des gestarteten
  Befehls gibt. Unabhängig davon bleibt der KWin-Fallback als KDE-spezifischer
  Weg verfügbar.
- Ein echter globaler KDE-Shortcut wurde in dieser Entwicklungsumgebung nicht
  automatisch registriert; die Einrichtung bleibt eine manuelle
  Benutzerentscheidung und ist auf Bazzite zu prüfen.

## Desktop-Shortcuts

Der Shortcut ist eine **manuelle Benutzerentscheidung**. CouchDeck registriert
keine globale Tastenkombination automatisch.

### KDE Plasma

Systemeinstellungen → Kurzbefehle → Benutzerdefinierte Kurzbefehle → Neuer
globaler Kurzbefehl:

```text
/home/USER/.local/bin/couchdeck --activate
```

Der vollständige Pfad ist zuverlässiger als ein PATH-Aufruf.

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

Für AppImage-Nutzer zeigt der Pfad auf die AppImage-Datei:

```text
/home/USER/Applications/CouchDeck-0.4.6-x86_64.AppImage --activate
```

## Steam Input (Vorbereitung für 0.5)

Steam Input kann eine Controller-Taste auf eine Desktop-Aktion abbilden, z. B.
über ein Steam-Input-Layout mit einem „Guide“-/Home-Button und einem
Desktop-Shortcut. In 0.4.6 gibt es **keine** globale Controller-Home-Funktion
und keine zusätzliche Controllerüberwachung im Hintergrund. Die Tastenkombination
wird ausschließlich vom Benutzer in Steam Input eingerichtet.

## Diagnose

```bash
couchdeck --diagnose-window
couchdeck --activate
```

Relevante Logzeilen:

- `activation: Anfrage vom lokalen Benutzer empfangen; Token vorhanden/nicht vorhanden.`
- `activation: IPC empfangen: focused=..., minimized=..., fullscreen=..., wayland=..., token=...`
- `activation: Wayland meldet minimized=...; der Minimierungszustand ist ... nicht zuverlässig`
- `activation: Activation-Token an die bestehende Wayland-Oberfläche angewendet ...`
- `activation: KWin-Fallback in ... ms geplant ...`
- `activation: Fokus tatsächlich beobachtet nach ... ms ...`
- `activation: Timeout: keine Fokusänderung beobachtet ...`
- `window: Fokus gewonnen/verloren ...`
- `kwin: KWin-Aktivierungsskript ...`
- `launch: gio launch ...`

Fehlerfälle:

- keine laufende Instanz → Exit-Code ungleich 0, verständliche Meldung
- Socket vorhanden, aber nicht erreichbar → keine falsche Erfolgsmeldung
- veralteter Socket nach Crash → wird beim nächsten Start/Client-Aufruf ersetzt
- anderer lokaler Benutzer → Socket-Permissions/`SO_PEERCRED` verweigern die
  Verbindung
- kein `xdg_activation_v1` → Log, keine Fokusbehauptung
- kein `qdbus6`/`qdbus` bei KDE → Log, keine Fokusbehauptung

## Manueller Testablauf

Die folgenden Fälle wurden auf **Manjaro/KDE 6.7.5/Wayland** mit isolierten
XDG-Verzeichnissen geprüft:

1. CouchDeck fokussiert: `--activate` → IPC angenommen, keine sichtbare Störung.
2. Minimiertes Windowed-Fenster: `--activate` ohne Token → KWin-Fallback,
   Fenster sichtbar und fokussiert.
3. Minimiertes Fullscreen-Fenster: `--activate` ohne Token → Fullscreen bleibt
   erhalten, Fenster wird fokussiert; kein unnötiger Fullscreen-Toggle.
4. Von KDE/KIO gestarteter Prozess mit `XDG_ACTIVATION_TOKEN` → Token wird über
   IPC weitergegeben und in 7 ms in einen beobachteten Fokus umgesetzt.
5. Absichtlich ungültiger Token → Token-Anwendung wird versucht, danach greift
   der KWin-Fallback nach 800 ms; Fokus wird beobachtet.
6. Mehrfache schnelle Aufrufe → keine zweite Instanz, stabiler Zustand.

Noch offen:

- Bazzite/KDE-Wayland auf echter Zielhardware
- echter benutzerdefinierter KDE-Shortcut
- GNOME/Wayland und XFCE/X11
- Steam-/Proton-/Larian-/Heroic-/Lutris-/Prism-/Emulator-Rückkehrszenarien
- SteamOS Desktop Mode

Die Desktop-Matrix und offene Hardwaretests stehen in
[`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md) und
[`BAZZITE-RELEASE-TEST.md`](BAZZITE-RELEASE-TEST.md).
