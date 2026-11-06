common-back = Back
common-cancel = Cancel
common-close = Close
common-continue = Continue
common-menu = Menu
common-start = Start
common-save = Save
common-remove = Remove
common-choose = Choose
common-active = Active
common-player = Player
common-admin = Admin
common-child = Child
common-minutes = { $minutes } min
common-page = Page { $page } / { $count }
common-unknown = Unknown
time-remaining = Time remaining: { $minutes } min

header-library = LIBRARY
header-profiles = PROFILES
header-admin = ADMIN
header-menu = Menu  ·  Start
header-input-keyboard = Input: keyboard/mouse
header-input-keyboard-error = Input: keyboard/mouse ({ $error })
header-controller = Controller: { $name }
header-controllers = Controller: { $count } connected
header-time-unlimited = No limit
header-time-exhausted = Time budget exhausted
header-time-remaining = Time remaining: { $minutes } min
profile-switch = Switch profile

time-warning-five = Less than 5 minutes of play time remaining.
time-warning-one = Less than 1 minute of play time remaining.
time-warning-exhausted = The play-time budget is exhausted.

library-page = Page { $page } / { $count }
library-file-missing = File missing
library-empty-title = Your library is empty
library-empty-admin = Import an existing .desktop file – source, name and Exec are shown before approval.
library-empty-player = An admin profile can add games. Pressing A only opens the import when admin rights are available.
library-import = Import desktop file
library-profile-switch = Switch profile
library-hints = A Start   ·   B Back   ·   Start Menu   ·   LB/RB Profiles
library-missing-launch = Desktop file missing – cannot start
library-no-games = No games yet
library-no-games-hint = A Import   ·   Start Menu   ·   Mouse/keyboard
library-no-games-profile = No games for this profile
library-no-games-profile-hint = B Back   ·   Start Menu   ·   Switch profile
library-start = Start  (A)
library-import-action = Import  (A)

profiles-title = Choose profile
profiles-subtitle = Local profiles in the same Linux account. No cloud, no Linux accounts.
profiles-empty = No profiles available
profiles-hints = LB/RB switch   ·   A choose profile   ·   B back
profiles-choose = Choose profile  (A)
profiles-back = Back  (B)

menu-title = Menu
menu-hint = A select  ·  B back  ·  Start closes the menu
menu-resume = Continue
menu-back-library = Back to library
menu-profiles = Switch profile
menu-extra-time = Request more play time
menu-admin = Admin area
menu-import = Import desktop file
menu-fullscreen-enter = Fullscreen
menu-fullscreen-exit = Leave fullscreen
menu-minimize = Minimize to KDE
menu-reboot = Reboot system
menu-poweroff = Shut down system
menu-about = About CouchDeck
menu-quit = Quit CouchDeck

