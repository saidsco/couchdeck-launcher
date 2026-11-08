# CouchDeck

> **CouchDeck ist ein familienfreundlicher, controller-first Game-Launcher für Linux-Wohnzimmer-PCs, unabhängig von einer bestimmten Distribution oder Desktopumgebung.**

Die App startet direkt in eine persönliche Spielebibliothek – kein Shop, keine News, keine Cloud, kein Overlay und keine Spieleplattform-Accounts.

Primäre Desktop-Ziele sind **KDE Plasma, GNOME und XFCE** unter Wayland oder X11. Bazzite und SteamOS im Desktop Mode sind unterstützte Zielplattformen, aber keine technischen Voraussetzungen. Steam Gaming Mode/Gamescope ist derzeit kein zugesichertes Ziel.

**Aktuelle Version:** 0.4.6 · **Sprache:** Deutsch · [English](README.md)

Die Anwendungsoberfläche ist über Fluent **auf Deutsch und Englisch** verfügbar, mit automatischer Spracherkennung und Englisch als Fallback. Einzelne Detaildokumente unter `docs/` sind weiterhin auf Deutsch.

Gespielt wird ausschließlich über vorhandene `.desktop`-Dateien und `gio launch`. Coverbilder werden getrennt von den Desktop-Dateien in CouchDeck verwaltet; die Originaldateien werden nie verändert.

![Bibliothek](docs/screenshots/library.png)

## Funktionsumfang

