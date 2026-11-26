# Ryujinx-Spike – Ergebnisse

Stand: 25. November 2026  
Getestet auf Manjaro/KDE Plasma/Wayland mit Ryujinx 1.3.3/.NET 9.0.9.

Der ursprüngliche Arbeitsauftrag liegt unverändert in
[`RYUJINX_SPIKE.md`](RYUJINX_SPIKE.md). Dieses Dokument hält ausschließlich die
technischen Ergebnisse des ersten Spikes fest.

## Umgebung

- Ryujinx: `~/dat2/publish/Ryujinx`
- Kein `Ryujinx.Headless.SDL2` in der Installation vorhanden
- Test mit einem temporären Ryujinx-Root (`-r /tmp/...`), um die bestehende
  Benutzerkonfiguration nicht zu verändern
- Keine `.nsp`/`.xci`-Datei unter `~/dat2` gefunden; daher wurde der
  Emulator-UI-Prozess getestet, nicht ein vollständiger Spiele-/Fullscreen-Start

## Direkter systemd-Start

Getestete Varianten:

1. transiente systemd-User-Unit:
   ```text
   systemd-run --user --unit=couchdeck-session-<id> --collect \
     --property=KillMode=control-group <Ryujinx> -r <temp-root>
   ```
2. systemd-User-Scope:
   ```text
   systemd-run --user --scope --unit=couchdeck-session-<id> --collect \
     --property=KillMode=control-group <Ryujinx> -r <temp-root>
   ```

Ergebnisse:

- Beide Varianten starten Ryujinx direkt, ohne `gio launch`.
- Die Unit lief in einer eigenen cgroup unter
  `user@1000.service/app.slice/...`.
- Die `MainPID` war die Ryujinx-PID.
- Das Ryujinx-Fenster in KWin hatte dieselbe PID.
- Bei der Scope-Variante muss der Stop auf `<unit>.scope` zielen.
  `systemctl --user stop <unit>` ohne Suffix sucht eine `.service` und schlug im
  Test fehl.
- `systemctl --user stop <unit>.scope` beendet die Sitzung.
- Ein Scope mit einem einfachen Testprozess blieb aktiv, solange der Prozess
  lief; nach dem Stop war er beendet.

## Reguläres Fensterschließen

Ein KWin-Script wurde mit exakter PID-Zuordnung ausgeführt:

```javascript
for (const window of workspace.windowList()) {
    if (window.pid === targetPid) {
        window.closeWindow();
    }
}
```

Ergebnis:

- Ryujinx wurde über `closeWindow()` beendet.
- Es wurde kein `killWindow()` und kein Prozessname-Matching verwendet.
- Der Prozess und die transiente Unit verschwanden anschließend.
- Das ist der bevorzugte reguläre Weg für ein Direct/Managed-Backend unter
  KDE/KWin.

## Notfallweg

`systemctl --user stop <unit>.scope` stoppte den getesteten Ryujinx-Prozess.
Das ist ein bestätigter Notfallweg, kein sauberer Emulator-Shutdown. Er darf nur
für die eigene, verifizierte Session-Unit verwendet werden.

## Wichtige Einschränkungen

- Ein vollständiger Spielstart mit einer echten `.nsp`/`.xci`-Datei wurde nicht
  durchgeführt.
- `Ryujinx.Headless.SDL2` war nicht vorhanden und wurde nicht getestet.
- Die KWin-Fensterschließung ist KDE-spezifisch; GNOME/XFCE brauchen eine eigene
  geprüfte Strategie.
- Das Verhalten von `systemd-run --scope`, wenn CouchDeck selbst beendet wird,
  wurde nicht vollständig getestet.
- Steam/Flatpak und Prism wurden in diesem Spike nicht untersucht.
- Eine Scope-Zuordnung um `gio launch` ist nicht verlässlich; für Direct/Managed
  wird der Emulator direkt mit strukturierten Argumenten gestartet.

## Fazit

Der Direct/Managed-Ansatz ist für Ryujinx technisch tragfähig:

- direkter Prozessstart,
- exakte PID-/cgroup-Zuordnung,
- Fensterschließen per exakter PID,
- Scope-Stop als bestätigter Notfallweg.

Er ist ein Proof of Concept für das Direct/Managed-Backend, nicht die
Gesamtarchitektur. Steam, Prism und generische GIO-Einträge benötigen getrennte
Backend-Spikes.
