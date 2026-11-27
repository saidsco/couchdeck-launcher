# Flatpak-Steam-Spike – Ergebnisse

Stand: 26. November 2026
Getestet auf Manjaro/KDE Plasma/Wayland.

Getestetes Spiel: **LEGO Bricktales**, App-ID `1898290`.

Flatpak-Steam ist lokal installiert (`com.valvesoftware.Steam`), die LEGO-
Instanz liegt in der Flatpak-Steam-Bibliothek:

```text
~/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/
```

## Desktop-Datei

Ein manuell angelegter Eintrag außerhalb der Flatpak-Sandbox funktioniert:

```ini
[Desktop Entry]
Type=Application
Name=Flatpak Steam LEGO Test
Exec=/usr/bin/flatpak run com.valvesoftware.Steam steam://rungameid/1898290
Terminal=false
Categories=Game;
```

Wichtig:

- `/usr/bin/flatpak` absolut verwenden, damit nicht der native Steam-
  Protokollhandler den URI übernimmt.
- Die Datei muss nicht zwingend von Steam selbst erzeugt werden; sie kann vom
  Benutzer in `~/.local/share/applications/` angelegt werden.
- `chmod +x` kann nötig sein, wenn die Datei über `gio launch` oder einen
  Dateimanager gestartet wird.

## Start

Nach dem Beenden alter Flatpak-Steam-Instanzen:

```bash
flatpak kill <instance>
flatpak run com.valvesoftware.Steam steam://rungameid/1898290
```

Der Start funktionierte. LEGO Bricktales öffnete ein Fenster.

## Prozess-/Cgroup-Struktur

Die Flatpak-Steam-Prozesse liegen gemeinsam in einer Flatpak-App-Scope:

```text
app-flatpak-com.valvesoftware.Steam-<id>.scope
```

Darin befanden sich:

- Steam-/Pressure-Vessel-Prozesse,
- der Spielprozess `LEGO_Bricktales.x86_64`,
- die Steam-Runtime.

Es gab **keine separate spielspezifische Host-Scope** wie bei nativem Steam.
Das bedeutet: Ein Stop der Flatpak-Scope würde die gesamte Flatpak-Steam-
Instanz beenden, nicht nur das Spiel.

## Fenster und reguläres Beenden

Das LEGO-Fenster hatte die PID des eigentlichen Spielprozesses. Ein KWin-Script
mit exakter PID:

```javascript
if (window.pid === targetPid) {
    window.closeWindow();
}
```

beendete das Spiel. Die Flatpak-Instanz, in der das Spiel lief, wurde danach
nicht mehr in `flatpak ps` geführt. Andere Flatpak-Steam-Instanzen blieben
bestehen.

## Notfallweg

- `flatpak kill <instance>` beendet die gesamte Flatpak-Steam-Instanz.
- Ein gezielter Stop nur des Spiels ist über die gemeinsame Flatpak-Scope
  nicht möglich.
- `flatpak kill` ist daher für Flatpak-Steam kein guter spielspezifischer
  Notfallweg, wenn Steam weiterlaufen soll.
- Ein reguläres Fensterschließen ist der bessere Weg, setzt aber eine
  zuverlässige Fenster-/PID-Zuordnung voraus.

## Dual-Steam-Beobachtungen

- Native Steam und Flatpak-Steam liefen gleichzeitig.
- Vor dem Test waren mehrere Flatpak-Steam-Instanzen aktiv; ein neuer Start
  wurde dadurch unübersichtlich.
- Nach dem Beenden der alten Flatpak-Instanzen startete das Spiel.
- Ein sauberer Test sollte nur **eine** Steam-Variante aktiv haben.
- Flatpak-Steam und natives Steam verwenden getrennte Datenverzeichnisse und
  getrennte Bibliotheken.
- Ob und wie Steam mit zwei gleichzeitig angemeldeten Clients umgeht, ist
  nicht abschließend getestet.

## Einschränkungen

- Flatpak-Steam verhält sich anders als natives Steam: gemeinsame App-Scope
  statt spielspezifischer `app-steam-app<ID>-<pid>.scope`.
- Kein sauberer spielspezifischer Notfall-Stop über die Flatpak-Scope.
- Mehrere gleichzeitige Flatpak-Steam-Instanzen erschweren die Zuordnung.
- Native Steam lief parallel; ein Test mit ausschließlich Flatpak-Steam ist
  offen.
- KWin-Fensterschließung ist KDE-spezifisch.
- Vollständiger Spielverlauf/Speichern wurde nicht getestet.

## Fazit

Flatpak-Steam ist startbar und das Spiel kann über die exakte Fenster-PID
regulär geschlossen werden. Die Session-Kontrolle ist aber schwächer als bei
nativem Steam:

- Start: möglich
- Fenster-/PID-Erkennung: möglich
- zuverlässige spielspezifische Scope: **nein**
- reguläres Beenden: Fensterschließen möglich
- Notfall-Stop nur der Session: **nicht sauber möglich**, ohne Steam zu beenden

Flatpak-Steam bleibt daher ein eigener Kompatibilitäts-/Teilbackend und sollte
nicht dieselben Capabilities wie nativer Steam erhalten.

## Phase-II-Status (26. November 2026)

Flatpak-Steam wurde in Phase II **bewusst zurückgestellt**. Bazzite verwendet
standardmäßig natives Steam; fehlende vollständige Flatpak-Steam-Unterstützung
ist kein Release-Blocker.

Die Phase-I-Ergebnisse bleiben gültig und dokumentiert. Sie werden erst wieder
priorisiert, wenn ein Zielsystem Flatpak-Steam tatsächlich als primären
Steam-Client einsetzt. Ein späterer Spike muss insbesondere prüfen:

- Test ohne parallel laufendes natives Steam.
- Zuordnung bei mehreren Flatpak-Steam-Instanzen.
- Ob eine spielspezifische Flatpak-Scope oder ein anderer eindeutiger
  Prozessanker existiert.
- Notfall-Stop ohne Beenden fremder Spiele.
- GNOME/XFCE-Fensterschließung.

Bis dahin gilt Flatpak-Steam als **experimentell/zurückgestellt**, nicht als
produktiv unterstützt.
