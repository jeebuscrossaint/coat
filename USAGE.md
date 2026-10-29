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
prefer_base24: false           # true = Base24, false = Base16, any = both (set prefers base24, random mixes)

enabled:
  - fish
  - kitty
  - mango
  - vscode
  - fuzzel
  - dunst
  - gtk
  - swaylock
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
    terminal: 12   # kitty, imv, xresources
    desktop:  10   # gtk, qt, zathura
    popups:   10   # fuzzel, dunst

opacity:
  terminal:     0.95   # kitty
  applications: 1.0
  desktop:      1.0
  popups:       0.95   # dunst, fuzzel backgrounds

# Repaint scheme slots, whatever scheme is loaded. A value is "#RRGGBB" or
# another slot's name (read from the scheme as loaded, so two slots can swap).
palette:
  base0D: "#89b4fa"

# Per-app patches of font, opacity and palette. Only the keys listed change.
overrides:
  kitty:
    opacity:
      terminal: 0.9
    palette:
      base00: base01   # kitty's background one step up the ramp
```

### What gets reloaded

coat only rewrites a file whose contents changed, and only tells an app to
reload (`kitty @ load-config`, `dunstctl reload`, `hyprctl reload`, …) when one
of *its* files changed. Bumping `overrides.kitty.opacity` reloads kitty and
nothing else; an apply with nothing new reloads nothing:

```
Applied scheme to 20/20 applications!
Changed: kitty
```

`coat apply --force` reloads every app anyway — for one that was not running
when its files last changed. `colors.send` copies are always attempted, since
the far side compares for itself and a failed copy should be retried.

`coat apply --dry` shows the same decision without acting on it: each file that
would change, with a diff, and nothing written or reloaded.

`coat watch` re-applies whenever `coat.yaml`, the current scheme, or the
`coat match` scheme changes on disk, so an edit shows up a moment after saving
and only in the app it concerns.

Typos are reported rather than ignored: an `enabled:` or `overrides:` name that
is no module, or an unknown top-level key, prints a warning with the closest
match; an unknown key inside `font:`, `opacity:`, `normalize:` or an override,
and a bad `palette:` entry, stop the apply with an error naming it.

---

## What coat writes

Colours and fonts, and nothing else. Two mechanisms:

- **Fragment + include** — where the app supports one. coat owns a separate file
  (`coat-theme`, `themes/coat.theme`, `dunstrc.d/50-coat.conf`) and adds a single
  include line to your config on first apply. Everything else in your config is
  untouched because coat never opens it again.
- **A file of its own** — where the app has no include mechanism. coat writes a
  file only it owns (a btop theme, a bat theme, swaylock's colour flags) and your
  config is never opened at all.

VS Code is the exception: coat merges `workbench.colorCustomizations` (and the
editor font) into your `settings.json`, leaving your other settings alone.

## Activation steps by application

### fish

Writes `~/.config/fish/themes/coat.theme`.

```bash
# One-time
fish_config theme save coat

# Or add to ~/.config/fish/config.fish
fish_config theme choose coat
```

### vscode

Merges colors directly into `~/.config/Code/User/settings.json` (Linux) or `%APPDATA%\Code\User\settings.json` (Windows) via `workbench.colorCustomizations` and `editor.tokenColorCustomizations`. No extension install needed — changes take effect immediately.

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

### qt

qt6ct as the platform theme, Fusion as the style. Needs `qt6ct` installed.

- `~/.config/qt6ct/colors/coat.conf` — the palette
- `~/.config/qt6ct/qt6ct.conf` — only `custom_palette`, `color_scheme_path`,
  `style=Fusion` and the fonts are set; anything else you set in qt6ct stays
- `~/.config/environment.d/90-coat-qt.conf` — `QT_QPA_PLATFORMTHEME=qt6ct`

qt5ct gets the same when it is installed. Running Qt apps change scheme live, a
few seconds after `coat set`. Apps started before the first apply need one
restart; coat restarts the screen-share portal itself.

### xresources

Writes `~/.Xresources.coat` and adds an `#include` for it to `~/.Xresources` on
first apply, then runs `xrdb -merge ~/.Xresources`. Your own DPI, cursor and
per-app settings in `~/.Xresources` are preserved.

### colors

Writes `~/.config/coat/colors.json`: the scheme as plain data, for anything coat
has no module for. Each colour is `"#RRGGBB"` under `colors.base00` … `base0F`
(and `base10` … `base17` for Base24 schemes), plus `scheme`, `author`, `variant`
(`dark`/`light`), `base24`, and `font`. Readers pick the slots they need:

```bash
jq -r .colors.base0D ~/.config/coat/colors.json
```

To keep a copy on other machines, list them as `host:/dir`. The copy goes over
ssh (it must not prompt) and is only replaced when it changed, so a reader there
can watch it:

```yaml
colors:
  send:
    - me@server:/opt/homelab/stacks/terminal/theme
```

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

### zathura

Writes `~/.config/zathura/coat-theme` (colors + font) and adds `include
coat-theme` to your `zathurarc` on first apply. Restart zathura to apply.

### qutebrowser

Writes `~/.config/qutebrowser/coat-theme.py` (colors + font) and adds
`config.source('coat-theme.py')` to the top of `config.py` on first apply
(creating it with `config.load_autoconfig()` if missing). A running qutebrowser
reloads via `:config-source` automatically.

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

## Commands

| Command | Description |
|---|---|
| `coat clone` | Clone the tinted-theming scheme library |
| `coat update` | Pull latest schemes |
| `coat list [--dark\|--light] [--no-preview]` | Browse all schemes |
| `coat search <term>` | Search by name or author |
| `coat browse [--dark\|--light]` | Scroll through schemes, apply on Enter |
| `coat set <scheme> [--base16\|--base24]` | Switch scheme and apply everywhere |
| `coat random [--dark\|--light] [--dry] [-y]` | Pick a random scheme, preview, then apply |
| `coat match [image] [--light\|--auto] [--slots] [--dry]` | Build a scheme from the wallpaper and apply it |
| `coat apply [app] [--dry] [--force]` | Apply current scheme from coat.yaml; `--dry` shows the diff and writes nothing, `--force` reloads apps whose files did not change |
| `coat watch` | Re-apply whenever coat.yaml or the scheme changes |
| `coat remove <app> [--dry] [--keep-enabled]` | Undo an app's theming and disable its module |
| `coat docs <app>` | Show activation instructions for an app |
| `coat completions <fish\|powershell> [--print]` | Install shell completions |
| `coat help` | Show help |
