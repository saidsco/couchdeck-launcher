# Release-Pakete und Distribution

CouchDeck wird als natives Linux-x86_64-Programm über GitHub Releases verteilt.
Es gibt **kein Flatpak** und **keine Root-Installation**. Beide Paketformen
nutzen dieselben XDG-Datenordner wie eine bestehende Benutzerinstallation.

## Artefakte

Bei einem Tag `v*` erzeugt GitHub Actions:

| Datei | Inhalt |
|---|---|
| `couchdeck-<version>-linux-x86_64.tar.gz` | portables Archiv mit Binary, Icon, Installer/Uninstaller, README, Docs, `runtime-info.txt` |
| `CouchDeck-<version>-x86_64.AppImage` | ausführbares AppImage mit eingebettetem Desktop-Eintrag und Icon |
| `couchdeck-<version>-runtime-info.txt` | Ergebnis der `ldd`-Prüfung, Build-glibc und maximal benötigte GLIBC-Symbolversion |
| `SHA256SUMS` | SHA-256-Prüfsummen für alle veröffentlichten Dateien |

Der Build läuft in einem **Fedora-42-Container** (glibc 2.41). Das erzeugte
Binary ist dynamisch gegen die System-glibc gelinkt; eine glibc-Version, die
mindestens der dokumentierten `max_required_glibc_symbol`-Version entspricht,
ist erforderlich. Der aktuelle Stand referenziert höchstens `GLIBC_2.39`. Die genaue Version steht in `runtime-info.txt` des jeweiligen
Releases.

## Warum kein Flatpak?

CouchDeck soll direkt auf die aktive KDE-/GNOME-/XFCE-Sitzung, GIO,
Controller-Devices, PipeWire/PulseAudio und systemctl/Polkit der Host-Session
zugreifen.
Eine Flatpak-Sandbox würde zusätzliche Portale, Berechtigungen und
Integrationsarbeit erfordern. Das AppImage bleibt eine normale
Benutzeranwendung ohne Sandbox und ohne `sudo`.

## Automatischer Release-Build

`.github/workflows/release.yml` läuft bei Tags `v*` und kann zusätzlich manuell
für einen vorhandenen Tag gestartet werden. Der Workflow:

1. prüft, dass der Tag exakt `v<Cargo.toml-Version>` entspricht,
2. baut und testet in Fedora 42 (Rust 1.89, `cargo fmt`, Clippy, Tests),
3. prüft `ldd` und erzeugt den Runtime-Report,
4. packt das portable tar.gz und baut das AppImage mit dem gepinnten `appimagetool`-Build-Werkzeug,
5. prüft das AppImage mit `--version` ohne Rust-Toolchain,
6. erzeugt `SHA256SUMS` und veröffentlicht die Assets über GitHub Releases.

## Lokal reproduzieren

Auf einem Fedora-42-System oder in einem passenden Container:

```bash
sudo dnf install -y \
  cargo rust clippy rustfmt gcc gcc-c++ pkgconf-pkg-config \
  systemd-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel \
  mesa-libGL-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel \
  glib2 librsvg2-tools desktop-file-utils file binutils curl tar gzip

cargo build --release --locked
scripts/check-runtime.sh target/release/couchdeck dist/runtime-info.txt

# Gepinntes appimagetool aus dem linuxdeploy-Build-Werkzeug holen
APPIMAGETOOL="$(scripts/fetch-release-tools.sh /tmp/couchdeck-release-tools)"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)" \
APPIMAGETOOL="$APPIMAGETOOL" \
scripts/package-release.sh
```

Die Release-Pakete selbst enthalten **keinen Rust-Compiler** und benötigen
keine Build-Werkzeuge. `appimagetool` wird nur im Build verwendet.

## Laufzeitabhängigkeiten

`ldd` auf dem Fedora-42-Release-Binary zeigt die direkt gelinkten Bibliotheken
(z. B. glibc, `libm`, `libgcc_s`, `libudev`, `libcap`). Die Wayland-/X11-/GL-
Bibliotheken werden von `winit`/`glutin` zur Laufzeit dynamisch geladen und
erscheinen daher nicht zwingend in `ldd`. Auf KDE Plasma, GNOME und XFCE werden
sie von der Desktop-Sitzung bereitgestellt:

- `libwayland-client.so.0`, `libxkbcommon.so.0`
- `libX11`, `libXcursor`, `libXrandr`, `libXi`, `libxcb`
- `libGL`/`libEGL` und der jeweils aktive Mesa-/NVIDIA-Treiber
- `libudev.so.1`, `libcap.so.2`

Das AppImage verpackt die **unveränderte native Binary** und die Desktop-/Icon-
Metadaten. glibc, GPU-Treiber, Wayland/X11 und `libudev` kommen bewusst von
dem Host, damit es keine Treiber-/ABI-Konflikte durch mitgelieferte
Systembibliotheken gibt. Es ist **keine** Flatpak-Sandbox. Wenn FUSE auf dem
System fehlt, lässt es sich mit `APPIMAGE_EXTRACT_AND_RUN=1` starten.

## Installation, Update, Deinstallation

### Archivinstallation

```bash
sha256sum -c SHA256SUMS
tar -xzf couchdeck-<version>-linux-x86_64.tar.gz
./couchdeck-<version>-linux-x86_64/scripts/install-user.sh
```

Installiert werden nur:

- `~/.local/bin/couchdeck`
- `~/.local/share/applications/couchdeck.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/couchdeck.svg`
- optional ein desktopbezogener XDG-Autostart-Eintrag mit `--autostart`

Bibliothek, Profile, PIN-Hash, Artwork und Einstellungen werden **nicht**
überschrieben. Ein Update ist daher derselbe Aufruf mit dem neuen Archiv.

### AppImage

```bash
chmod +x CouchDeck-<version>-x86_64.AppImage
./CouchDeck-<version>-x86_64.AppImage
```

Das AppImage kann beliebig abgelegt werden und nutzt dieselben XDG-Datenordner.
Es trägt sich nicht automatisch in das Desktop-Menü ein.

### Deinstallation

```bash
./scripts/uninstall-user.sh
```

Entfernt Binary, Menüeintrag, Icon und Autostart; Benutzerdaten bleiben
erhalten. `--purge` löscht zusätzlich Bibliothek, Profile, Artwork,
Einstellungen, Cache und Logs nach expliziter Bestätigung.

## Integrität und Sicherheit

- `SHA256SUMS` wird nach dem Packaging aus den finalen Dateien erzeugt.
- Der Tag-Commit und die Release-Assets stammen aus demselben Workflow-Lauf.
- Es werden keine root-Rechte, `rpm-ostree`-Layer, Flatpaks oder
  Netzwerkdienste für den Betrieb benötigt.
- Die üblichen Grenzen der lokalen Admin-PIN bleiben bestehen; sie ist keine
  OS-Sicherheitsgrenze.

## Zielhardware- und Desktop-Test

Der automatisierte Build testet das AppImage in einem sauberen Fedora-42-
Container (Start, `--version`, dynamische Bibliotheken). Ein echter Test auf
einer frischen Bazzite-/KDE-Installation und den in der Desktop-Testmatrix
genannten GNOME-/XFCE-/SteamOS-Kombinationen ist ein separater manueller Test.
Die Bazzite-Checkliste steht in
[`BAZZITE-RELEASE-TEST.md`](BAZZITE-RELEASE-TEST.md).
