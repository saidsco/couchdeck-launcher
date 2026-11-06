# CouchDeck

CouchDeck is a native, controller-first fullscreen game launcher for **Bazzite / KDE Plasma / Wayland**. It opens directly into a personal game library – no store, no news, no cloud, no overlay, and no game-platform accounts.

**Current version:** 0.2.5 · **Language:** English · [Deutsch](README-de.md)

**Note:** the application UI and the detailed documents in `docs/` are currently in German.

Games are launched exclusively through existing `.desktop` files and `gio launch`. Cover images are managed separately from the desktop files; the original files are never modified.

![Library](docs/screenshots/library.png)

## Features

- **Fullscreen library** with a cover grid, 3:4 center crop, focus animation, and fallback order: custom cover → desktop icon → neutral tile.
- **Gamepad navigation** via `gilrs`: D-pad and left stick with deadzone, hold-repeat, debouncing, and hotplug. South = confirm/start, East = back, Start = menu. LB/RB (shoulder buttons) open the profile overview; profile switching happens there. Navigation is only processed while CouchDeck has window focus; connect/disconnect events are still handled, and stale input is discarded on focus changes.
- **Keyboard and mouse in parallel**: arrow keys, Enter/Space, Escape, M, F11, mouse click, and double-click.
- **Desktop files**: XDG discovery, arbitrary file import, preview of name, path, and raw `Exec` content; support for `NoDisplay`, `Hidden`, `TryExec`, `Path`, `DBusActivatable`, categories, and localized names.
- **Launch via GIO**: only `gio launch /absolute/path.desktop`. `Exec=` is not interpreted for launching; the raw content is only shown and checked for diagnostics.
- **Custom artwork management**: cover, per-game background, and global background via a file picker; imports go to `$XDG_DATA_HOME/couchdeck/assets/`, thumbnail cache to `$XDG_CACHE_HOME/couchdeck/`.
- **Local profiles**: display name, role (admin/player), avatar, and per-profile game visibility. No Linux accounts, no cloud.
- **Admin area**: import/hide/remove/sort games, edit titles and artwork, manage profiles, per-profile visibility, settings, and PIN management.
- **Admin PIN**: Argon2id hash in a separate file with mode `0600`; input via keyboard and an on-screen numpad with a controller. Profile switching itself is not authentication; admin actions are unlocked separately when a PIN is set.
- **Steam Flatpak repair**: detects `Exec=steam steam://rungameid/ID`, never changes anything automatically, and after explicit confirmation creates a **separate managed** file using `flatpak run com.valvesoftware.Steam …`.
- **Prism Launcher**: import existing instance desktop files; CouchDeck can also generate a managed file using `flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID`. `INSTANCE_ID` is the instance folder, not the visible name.
- **Autostart (opt-in)**: CouchDeck can register itself as a KDE-only XDG autostart entry and then start borderless in fullscreen. Steam Gaming Mode/Gamescope is deliberately not started thanks to `OnlyShowIn=KDE;`. The entry can be managed via Admin → Settings or `install-user.sh --autostart`.
- **Minimize to KDE**: exits fullscreen first and then minimizes the window so KDE/panel/taskbar remain reliably reachable.
- **Power menu**: quit CouchDeck, reboot the system, or shut down the system – with a confirmation dialog and fixed `systemctl` calls (no shell).
- **Robust persistence**: atomic writes, backups for damaged files, no silent overwrites, logs under `$XDG_STATE_HOME/couchdeck/`.

More screenshots: [Profiles](docs/screenshots/profiles.png), [Library as a player profile](docs/screenshots/library-profile-mia.png), [Admin](docs/screenshots/admin.png), [Game editor](docs/screenshots/game-editor.png), [Import](docs/screenshots/import.png), [PIN pad](docs/screenshots/pinpad.png), [Launch overlay](docs/screenshots/launch.png), [Power menu](docs/screenshots/menu.png), [Autostart settings](docs/screenshots/autostart-settings.png), [Damaged data](docs/screenshots/corrupt-data.png).

## Requirements

