# coat — color scheme configurator

A Rust CLI that applies Base16/Base24 color schemes across 22 Linux applications — and themes Windows system colors — from a single config file.

## Features

- **32 application modules** — terminals, editors, compositors, bars, launchers, and more
- **Windows support** — accent color, dark/light mode, Windows Terminal, and VSCode via `coat set`
- **Base16 & Base24** — compatible with the full [tinted-theming](https://github.com/tinted-theming/home) ecosystem (~700 schemes)
- **Scheme browser** — search and preview with live RGB color swatches in the terminal
- **Wallpaper matching** — `coat match` samples whatever the wallpaper daemon is displaying and builds a scheme from it
- **Reversible** — `coat remove <app>` undoes an app's theming from a manifest the apply wrote
- **Font & opacity** — centralized font and transparency settings across all modules

## Project structure

```
coat/
├── src/
│   ├── main.rs       # CLI dispatch
│   ├── config.rs     # coat.yaml deserialization
│   ├── scheme.rs     # scheme loading, search, list
│   ├── modules.rs    # all 32 module apply functions
│   └── windows.rs    # Windows-specific theming (registry, WT, VSCode)
├── templates/        # Tera templates for each module
├── Cargo.toml
└── USAGE.md
```

## Building

```bash
cargo build --release
```

The binary appears at `target/release/coat` (or `coat.exe` on Windows). No external dependencies — all scheme parsing, JSON/YAML handling, and templating are compiled in.

## Usage

```bash
# First time: clone the scheme library
coat clone

# Browse schemes
coat list --dark
coat search gruvbox

# Scroll through schemes interactively (↑/↓ or j/k, / to filter, Enter applies)
coat browse
coat browse --light

# Switch scheme and apply everywhere in one shot
coat set catppuccin-mocha   # suggests close matches on a typo

# Pick a random scheme — previews it, then prompts [Y]es / [r]eroll / [n]o
coat random
coat random --dark      # restrict to dark schemes
coat random -y          # apply immediately, skip the prompt
coat random --dry       # preview a random pick without applying

# Apply current scheme from coat.yaml to all enabled apps
coat apply

# Apply to a single app
coat apply foot

# Build a scheme from the wallpaper that is on screen right now (awww or swww),
# write it into the schemes directory, and apply it
coat match
coat match ~/walls/whatever.png   # or a specific image
coat match --light                # a light scheme instead (dark is the default)
coat match --auto                 # let the image's own brightness decide
coat match --raw                  # pywal-style: image hues verbatim, no slot rules
coat match --dry                  # generate and preview, apply nothing

# Undo one app: delete the files coat generated for it, strip the include line
# it added to your own config, and drop it from the enabled list
coat remove foot
coat remove foot --dry            # show what that would do
coat remove foot --keep-enabled   # clean up but leave it enabled
```

### How `coat match` picks colours

The image decides the **hues**; the base16 slot contract decides where they land.
Lightness and chroma come from fixed per-polarity ladders, so a muddy photo
cannot produce a scheme whose foreground is invisible on its background.

- the image is downscaled to 160px and clustered in Oklab (k=12, farthest-point
  seeded, so the same wallpaper always yields the same scheme)
- **dark by default**, whatever the image's brightness. Inferring polarity means a
  snowy wallpaper turns the desktop white, which nobody asks for; `--auto` opts
  back into inference, `--light` forces the other way
- `base00`–`base07` are a fixed lightness ramp tinted with the image's
  chroma-weighted mean hue, so the background reads as *of* the wallpaper
- `base08`–`base0F` keep their conventional meanings: each slot takes the nearest
  matching hue from the image within 45°. A slot the image has no colour for keeps
  its identity but leans up to 30° toward the image's nearest hue — without that,
  every photo lacking a red produced the *same* red, and wildly different
  wallpapers came out with identical accent rows
- accent chroma scales with how colourful the image actually is, so a washed-out
  photo gives muted accents rather than eight candy-bright hues stapled on
- no two accents may sit within 20° of each other, so `base0D` and `base0E` cannot
  collapse into the same blue

`--raw` drops all of that: the image's eight most prominent colours are assigned
one-per-slot (globally nearest pair first, never the same cluster twice) at their
own saturation, with only the lightness ladder left in place. That is pywal's
behaviour — on a wallpaper with one strong hue it will hand you eight shades of
that hue, which is the point of asking for it.

Generated schemes are written to `~/.config/coat/schemes/generated/`, so
`coat set`, `coat list` and `coat browse` see them like any other scheme.

## Shell completions

Fish and PowerShell completions cover subcommands, flags, and (dynamically)
scheme and module names. Install them with:

```bash
coat completions fish            # → ~/.config/fish/completions/coat.fish
coat completions fish --print    # print to stdout instead of installing
```

```powershell
coat completions powershell      # → coat.completion.ps1 beside $PROFILE, sourced from it
coat completions powershell --print
```

PowerShell has no autoloaded completions directory, so coat writes the script
next to your `$PROFILE` and adds a line there to dot-source it. Both steps are
idempotent — re-running just refreshes the script. `$PROFILE` is resolved by
asking PowerShell itself, so a OneDrive-redirected Documents folder and the
5.1 / 7 split are handled.

The scripts also live in [`completions/`](completions) so distro packages can
install them automatically (e.g. cargo-deb drops the fish one in
`/usr/share/fish/vendor_completions.d/`). Note that a plain `cargo install`
places only the binary — run `coat completions <shell>` once afterward.

## Configuration

Create `~/.config/coat/coat.yaml`:

```yaml
scheme: gruvbox-dark-hard
prefer_base24: false

enabled:
  - fish
  - neovim
  - sway
  - swaybar
  - tofi
  - dunst
  - gtk
  - vesktop

font:
  monospace: "JetBrains Mono"
  sansserif: "Ubuntu"
  sizes:
    terminal: 12
    desktop: 10
    popups: 10

opacity:
  terminal: 0.95
  popups: 0.95
```

## Windows

On Windows, `coat set <scheme>` themes the OS directly — no config file required:

- System accent color (registry, AccentPalette, taskbar/Start)
- Dark/light mode
- Windows Terminal — scheme, selected via `profiles.defaults`, plus font and opacity from `coat.yaml`
- VSCode color customizations and editor font

```powershell
coat set nord
coat set gruvbox-dark-hard
coat set nord --elevate   # also theme the logon screen (one UAC prompt)
```

The logon-screen keys live in `HKU\.DEFAULT` and need admin. `--elevate` spawns
a short-lived elevated helper for just those, so the rest of the apply stays in
your current terminal. Already running elevated? No flag needed.

## Supported modules

coat writes **colours and fonts. Nothing else.** Where an app has an include
mechanism, coat writes a fragment beside your config and adds one include line;
where it does not, coat patches only its own keys into your file and leaves every
other line alone. Your geometry, timeouts, keybindings and behaviour stay yours
across a theme change.

| Category | Modules |
|---|---|
| Terminals | kitty, foot |
| Shell | fish |
| Editors | neovim, vscode |
| WM / Compositors | mango, hyprland, sway |
| Bars | waybar, swaybar |
| Screen locker | swaylock |
| Launchers | fuzzel, tofi |
| Notifications | fnott, dunst |
| System | gtk, xresources |
| Utilities | bat, btop, zathura, mpv |
| Browsers | firefox (chrome + content) |
| Other | vesktop |

The one exception is `swaybar`: sway's parser rejects `include` inside a
`bar { }` block, so coat has to emit the whole block. See [USAGE.md](USAGE.md).

See [USAGE.md](USAGE.md) for per-application activation steps, or run `coat docs <app>`.

## License

See [LICENSE](LICENSE).