admin-locked = Admin area locked
admin-footer-hint = B back   ·   D-pad/Enter navigate   ·   M menu   ·   Mouse/keyboard in parallel
admin-footer-status = Admin: { $name }   ·   PIN: { $pin }
admin-pin-status-set = set
admin-pin-status-not-set = not set
admin-menu-title = Admin area
admin-menu-subtitle = Only visible to admin profiles. The PIN is a local UI lock, not an OS security boundary.
admin-games-count = Manage games  ({ $count })
admin-profiles-count = Manage profiles  ({ $count })
admin-steam-prism = Steam / Prism
admin-settings = Settings
admin-pin-security = PIN & security
admin-no-pin-warning = No admin PIN set. Anyone with access to this Linux account can make changes.
admin-set-pin-now = Set PIN now
admin-back-library = Back to library  (B)
admin-back = Back  (B)
admin-import-desktop = Import desktop file
admin-games-count-label = { $count } entries (original files are never modified)
admin-games-empty = No entries imported yet.
admin-hidden = [hidden]
admin-file-missing = [file missing]
admin-select-game = Select an entry on the left or import a desktop file.
admin-managed-file = CouchDeck-managed desktop file
admin-original-file = Original desktop file (unchanged)
admin-file-missing-start = File missing – cannot start
admin-title-override = Title override
admin-title-hint = Empty = name from desktop file
admin-cover-choose = Choose cover
admin-cover-remove = Remove cover
admin-background-choose = Choose background
admin-background-remove = Remove background
admin-hide = Hide
admin-show = Show
admin-move-up = Move up
admin-move-down = Move down
admin-refresh = Re-read metadata
admin-remove = Remove
admin-refresh-toast = Name and icon re-read from the desktop file.
admin-visibility = Visibility for profiles
admin-launch-check = Import and launch check
admin-exec-label = Exec (unchanged, executed by GIO):
admin-no-exec = (no Exec)
admin-tryexec = TryExec: { $value }
admin-path = Path: { $value }
admin-steam-repair-button = Create Steam Flatpak variant ...
admin-desktop-unreadable = Desktop file unreadable: { $error }
admin-add-profile = Add profile
admin-local-profiles = Local profiles in the same Linux account
admin-profile-none = No profile selected.
admin-active-profile = Active profile
admin-role-admin = Admin rights for this local profile
admin-child-budget = Child profile & time budget
admin-child-checkbox = Child profile with daily time budget
admin-pin-required-title = Admin PIN required
admin-child-pin-warning = Before enabling a child profile, an admin PIN must be set in an admin profile. Otherwise the time limit could not be lifted.
admin-limit-minus = -15 min
admin-limit = Daily limit: { $minutes } min
admin-limit-plus = +15 min
admin-no-limit = No limit
admin-set-limit = Set 60 min limit
admin-today-usage = Today: { $used } min used · +{ $bonus } min extra · { $remaining } min remaining
admin-reset-extra = Reset extra time
admin-avatar-choose = Choose avatar
admin-avatar-remove = Remove avatar
admin-pin-change = Change PIN
admin-pin-set = Set PIN
admin-pin-remove = Remove PIN
admin-pin-active-hint = The PIN is managed for the active admin profile.
admin-profile-background = Profile background
admin-profile-remove = Remove profile
admin-remove-active-profile-hint = The active profile can be removed; another profile will become active.
admin-integrations-hint = GIO only launches files – CouchDeck does not interpret Exec.
admin-environment = Environment
admin-status-gio = GIO
admin-status-flatpak = Flatpak
admin-status-steam = Steam Flatpak
admin-status-prism = Prism Flatpak
admin-status-available = available
admin-status-missing = missing
admin-status-installed = installed
admin-status-not-found = not found
admin-steam-repair = Steam Flatpak repair
admin-steam-repair-hint = Detected Steam rungameid shortcuts without a Flatpak call. A separate managed file is created only after explicit confirmation.
admin-steam-none = No Steam shortcuts requiring repair found.
admin-steam-candidate = { $title } · Steam app ID { $id }
admin-steam-create = Create managed Steam Flatpak variant ...
admin-steam-not-installed = com.valvesoftware.Steam is not installed.
admin-prism-title = Prism Launcher instances
admin-prism-none = No Prism instances found. Checked folders:
admin-prism-entry-title = Title for the CouchDeck entry
admin-prism-create = Create Prism entry
admin-prism-failed = Prism entry failed
admin-prism-not-installed = org.prismlauncher.PrismLauncher is not installed as a Flatpak.
admin-language = Language / Sprache
admin-language-auto = Automatic
admin-language-de = Deutsch
admin-language-en = English
admin-language-status = Active: { $language }   ·   Automatic follows LANGUAGE/LC_ALL/LC_MESSAGES/LANG; fallback is English.
admin-animations = Subtle focus animations
admin-show-backgrounds = Show background images
admin-controller-hints = Controller hints in the library
admin-alarm-sound = Alert sound for time warnings (best effort)
admin-autostart-header = Autostart
admin-autostart = Start automatically on login (KDE only)
admin-autostart-error = Cannot determine own executable path: { $error }
admin-autostart-enabled = Autostart enabled: { $path }
admin-autostart-disabled = Autostart disabled.
admin-autostart-change-error = Could not change autostart
admin-autostart-status = Status: { $status }   ·   File: { $path }
admin-autostart-active = active
admin-autostart-inactive = inactive
admin-autostart-binary = Autostart executable: { $path }
admin-autostart-dev-warning = Note: this is a development binary. Use the installed version under ~/.local/bin for autostart.
admin-auto-columns = Automatic column count
admin-global-background = Global background image
admin-background-image-choose = Choose background image
admin-storage = Storage locations
admin-storage-line = { $label }: { $path }
admin-pin-header = PIN & security
admin-pin-set-status = A PIN is set for the active admin profile.
admin-pin-unset-status = No PIN is set for the active admin profile.
admin-pin-file = PIN file: { $path }
admin-pin-controller-hint = PIN entry is also available with a controller via the on-screen numpad.
admin-pin-remove-title = Remove admin PIN?
admin-pin-remove-message = Afterwards the admin area is reachable without a PIN. This is only a local UI lock.
admin-pin-last-blocked-title = Admin PIN required
admin-pin-last-blocked-message = The last admin PIN cannot be removed while child profiles with a time budget exist. Otherwise profile switching and approvals would no longer be possible.
admin-pin-wrong = Admin PIN incorrect.
admin-pin-removed = Admin PIN removed.