- **Manuelle Fensteraktivierung**: `couchdeck --activate` kontaktiert die bereits laufende Instanz über einen benutzerspezifischen Unix-Socket, ohne eine zweite UI zu starten. Unter Wayland wird ein vom Aufrufer bereitgestelltes `XDG_ACTIVATION_TOKEN` an die bestehende Oberfläche weitergegeben; für KDE/Wayland gibt es einen begrenzten KWin-Scripting-Fallback für Terminal-/No-Token-Aufrufe. Beim Start von Spielen/Steam/Launchern wird der Fokus nie automatisch gestohlen. `--diagnose-window` protokolliert Fokus-, Minimierungs- und Fullscreen-Wechsel.
- **Desktopunabhängiger Kern**: keine KDE-only-Laufzeitvoraussetzung. KDE Plasma, GNOME und XFCE unter Wayland/X11 sind die unterstützten Desktop-Ziele; Compositor-Grenzen bei Fokus/Minimieren werden dokumentiert.
- **Native Release-Pakete**: portables x86_64-tar.gz und ausführbares AppImage über GitHub Releases, mit SHA-256-Prüfsummen, `ldd`/glibc-Runtime-Report, ohne Root und ohne Rust-Compiler bei der Installation.
- **Zweisprachige UI (de/en)** über Fluent: automatische Erkennung aus `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG`, Englisch als Fallback, live umschaltbar unter Admin → Einstellungen.
- **Fullscreen-Bibliothek** mit Cover-Grid, 3:4-Center-Crop, Fokusanimation und Fallback-Reihenfolge eigenes Cover → Desktop-Icon → neutrale Kachel.
- **Gamepad-Navigation** über `gilrs`: D-Pad und linker Stick mit Deadzone, Hold-Repeat, Debouncing und Hotplug. South = bestätigen/starten, East = zurück, Start = Menü. LB/RB (Schultertasten) öffnen die Profilübersicht; dort wird zwischen Profilen gewechselt. Navigation wird nur verarbeitet, solange CouchDeck den Fensterfokus hat; Connect/Disconnect werden weiterhin erkannt, alte Eingaben werden beim Fokuswechsel verworfen.
- **Tastatur und Maus parallel**: Pfeiltasten, Enter/Leertaste, Escape, M, F11, Mausklick und Doppelklick.
- **Erweiterte Desktop-Discovery**: Benutzer-XDG, alle `XDG_DATA_DIRS`, Flatpak-Exports für Benutzer/System, der echte XDG-Desktopordner aus `user-dirs.dirs`, CouchDeck-verwaltete Dateien und zusätzliche Admin-Suchverzeichnisse. XDG-Überlagerung, Desktop-IDs, Symlinks, lokalisierte Namen und Dateinamen mit Leerzeichen werden berücksichtigt.
- **Desktop-Dateien**: Import beliebiger Dateien, Vorschau von Name, Quelle, Pfad, Desktop-ID und rohem `Exec`-Inhalt; Unterstützung für `NoDisplay`, `Hidden`, `TryExec`, `Path`, `DBusActivatable`, `OnlyShowIn`, `NotShowIn`, Kategorien und lokalisierte Namen. `NoDisplay`-Einträge können vom Admin bewusst importiert werden.
- **Suchpfad-Verwaltung**: Admin → Suchpfade zeigt automatische Quellen, erlaubt das Hinzufügen/Entfernen zusätzlicher Verzeichnisse über einen controllerbedienbaren Ordnerdialog, scannt im Hintergrund neu und zeigt fehlende/unlesbare Pfade.
- **Start über GIO**: ausschließlich `gio launch /absoluter/pfad.desktop`. `Exec=` wird nicht zum Starten interpretiert; der rohe Inhalt dient nur der Anzeige und Diagnose.
- **Eigene Artwork-Verwaltung**: Cover, Spiel-Hintergrund, globales Hintergrundbild, Avatar und **Profil-Hintergrund** per Dateiauswahl. Profil-Hintergründe fallen auf das globale Hintergrundbild zurück; Spiel-Hintergründe und vorhandene Daten bleiben erhalten. Import nach `$XDG_DATA_HOME/couchdeck/assets/`, Vorschaucache unter `$XDG_CACHE_HOME/couchdeck/`.
- **Lokale Profile**: Anzeigename, Rolle (Admin/Spieler), Avatar, optionales Kinderprofil, tägliches Zeitbudget, eigener Profil-Hintergrund und profilabhängige Sichtbarkeit von Spielen. Keine Linux-Konten, keine Cloud. Neue Importe sind nie automatisch für Kinderprofile sichtbar.
- **Zukunftsfähiges Spielemodell**: stabile Spiele-ID plus Herkunft (`desktop_entry`, `managed_desktop`, `steam`, `prism`, `manual`) und Startmethode (`gio_desktop`). Bestehende 0.3.1-Bibliotheken laden mit Defaults.
- **Kinderprofile und Zeitkontingente**: messen aktive Profilzeit, solange CouchDeck läuft, zeigen Restzeit, Warnungen bei 5/1 Minuten, optionalen Best-Effort-Alarmton und sperren neue Spielstarts zentral bei erschöpftem Budget. Laufende Spiele werden nicht zwangsweise geschlossen. Profilwechsel, Minimieren, Beenden und Systemaktionen erfordern bei Kinderprofilen die Admin-PIN.
- **Elternfreigabe „Mehr Spielzeit“**: aus dem Startmenü oder dem Bildschirm „Zeitkontingent verbraucht“, Admin-PIN, dann +5/+15/+30 oder benutzerdefinierte Minuten. Zusatzzeit wird pro Kalendertag gespeichert, addiert sich, verändert das reguläre Budget nicht und kann vom Admin zurückgesetzt werden.
- **Admin-Bereich**: Spiele importieren/ausblenden/entfernen/sortieren, Titel und Artwork ändern, Profile verwalten, Sichtbarkeit je Profil, Einstellungen, PIN-Verwaltung.
- **Admin-PIN**: Argon2id-Hash in separater Datei mit Modus `0600`; Eingabe per Tastatur und On-Screen-Ziffernfeld mit Controller. Der Profilwechsel selbst ist keine Authentifizierung; Admin-Aktionen werden bei gesetzter PIN separat entsperrt.
- **Steam-Flatpak-Reparatur**: erkennt `Exec=steam steam://rungameid/ID`, ändert nichts automatisch und erzeugt nach ausdrücklicher Bestätigung eine **separate verwaltete** Datei mit `flatpak run com.valvesoftware.Steam …`.
- **Prism-Launcher**: Import vorhandener Instanz-Desktop-Dateien; zusätzlich kann CouchDeck eine verwaltete Datei mit `flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID` erzeugen. `INSTANCE_ID` ist der Instanzordner, nicht der sichtbare Name.
- **Autostart (Opt-in, desktopbezogen)**: XDG-Autostart für die erkannte KDE-, GNOME- oder XFCE-Sitzung. Die Datei nutzt `OnlyShowIn=<erkannte Sitzung>` und `NotShowIn=gamescope`; unbekannte Sitzungen und Steam Gaming Mode/Gamescope werden nie geraten. Verwaltung über Admin → Einstellungen oder `install-user.sh --autostart`.
- **Zum Desktop minimieren**: verlässt zuerst den Fullscreen und minimiert dann das Fenster, damit Panel/Taskleiste unter KDE, GNOME, XFCE und anderen Desktops erreichbar bleiben. Beim Zurückholen aus der Taskleiste wird Fullscreen wieder aktiviert, wenn CouchDeck zuvor im Fullscreen lief.
- **Power-Menü**: CouchDeck beenden, System neu starten oder herunterfahren – mit Bestätigungsdialog und festen `systemctl`-Aufrufen (kein Shell-Einsatz).
- **Robuste Persistenz**: atomisches Schreiben, Backups bei beschädigten Dateien, keine stillen Überschreibungen, Logs unter `$XDG_STATE_HOME/couchdeck/`. Ein benutzerspezifischer Instanz-Lock verhindert, dass Autostart und manueller Start zwei UIs auf dieselbe Bibliothek/dasselbe Zeitbudget loslassen.

Weitere Screenshots: [Zeitkontingent verbraucht](docs/screenshots/time-budget.png), [Profilansicht](docs/screenshots/profiles.png), [Bibliothek als Spielerprofil](docs/screenshots/library-profile-mia.png), [Admin](docs/screenshots/admin.png), [Spieleeditor](docs/screenshots/game-editor.png), [Import](docs/screenshots/import.png), [PIN-Pad](docs/screenshots/pinpad.png), [Start-Overlay](docs/screenshots/launch.png), [Power-Menü](docs/screenshots/menu.png), [Autostart-Einstellungen](docs/screenshots/autostart-settings.png), [Beschädigte Daten](docs/screenshots/corrupt-data.png).

