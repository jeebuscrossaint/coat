# coat - Color Scheme Configuration Tool

A C-based tool for applying Base16 color schemes across multiple applications.

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
│   │   └── tinted_parser.c
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

# Update schemes repository
coat update
```

## Supported Applications

- **Terminals**: fish, kitty, tty
- **Editors**: helix, vscode
- **Window Managers**: i3, sway, swaylock
- **Launchers**: rofi, bemenu
- **Utilities**: bat, btop, cava, dunst, yazi, zathura
- **System**: gtk, xresources, firefox, vesktop
- **Monitoring**: avizo, mangowc

## Adding New Modules

1. Create `src/modules/myapp.c` and `include/myapp.h`
2. Implement `myapp_apply_theme()` function
3. Add to the `app_modules` table in `src/core/main.c`
4. Update `Makefile` APP_SRCS list

## License

See [LICENSE](LICENSE) file.