dialog-import-title = Import desktop file
dialog-import-subtitle = Source, name and Exec are shown before approval. Original files are never modified.
dialog-found-apps = Found applications ({ $count })
dialog-no-apps = No visible applications found.
dialog-already-imported = · already imported
dialog-no-exec-warning = Warning: no Exec and no D-Bus activation.
dialog-exec-label = Exec (unchanged):
dialog-icon = Icon: { $icon }
dialog-tryexec = TryExec: { $value }
dialog-workdir = Working directory: { $value }
dialog-choose-app = Select an application on the left.
dialog-import = Import  (A)
dialog-other-file = Choose another file ...
dialog-cancel = Cancel  (B)
dialog-import-hint = D-pad: select list  ·  Right: actions  ·  Left: back to list  ·  A imports
dialog-picker-hint = A opens folder/file  ·  Left: parent  ·  B cancel
dialog-picker-parent = Parent folder
dialog-picker-open = Open
dialog-picker-empty = No matching entries in this folder.
dialog-picker-select = Select  (A)
dialog-pin-unlock = PIN to unlock the admin area
dialog-pin-set-subtitle = 4–12 digits
dialog-pin-confirm-subtitle = Enter again for confirmation
dialog-pin-parent-subtitle = Parent approval – for this action only
dialog-pin-store-hint = The PIN is stored only as an Argon2id hash.
dialog-confirm-cancel = Cancel  (B)
dialog-message-close = { $label }  (A)
dialog-power-success = System command submitted
dialog-power-quit = CouchDeck will quit; the system takes over.
dialog-power-hint = The command is passed to systemd via `systemctl`. A Polkit prompt may appear if necessary.
dialog-power-wait = systemctl is still responding. Please wait …
dialog-launch-starting = Starting { $title }
dialog-launch-gio-hint = GIO is being called. This may take a moment. CouchDeck does not block.
dialog-launch-wait = gio launch is still responding. The launcher remains usable.
dialog-launch-sent = Launch command sent
dialog-launch-no-status = CouchDeck deliberately shows no 'running' status: Steam/Prism may keep running and game exit cannot be detected reliably.
dialog-launch-continue = Continue  (A/B)
dialog-launch-minimize = Minimize launcher
dialog-fatal-title = Library data unreadable
dialog-fatal-backup = Backup: { $path }
dialog-fatal-reset = Create backup & new library
dialog-fatal-hint = The damaged file is never silently overwritten. D-pad/A or mouse.
dialog-fatal-quit = Quit

time-expired-title = Time budget exhausted
time-expired-message = The daily play-time budget for { $name } is exhausted.
time-expired-running = Already running games are not closed. New games can only start after parent approval.
time-expired-request = Request more play time  (A)
time-expired-back = Back to library  (B)
time-expired-pin-hint = Parent approval requires the admin PIN.
time-extra-title = Grant more play time
time-extra-status = Profile: { $name }   ·   Extra time today: { $bonus } min   ·   Remaining: { $remaining } min
time-extra-custom = Custom minutes
time-extra-custom-hint = Left/right: ±5   ·   Up/down: ±15   ·   A: grant   ·   B: back
time-extra-minutes = { $minutes } min
time-extra-grant = Grant  (A)
time-extra-back = Back  (B)
time-extra-option-5 = +5 minutes
time-extra-option-15 = +15 minutes
time-extra-option-30 = +30 minutes
time-extra-option-custom = Custom minutes
time-extra-cancel = Cancel  (B)