## Voraussetzungen

- Eine unterstützte Linux-Desktop-Sitzung (KDE Plasma, GNOME oder XFCE) unter Wayland oder X11. Bazzite und SteamOS Desktop Mode sind Zielplattformen; Steam Gaming Mode/Gamescope ist nicht zugesichert.
- GLib/GIO (`gio`), ein funktionierendes OpenGL-Setup und udev/libudev für `gilrs`; auf allen Desktop-Zielen erforderlich.
- systemd/logind mit `systemctl` für Neustart/Herunterfahren auf Distributionen, die es bereitstellen; Polkit muss die Aktion für die aktive Sitzung erlauben. Andere Init-Systeme werden für Power-Aktionen nicht unterstützt.
- Rust stable **≥ 1.85** zum Bauen aus dem Quellcode. Getestet mit Rust 1.89.
- Für fertige Releases: Linux x86_64 mit mindestens der in `runtime-info.txt` dokumentierten glibc-Version (gebaut auf Fedora 42 / glibc 2.41) sowie den üblichen KDE-/GNOME-/XFCE-Bibliotheken für Wayland/X11. Kein Rust-Compiler erforderlich.
- Kein root, kein `rpm-ostree`-Layer, kein Flatpak für CouchDeck selbst.
- Für Steam-/Prism-Integrationen müssen die jeweiligen Flatpaks auf dem Host installiert sein.

## Installation aus GitHub Releases (Linux-Desktop, ohne Rust)

