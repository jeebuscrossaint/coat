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
├── docs/             # documentation site
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

# Switch scheme and apply everywhere in one shot
coat set catppuccin-mocha

# Apply current scheme from coat.yaml to all enabled apps
coat apply

# Apply to a single app
coat apply kitty
```

## Configuration

Create `~/.config/coat/coat.yaml`:

```yaml
scheme: gruvbox-dark-hard
prefer_base24: false

enabled:
  - fish
  - kitty
  - helix
  - sway
  - waybar
  - fuzzel
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
- Windows Terminal color scheme
- VSCode color customizations

```powershell
coat set nord
coat set gruvbox-dark-hard
```

If run as administrator, also themes the logon screen.

## Supported modules

| Category | Modules |
|---|---|
| Terminals | fish, foot, kitty |
| Editors | helix, vscode |
| WM / Compositors | i3, sway, labwc |
| Bars | waybar |
| Screen locker | swaylock |
| Launchers | fuzzel, rofi |
| Notifications | dunst |
| System | gtk, qt, xresources |
| Utilities | bat, btop, ranger, lf, zathura |
| Other | vesktop |

See [USAGE.md](USAGE.md) for per-application activation steps, or run `coat docs <app>`.

## License

See [LICENSE](LICENSE).
