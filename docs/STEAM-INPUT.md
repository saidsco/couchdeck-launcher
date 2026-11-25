# Steam Input: Controller-Home / Rückkehr zu CouchDeck

Ab CouchDeck 0.5.1 gibt es den **nativen Controller-Home-Chord** (standardmäßig
Start + Select, 2 Sekunden halten). Steam Input ist dafür **nicht** nötig.

Diese Anleitung beschreibt Steam Input nur als optionalen alternativen Weg für
Benutzer, die eine eigene Steam-Input-Kombination bevorzugen. CouchDeck
implementiert keinen eigenen Input-Treiber und keinen exklusiven Geräte-Grab.
Der optionale Steam-Input-Weg ist:

```text
Steam Input / Controller-Kombination
        → globale Tastenkombination oder Desktop-Aktion
        → couchdeck --home
```

`couchdeck --home` aktiviert eine laufende CouchDeck-Instanz oder startet
CouchDeck, wenn keine Instanz läuft. Es startet keinen zweiten Prozess, beendet
keine Spiele und umgeht keine PIN-/Kinderprüfung.

## Empfohlene KDE-Einrichtung

1. In Steam Input ein Layout/eine Aktionsebene anlegen.
2. Eine Controller-Kombination auf eine freie Tastenkombination legen, z. B.
   `Guide + Back` auf `Meta+Alt+C`.
3. In KDE Plasma unter **Systemeinstellungen → Kurzbefehle → Benutzerdefinierte
   Kurzbefehle** einen neuen globalen Kurzbefehl anlegen:

```text
/home/USER/.local/bin/couchdeck --home
```

Bei einer AppImage-Installation den absoluten AppImage-Pfad verwenden:

```text
/home/USER/Applications/CouchDeck-0.5.2-x86_64.AppImage --home
```

Optional kann vorher ein Menüeintrag erzeugt werden:

```bash
couchdeck --install-home-launcher
```

Der Eintrag heißt „CouchDeck Home“ und startet `couchdeck --home`. Die
Zuweisung der globalen Tastenkombination bleibt eine manuelle Entscheidung.

## GNOME / XFCE

Die Tastenkombination wird in den jeweiligen Systemeinstellungen auf denselben
Befehl gelegt. Genaue Pfade und Beispiele stehen in
[`WINDOW-ACTIVATION.md`](WINDOW-ACTIVATION.md).

## Bazzite/KDE-Wayland

- Steam Input darf die Kombination als **Desktop-Tastatur** senden.
- Die Tastenkombination muss von KDE als globaler Kurzbefehl erkannt werden.
- CouchDeck nutzt bei KDE/Wayland den begrenzten KWin-Fallback, wenn kein
  xdg-activation-Token vorhanden ist.
- Der native CouchDeck-Chord funktioniert unabhängig von Steam, solange
  CouchDeck läuft und die `/dev/input`-Berechtigung vorhanden ist.
- Ein laufendes Spiel bleibt im Hintergrund aktiv; CouchDeck beendet es nicht.

## Grenzen

- Steam Input kann von Spielen oder Steam selbst abgefangen werden.
- Manche Spiele verwenden exklusive Controller-Modi; das Verhalten ist dann
  spielabhängig.
- Es gibt keine Garantie für eine bestimmte Controller-Taste oder ein
  bestimmtes Steam-Layout.
- CouchDeck schreibt keine Steam-Input-Konfiguration und registriert keinen
  systemweiten Hotkey automatisch.
- Der native Controller-Home-Chord ist Teil von 0.5.1; ein separater
  Hintergrunddienst ist **nicht** Teil von 0.5.1.

## Manueller Test

1. CouchDeck starten, ein Spiel über Steam starten.
2. Controller-Kombination auslösen.
3. Erwartung: CouchDeck kommt sichtbar und fokussiert zurück, das Spiel läuft
   weiter.
4. CouchDeck über die Taskleiste minimieren, Kombination erneut auslösen.
5. Erwartung: Rückkehr ohne Maus-Hover; auf KDE/Wayland ggf.
   `--diagnose-window` und das Log prüfen.

Offen: echter Test mit dem verwendeten Controller/Steam-Input-Layout auf
Bazzite-KDE/Wayland sowie GNOME/XFCE.
