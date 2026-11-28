# Steam Input: Controller-Home / Rückkehr zu CapyDeck

Seit 0.5.1 gibt es den **nativen Controller-Home-Chord** (standardmäßig
Start + Select, 2 Sekunden halten). Steam Input ist dafür **nicht** nötig.

Diese Anleitung beschreibt Steam Input nur als optionalen alternativen Weg für
Benutzer, die eine eigene Steam-Input-Kombination bevorzugen. CapyDeck
implementiert keinen eigenen Input-Treiber und keinen exklusiven Geräte-Grab.
Der optionale Steam-Input-Weg ist:

```text
Steam Input / Controller-Kombination
        → globale Tastenkombination oder Desktop-Aktion
        → capydeck --home
```

`capydeck --home` aktiviert eine laufende CapyDeck-Instanz oder startet
CapyDeck, wenn keine Instanz läuft. Es startet keinen zweiten Prozess, beendet
keine Spiele und umgeht keine PIN-/Kinderprüfung.

## Empfohlene KDE-Einrichtung

1. In Steam Input ein Layout/eine Aktionsebene anlegen.
2. Eine Controller-Kombination auf eine freie Tastenkombination legen, z. B.
   `Guide + Back` auf `Meta+Alt+C`.
3. In KDE Plasma unter **Systemeinstellungen → Kurzbefehle → Benutzerdefinierte
   Kurzbefehle** einen neuen globalen Kurzbefehl anlegen:

```text
/home/USER/.local/bin/capydeck --home
```

Bei einer AppImage-Installation den absoluten AppImage-Pfad verwenden:

```text
/home/USER/Applications/CapyDeck-0.6.0-x86_64.AppImage --home
```

Optional kann vorher ein Menüeintrag erzeugt werden:

```bash
capydeck --install-home-launcher
```

Der Eintrag heißt „CapyDeck Home“ und startet `capydeck --home`. Die
Zuweisung der globalen Tastenkombination bleibt eine manuelle Entscheidung.

## GNOME / XFCE

Die Tastenkombination wird in den jeweiligen Systemeinstellungen auf denselben
Befehl gelegt. Genaue Pfade und Beispiele stehen in
[`WINDOW-ACTIVATION.md`](WINDOW-ACTIVATION.md).

## Bazzite/KDE-Wayland

- Steam Input darf die Kombination als **Desktop-Tastatur** senden.
- Die Tastenkombination muss von KDE als globaler Kurzbefehl erkannt werden.
- CapyDeck nutzt bei KDE/Wayland den begrenzten KWin-Fallback, wenn kein
  xdg-activation-Token vorhanden ist.
- Der native CapyDeck-Chord funktioniert unabhängig von Steam, solange
  CapyDeck läuft und die `/dev/input`-Berechtigung vorhanden ist.
- Ein laufendes Spiel bleibt im Hintergrund aktiv; CapyDeck beendet es nicht.

## Grenzen

- Steam Input kann von Spielen oder Steam selbst abgefangen werden.
- Manche Spiele verwenden exklusive Controller-Modi; das Verhalten ist dann
  spielabhängig.
- Es gibt keine Garantie für eine bestimmte Controller-Taste oder ein
  bestimmtes Steam-Layout.
- CapyDeck schreibt keine Steam-Input-Konfiguration und registriert keinen
  systemweiten Hotkey automatisch.
- Der native Controller-Home-Chord ist Teil von 0.5.1; ein separater
  Hintergrunddienst ist **nicht** Teil von 0.5.1.

## Manueller Test

1. CapyDeck starten, ein Spiel über Steam starten.
2. Controller-Kombination auslösen.
3. Erwartung: CapyDeck kommt sichtbar und fokussiert zurück, das Spiel läuft
   weiter.
4. CapyDeck über die Taskleiste minimieren, Kombination erneut auslösen.
5. Erwartung: Rückkehr ohne Maus-Hover; auf KDE/Wayland ggf.
   `--diagnose-window` und das Log prüfen.

Offen: echter Test mit dem verwendeten Controller/Steam-Input-Layout auf
Bazzite-KDE/Wayland sowie GNOME/XFCE.
