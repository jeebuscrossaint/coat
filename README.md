# coat - Color Scheme Configuration Tool

A C-based tool for applying Base16 color schemes across multiple applications.

## Features

- **26+ Application Support**: Unified theming for terminals, editors, window managers, and more
- **Material You Transformation**: Optional color enhancement for more vibrant, harmonious palettes
- **Base16/Base24 Support**: Compatible with the full tinted-theming ecosystem
- **Interactive Browser**: Search and preview schemes with RGB color display
- **Font Management**: Centralized font configuration across all applications
- **Opacity Control**: Per-application transparency settings

## Project Structure

```
coat/
├── include/          # Header files
│   ├── *.h          # All module and core headers
├── src/             # Source files
│   ├── core/        # Core functionality
│   │   ├── main.c
│   │   ├── yaml.c
│   │   ├── schemes.c
│   │   ├── schemes_list.c
│   │   ├── tinted_parser.c
│   │   └── material_you.c
│   └── modules/     # Application modules
│       ├── fish.c
│       ├── kitty.c
│       ├── helix.c
│       └── ... (22 modules total)
├── build/           # Build artifacts (generated)
│   ├── core/
│   └── modules/
├── Makefile
└── USAGE.md
```

## Building

```bash
make            # Build the project
make clean      # Clean build artifacts
make install    # Install to /usr/local/bin
```

## Usage

See [USAGE.md](USAGE.md) for detailed usage instructions.

```bash
# Clone color schemes repository
coat clone

# List available schemes
coat list
coat list --dark
coat list --light

# Apply current scheme to all configured apps
coat apply

# Extract colors from wallpaper and apply (one-time)
coat wallpaper

# Update schemes repository
coat update
```

## Configuration

Edit `~/.config/coat/coat.yaml`:

```yaml
# Use a specific color scheme
scheme: gruvbox-dark-hard

# Or extract colors from your wallpaper automatically
scheme: wallpaper

# Enable Material You color transformation
material_you: true

# Apps to theme
enabled:
  - fish
  - kitty
  - hyprland
  # ... etc
```

**Wallpaper extraction**: Set `scheme: wallpaper` to automatically extract colors from your current wallpaper (requires `swww`). When you run `coat apply`, it will query `swww` for your wallpaper, extract dominant colors using k-means clustering, generate a harmonious Base16 palette, and optionally apply Material You color transformations.

## Supported Applications

- **Terminals**: fish, foot, kitty, tty
- **Editors**: helix, vscode
- **Window Managers**: hyprland, i3, labwc, niri, sway
- **Launchers**: bemenu, fuzzel, rofi
- **Bars**: waybar, swaybar (via sway module)
- **Screen locker**: swaylock
- **Notifications**: dunst, mako
- **System**: avizo, gtk, qt, xresources
- **Utilities**: bat, btop, cava, ranger, yazi, zathura
- **Other**: conky, mangowc, vesktop (Discord)

## Adding New Modules

1. Create `src/modules/myapp.c` and `include/myapp.h`
2. Implement `myapp_apply_theme()` function
3. Add to the `app_modules` table in `src/core/main.c`
4. Update `Makefile` APP_SRCS list

## License

See [LICENSE](LICENSE) file.
