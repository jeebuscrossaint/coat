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

# 3b. Or: edit coat.yaml manually, then apply
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
  - kitty
  - helix
  - vscode
  - sway
  - waybar
  - fuzzel
  - dunst
  - gtk
  - qt
  - vesktop
  - bat
  - btop
  - lf
  - zathura

font:
  monospace: "JetBrains Mono"
  sansserif: "Ubuntu"
  serif:     "Ubuntu Serif"
  emoji:     "Noto Color Emoji"
  sizes:
    terminal: 12   # foot, kitty, xresources
    desktop:  10   # waybar, btop
    popups:   10   # fuzzel, rofi, dunst

opacity:
  terminal:     0.95   # foot, kitty
  applications: 1.0
  desktop:      1.0
  popups:       0.95   # swaylock ring alpha
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

### kitty

Writes `~/.config/kitty/coat-theme.conf`.

```
# Add to ~/.config/kitty/kitty.conf
include coat-theme.conf
```

Reload: `Ctrl+Shift+F5`

### foot

Writes `~/.config/foot/coat-theme.ini`.

```ini
# Add to ~/.config/foot/foot.ini
[main]
include=~/.config/foot/coat-theme.ini
```

Open a new window to apply.

### helix

Writes `~/.config/helix/themes/coat.toml`.

```toml
# Add to ~/.config/helix/config.toml
[editor]
theme = "coat"
```

Reload: `:config-reload`

### vscode

Merges colors directly into `~/.config/Code/User/settings.json` (Linux) or `%APPDATA%\Code\User\settings.json` (Windows) via `workbench.colorCustomizations` and `editor.tokenColorCustomizations`. No extension install needed — changes take effect immediately.

### i3

Writes `~/.config/i3/coat-theme.conf`. `i3-msg reload` is run automatically.

```
# Add to ~/.config/i3/config
include coat-theme.conf
```

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

### swaylock

Writes `~/.config/swaylock/config` directly. Takes effect next time you run `swaylock`.

### fuzzel

Writes the entire `~/.config/fuzzel/fuzzel.ini`. Just launch fuzzel — no include step needed.

> **Note:** coat overwrites `fuzzel.ini` completely on each apply.

### rofi

Writes `~/.config/rofi/coat.rasi`.

```rasi
/* Add to ~/.config/rofi/config.rasi */
@import "coat.rasi"
```

### dunst

Writes `~/.config/dunst/dunstrc` and restarts dunst automatically.

```bash
# If dunst doesn't restart automatically
killall dunst && dunst &
```

### gtk

Writes to `~/.config/gtk-3.0/gtk.css` and `~/.config/gtk-4.0/gtk.css`. Runs `gsettings` to apply immediately. Restart GTK apps to pick up the new colors.

### qt

Writes color palettes to `~/.config/qt5ct/colors/coat.conf` and/or `~/.config/qt6ct/colors/coat.conf` (whichever directories exist).

```bash
# Add to ~/.profile or shell config
export QT_QPA_PLATFORMTHEME=qt5ct   # or qt6ct
```

Open `qt5ct`/`qt6ct` and verify **coat** is selected under Colors.

### xresources

Writes `~/.Xresources` and runs `xrdb -merge ~/.Xresources`.

> **Note:** This overwrites `~/.Xresources`. Back up any custom settings first.

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

### lf

Writes `~/.config/lf/colors` using 24-bit truecolor ANSI codes.

```
# Add to ~/.config/lf/lfrc
set color true
```

### zathura

Writes color settings to `~/.config/zathura/zathurarc`. Restart zathura to apply.

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
- Windows Terminal — injects a `"coat"` entry into `settings.json`
- VSCode — merges into `%APPDATA%\Code\User\settings.json`

Run as administrator to also theme the logon screen.

```powershell
coat set nord
coat set ayu-dark
coat set catppuccin-mocha
```

After applying, Explorer/taskbar picks up changes live. Some accent color changes may require signing out and back in.

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