profile-role-admin = Admin
profile-role-player = Player
profile-role-child = Child
profile-active = Active

menu-about-text = Controller-first fullscreen game library for KDE Plasma/Wayland. Data: { $data } · Settings: { $settings } · Cache: { $cache } · Log: { $log } · { $gio }. Launching always goes through `gio launch`. CouchDeck cannot know whether a game is still running. The admin PIN is a local UI lock, not an operating-system security boundary.
menu-gio-ok = GIO: available
menu-gio-problem = GIO: problem – { $error }

picker-cover = Choose cover

picker-game-background = Choose game background

picker-global-background = Choose global background image

picker-avatar = Choose profile avatar

picker-profile-background = Choose profile background

picker-desktop = Choose desktop file

confirm-power-message = All running applications and games will be closed. CouchDeck submits the command to systemd.

confirm-reboot-title = Reboot system?

confirm-reboot-label = Reboot

confirm-poweroff-title = Shut down system?

confirm-poweroff-label = Shut down

confirm-quit-title = Quit CouchDeck?

confirm-quit-message = An already running game is not closed.

confirm-quit-label = Quit

power-progress-reboot = System is rebooting …

power-progress-poweroff = System is shutting down …

power-error-title = System command failed

power-error-message = systemctl could not be executed: { $detail }

power-disconnected = The system thread terminated unexpectedly.

dialog-picker-home = Home

dialog-picker-downloads = Downloads

dialog-picker-apps = Applications

dialog-picker-path = Path

dialog-picker-folder = [Folder]

dialog-picker-file = [File]

dialog-severity-info = Notice

dialog-severity-warning = Warning

dialog-severity-error = Error

pin-title-unlock = Enter admin PIN

pin-title-set = Set a new admin PIN

pin-title-confirm = Repeat admin PIN

pin-title-parent = Admin PIN required

app-startup-notice-title = Startup notice

app-profile-active = Profile active: { $name }

app-no-admin-pin-title = No admin PIN

app-no-admin-pin-message = An admin PIN must first be set in an admin profile for this action.

app-extra-child-only = Extra time is only available for child profiles.

app-extra-invalid = Extra time must be between 1 and 240 minutes.

app-profile-not-found = Profile not found.

app-extra-granted = { $minutes } minutes of extra time granted for { $name }.

app-extra-reset = Extra time for today has been reset.

app-admin-only = Only an admin profile can open the admin area.

app-admin-import-only = Only admins can import desktop files.

app-admin-import-hint = Admin PIN required. After unlocking, import is available under Games.

app-desktop-invalid-title = Invalid desktop file

app-desktop-invalid-message = { $error } The file was not modified.

app-import-not-admin = Only admins can import entries.

app-already-imported = This desktop entry is already in the library.

app-imported = Imported: { $name }

app-picker-no-directory = Not a directory: { $path }

app-picker-path-not-found = Path not found: { $path }

app-picker-dir-unreadable = Directory unreadable: { $path }

app-game-not-found = Game entry not found.

app-cover-updated = Cover for { $title } updated.

app-game-background-updated = Background for { $title } updated.

app-global-background-updated = Global background image updated.

app-avatar-updated = Avatar for { $name } updated.

app-profile-background-updated = Profile background for { $name } updated.

app-cover-removed = Cover override removed.

app-game-background-removed = Game background removed.

app-global-background-removed = Global background image removed.

app-profile-background-removed = Profile background removed.

app-avatar-removed = Avatar removed.

app-new-profile = New local profile created. Edit the name in the field.

app-last-admin-remove-blocked = The last admin profile cannot be removed.

app-profile-removed = Profile removed.

app-entry-removed = Entry removed from the CouchDeck library. The desktop file is unchanged.

app-entry-hidden = Entry hidden.

app-entry-shown = Entry visible again.

app-steam-repair-not-possible-title = Steam repair not possible

app-desktop-unreadable = The desktop file cannot be read.

app-not-steam-shortcut = This entry is not a recognized Steam rungameid shortcut.

app-steam-already-flatpak = This entry already uses Steam as a Flatpak.

app-flatpak-missing-title = Flatpak missing

app-flatpak-missing-message = The `flatpak` command was not found. The repair would create a non-functional entry.

app-steam-flatpak-missing-title = Steam Flatpak not found

