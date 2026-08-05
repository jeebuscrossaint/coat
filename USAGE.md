# coat — Usage Guide

## Quick start

```bash
# 1. Clone the scheme library (~700 Base16/Base24 schemes)
coat clone

# 2. Browse and pick a scheme
coat list --dark
coat search catppuccin

# 3a. One-shot: switch scheme and apply everywhere
coat set catppuccin-mocha

# 3b. Feeling lucky: switch to a random scheme (add --dark/--light to constrain,
#     or --dry to preview the pick without applying)
coat random
coat random --dry

# 3c. Or: edit coat.yaml manually, then apply
coat apply
```

## coat.yaml reference

`~/.config/coat/coat.yaml` controls everything. `coat set <scheme>` creates it automatically if it doesn't exist.

```yaml
scheme: gruvbox-dark-hard     # scheme name (case-insensitive)
prefer_base24: false           # prefer Base24 variant when available

enabled:
  - fish
  - foot
  - neovim
  - vscode
  - sway
  - swaybar
  - tofi
  - dunst
  - gtk
  - gtklock
  - vesktop
  - bat
  - btop
  - zathura

font:
  monospace: "JetBrains Mono"
  sansserif: "Ubuntu"
  serif:     "Ubuntu Serif"
  emoji:     "Noto Color Emoji"
  sizes:
    terminal: 12   # foot, xresources
    desktop:  10   # swaybar, btop, zathura
    popups:   10   # tofi, dunst

opacity:
  terminal:     0.95   # foot
  applications: 1.0
  desktop:      1.0
  popups:       0.95   # dunst, gtklock, tofi backgrounds
```

---

## Activation steps by application

### fish

Writes `~/.config/fish/themes/coat.theme`.

```bash
# One-time
fish_config theme save coat

# Or add to ~/.config/fish/config.fish
fish_config theme choose coat
```

### foot

Writes `~/.config/foot/coat-theme.ini`.

```ini
# Add to ~/.config/foot/foot.ini
[main]
include=~/.config/foot/coat-theme.ini
```

Open a new window to apply.

### vscode

Merges colors directly into `~/.config/Code/User/settings.json` (Linux) or `%APPDATA%\Code\User\settings.json` (Windows) via `workbench.colorCustomizations` and `editor.tokenColorCustomizations`. No extension install needed — changes take effect immediately.

### sway

Writes `~/.config/sway/coat-theme` (window colors) and runs `swaymsg reload`.

```
# Add to ~/.config/sway/config
include ~/.config/sway/coat-theme
```

### labwc

Writes `~/.config/labwc/themerc` and runs `labwc --reconfigure`. No manual step needed.

### waybar

Writes `~/.config/waybar/coat-theme.css` with `@define-color` variables.

```css
/* Add to ~/.config/waybar/style.css */
@import "coat-theme.css";
```

Reload: `killall -SIGUSR2 waybar`

### swaybar

Themes sway's built-in bar. Writes `~/.config/sway/coat-bar`.

```
# Add to ~/.config/sway/config
include ~/.config/sway/coat-bar
```

Remove every other `bar { }` block from your config first — each one creates an
additional bar rather than merging with this one.

> **Note:** the whole `bar { }` block is generated, not just the colors. Sway
> rejects `include` inside `bar { }`, so a colors-only fragment cannot be
> spliced into a hand-written block. The status line is `status_command
> swayrbar`; configure its modules in `~/.config/swayrbar/config.toml`.

coat runs `swaymsg reload` automatically, which also repaints the `sway`
module's window colors.

### tofi

Writes `~/.config/tofi/coat-theme` (font, font size, colors) and adds the include
to your `config` on first apply. Geometry and behaviour keys stay yours.

```
include=coat-theme
```

tofi resolves the include the moment it reads the line, so anything you set after
it overrides the theme.

### swayosd

Writes `~/.config/swayosd/style.css` (GTK CSS) and restarts `swayosd-server`,
which only parses its stylesheet at startup.

```
# Run the server from your compositor config, e.g. sway
exec swayosd-server
```

> **Note:** the caps/num/scroll-lock OSD works from an ordinary compositor bind
> (`bindsym --release Caps_Lock exec swayosd-client --caps-lock`).
> `swayosd-libinput-backend` is only needed to catch those keys *without* a
> bind, and it requires root plus a service-manager unit — its shipped D-Bus
> activation file has `Exec=/bin/false` and defers to systemd, so it does not
> auto-activate on non-systemd systems.

### dunst

Writes the drop-in `~/.config/dunst/dunstrc.d/50-coat.conf` (colors + font) and
runs `dunstctl reload`. Your `dunstrc` — geometry, timeouts, icons, mouse
actions, `dmenu`/`browser` — is left alone.

Drop-ins are applied after the base config in lexical order, last winning, so
coat overrides colors in your `dunstrc` and you can override coat with a
later-sorting drop-in such as `99-mine.conf`.

