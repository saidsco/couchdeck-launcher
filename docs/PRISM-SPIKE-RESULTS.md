# Prism/Minecraft-Spike – Ergebnisse

Stand: 26. November 2026  
Getestet auf Manjaro/KDE Plasma/Wayland.

Verwendete Desktop-Datei: `Minecraft.desktop` aus dem Projektordner
(nicht Teil des CouchDeck-Repositories, nur Testartefakt).

```ini
Exec="flatpak" 'run' 'org.prismlauncher.PrismLauncher' '--launch' \
  '9a6ec2f8fa87498180bfcc30422fc14d'
```

Installierte Version: Prism Launcher 11.1.1 (System-Flatpak),
Instanz `26.3` mit Minecraft/Fabric.

## Ergebnisse

### Start

`flatpak run org.prismlauncher.PrismLauncher --launch <INSTANCE_ID>` startet
Prism und die Minecraft-Java-Instanz.

Beobachtete Prozesse:

- `prismrun --launch <INSTANCE_ID>` im Flatpak
- Java/Minecraft als Kindprozess
- Minecraft-Fenster mit der PID des Java-Prozesses
- KWin-Caption: `Minecraft* 26.3`
- App-ID/Ressourcenklasse: `org.prismlauncher.PrismLauncher`

### systemd-Scope um Flatpak

Ein Wrapper

```text
systemd-run --user --unit=couchdeck-session-<id> --collect \
  --property=KillMode=control-group \
  flatpak run org.prismlauncher.PrismLauncher --launch <INSTANCE_ID>
```

startete zwar eine transiente Unit, aber Flatpak verschob die eigentlichen
Prozesse in eine eigene Unit:

```text
app-flatpak-org.prismlauncher.PrismLauncher-<id>.scope
```

Der MainPID der systemd-run-Unit war `bwrap`; die eigentlichen Prism-/Java-
Prozesse lagen in der Flatpak-Scope. Eine Steuerung über die äußere
systemd-run-Unit ist daher **nicht verlässlich**.

### Session-Zuordnung

`flatpak ps` liefert die laufende Flatpak-Instanz und deren PID. Die
Minecraft-Java-PID ist ein Kind des Prism-Prozesses; das KWin-Fenster hat
dieselbe PID wie der Java-Prozess.

Damit ist im `--launch`-Fall eine Zuordnung möglich über:

- `flatpak ps` (Instanz/PID),
- Prozessbaum (Prism → Java),
- KWin-Fenster-PID.

### Reguläres Beenden

Ein KWin-Script mit exakter Java-PID:

```javascript
for (const window of workspace.windowList()) {
    if (window.pid === targetPid) {
        window.closeWindow();
    }
}
```

beendete das Minecraft-Fenster. Anschließend waren die Java-, Prism- und
Flatpak-Prozesse der gestarteten Instanz beendet; `flatpak ps` war leer.
Es wurde kein `killWindow()`, kein `pkill` und keine Prozessnamen-Suche benutzt.

### Notfall-/App-Stop

`flatpak kill <INSTANCE>` beendete die gestartete Flatpak-Instanz ebenfalls.
Das ist ein applikationsbezogener Stop des Flatpak-Instanzprozesses, kein
generischer Prozess-Kill. Er tötet aber die gesamte Flatpak-Instanz; bei
mehreren Spielen/Prism-Fenstern müsste die Zuordnung vorher eindeutig sein.

## Einschränkungen

- Getestet wurde der `--launch`-Modus ohne bereits geöffnetes Prism-Launcher-
  Fenster. Das Verhalten mit zusätzlich geöffnetem Prism-UI ist offen.
- Ob mehrere parallele Prism-Instanzen zuverlässig getrennt werden können,
  wurde nicht getestet.
- `flatpak run` kann bei bereits laufendem Prism eine bestehende Instanz
  wiederverwenden; die Session-Zuordnung muss diesen Fall erkennen.
- Ein vollständiger Spielverlauf (Welt laden, spielen, speichern) wurde nicht
  getestet; der Spike prüft Start, Prozess-/Fensterzuordnung und Beenden.
- KWin-Fensterschließung ist KDE-spezifisch. GNOME/XFCE brauchen eine eigene
  getestete Strategie.

## Fazit

Prism/Minecraft ist für ein eigenes Backend **deutlich besser geeignet als
Steam**, weil Java- und Fenster-PID eindeutig zugeordnet werden konnten und
sowohl reguläres Fensterschließen als auch ein Flatpak-Instanz-Stopp
funktionierten.

Für CouchDeck gilt:

- Start über die vorhandene `.desktop`-Datei bzw. strukturierte Flatpak-Argumente.
- Kein systemd-run um `flatpak` als alleinige Kontrollinstanz.
- Flatpak-Instanz und Java-/Fenster-PID getrennt erfassen.
- Regulär zuerst das Minecraft-Fenster schließen.
- `flatpak kill <instance>` nur als bestätigter Notfallweg für die eigene
  Instanz.
- Prism-UI-offen- und Parallelinstanz-Fälle vor einer Capability-Freigabe testen.