- Bazzite with KDE Plasma on Wayland (target system).
- GLib/GIO (`gio`), a working OpenGL setup, and udev/libudev for `gilrs`.
- systemd/logind with `systemctl` for reboot/shutdown (present on Bazzite); Polkit must allow the action for the active session.
- Rust stable **≥ 1.85**. Tested with Rust 1.89.
- No root, no `rpm-ostree` layer, no Flatpak for CouchDeck itself.
- The respective Flatpaks must be installed on the host for Steam/Prism integration.

## Building on Bazzite (Distrobox)

Use a Fedora base image that matches the **host Fedora version**, so the glibc inside the container is not newer than on the host.

```bash
# On the host
grep -E '^(NAME|VERSION_ID)=' /etc/os-release

# Example: host is Fedora 42
distrobox create \
  --name couchdeck-build \
  --image registry.fedoraproject.org/fedora-toolbox:42

# If the toolbox tag for your release is unavailable:
#   --image registry.fedoraproject.org/fedora:42

distrobox enter couchdeck-build
```

Inside the container:

```bash
sudo dnf install -y \
  cargo rust clippy rustfmt gcc gcc-c++ pkgconf-pkg-config \
  systemd-devel libxkbcommon-devel wayland-devel mesa-libGL-devel \
  libX11-devel libXcursor-devel libXrandr-devel libXi-devel

cd /path/to/couchdeck
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

If the Fedora `rust` package is older than 1.85, use rustup inside the container:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
```

`Cargo.lock` is included and pins the tested versions (including `eframe 0.32.3`, `gilrs 0.11.2`, `image 0.25.10`, `argon2 0.5.3`).

## Rootless installation

After the release build on the **host** (not in the container), run this from the repository:

```bash
./scripts/install-user.sh
# optionally enable KDE autostart right away:
./scripts/install-user.sh --autostart
```

This installs:

- `~/.local/bin/couchdeck`
- `~/.local/share/applications/couchdeck.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/couchdeck.svg`

Start it from the KDE application menu or with:

```bash
~/.local/bin/couchdeck
```

Development/debug:

```bash
~/.local/bin/couchdeck --windowed          # window instead of fullscreen
~/.local/bin/couchdeck --no-controller     # disable the gamepad thread
~/.local/bin/couchdeck --help
```

Uninstall without losing data:

```bash
./scripts/uninstall-user.sh
```

Complete removal of library, profiles, PIN hash, artwork, cache, and logs only after explicit confirmation:

```bash
./scripts/uninstall-user.sh --purge
```

## Controls

| Action | Controller | Keyboard/mouse |
|---|---|---|
| Move focus | D-pad / left stick | Arrow keys, mouse |
| Start / confirm | A (South) | Enter / Space, double-click |
| Back / close | B (East) | Escape |
| Menu | Start | M |
| Switch profile | LB/RB (opens profile view; switch there) | PageUp/PageDown |
| Minimize to KDE | Menu → Minimize to KDE | Menu |
| Toggle fullscreen | – | F11 |
| Reboot / shut down system | Menu → System … | Menu |
| Quit CouchDeck | Menu → Quit CouchDeck | Ctrl+Q |

The library shows only covers, titles, and a reduced action bar on focus. LB/RB open the profile overview from the library; switching happens there.

## Autostart, kiosk mode, and system actions

CouchDeck is intended for a KDE Plasma desktop under Wayland, not for Steam Gaming Mode. The autostart entry therefore contains `OnlyShowIn=KDE;` and is ignored in a Gamescope/Gaming Mode session.

- **Enable autostart**: Admin → Settings → “Start automatically on login (KDE only)”. Alternatively `./scripts/install-user.sh --autostart`.
- **Autostart file**: `$XDG_CONFIG_HOME/autostart/couchdeck.desktop` (normally `~/.config/autostart/couchdeck.desktop`).
- **Fullscreen overlay**: On startup, the window opens borderless in fullscreen. KDE remains active in the background.
- **To KDE**: The “Minimize to KDE” menu item exits fullscreen first and then minimizes the window. CouchDeck can be restored from the KDE taskbar. If a Wayland compositor ignores the minimize request, the window is already in windowed mode and remains reachable via taskbar/Alt+Tab.
- **Quit**: “Quit CouchDeck” in the Start menu only exits the launcher, not already running games.
- **Reboot / shut down**: CouchDeck asks for confirmation and then calls `systemctl --no-block --no-wall reboot` or `systemctl --no-block --no-wall poweroff`. Authorization is handled by systemd/logind via Polkit; a Polkit prompt may appear.

