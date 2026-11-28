# Fensteraktivierung, Controller-Home und Rückkehr (0.6.0)

CapyDeck fordert den Fokus **nicht** automatisch zurück, wenn ein Spiel, Steam,
ein vorgeschalteter Launcher (z. B. Larian), Heroic, Lutris, Wine, Prism oder ein
Emulator ein Fenster öffnet. Ein erfolgreicher `gio launch`-Aufruf ist nur ein
Startauftrag und keine Aussage darüber, ob ein Spiel läuft oder beendet ist.

## Architektur

### IPC

Die laufende UI besitzt einen lokalen Server:

- Unix-Domain-Socket in `$XDG_RUNTIME_DIR/capydeck-activate.sock`
  (Fallback: `$XDG_STATE_HOME/capydeck/activate.sock`).
- Socket-Modus `0600`; zusätzlich prüft der Server `SO_PEERCRED` und akzeptiert
  nur die eigene lokale UID.
- Der Server nimmt kleine, begrenzte Anfragen an, sendet ein Ereignis an den
  UI-Thread und ruft `egui::Context::request_repaint()` auf. Die UI blockiert
  dadurch nicht.
- Veraltete Sockets nach einem Absturz werden beim nächsten Start/Client-Aufruf
  ersetzt.
- Der bestehende Single-Instance-`flock` bleibt die Autorität dafür, ob eine
  Instanz läuft.

`capydeck --activate` ist ein kurzer Client:

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
- `CapyDeck läuft nicht` / `Aktivierung nicht möglich`

### xdg-activation-v1 auf Wayland

Winit/Wayland kann bei einem **bereits bestehenden** Fenster:

- `set_visible()` nicht umsetzen,
- `focus_window()` nicht umsetzen,
- `set_minimized(false)` nicht umsetzen (Winit ignoriert es),
- den Minimierungszustand nicht zuverlässig liefern (`is_minimized()`
  liefert auf Wayland nichts Verlässliches; egui zeigt deshalb ggf.
  `minimized=Some(false)`, obwohl das Fenster minimiert ist).

eframe 0.32 stellt keinen direkten Weg bereit, einen eingehenden
Activation-Token auf eine bestehende Oberfläche anzuwenden. CapyDeck nutzt
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
KDE/KIO gestarteter Prozess kann dagegen einen Token erhalten. CapyDeck
behauptet nie, dass ein Token ohne Benutzeraktion einen Fokus erzwingt.

### KDE/KWin-Fallback

Wenn unter KDE/Wayland kein Token vorhanden ist oder der Compositor den Token
nicht in einen beobachteten Fokus umsetzt, startet CapyDeck einen begrenzten
KWin-Scripting-Fallback:

- nur bei erkannter KDE-Sitzung und Wayland,
- nur nach einer expliziten `--activate`-Anforderung,
- ein einmaliger Versuch pro Anforderung, kein Polling,
- das Skript sucht ausschließlich das CapyDeck-Fenster anhand von
  `desktopFileName`, `resourceClass` oder dem Titel `CapyDeck`,
- es setzt `minimized = false` und `workspace.activeWindow`,
- es wird bei KDE/Wayland aus dem IPC-Thread gestartet, wenn die UI keine
  Frames erhält (Taskleisten-Minimierung),
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

- Ist CapyDeck bereits fokussiert und sichtbar: keine Aktion.
- Minimiert: unter X11 `Minimized(false)`, unter Wayland Token und ggf.
  KWin-Fallback. Der Minimalzustand wird nicht als verlässlich vorausgesetzt.
- Sichtbar, aber unfokussiert: Token, KWin-Fallback oder X11-`Focus`;
  `RequestUserAttention` ist nur ein Hinweis, falls kein Token vorliegt.
- Vorher Fullscreen: Fullscreen wird nur nach tatsächlich beobachtetem Fokus
  wiederhergestellt, wenn CapyDeck für das Minimieren aus dem Fullscreen
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

## `--home`: aktivieren oder starten

`capydeck --home` ist der empfohlene Befehl für Controller-/Desktop-Shortcuts:

- läuft CapyDeck bereits → Aktivierung wie bei `--activate`
- läuft CapyDeck nicht → normaler UI-Start
- keine zweite Instanz, keine Prozessbeendigung, keine PIN-/Kinderumgehung
- `--activate` bleibt der reine Aktivierungsbefehl und endet ohne Instanz mit Fehler

Optional kann ein benutzerspezifischer Menüeintrag angelegt werden:

```bash
capydeck --install-home-launcher
capydeck --remove-home-launcher
```

Der Eintrag landete in `$XDG_DATA_HOME/applications/capydeck-home.desktop` und
startet `capydeck --home`. Eine globale Tastenkombination wird dabei **nicht**
registriert.

## Nativer Controller-Home-Chord (0.5.1)

Während CapyDeck läuft, erkennt der bestehende gilrs-Thread global eine
Controller-Kombination – auch bei minimiertem Fenster:

- Standard: **Start + Select 2 Sekunden halten**
- Alternative: Start + linker Stickklick
- Alternative: Start + rechter Stickklick

Der Chord ist standardmäßig aktiv und wird im Bibliotheks-Footer sowie unter
Admin → Einstellungen angezeigt. Er löst dieselbe Aktivierung aus wie
`capydeck --home`; auf KDE/Wayland wird der bestehende KWin-Fallback genutzt.

Wichtig:

