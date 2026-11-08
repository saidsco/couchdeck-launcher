# Desktop-Testmatrix (Stand 0.4.6)

CouchDeck ist desktopunabhängig konzipiert. Die folgende Matrix trennt
**automatisierte/Unit-geprüfte Logik** von tatsächlich durchgeführten
Sitzungstests. Nicht ausgeführte Kombinationen sind ausdrücklich offen.

| Umgebung | Start/UI | Fullscreen | Controller | Autostart | Minimize | Activate | GIO-Launch | Systemaktionen | Status |
|---|---|---|---|---|---|---|---|---|---|
| KDE Plasma / Wayland (Manjaro, Entwicklung) | App läuft; native Wayland-Aktivierung praktisch getestet | `fullscreen=true` in App-Logs; kein unnötiger Toggle bei externer Minimierung | 8BitDo Ultimate erkannt; vollständige Navigation nicht dokumentiert | Logik unit-getestet; echte Anmeldung offen | KWin-Scripting praktisch getestet | Token-/KWin-Fallback praktisch getestet; Bazzite offen | GIO-Integrationstest grün; UI-Start gesehen | nicht real ausgeführt | **teilweise** |
| KDE Plasma / X11 (Xwayland-Smoke) | Fensterstart sichtbar | Windowed/Fullscreen getestet | nicht getestet | nicht getestet | minimiert/wiederhergestellt | fokussiert, minimiert, Fullscreen-minimiert, No-Instance | Integrationstest | nicht getestet | **teilweise** |
| GNOME / Wayland | nicht ausgeführt | nicht ausgeführt | nicht ausgeführt | Session-Erkennung unit-getestet | nicht ausgeführt | IPC unit-getestet; Fokus offen | nicht ausgeführt | nicht ausgeführt | **offen** |
| XFCE / X11 | nicht ausgeführt | nicht ausgeführt | nicht ausgeführt | Session-Erkennung unit-getestet | nicht ausgeführt | IPC unit-getestet; Fokus offen | nicht ausgeführt | nicht ausgeführt | **offen** |
| SteamOS Desktop Mode | nicht ausgeführt | nicht ausgeführt | nicht ausgeführt | Session-Erkennung unterscheidet Gamescope; Desktop-Mode-Erkennung offen | nicht ausgeführt | IPC unit-getestet; Fokus offen | nicht ausgeführt | nicht ausgeführt | **offen** |
| Steam Gaming Mode / Gamescope | bewusst nicht Ziel | – | – | ausgeschlossen | – | – | – | – | **nicht unterstützt** |

## Fensteraktivierung (0.4.6)

`couchdeck --activate` wurde unter X11/Xwayland und KDE 6.7.5/Wayland
praktisch geprüft:

- fokussiert → IPC angenommen, keine Fensteraktion
- minimiert (X11) → wiederhergestellt und fokussiert
- minimiert aus Fullscreen (X11) → Fullscreen und Fokus wiederhergestellt
- keine laufende Instanz → Exit-Code ungleich 0, keine zweite Instanz
- mehrere schnelle Aufrufe → stabiler Zustand
- KDE/Wayland, Terminal ohne Token → KWin-Fallback, sichtbar/fokussiert
- KDE/Wayland, KIO-gestarteter Prozess mit Token → Token angewendet, Fokus
  beobachtet
- KDE/Wayland, minimiertes Windowed-/Fullscreen-Fenster → Fokus beobachtet,
  Fullscreen ohne zusätzlichen Toggle
- KDE/Wayland, ungültiger Token → KWin-Fallback nach 800 ms

Unter GNOME/XFCE/SteamOS entscheidet der Compositor; die IPC-Annahme ist keine
Fokusbestätigung. Bazzite-Zielhardware, echter KDE-Shortcut und die weiteren
Desktops bleiben offen.

## Autostart-Tests

Unit-Tests prüfen:

- Erkennung von `KDE`, `ubuntu:GNOME`, `XFCE` und mehrteiligen Werten.
- Gamescope/Gaming-Mode wird nie als Desktop-Sitzung akzeptiert.
- Die erzeugte Datei enthält `OnlyShowIn=<erkannte Sitzung>`, `NotShowIn=gamescope` und kein festes `OnlyShowIn=KDE`.
- Entfernen der Autostart-Datei funktioniert.

Echte Anmeldesitzungen unter KDE, GNOME, XFCE und SteamOS Desktop Mode bleiben
manuell zu prüfen.

## Discovery-Tests

Automatisiert geprüft:

- XDG_DATA_HOME vor XDG_DATA_DIRS; Reihenfolge der XDG_DATA_DIRS.
- Desktop-Datei-IDs, Unterverzeichnisse und Überlagerung bei Duplikaten.
- Flatpak-Benutzerexport, XDG-Desktopordner, Managed-Desktop und zusätzliche Verzeichnisse.
- `NoDisplay`, `Hidden`, `OnlyShowIn`, `NotShowIn`, fehlendes `TryExec`.
- Symlink-Dateien; Symlink-Verzeichnisse werden nicht rekursiv verfolgt.
- Lokalisierte Namen, Dateinamen mit Leerzeichen.

## Bekannte Grenzen

- Wayland-Compositor-Fokus und Minimieren sind nicht vollständig kontrollierbar.
- Der echte Controller-, GIO- und systemctl-Test auf einer frischen Bazzite-/SteamOS-Installation ist offen.
- `OnlyShowIn`-Werte werden anhand der Sitzungsumgebung erkannt; ungewöhnliche
  oder fehlende Desktop-Strings können Autostart bewusst deaktivieren.
