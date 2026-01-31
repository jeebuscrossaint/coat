# coat - Usage Guide

## Applying Themes to Applications

After running `coat apply`, you need to activate the generated themes in each application.

### Fish Shell

**One-time activation:**
```bash
fish_config theme save coat
```

**Permanent activation (add to `~/.config/fish/config.fish`):**
```fish
fish_config theme choose coat
```

**Or manually source the theme:**
```fish
source ~/.config/fish/themes/coat.theme
```

### Kitty Terminal

**Permanent activation (add to `~/.config/kitty/kitty.conf`):**
```
include coat-theme.conf
```

**Then reload the config:**
- Press `Ctrl+Shift+F5` in kitty
- Or restart kitty

### i3 Window Manager

**Permanent activation (add to `~/.config/i3/config`):**
```
include coat-theme.conf
```

**Then reload the config:**
- `i3-msg reload` (done automatically by `coat apply`)
- Or press `$mod+Shift+r`

### Helix Editor

**Permanent activation (add to `~/.config/helix/config.toml`):**
```toml
theme = "coat"
```

**Then reload:**
- Restart helix
- Or use `:config-reload` command

### GTK Applications

GTK theming is applied automatically to `~/.config/gtk-3.0/gtk.css` and `~/.config/gtk-4.0/gtk.css`.

**For immediate effect:**
- Restart GTK applications (GNOME apps, file managers, etc.)
- Some apps may need a full logout/login

**The GTK module themes:**
- All GTK 3.0 applications
- All GTK 4.0 applications
- GNOME applications
- GTK-based file managers (Nautilus, Thunar, etc.)
- Most modern Linux desktop applications

**Note:** This uses Libadwaita color definitions compatible with GNOME 40+ and modern GTK apps.

### Chromium (Chrome, Brave, Edge, Vivaldi, etc.)

Chromium theming is applied by generating a browser extension theme.

**The module automatically:**
- Generates a complete Chrome theme extension with manifest.json
- Themes all UI elements (tabs, toolbar, address bar, new tab page)
- Supports all Chromium-based browsers

**To activate:**
1. Open your browser and navigate to:
   - Chrome: `chrome://extensions/`
   - Brave: `brave://extensions/`
   - Edge: `edge://extensions/`
   - Vivaldi: `vivaldi://extensions/`
2. Enable **Developer mode** (toggle in top-right corner)
3. Click **Load unpacked**
4. Navigate to: `~/.config/coat/chromium/coat-theme/`
5. Select the folder

**The theme will be applied immediately!**

**Features themed:**
- Window frame colors
- Tab colors (active and inactive)
- Toolbar and address bar (omnibox)
- New tab page background and text
- Button colors
- Bookmark bar
- Link colors

**Note:** The theme persists across browser restarts. To change themes, run `coat apply` again with a different scheme.

### Dunst Notification Daemon

Dunst configuration is applied automatically to `~/.config/dunst/dunstrc`.

**The module automatically:**
- Restarts dunst to apply changes immediately
- Configures urgency levels (low, normal, critical) with appropriate colors
- Sets up progress bar colors
- Applies frame and separator colors

**If dunst doesn't restart automatically:**
```bash
killall dunst && dunst &
```

**Features themed:**
- Background and foreground colors for all urgency levels
- Frame colors (low=cyan, normal=blue, critical=red)
- Progress bar styling
- Font configuration from your coat config

---

## Quick Start

1. **Edit your config:**
   ```bash
   vim ~/.config/coat/coat.yaml
   ```

2. **Set your scheme and enabled apps:**
   ```yaml
   scheme: gruvbox-dark-hard
   enabled:
     - fish
     - kitty
     - i3
   font:
     monospace: "JetBrains Mono"
   ```

3. **Apply the theme:**
   ```bash
   coat apply
   ```

4. **Activate in each application** (see sections above)

---

## Available Commands

- `coat list` - List all available color schemes (with preview)
- `coat list --dark` - Show only dark schemes
- `coat list --light` - Show only light schemes
- `coat search gruvbox` - Search for schemes
- `coat apply` - Generate theme files for enabled applications
- `coat update` - Update the schemes repository
- `coat help` - Show help message

---

## Supported Applications

Currently supported:
- **fish** - Fish shell syntax highlighting and pager colors
- **kitty** - Kitty terminal emulator colors and theme
- **i3** - i3 window manager colors (window borders, bar, workspaces)
- **helix** - Helix text editor theme
- **rofi** - Rofi application launcher theme
- **bat** - Bat syntax highlighter theme
- **tty** - Linux TTY console colors
- **avizo** - Avizo notification overlay theme
- **bemenu** - Bemenu dynamic menu theme
- **btop** - Btop system monitor theme
- **cava** - CAVA audio visualizer theme
- **zathura** - Zathura document viewer theme
- **yazi** - Yazi file manager theme
- **vscode** - Visual Studio Code color theme
- **gtk** - GTK 3.0/4.0 applications (GNOME, etc.)
- **dunst** - Dunst notification daemon theme
- **chromium** - Chromium-based browsers (Chrome, Brave, Edge, Vivaldi)

Coming soon:
- alacritty
- vim/neovim
- tmux
- And more!
