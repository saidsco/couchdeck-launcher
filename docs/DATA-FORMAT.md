# CouchDeck Datenformat (Schema v1)

Alle Dateien sind menschenlesbar und versioniert. CouchDeck schreibt Bibliothek und Einstellungen atomar (Temp-Datei → `fsync` → `rename`). Bei beschädigten Dateien wird zuerst eine Sicherungskopie angelegt; unlesbare Daten werden **nie still überschrieben**.

## `$XDG_DATA_HOME/couchdeck/library.json`

```json
{
  "schema_version": 1,
  "active_profile_id": "admin",
  "profiles": [
    {
      "id": "admin",
      "name": "Chris",
      "role": "admin",
      "avatar": "assets/avatars/admin-1234567890.png",
      "favorites": []
    },
    {
      "id": "mia",
      "name": "Mia",
      "role": "player",
      "avatar": null,
      "favorites": []
    }
  ],
  "games": [
    {
      "id": "game-1234567890",
      "desktop_file": "/home/USER/.local/share/applications/prism-minecraft.desktop",
      "title_override": "Minecraft",
      "desktop_name": "Minecraft (Prism-Instanz)",
      "desktop_icon": "org.prismlauncher.PrismLauncher",
      "cover": "assets/covers/game-1234567890-1234567890123.png",
      "background": null,
      "visible_to_profiles": null,
      "hidden": false,
      "sort_order": 10,
      "managed": false,
      "created_at_unix": 1793910000
    }
  ]
}
```

### Felder

| Feld | Bedeutung |
|---|---|
| `schema_version` | Aktuell `1`. Kleinere Versionen werden nach Backup auf `1` angehoben; größere Versionen werden abgelehnt. |
| `active_profile_id` | Zuletzt aktives Profil. Beim Start wird direkt dessen Bibliothek geöffnet. |
| `profiles[].id` | Stabile lokale ID, keine Linux-UID. |
| `profiles[].role` | `admin` oder `player`. Mindestens ein Admin-Profil bleibt erhalten. |
| `profiles[].avatar` | Relativer Pfad unterhalb des CouchDeck-Datenordners oder absoluter Pfad. |
| `profiles[].favorites` | Reserviertes Feld; im MVP noch ohne eigene UI. |
| `games[].desktop_file` | **Absoluter** Pfad zur Original- oder verwalteten `.desktop`-Datei. Wird nie von CouchDeck editiert. |
| `games[].title_override` | Optionaler Anzeigename; leer/NULL = Name aus Desktop-Datei bzw. gespeicherter `desktop_name`. |
| `games[].desktop_name` | Zwischengespeicherter Name, damit ein Eintrag auch bei entfernter Desktop-Datei lesbar bleibt. |
| `games[].desktop_icon` | Zwischengespeicherter `Icon=`-Wert für den Fallback, unabhängig vom eigenen Cover. |
| `games[].cover` | Eigenes Cover, relativ zum CouchDeck-Datenordner. |
| `games[].background` | Optionales spielspezifisches Hintergrundbild. |
| `games[].visible_to_profiles` | `null` = für alle sichtbar; sonst Liste von Profil-IDs. |
| `games[].hidden` | Admin-Ausblendung; die Desktop-Datei bleibt unangetastet. |
| `games[].sort_order` | Aufsteigende Reihenfolge im Grid; `10`-Schritte erleichtern Einfügen. |
| `games[].managed` | `true`, wenn die Datei in `managed-desktop/` von CouchDeck erzeugt wurde. |
| `games[].created_at_unix` | Unix-Zeit der Aufnahme (informativ). |

### Asset-Pfade

Relative Pfade werden gegen `$XDG_DATA_HOME/couchdeck/` aufgelöst. Absolute Pfade bleiben aus Kompatibilitätsgründen gültig. Beim Import werden Bilder nach `assets/covers/`, `assets/backgrounds/` bzw. `assets/avatars/` kopiert und mit einem sicheren, zeitstempelbasierten Dateinamen versehen.

## `$XDG_DATA_HOME/couchdeck/secrets.json`

```json
{
  "schema_version": 1,
  "pins": {
    "admin": "$argon2id$v=19$m=19456,t=2,p=1$...$..."
  }
}
```

- Dateirechte: `0600`.
- Map: Profil-ID → Argon2id-PHC-String.
- Keine Klartext-PINs. Die PIN ist eine lokale UI-Sperre, keine Betriebssystem-Sicherheitsgrenze.
- Bei beschädigter Datei wird eine Sicherung angelegt und die Datei erst beim nächsten Setzen einer PIN neu geschrieben.

## `$XDG_CONFIG_HOME/couchdeck/settings.toml`

```toml
schema_version = 1
animations = true
background = "assets/backgrounds/global-1234567890.png"
columns = 6
show_controller_hints = true
show_backgrounds = true
```

| Feld | Bedeutung |
|---|---|
| `animations` | Dezente Fokus-/Kachelanimation (ca. 180 ms). |
| `background` | Globales Hintergrundbild; relativ zum Datenordner. |
| `columns` | Optionale feste Spaltenzahl 3–6; ohne Wert automatisch nach Fensterbreite. |
| `show_controller_hints` | Blendet Controller-Hinweise in Bibliothek/Profilen ein. |
| `show_backgrounds` | Hintergrundbilder an/aus. |

Ungültige `settings.toml` führt zu Standardwerten im Speicher und einer Sicherungskopie; die Datei wird erst bei einer expliziten Einstellungsänderung neu geschrieben.

## Verwaltete Desktop-Dateien

`$XDG_DATA_HOME/couchdeck/managed-desktop/` enthält ausschließlich von CouchDeck erzeugte Dateien, z. B.:

```ini
[Desktop Entry]
Type=Application
Name=Steam-Spiel (Flatpak)
Exec=flatpak run com.valvesoftware.Steam steam://rungameid/12345
Icon=com.valvesoftware.Steam
Terminal=false
Categories=Game;
StartupNotify=true
X-CouchDeck-Managed=true
```

Originaldateien werden weder verändert noch überschrieben. Der Start erfolgt weiterhin über `gio launch` auf den absoluten Pfad.

## KDE-Autostart

`$XDG_CONFIG_HOME/autostart/couchdeck.desktop` wird nur auf ausdrücklichen Wunsch erzeugt:

```ini
[Desktop Entry]
Type=Application
Name=CouchDeck
Exec="/home/USER/.local/bin/couchdeck" --autostart
Icon=couchdeck
Terminal=false
Categories=Game;
StartupNotify=false
OnlyShowIn=KDE;
X-GNOME-Autostart-enabled=true
X-KDE-autostart-after=panel
X-CouchDeck-Autostart=true
```

- `OnlyShowIn=KDE;` verhindert den Start im Steam Gaming Mode/Gamescope.
- Der Pfad wird aus dem aktuell laufenden Binary erzeugt.
- Deinstallation entfernt den Eintrag; die Deinstallation mit `--purge` löscht zusätzlich die Benutzerdaten.

## Cache und Logs

- `$XDG_CACHE_HOME/couchdeck/thumbnails/`: abgeleitete PNG-Thumbnails, Schlüssel enthält Quellpfad, Änderungszeit, Größe und Zielformat. Kann jederzeit gelöscht werden.
- `$XDG_STATE_HOME/couchdeck/couchdeck.log`: Append-Log; keine Secrets.
