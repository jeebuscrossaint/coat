# coat — color scheme configurator

A Rust CLI that applies Base16/Base24 color schemes across 22 Linux applications — and themes Windows system colors — from a single config file.

## Features

- **22 application modules** — terminals, editors, window managers, bars, launchers, and more
- **Windows support** — accent color, dark/light mode, Windows Terminal, and VSCode via `coat set`
- **Base16 & Base24** — compatible with the full [tinted-theming](https://github.com/tinted-theming/home) ecosystem (~700 schemes)
- **Scheme browser** — search and preview with live RGB color swatches in the terminal
- **Font & opacity** — centralized font and transparency settings across all modules

## Project structure

```
coat/
├── src/
│   ├── main.rs       # CLI dispatch
│   ├── config.rs     # coat.yaml deserialization
│   ├── scheme.rs     # scheme loading, search, list
│   ├── modules.rs    # all 22 module apply functions
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
coat apply kitty
```

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
  - kitty
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
- Zed theme and buffer font (if Zed is installed)

```powershell
coat set nord
coat set gruvbox-dark-hard
coat set nord --elevate   # also theme the logon screen (one UAC prompt)
```

The logon-screen keys live in `HKU\.DEFAULT` and need admin. `--elevate` spawns
a short-lived elevated helper for just those, so the rest of the apply stays in
your current terminal. Already running elevated? No flag needed.

## Supported modules

| Category | Modules |
|---|---|
| Terminals | fish, foot, kitty |
| Editors | neovim, vscode, code-oss, zed |
| WM / Compositors | sway, hyprland, labwc |
| Bars | swaybar, waybar |
| Screen locker | gtklock |
| Launchers | tofi |
| Notifications | dunst |
| OSD | swayosd |
| System | gtk, xresources |
| Utilities | bat, btop, ranger, zathura |
| Browsers | firefox |
| Other | vesktop |

See [USAGE.md](USAGE.md) for per-application activation steps, or run `coat docs <app>`.

## License

See [LICENSE](LICENSE).