> **Note:** dunst only reads the `dunstrc.d` next to the *first* `dunstrc` it
> finds. If you have no `~/.config/dunst/dunstrc`, coat creates one (seeded from
> `/etc/dunst/dunstrc`) — otherwise the base config would resolve to `/etc` and
> the drop-in would never be read.

### gtk

Writes `coat-theme.css` into `~/.config/gtk-3.0/` and `~/.config/gtk-4.0/`, and
adds `@import url("coat-theme.css");` at the top of each `gtk.css` on first
apply. Runs `gsettings` to apply immediately. Restart GTK apps to pick up the new
colors.

### xresources

Writes `~/.Xresources.coat` and adds an `#include` for it to `~/.Xresources` on
first apply, then runs `xrdb -merge ~/.Xresources`. Your own DPI, cursor and
per-app settings in `~/.Xresources` are preserved.

### bat

Writes `~/.config/bat/themes/coat.tmTheme`.

```bash
bat cache --build
```

```
# Add to ~/.config/bat/config
--theme="coat"
```

### btop

Writes `~/.config/btop/themes/coat.theme`.

In btop: `Esc` → Options → Color theme → **coat**

### ranger

Writes `~/.config/ranger/colorschemes/coat.py`.

```
# Add to ~/.config/ranger/rc.conf
set colorscheme coat
```

### zathura

Writes `~/.config/zathura/coat-theme` (colors + font) and adds `include
coat-theme` to your `zathurarc` on first apply. Restart zathura to apply.

### vesktop

Writes a CSS theme to whichever of these directories exists:
- `~/.config/vesktop/themes/coat.theme.css`
- `~/.config/Vencord/themes/coat.theme.css`

In Vesktop/Vencord settings → Themes → enable **coat**.

---

## Windows

`coat set <scheme>` themes Windows directly — no `coat.yaml` required:

- System accent color via registry (`Explorer\Accent`, DWM, `Control Panel\Colors`)
- Dark/light mode (`Themes\Personalize`)
- Windows Terminal — see below
- VSCode — merges into `%APPDATA%\Code\User\settings.json`, including `editor.fontFamily` from `font.monospace`
- Zed — writes `%APPDATA%\Zed\themes\coat.json` and selects it in `settings.json`, including `buffer_font_family` (only if Zed's config dir exists)

```powershell
coat set nord
coat set ayu-dark
coat set catppuccin-mocha
```

After applying, Explorer/taskbar picks up changes live. Some accent color changes may require signing out and back in.

### Windows Terminal

coat injects a `"coat"` entry into `schemes`, then selects it by writing
`profiles.defaults.colorScheme`, so every profile that hasn't overridden the
scheme picks it up — no hand-editing. It also sets the top-level `theme` to
`dark` or `light` to match the scheme.

Font and opacity come from `coat.yaml`, mapped onto `profiles.defaults`:

| coat.yaml | Windows Terminal |
|---|---|
| `font.monospace` | `font.face` |
| `font.sizes.terminal` | `font.size` |
| `opacity.terminal` (0.0–1.0) | `opacity` (0–100), with `useAcrylic: false` |

Keys your `coat.yaml` doesn't mention are left untouched, as are per-profile
overrides in `profiles.list` and any other settings you've customized. Setting
a value back to its default (e.g. `opacity.terminal: 1.0`) does get written, so
changes revert cleanly.

### Logon screen (`--elevate`)

The logon-screen accent lives in `HKU\.DEFAULT` and needs administrator rights.
Pass `--elevate` to `set`, `random`, or `browse` and coat spawns a short-lived
elevated helper for just those keys — one UAC prompt, with everything else
applying unelevated in your existing terminal:

```powershell
coat set nord --elevate
```

Without the flag, that one step is reported as skipped and the rest still
applies. Running coat from an already-elevated shell needs no flag.

---

## Firefox

`coat apply firefox` writes `userChrome.css` (browser UI) and `userContent.css`
(`about:` / new-tab pages) into your default profile's `chrome/` folder and
enables `toolkit.legacyUserProfileCustomizations.stylesheets` in `user.js`.

Because Firefox reads these stylesheets **only at startup**, you must fully quit
and relaunch Firefox after applying (make sure no `firefox.exe` lingers). If
colors still don't appear, confirm in `about:config` that
`toolkit.legacyUserProfileCustomizations.stylesheets` is `true`, and set any
active color **theme add-on** in `about:addons` back to *System theme (auto)* or
*Default* so it doesn't override the CSS.

---

## Commands

| Command | Description |
|---|---|
| `coat clone` | Clone the tinted-theming scheme library |
| `coat update` | Pull latest schemes |
| `coat list [--dark\|--light] [--no-preview]` | Browse all schemes |
| `coat search <term>` | Search by name or author |
| `coat set <scheme>` | Switch scheme and apply everywhere |
| `coat apply [app]` | Apply current scheme from coat.yaml |
| `coat docs <app>` | Show activation instructions for an app |
| `coat help` | Show help |