Fertige Linux-x86_64-Pakete werden unter
[github.com/saidsco/couchdeck-launcher/releases](https://github.com/saidsco/couchdeck-launcher/releases)
veröffentlicht. Es sind native Anwendungen: **keine Flatpak-Sandbox, keine
Root-Rechte, kein Rust-Compiler**.

Aus dem neuesten Release herunterladen:

- `couchdeck-<version>-linux-x86_64.tar.gz`
- `CouchDeck-<version>-x86_64.AppImage`
- `SHA256SUMS`
- optional: `couchdeck-<version>-runtime-info.txt`

Prüfsummen prüfen (nur heruntergeladene Dateien):

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

### Portables Archiv → Benutzerinstallation

```bash
tar -xzf couchdeck-<version>-linux-x86_64.tar.gz
./couchdeck-<version>-linux-x86_64/scripts/install-user.sh
# optionaler desktopbezogener XDG-Autostart:
./couchdeck-<version>-linux-x86_64/scripts/install-user.sh --autostart
```

Installiert werden `~/.local/bin/couchdeck`, Menüeintrag, Icon und optional der
desktopbezogene Autostart-Eintrag. Bibliothek, Profile, PIN-Hash, Artwork und Einstellungen
unter den XDG-Ordnern werden **nicht angefasst**.

### AppImage

```bash
chmod +x CouchDeck-<version>-x86_64.AppImage
./CouchDeck-<version>-x86_64.AppImage
```

Falls FUSE fehlt:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./CouchDeck-<version>-x86_64.AppImage
```

Das AppImage nutzt dieselben XDG-Datenordner und installiert nichts
systemweit.

### Update

Das neue Archiv mit demselben `install-user.sh` installieren oder die
AppImage-Datei ersetzen. Benutzerdaten liegen getrennt und bleiben erhalten:
Admin-PIN, Profile, importierte Spiele, Cover/Hintergründe und Einstellungen.

### Deinstallation

Release-Archiv aufbewahren oder erneut herunterladen, dann:

```bash
./couchdeck-<version>-linux-x86_64/scripts/uninstall-user.sh
```

Entfernt nur Binary, Menüeintrag, Icon und Autostart. Zum vollständigen Löschen
der Benutzerdaten:

```bash
./couchdeck-<version>-linux-x86_64/scripts/uninstall-user.sh --purge
```

`--purge` verlangt eine explizite Bestätigung.

### Laufzeit und glibc

Der Release wird in Fedora 42 mit glibc 2.41 gebaut; das aktuelle Binary referenziert höchstens `GLIBC_2.39`. Die erzeugte
`runtime-info.txt` dokumentiert Build-glibc, die maximal referenzierte
GLIBC-Symbolversion und den `ldd`-Befund. `ldd` auf dem Release-Binary zeigt
die direkten Abhängigkeiten (glibc, `libm`, `libgcc_s`, `libudev`, `libcap`);
Wayland-/X11-/GL-Bibliotheken werden dynamisch von der Desktop-Sitzung geladen.
Ihre KDE-/GNOME-/XFCE-Sitzung stellt sie auf dem Host bereit. Das AppImage behält bewusst
Host-glibc, GPU-Treiber, Wayland/X11 und `libudev`, um Treiber-/ABI-Konflikte zu
vermeiden.

Details: [docs/RELEASE.md](docs/RELEASE.md), Bazzite-Test:
[docs/BAZZITE-RELEASE-TEST.md](docs/BAZZITE-RELEASE-TEST.md).

## Build aus dem Quellcode (Fedora/Bazzite-Distrobox)

Bitte ein Fedora-Basisimage passend zur **Host-Fedora-Version** verwenden, damit die glibc im Container nicht neuer ist als auf dem Host.

```bash
# Auf dem Host
grep -E '^(NAME|VERSION_ID)=' /etc/os-release

# Beispiel: Host ist Fedora 42
distrobox create \
  --name couchdeck-build \
  --image registry.fedoraproject.org/fedora-toolbox:42

# Falls der toolbox-Tag für euer Release fehlt:
#   --image registry.fedoraproject.org/fedora:42

distrobox enter couchdeck-build
```

Im Container:

```bash
sudo dnf install -y \
  cargo rust clippy rustfmt gcc gcc-c++ pkgconf-pkg-config \
  systemd-devel libxkbcommon-devel wayland-devel mesa-libGL-devel \
  libX11-devel libXcursor-devel libXrandr-devel libXi-devel

cd /pfad/zu/couchdeck
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Falls das Fedora-`rust`-Paket älter als 1.85 ist, im Container rustup verwenden:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
```

`Cargo.lock` wird mitgeliefert und pinnt die getesteten Versionen (u. a. `eframe 0.32.3`, `gilrs 0.11.2`, `image 0.25.10`, `argon2 0.5.3`).

## Installation ohne root

Nach dem Release-Build auf dem **Host** (nicht im Container) aus dem Repo:

```bash
./scripts/install-user.sh
# optional direkt mit desktopbezogenem XDG-Autostart:
./scripts/install-user.sh --autostart
```

Das installiert:

- `~/.local/bin/couchdeck`
- `~/.local/share/applications/couchdeck.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/couchdeck.svg`

Start über die Desktop-Anwendungsliste oder:

```bash
~/.local/bin/couchdeck
```

Entwicklung/Debug:

```bash
~/.local/bin/couchdeck --windowed          # Fenster statt Vollbild
~/.local/bin/couchdeck --no-controller     # Gamepad-Thread deaktivieren
~/.local/bin/couchdeck --activate          # laufende Instanz zurückholen
~/.local/bin/couchdeck --diagnose-window   # ausführliche Fokus-/Fensterlogs
~/.local/bin/couchdeck --help
```

Deinstallation ohne Datenverlust:

```bash
./scripts/uninstall-user.sh
```

Vollständige Löschung von Bibliothek, Profilen, PIN-Hash, Artwork, Cache und Logs nur nach expliziter Bestätigung:

```bash
./scripts/uninstall-user.sh --purge
```

## Fensteraktivierung und Rückkehr

CouchDeck fordert den Fokus nicht automatisch zurück, während ein Spiel, Steam,
Proton, ein Drittanbieter-Launcher oder ein Emulator ein Fenster öffnet. Für die
bewusste Rückkehr:

```bash
~/.local/bin/couchdeck --activate
```

Der Befehl kontaktiert die laufende Instanz über einen lokalen Unix-Socket und
beendet sich. Er startet keine zweite CouchDeck-Instanz und bestätigt nicht,
dass der Compositor den Fokus tatsächlich gewährt. Die CLI unterscheidet
„Activation-Token weitergegeben“ von „kein Activation-Token vorhanden“; in
beiden Fällen gilt der Fokus erst nach einem beobachteten Fokus-Ereignis im Log
als bestätigt.

Unter Wayland wird ein vom Aufrufer bereitgestelltes `XDG_ACTIVATION_TOKEN` an
die laufende Instanz weitergegeben und über `xdg-activation-v1` auf die
bestehende Oberfläche angewendet. Bei KDE/Wayland hat ein Terminalaufruf
üblicherweise keinen Token; deshalb gibt es zusätzlich einen einmaligen
KWin-Scripting-Fallback, der ausschließlich das CouchDeck-Fenster aktiviert.
Er benötigt `qdbus6`/`qdbus` aus den KDE-Werkzeugen. Ein vom Benutzer
eingerichteter KDE-Shortcut oder ein über KDE gestarteter Befehl kann über
KIO/KProcessRunner einen Token liefern und den Fallback vermeiden.

- War CouchDeck minimiert, wird die Wiederherstellung angefordert; Wayland/Winit
  kann selbst nicht entminimieren, daher ist die Compositor-Aktivierung nötig.
- War es zuvor im Fullscreen, wird Fullscreen wiederhergestellt; ein extern
  minimiertes Fullscreen-Fenster wird nicht unnötig umgeschaltet.
- Bei `--windowed` oder manuell verlassenem Fullscreen bleibt Windowed erhalten.
- Ist CouchDeck bereits fokussiert, erfolgt keine Fensteraktion.
- Läuft keine Instanz, endet der Befehl mit verständlichem Fehler und
  Exit-Code ungleich 0.
- Verweigert der Compositor die Aktivierung, zeigen die Logs Token-Status,
  KWin-Fallback und einen Timeout ohne falsche Erfolgsmeldung.

Desktop-Shortcuts werden bewusst **manuell** eingerichtet: in KDE Plasma, GNOME
oder XFCE auf den absoluten Pfad `couchdeck --activate` legen. Genaue Schritte
und Wayland-Grenzen: [docs/WINDOW-ACTIVATION.md](docs/WINDOW-ACTIVATION.md).
Steam Input kann eine Controller-Taste auf einen solchen Shortcut abbilden; das
ist in 0.4.6 nur dokumentierte Vorbereitung, keine globale Controller-Home-
Funktion.

`couchdeck --diagnose-window` schreibt zusätzliche Fokus-/Minimierungs-/
Fullscreen-Wechsel in `$XDG_STATE_HOME/couchdeck/couchdeck.log`.

## Bedienung

| Aktion | Controller | Tastatur/Maus |
|---|---|---|
| Fokus bewegen | D-Pad / linker Stick | Pfeiltasten, Maus |
| Starten / bestätigen | A (South) | Enter / Leertaste, Doppelklick |
| Zurück / schließen | B (East) | Escape |
| Menü | Start | M |
| Profile wechseln | LB/RB (öffnet Profilansicht, dort wechseln) | PageUp/PageDown |
| Zum Desktop minimieren | Menü → Zum Desktop minimieren | Menü |
| CouchDeck aktivieren/zurückholen | – | `couchdeck --activate` / Desktop-Shortcut |
| Vollbild umschalten | – | F11 |
| Mehr Spielzeit anfragen (Kinderprofil) | Start → Mehr Spielzeit anfragen | Menü |
| System neu starten / herunterfahren | Menü → System … | Menü |
| CouchDeck beenden | Menü → CouchDeck beenden | Strg+Q |

Die Bibliothek zeigt nur Cover, Titel und bei Fokus eine reduzierte Aktionsleiste. LB/RB öffnen aus der Bibliothek die Profilübersicht; direkt gewechselt wird dort.

## Autostart, Kiosk-Betrieb und Systemaktionen

CouchDeck ist eine normale Desktop-Anwendung und ersetzt keine Desktop-Sitzung. Autostart ist XDG-basiert und opt-in. Die erzeugte Datei ist an die erkannte Sitzung gebunden:

- `OnlyShowIn=KDE;`, `OnlyShowIn=GNOME;` oder `OnlyShowIn=XFCE;`
- `NotShowIn=gamescope;`
- `X-CouchDeck-Desktop=<erkannte Sitzung>`

Unbekannte Desktop-Strings und Steam Gaming Mode/Gamescope erzeugen keinen Autostart-Eintrag. Eine alte Datei für eine andere Sitzung wird als inaktiv angezeigt und kann entfernt werden. Das Installationsskript schreibt Autostart nur mit `--autostart` und erkannter unterstützter Sitzung.

- **Autostart aktivieren**: Admin → Einstellungen → „Beim Anmelden automatisch starten (KDE, GNOME oder XFCE)“. Alternativ `./scripts/install-user.sh --autostart` aus einer unterstützten Desktop-Sitzung.
- **Autostart-Datei**: `$XDG_CONFIG_HOME/autostart/couchdeck.desktop` (normalerweise `~/.config/autostart/couchdeck.desktop`).
- **Vollbild-Overlay**: Beim Start wird das Fenster rahmenlos im Vollbild geöffnet. Der Desktop bleibt im Hintergrund aktiv.
- **Zum Desktop**: Das Menü „Zum Desktop minimieren“ beendet zuerst den Vollbildmodus und minimiert danach das Fenster. Über die Taskleiste kann CouchDeck wiederhergestellt werden. Falls ein Wayland-Compositor das Minimieren ignoriert, ist das Fenster bereits im Fenstermodus und bleibt über Taskleiste/Alt+Tab erreichbar.
- **Beenden**: „CouchDeck beenden“ im Start-Menü beendet nur den Launcher, keine bereits gestarteten Spiele.
- **Neu starten / Herunterfahren**: CouchDeck fragt vorher nach und ruft anschließend `systemctl --no-block --no-wall reboot` bzw. `systemctl --no-block --no-wall poweroff` auf. Die Berechtigung wird von systemd/logind über Polkit geprüft; eine Polkit-Abfrage kann erscheinen.

Der Autostart ist eine bewusste Opt-in-Funktion und wird bei der Deinstallation über `scripts/uninstall-user.sh` entfernt. Die Bibliothek, Profile und Artwork bleiben dabei erhalten.

## Erste Schritte

1. **Admin-Bereich öffnen**: Menü → Admin-Bereich. Beim ersten Start ist ein lokales Admin-Profil `Spieler 1` vorhanden. Name und Avatar lassen sich unter Admin → Profile ändern.
2. **Spiel importieren**: Admin → Spiele → *Desktop-Datei importieren*. Name, Pfad und roher `Exec`-Inhalt werden vor der Freigabe angezeigt. Alternativ eine beliebige Datei über den Dateibrowser auswählen.
3. **Cover setzen**: Spiel auswählen → *Cover wählen*. Das Bild wird kopiert und als eigener Override gespeichert; `Icon=` in der `.desktop`-Datei bleibt unverändert.
4. **Admin-PIN setzen** (empfohlen): Admin → PIN & Sicherheit → *PIN setzen*. Die PIN ist eine lokale UI-Sperre, keine Betriebssystem-Sicherheitsgrenze.
5. **Starten**: In der Bibliothek A/Enter drücken. CouchDeck übergibt die Datei an GIO und zeigt bewusst keinen „Spiel läuft“-Status.

## Steam und Prism

### Steam als Flatpak

Ein vorhandener Shortcut mit `Exec=steam steam://rungameid/12345` wird im Spieleeditor und unter Admin → Steam/Prism erkannt. CouchDeck ändert die Originaldatei nicht. Nach Bestätigung wird eine separate Datei unter `$XDG_DATA_HOME/couchdeck/managed-desktop/` erzeugt:

```ini
Exec=flatpak run com.valvesoftware.Steam steam://rungameid/12345
```

Der neue Eintrag wird zusätzlich in die Bibliothek aufgenommen; der ursprüngliche Eintrag bleibt erhalten. Ist `com.valvesoftware.Steam` nicht installiert, wird die Reparatur abgelehnt.

### Prism Launcher

Vorhandene `.desktop`-Dateien mit:

```ini
Exec=flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID
```

können direkt importiert werden. Beim Import wird geprüft, ob der Instanzordner existiert:

- `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/<ID>`
- `~/.local/share/PrismLauncher/instances/<ID>`
- `~/.var/app/org.prismlauncher.PrismLauncher/.local/share/PrismLauncher/instances/<ID>`

`INSTANCE_ID` ist der **Ordnername**, nicht der sichtbare Instanzname. Unter Admin → Steam/Prism können vorhandene Instanzen ausgewählt und eine verwaltete Desktop-Datei erzeugt werden. Das Cover wird anschließend wie bei jedem anderen Eintrag gesetzt und ist völlig unabhängig vom Desktop-Icon.

Beispieldateien liegen unter `examples/`: `prism-minecraft.desktop`, `steam-game-flatpak.desktop`, `steam-game-original.desktop` und `library.example.json`.

## Datenformat und Speicherorte

| Zweck | Pfad |
|---|---|
| Einstellungen (inkl. zusätzlicher Suchpfade) | `$XDG_CONFIG_HOME/couchdeck/settings.toml` |
| XDG-Autostart | `$XDG_CONFIG_HOME/autostart/couchdeck.desktop` |
| Instanz-Lock | `$XDG_RUNTIME_DIR/couchdeck.lock` (Fallback `$XDG_STATE_HOME/couchdeck/instance.lock`) |
| Bibliothek/Profile/Spiele | `$XDG_DATA_HOME/couchdeck/library.json` |
| Tagesnutzung/Zusatzzeit | `$XDG_DATA_HOME/couchdeck/timekeeping.json` |
| PIN-Hashes | `$XDG_DATA_HOME/couchdeck/secrets.json` (Modus `0600`) |
| Importierte Bilder | `$XDG_DATA_HOME/couchdeck/assets/{covers,backgrounds,avatars}/` |
| Verwaltete Desktop-Dateien | `$XDG_DATA_HOME/couchdeck/managed-desktop/` |
| Thumbnail-Cache | `$XDG_CACHE_HOME/couchdeck/thumbnails/` |
| Logs | `$XDG_STATE_HOME/couchdeck/couchdeck.log` |

Details: [docs/DATA-FORMAT.md](docs/DATA-FORMAT.md), [Desktop-/Discovery-Architektur](docs/DESKTOP-DISCOVERY.md) und [Desktop-Testmatrix](docs/DESKTOP-TEST-MATRIX.md).

## Sicherheitsmodell

- CouchDeck startet **nur** Dateien, die ein Admin explizit importiert hat. Beim Import werden Name, Pfad und `Exec` angezeigt.
- `Exec=` wird nie von CouchDeck ausgeführt oder zum Starten in Argumente zerlegt. Die Datei geht unverändert an `gio launch`; der rohe Inhalt wird nur lesend angezeigt und für Diagnosehinweise geprüft.
- Der Admin-Bereich und alle Änderungsaktionen sind nur für Admin-Profile sichtbar; Kinderprofile erhalten nie Admin-UI-Zugriff.
- Die PIN schützt vor versehentlichen Änderungen in der UI. Sie ist **keine echte OS-Sicherheitsgrenze**: Wer Zugriff auf dasselbe Linux-Benutzerkonto hat, kann die JSON-Dateien direkt ändern. Für echte Trennung separate Linux-Konten verwenden.
- Power-Aktionen werden nur nach einem Bestätigungsdialog ausgelöst. Es werden feste, argumentgetrennte `systemctl`-Aufrufe verwendet; keine Shell, keine freie Befehlsverkettung.
- Zeitlimits sind eine Familienfunktion, **keine OS-Sandbox**. Wer den Prozess beendet, auf eine TTY wechselt, Dateien im selben Linux-Konto ändert oder Compositor-Shortcuts nutzt, liegt außerhalb des Schutzmodells.
- Keine Netzwerkabhängigkeit, keine Cloud, keine Telemetrie, keine externen Downloads.

## Architektur

```
main.rs        CLI, Logging, eframe-Start, Viewport
app.rs         Zustandsmaschine, Navigation, Aktionen, Persistenz-Orchestrierung
model.rs       Library/Profile/GameEntry/Settings/AdminSecrets
storage.rs     XDG-Persistenz, atomisches Schreiben, Backups, Migration
paths.rs       XDG-Auflösung, Applikations-/Icon-Pfade
desktop.rs     Desktop-Parsing, Discovery, Icon-Auflösung, Steam-/Prism-Diagnose
activation.rs  lokaler --activate-Socket, IPC-Server und Aktivierungsprotokoll
wayland_activation.rs  xdg-activation-v1-Token auf bestehende Fläche anwenden
kwin_activation.rs    begrenzter KDE-/KWin-Aktivierungsfallback
instance.rs    benutzerspezifischer flock und stale-lock-sicherer Single-Instance-Schutz
session.rs     Desktop-Erkennung und XDG-Autostart
launch.rs      GIO-Launch im Hintergrundthread, Fehlerdiagnose
system.rs      Systemaktionen, systemctl-Neustart/Herunterfahren, Polkit-Fehlerbehandlung
timekeeping.rs Tagesnutzung, Zusatzminuten, Tageswechsel, Persistenz
i18n.rs        Fluent-Bundles, Locale-Erkennung, Live-Sprachwechsel
sound.rs       Best-Effort-Warnton über System-Player
controller.rs  gilrs-Thread, Deadzone, Repeat, Hotplug, Input-Normalisierung
artwork.rs     Bildimport, Center-Crop, Thumbnail-Cache, Texturen
ui/            Bibliothek, Profile, Admin, Dialoge/Overlays, Zeitdialoge
```

Datenfluss: `UI → AppState → Library/Profiles → GIO (gio launch)`.

## Bekannte Einschränkungen / noch auf Zielhardware zu testen

- **Desktop-Zieltest**: Entwicklung und automatisierte Tests liefen auf Manjaro/KDE/Wayland; die Pakete werden in einem sauberen Fedora-42-Container gebaut und per Smoke-Test gestartet. KDE/Wayland ist nur teilweise getestet. GNOME/Wayland, XFCE/X11, SteamOS Desktop Mode, Wayland-/X11-Fullscreen, NVIDIA-Darstellung und echtes Controller-Hotplug bleiben offen; siehe `docs/DESKTOP-TEST-MATRIX.md`. Die Bazzite-Release-Checkliste steht in `docs/BAZZITE-RELEASE-TEST.md`.
- **Controller-Hardware**: Ein 8BitDo Ultimate wurde unter Manjaro/KDE/Wayland erkannt und war bedienbar (USB-/Bluetooth-Modus noch zu dokumentieren). Xbox-/PlayStation-Controller, GNOME/XFCE und Hotplug/Reconnect auf Bazzite sind noch zu verifizieren. `--no-controller` und „kein Controller gefunden“ wurden geprüft.
- **Kein Spiel-Lebenszyklus**: `gio launch` ist nur ein Startbefehl. CouchDeck erkennt weder zuverlässig, ob ein Spiel läuft, noch wann es endet. Steam/Prism können im Hintergrund weiterlaufen. Zeitbudgets messen daher **aktive Profilzeit, solange CouchDeck läuft**, nicht tatsächliche Spielprozesszeit; eine Spielerkennung bleibt ein späterer Schritt.
- **Zeitbudget-Grenzfälle**: Nutzung wird alle 30 Sekunden, bei Warnungen, Freigaben, Profilwechseln und beim Beenden gespeichert. Ein harter Kill/Stromausfall kann das letzte Intervall verlieren. Läuft der Launcher nicht, wird keine Zeit gezählt. Die Kindersperre umfasst App-Menü-Minimieren/Beenden/Power/Profilwechsel sowie Best-Effort-WM-Close/Minimize, kann aber TTY-Wechsel, Prozess-Kill oder Dateiänderungen im selben Linux-Konto nicht verhindern.
- **Benachrichtigungen/Alarm Best-Effort**: Warnungen erscheinen im CouchDeck-Fenster. Unter Wayland gibt es kein garantiertes Overlay über Fullscreen-Spielen; der optionale Ton nutzt `canberra-gtk-play`/`paplay`/`pw-play`/`aplay`, falls vorhanden, und wird sonst still übersprungen.
- **Fokus nach Spielstart/-ende**: Die Desktop-Sitzung/der Compositor soll das Spielfenster in den Vordergrund holen; unter Wayland ist das Verhalten unterschiedlich. CouchDeck beendet keine Spiele, stiehlt nach einem Start nie automatisch den Fokus und bietet explizites „Zum Desktop minimieren“ sowie `--activate` zur manuellen Rückkehr.
- **Manuelle Aktivierung ist Best-Effort**: `--activate` übermittelt eine lokale IPC-Anforderung und, falls verfügbar, einen Activation-Token. X11-Fenstermanager beachten Fokusanforderungen meist. Unter KDE/Wayland gibt es zusätzlich einen begrenzten KWin-Fallback; GNOME/Wayland kann tokenlose Aktivierung ignorieren. Es gibt keine aggressiven Wiederholungen oder Prozessmanipulation. Nur KDE hat diesen compositorspezifischen Fallback; echte Bazzite-/KDE- und GNOME-Hardwaretests bleiben offen.
- **SVG-Desktop-Icons** werden nicht rasterisiert (kein SVG-Decoder eingebunden). PNG/WebP/JPEG-Icons und eigene Cover funktionieren; sonst greift die neutrale Kachel.
- **Kein Cover-Scraping, keine Steam-API, keine Shop-Anbindung**, keine automatische Suche.
- **Favoriten**: Das Datenfeld ist vorhanden, eine Favoriten-UI ist im MVP nicht implementiert.
- **Prism-Verwaltung**: Das Erzeugen einer verwalteten Datei setzt das Prism-Flatpak voraus. Native Prism-Installationen können importiert werden, wenn eine passende `.desktop`-Datei existiert.
- **Steam Input**: Nicht mit aktivem Steam Input getestet. Steam kann zusätzliche virtuelle Controller erzeugen; bei Doppel-Eingaben ggf. Steam Input für den Launcher deaktivieren.
- **Autostart**: Sitzungserkennung und Entry-Erzeugung sind unit-getestet; echte KDE-/GNOME-/XFCE-Anmeldesitzungen und SteamOS Desktop Mode bleiben zu verifizieren.
- **Minimieren aus Fullscreen**: Der Ablauf „Fullscreen verlassen → minimieren“ ist implementiert. KDE/Wayland wurde auf dem Entwicklungshost getestet; GNOME, XFCE und Bazzite bleiben offen.
- **System Power**: Shutdown/Reboot nutzt `systemctl` und benötigt eine passende Polkit-Berechtigung. Die tatsächliche Ausführung wurde in der Entwicklungsumgebung nicht gestartet, um das System nicht herunterzufahren; geprüft sind Befehlskonstruktion, Bestätigungsdialog und Fehlerbehandlung.
- **Mehrfachstart**: Ein benutzerspezifischer `flock` verhindert eine zweite unabhängige UI. Der Lock wird beim Prozessende/Absturz automatisch freigegeben; eine alte Lock-Datei blockiert den Start nicht.
- **Kein Flatpak für CouchDeck selbst** in v0.4.0, wie spezifiziert. Release-Packaging besteht aus portablem tar.gz und nativem AppImage, nicht aus RPM/Flatpak.
- **Unterstützte Desktop-Sitzungen** sind KDE Plasma, GNOME und XFCE unter Wayland oder X11; Steam Gaming Mode/Gamescope ist kein zugesichertes Ziel. Gamescope-/Steam-Input-Konflikte wurden nicht geprüft.

## Tests und Abnahme

- Unit-/Integrationstests: **83 Tests grün** (`cargo test`: 81 Unit-Tests inkl. Zeitkontingent, i18n, Aktivierungsprotokoll, Eltern-PIN-/Wechsel-/Start-Sperren, plus 2 GIO-Integrationstests).
- `cargo clippy --all-targets -- -D warnings`: sauber.
- `cargo fmt --check`: sauber.
- Release-Build: `cargo build --release --locked`.
- UI-Smoke-Tests und Messungen: [docs/TEST_REPORT.md](docs/TEST_REPORT.md).
- Abnahmematrix: [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md).

## Changelog

- **v0.4.6**: Wayland-/KDE-Aktivierung korrigiert: xdg-activation-Token werden an die bestehende Oberfläche weitergegeben, plus begrenzter KDE-/KWin-Fallback für Terminal-/No-Token-Aufrufe; verbesserte Fokusdiagnose.
- **v0.4.5**: Lokale `--activate`-IPC, Fenster-/Fokusdiagnose, fullscreenbewusste Rückkehr, dokumentierte Desktop-Shortcuts; beim Spielstart wird der Fokus nie automatisch gestohlen.
- **v0.4.1**: Menü-Icon zuverlässig installiert (PNG + absoluter Icon-Pfad) und Fullscreen beim Zurückholen aus dem Minimieren wiederhergestellt.
- **v0.4.0**: Desktopunabhängige KDE-/GNOME-/XFCE-Unterstützung, sicherer XDG-Autostart, erweiterte XDG-/Flatpak-/Desktopordner-Discovery, Admin-Suchpfade, gruppierte Importübersicht, zukunftsfähige Spiele-Herkunft und Single-Instance-Lock.
- **v0.3.1**: GitHub-Release-Automatisierung: portables x86_64-Archiv, natives AppImage, SHA256SUMS, ldd/glibc-Report und Installation ohne Rust.
- **v0.3.0**: Deutsche/englische Fluent-UI mit Live-Wechsel, Profil-Hintergründe, Kinderprofile mit Tagesbudget, Elternfreigabe für Zusatzzeit, zentrale Start-Sperre, Warntöne.
- **v0.2.5**: Controller-Fokus-Gating/Input-Puffer-Reset, korrektes LB/RB-Schultertasten-Mapping, Systemaktionen in `system.rs`.
- **v0.2.0**: KDE-Autostart, „Zu KDE minimieren“, Shutdown/Reboot-Power-Menü, robustere Beenden-/Vollbildlogik.
- **v0.1.0**: erster MVP mit Bibliothek, Profilen, Admin-PIN, Artwork, Steam-/Prism-Integration.

Details: [CHANGELOG.md](CHANGELOG.md).

## Lizenz

Apache License 2.0. Siehe [LICENSE](LICENSE) und [NOTICE](NOTICE).
