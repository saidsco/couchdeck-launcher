# Desktop-, Autostart- und Discovery-Architektur (0.4)

CouchDeck is a family-friendly, controller-first game launcher for Linux
living-room PCs, independent of a specific distribution or desktop environment.

## Unterstützte Sitzungen

- KDE Plasma, GNOME und XFCE unter Wayland oder X11, soweit eframe/egui und der
  jeweilige Compositor dies unterstützen.
- Bazzite und SteamOS im Desktop Mode sind Zielplattformen, aber keine
  technischen Voraussetzungen.
- Steam Gaming Mode/Gamescope ist derzeit nicht zugesichert und erhält keinen
  automatischen Autostart.

Die tatsächlich getesteten Kombinationen stehen in
[`DESKTOP-TEST-MATRIX.md`](DESKTOP-TEST-MATRIX.md).

## Discovery-Quellen und Priorität

Discovery läuft in einem Hintergrund-Thread. Das UI bleibt bedienbar, auch wenn
ein Verzeichnis langsam oder nicht erreichbar ist. Die Suche ist auf eine
begrenzte Tiefe beschränkt und folgt keinen Symlink-Verzeichnissen.

Priorität (bei gleicher Desktop-ID gewinnt die frühere Quelle):

1. `$XDG_DATA_HOME/applications`
2. Flatpak-Benutzerexport `$XDG_DATA_HOME/flatpak/exports/share/applications`
3. Einträge aus `$XDG_DATA_DIRS` in ihrer Reihenfolge, jeweils `applications`
4. Flatpak-Systemexport `/var/lib/flatpak/exports/share/applications`
5. Tatsächlicher XDG-Desktopordner aus `$XDG_CONFIG_HOME/user-dirs.dirs`
6. Administrator-Suchverzeichnisse aus `settings.toml`
7. CouchDeck-verwaltete Dateien in `$XDG_DATA_HOME/couchdeck/managed-desktop`

Desktop-IDs werden relativ zur Quelle gebildet; Unterverzeichnisse werden mit
`-` verbunden, die `.desktop`-Endung bleibt Teil der ID. Dateinamen mit
Leerzeichen und Sonderzeichen bleiben erhalten. Unterschiedliche Anzeigenamen
führen nie zu einer Zusammenführung.

## Sichtbarkeit und Import

- `Hidden`-, `NoDisplay`-, `OnlyShowIn`-/`NotShowIn`- und fehlende
  `TryExec`-Fälle bleiben in der Admin-Importübersicht sichtbar, werden aber als
  nicht menü-sichtbar markiert und diagnostiziert.
- `NoDisplay=true` verhindert keinen bewussten Admin-Import.
- Jeder Import zeigt Name, Quelle, Pfad, Desktop-ID, rohes `Exec=` und
  Startdiagnostik.
- Neu gefundene Einträge werden nie automatisch importiert.
- Sind Kinderprofile vorhanden, wird ein neuer Import standardmäßig nur für
  Nicht-Kinderprofile sichtbar. Eltern können die Freigabe im Spieleeditor
  explizit erweitern.
- Originaldateien werden nie verändert; der Start bleibt ausschließlich
  `gio launch /absoluter/pfad.desktop`.

## Spielemodell

`GameEntry` behält die stabile CouchDeck-ID. Zusätzlich werden Herkunft
(`desktop_entry`, `managed_desktop`, `steam`, `prism`, `manual`, `other`),
optionale Desktop-ID und Startmethode (`gio_desktop`) gespeichert. Alte
`library.json`-Dateien laden mit Defaults; ein Discovery-Lauf verändert keine
Bibliothekseinträge und entfernt keine fehlenden Quelldateien.

## Autostart

Die Sitzungserkennung liest `XDG_CURRENT_DESKTOP`, `XDG_SESSION_DESKTOP` und
`DESKTOP_SESSION`, zerlegt mehrteilige Werte und erkennt KDE, GNOME und XFCE.
Gamescope/Gaming-Mode-Tokens deaktivieren Autostart. Die erzeugte Datei enthält:

```ini
OnlyShowIn=<KDE|GNOME|XFCE>;
NotShowIn=gamescope;
X-CouchDeck-Desktop=<KDE|GNOME|XFCE>;
```

Unbekannte Sitzungen erhalten keinen Eintrag. Vorhandene Einträge für andere
Sitzungen werden als inaktiv angezeigt. Autostart wird nur über die
Admin-Einstellung oder `install-user.sh --autostart` aktiviert.

## Single Instance

Ein `flock` auf `$XDG_RUNTIME_DIR/couchdeck.lock` (Fallback im State-Verzeichnis)
verhindert parallele UIs. Der Kernel gibt den Lock bei Abstürzen frei. Ein
zweiter Start beendet sich vor jedem Bibliotheks- oder Zeitbudget-Zugriff.

## Grenzen

- Wayland-Compositoren können Fokus und Minimieren anders behandeln als X11.
- Steam Gaming Mode/Gamescope ist keine getestete Zielumgebung.
- Die echte Anmeldung unter GNOME/XFCE/SteamOS Desktop Mode und die
  Controller-/GIO-/systemctl-Tests auf Zielhardware bleiben offen.
- Discovery ist dateibasiert; Prozess-/cgroup-Spielzeittracking, Emulator-Adapter,
  Steam-Reaper oder Kodi/DVD-Integration sind nicht Teil von 0.4.