- CapyDeck liest Gamepad-Events nur read-only und greift nicht exklusiv zu.
- Kein `uinput`, kein eigener Input-Treiber, kein Root-Daemon.
- CapyDeck muss laufen; wenn es beendet ist, gibt es keinen Chord-Listener.
- Der Chord wird nur für die Home-Aktion ausgewertet; normale Bibliotheks-
  navigation bleibt auf das fokussierte Fenster beschränkt.

### Berechtigung

Der Zugriff erfolgt auf `/dev/input/event*` (typisch `root:input`, Modus 0660).
CapyDeck kann die Rechte nicht umgehen.

- Beim Start prüft CapyDeck den Zugriff und bietet einen Dialog an:
  „Jetzt einrichten“ startet die Admin-Einrichtung über Polkit; „Später“
  unterdrückt den Hinweis für weitere Starts.
- Das Installationsskript fragt ebenfalls nach der Installation, ob der Benutzer
  zur Gruppe `input` hinzugefügt werden soll.
- Der CLI-Fallback bleibt verfügbar:

  ```bash
  ./CapyDeck-0.6.0-x86_64.AppImage --setup-input-permissions
  ```

- Die Einrichtung nutzt `pkexec` (Polkit) oder, bei einem Terminal, `sudo`.
- Nach dem Hinzufügen zur Gruppe ist eine **Neuanmeldung** erforderlich.
- Admin → Einstellungen zeigt den aktuellen Status.
- Die Gruppe `input` kann auch Tastatur-/Mausereignisse lesen; wer das vermeiden will, nutzt stattdessen eine restriktive udev-`uaccess`-Regel nur für `ID_INPUT_JOYSTICK`. CapyDeck installiert keine solche Regel automatisch.

## Desktop-Shortcuts

Der Shortcut ist eine **manuelle Benutzerentscheidung**. CapyDeck registriert
keine globale Tastenkombination automatisch.

### KDE Plasma

Systemeinstellungen → Kurzbefehle → Benutzerdefinierte Kurzbefehle → Neuer
globaler Kurzbefehl:

```text
/home/USER/.local/bin/capydeck --home
```

Der vollständige Pfad ist zuverlässiger als ein PATH-Aufruf.

### GNOME

Einstellungen → Tastatur → Tastenkürzel → Benutzerdefiniert:

```text
Name:    CapyDeck aktivieren
Befehl:  /home/USER/.local/bin/capydeck --home
Tasten:  z. B. Super+Alt+C
```

### XFCE

Einstellungen → Tastatur → Tastenkürzel → Hinzufügen:

```text
Befehl: /home/USER/.local/bin/capydeck --home
```

Für AppImage-Nutzer zeigt der Pfad auf die AppImage-Datei:

```text
/home/USER/Applications/CapyDeck-0.6.0-x86_64.AppImage --home
```

## Steam Input

Steam Input kann eine Controller-Kombination auf eine Desktop-Tastenkombination
abbilden; diese startet `capydeck --home`. Die vollständige Anleitung steht in
[`STEAM-INPUT.md`](STEAM-INPUT.md). CapyDeck überwacht keine Controller-Eingaben
global, schreibt keine Steam-Konfiguration und registriert keinen systemweiten
Hotkey automatisch.

## Diagnose

```bash
capydeck --diagnose-window
capydeck --home
capydeck --activate
capydeck --setup-input-permissions
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

- `--activate` ohne laufende Instanz → Exit-Code ungleich 0, verständliche Meldung
- `--home` ohne laufende Instanz → normaler Start
- Socket vorhanden, aber nicht erreichbar → keine falsche Erfolgsmeldung
- veralteter Socket nach Crash → wird beim nächsten Start/Client-Aufruf ersetzt
- anderer lokaler Benutzer → Socket-Permissions/`SO_PEERCRED` verweigern die
  Verbindung
- kein `xdg_activation_v1` → Log, keine Fokusbehauptung
- kein `qdbus6`/`qdbus` bei KDE → Log, keine Fokusbehauptung

## Manueller Testablauf

Die folgenden Fälle wurden auf **Manjaro/KDE 6.7.5/Wayland** mit isolierten
XDG-Verzeichnissen geprüft:

1. CapyDeck fokussiert: `--activate` → IPC angenommen, keine sichtbare Störung.
2. Minimiertes Windowed-Fenster: `--activate` ohne Token → KWin-Fallback,
   Fenster sichtbar und fokussiert.
3. Minimiertes Fullscreen-Fenster: `--activate` ohne Token → Fullscreen bleibt
   erhalten, Fenster wird fokussiert; kein unnötiger Fullscreen-Toggle.
4. Von KDE/KIO gestarteter Prozess mit `XDG_ACTIVATION_TOKEN` → Token wird über
   IPC weitergegeben und in 7 ms in einen beobachteten Fokus umgesetzt.
5. Absichtlich ungültiger Token → Token-Anwendung wird versucht, danach greift
   der KWin-Fallback nach 800 ms; Fokus wird beobachtet.
6. Interne Fullscreen-Minimierung → `--activate` → KWin meldet
   `minimized=false`, `active=true`, `fullScreen=true`, kein Hover nötig.
7. Mehrfache schnelle Aufrufe → keine zweite Instanz, stabiler Zustand.

Noch offen:

- Bazzite/KDE-Wayland auf echter Zielhardware
- echter benutzerdefinierter KDE-Shortcut
- GNOME/Wayland und XFCE/X11
- Steam-/Proton-/Larian-/Heroic-/Lutris-/Prism-/Emulator-Rückkehrszenarien
- SteamOS Desktop Mode

Die Desktop-Matrix und offene Hardwaretests stehen in
[`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md) und
[`BAZZITE-RELEASE-TEST.md`](BAZZITE-RELEASE-TEST.md).