Autostart is a deliberate opt-in feature and is removed by `scripts/uninstall-user.sh`. The library, profiles, and artwork remain intact.

## First steps

1. **Open the admin area**: Menu → Admin Area. On first start, a local admin profile `Player 1` exists. Name and avatar can be changed under Admin → Profiles.
2. **Import a game**: Admin → Games → *Import desktop file*. Name, path, and raw `Exec` content are shown before approval. Alternatively, choose any file via the file browser.
3. **Set a cover**: Select a game → *Choose cover*. The image is copied and stored as a custom override; `Icon=` in the `.desktop` file remains unchanged.
4. **Set an admin PIN** (recommended): Admin → PIN & Security → *Set PIN*. The PIN is a local UI lock, not an operating-system security boundary.
5. **Start**: Press A/Enter in the library. CouchDeck hands the file to GIO and deliberately does not show a “game is running” status.

## Steam and Prism

### Steam as Flatpak

An existing shortcut with `Exec=steam steam://rungameid/12345` is detected in the game editor and under Admin → Steam/Prism. CouchDeck does not modify the original file. After confirmation, a separate file is created under `$XDG_DATA_HOME/couchdeck/managed-desktop/`:

```ini
Exec=flatpak run com.valvesoftware.Steam steam://rungameid/12345
```

The new entry is additionally added to the library; the original entry remains. If `com.valvesoftware.Steam` is not installed, the repair is refused.

### Prism Launcher

Existing `.desktop` files with:

```ini
Exec=flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID
```

can be imported directly. During import, CouchDeck verifies that the instance folder exists:

- `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/<ID>`
- `~/.local/share/PrismLauncher/instances/<ID>`
- `~/.var/app/org.prismlauncher.PrismLauncher/.local/share/PrismLauncher/instances/<ID>`

`INSTANCE_ID` is the **folder name**, not the visible instance name. Under Admin → Steam/Prism, existing instances can be selected and a managed desktop file can be created. The cover is then set like for any other entry and is completely independent of the desktop icon.

Example files are in `examples/`: `prism-minecraft.desktop`, `steam-game-flatpak.desktop`, `steam-game-original.desktop`, and `library.example.json`.

## Data format and locations

| Purpose | Path |
|---|---|
| Settings | `$XDG_CONFIG_HOME/couchdeck/settings.toml` |
| KDE autostart | `$XDG_CONFIG_HOME/autostart/couchdeck.desktop` |
| Library/profiles/games | `$XDG_DATA_HOME/couchdeck/library.json` |
| PIN hashes | `$XDG_DATA_HOME/couchdeck/secrets.json` (mode `0600`) |
| Imported images | `$XDG_DATA_HOME/couchdeck/assets/{covers,backgrounds,avatars}/` |
| Managed desktop files | `$XDG_DATA_HOME/couchdeck/managed-desktop/` |
| Thumbnail cache | `$XDG_CACHE_HOME/couchdeck/thumbnails/` |
| Logs | `$XDG_STATE_HOME/couchdeck/couchdeck.log` |

Details: [docs/DATA-FORMAT.md](docs/DATA-FORMAT.md) (currently in German).

## Security model

- CouchDeck **only** launches files that an admin has explicitly imported. During import, name, path, and `Exec` are shown.
- `Exec=` is never executed by CouchDeck or split into arguments for launching. The file is passed unchanged to `gio launch`; the raw content is only displayed and checked for diagnostic hints.
- The admin area and all modification actions are only visible to admin profiles.
- The PIN protects against accidental UI changes. It is **not a real OS security boundary**: anyone with access to the same Linux user account can modify the JSON files directly. Use separate Linux accounts for real isolation.
- Power actions are only triggered after a confirmation dialog. Fixed, argument-separated `systemctl` calls are used; no shell, no arbitrary command chaining.
- No network dependency, no cloud, no telemetry, no external downloads.