app-steam-flatpak-missing-message = com.valvesoftware.Steam was not found installed. The repair would create a non-functional entry.

app-confirm-steam-title = Create Steam Flatpak variant?

app-confirm-steam-message = The original file remains unchanged. A separate managed file will be created with: flatpak run com.valvesoftware.Steam steam://rungameid/{ $id }. The new entry will also be added to the library.

app-confirm-steam-label = Create managed file

app-steam-repair-failed = Steam repair failed

app-steam-id-missing = Steam app ID could not be determined.

app-no-repair = No repair needed.

app-steam-managed-created = Managed Steam Flatpak entry created. Original unchanged.

app-prism-instance-missing = Prism instance folder “{ $id }” was not found. --launch expects the folder name.

app-prism-flatpak-missing = The `flatpak` command was not found. The generated entry would not start.

app-prism-not-installed = org.prismlauncher.PrismLauncher is not installed as a Flatpak. The generated entry would not start.

app-prism-created = Prism entry for “{ $id }” created. A cover can now be assigned.

app-minimizing-kde = Switching to KDE …

app-minimized = Launcher minimized. Restore it from the taskbar.

app-library-empty = The library is empty. An admin can add games.

app-launch-running = A launch is already in progress.

app-gio-missing-title = GIO missing

app-gio-missing-message = Without `gio` (GLib/GIO), desktop entries cannot be launched. Install GLib or check the Bazzite installation.

app-desktop-missing-title = Desktop file missing

app-desktop-missing-message = { $path } The entry stays in the library but cannot be started. An admin can remove it or import it again.

app-launch-sent = Launch command sent: { $title }

app-launch-failed = Launch failed: { $title }

app-launch-thread-ended-title = Launch thread ended

app-launch-thread-ended-message = The launch process ended unexpectedly. Please check the log.

app-launch-note = Launch command for { $title } sent. CouchDeck does not detect game exit automatically.

app-controller-connected = Controller connected: { $name }

app-controller-disconnected = Controller disconnected: { $name }. Other input devices remain active.

app-minimize-parent-only = Minimizing is only possible with the admin PIN.

app-pin-wrong = Incorrect PIN.

app-admin-pin-wrong = Incorrect admin PIN.

app-pin-mismatch = The PINs do not match.

app-admin-pin-set = Admin PIN set (Argon2id, hashed).

app-admin-unlocked = Admin area unlocked.

app-remove-pin-title = Remove admin PIN?

app-remove-pin-message = Afterwards the admin area is reachable without a PIN. This is only a local UI lock.

app-remove-pin-label = Remove PIN

app-last-pin-title = Admin PIN required

app-last-pin-message = The last admin PIN cannot be removed while child profiles with a time budget exist. Otherwise profile switching and approvals would no longer be possible.

app-pin-removed = Admin PIN removed.

app-remove-entry-title = Remove entry?

app-remove-entry-message = “{ $title }” will only be removed from the CouchDeck library. The desktop file and the game remain unchanged.

app-remove-entry-label = Remove from library

app-remove-profile-title = Remove profile?

app-remove-profile-message = “{ $name }” and its PIN will be removed from the local CouchDeck file. Linux accounts and games remain unchanged.

app-remove-profile-label = Remove profile

app-new-library-created = New library created.

app-new-library-backup = New library created. Old file backed up at { $path }.

app-fatal-reset-failed = Reset failed: { $error }

admin-games-header = Games

admin-profiles-header = Profiles

admin-name-label = Display name

admin-name-hint = Name

admin-default-profile-name = Profile

admin-integrations-prism-hint = INSTANCE_ID is the folder name of the instance, not the visible name. The generated file uses: flatpak run org.prismlauncher.PrismLauncher --launch INSTANCE_ID

admin-columns = Columns

admin-storage-library = Library

admin-storage-settings = Settings

admin-storage-assets = Assets

admin-storage-cache = Cache

admin-storage-log = Log

admin-pin-storage-hint = The PIN is stored as an Argon2id hash in a separate file with 0600 permissions. It protects against accidental UI changes, not against access to the same Linux account.

admin-gio-available = available

app-entry = Entry

app-profile = Profile

app-default-profile-name = Profile { $number }

app-steam-entry-title = { $title } (Steam Flatpak)

zz-fallback-test = English fallback works