## Architecture

```
main.rs        CLI, logging, eframe startup, viewport
app.rs         state machine, navigation, actions, persistence orchestration
model.rs       Library/Profile/GameEntry/Settings/AdminSecrets
storage.rs     XDG persistence, atomic writes, backups, migration
paths.rs       XDG resolution, application/icon paths
desktop.rs     desktop parsing, discovery, icon resolution, Steam/Prism diagnostics
launch.rs      GIO launch in a background thread, error diagnostics
system.rs      system actions, systemctl reboot/shutdown, Polkit error handling
controller.rs  gilrs thread, deadzone, repeat, hotplug, input normalization
artwork.rs     image import, center crop, thumbnail cache, textures
ui/            library, profiles, admin, dialogs/overlays
```

Data flow: `UI → AppState → Library/Profiles → GIO (gio launch)`.

## Known limitations / still to be tested on target hardware

- **Not tested on Bazzite**: Development and automated tests ran on Manjaro/KDE/Wayland. Wayland fullscreen, KDE focus behavior, real controller hotplug scenarios, 4K/DPI, and NVIDIA Wayland rendering on Bazzite must be verified on the target device.
- **No controller in the test environment**: The logic is unit-tested (`gilrs` mapping, deadzone, repeat); a real Xbox/PlayStation controller was not connected here. `--no-controller` and the “no controller found” case were tested.
- **No game lifecycle tracking**: `gio launch` is only a start command. CouchDeck does not reliably know whether a game is running or when it ends. Steam/Prism may continue running in the background.
- **Focus after game start/exit**: Plasma is expected to bring the game window to the foreground. This has not been verified on the target device. CouchDeck never terminates games and offers an explicit “Minimize to KDE”.
- **SVG desktop icons** are not rasterized (no SVG decoder is bundled). PNG/WebP/JPEG icons and custom covers work; otherwise the neutral tile is used.
- **No cover scraping, no Steam API, no store integration**, no automatic search.
- **Favorites**: the data field exists, but a favorites UI is not implemented in the MVP.
- **Prism management**: generating a managed file requires the Prism Flatpak. Native Prism installations can be imported if a matching `.desktop` file exists.
- **Steam Input**: not tested with Steam Input active. Steam may create additional virtual controllers; if double input occurs, disable Steam Input for the launcher.
- **Autostart**: the XDG autostart entry is KDE-only and has not yet been verified in a real Bazzite KDE login session. It starts fullscreen by default.
- **Fullscreen minimize**: the “leave fullscreen → minimize” flow is implemented, but `ViewportCommand::Minimized` behavior under Bazzite/KDE Wayland still needs testing.
- **System power**: shutdown/reboot uses `systemctl` and requires a suitable Polkit permission. The actual action was not executed in the development environment to avoid shutting down the system; command construction, confirmation dialog, and error handling were tested.
- **Multiple instances**: there is no single-instance lock yet. Autostart plus a manual start can create two instances.
- **No Flatpak for CouchDeck itself** in v0.2.5, as specified. Release packaging consists of the user install script plus binary, not RPM/AppImage.
- **Target session is the KDE Plasma desktop under Wayland**, not Steam Gaming Mode/Gamescope. Gamescope/Steam Input conflicts have not been tested.

## Tests and acceptance

- Unit/integration tests: **49 tests passing** (`cargo test`, including the GIO integration test and v0.2.5 autostart/power tests).
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- Release build: `cargo build --release --locked`.
- UI smoke tests and measurements: [docs/TEST_REPORT.md](docs/TEST_REPORT.md) (currently in German).
- Acceptance matrix: [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md) (currently in German).

## Changelog

- **v0.2.5**: controller focus gating/input-buffer reset, correct LB/RB shoulder mapping, system actions moved to `system.rs`.
- **v0.2.0**: KDE autostart, “Minimize to KDE”, shutdown/reboot power menu, more robust quit/fullscreen behavior.
- **v0.1.0**: first MVP with library, profiles, admin PIN, artwork, and Steam/Prism integration.

Details: [CHANGELOG.md](CHANGELOG.md).

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
