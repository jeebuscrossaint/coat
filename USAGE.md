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

### Sway Window Manager

**Permanent activation (add to `~/.config/sway/config`):**
```
include ~/.config/sway/coat-theme
```

**Then reload the config:**
```bash
swaymsg reload
```

**What's themed:**
- Window borders and backgrounds
- Client colors (focused, unfocused, urgent, placeholder)
- Child borders and indicators
- Font configuration (if configured in coat.yaml)

### Swaybar

**Permanent activation (add to `~/.config/sway/config`):**
```
include ~/.config/sway/coat-swaybar-theme
```

**Note:** The generated swaybar configuration includes a complete `bar { }` block, which may conflict with your existing bar configuration. You have two options:

**Option 1:** Comment out or remove your existing bar configuration in `~/.config/sway/config` and use the generated one.

**Option 2:** Manually extract only the `colors { }` section from `~/.config/sway/coat-swaybar-theme` and add it to your existing bar configuration.

**Then reload:**
```bash
swaymsg reload
```

**Customization:**
Edit `~/.config/sway/coat-swaybar-theme` to customize:
- Bar position (top/bottom)
- Status command (use i3status, waybar, etc.)
- Font settings
- Colors remain managed by coat

**What's themed:**
- Bar background and separator
- Statusline text color
- Workspace colors (focused, active, inactive, urgent)
- Binding mode indicator

### Helix Editor

**Permanent activation (add to `~/.config/helix/config.toml`):**
```toml
theme = "coat"
```

**Then reload:**
- Restart helix
- Or use `:config-reload` command

### Hyprland Compositor

**Permanent activation (add to `~/.config/hypr/hyprland.conf`):**
```bash
source = ~/.config/hypr/coat-theme.conf
```

**Then reload:**
```bash
hyprctl reload
```

**What's provided:**
- Base16 color variables (`$base00` through `$base0F`)
- Example theme configurations as comments
- Color reference guide

**Color variables:**
```bash
$base00 = rgb(...)  # Default Background
$base01 = rgb(...)  # Lighter Background
$base02 = rgb(...)  # Selection Background
$base03 = rgb(...)  # Comments, Invisibles
$base04 = rgb(...)  # Dark Foreground
$base05 = rgb(...)  # Default Foreground
$base06 = rgb(...)  # Light Foreground
$base07 = rgb(...)  # Light Background
$base08 = rgb(...)  # Red
$base09 = rgb(...)  # Orange
$base0A = rgb(...)  # Yellow
$base0B = rgb(...)  # Green
$base0C = rgb(...)  # Cyan
$base0D = rgb(...)  # Blue
$base0E = rgb(...)  # Magenta
$base0F = rgb(...)  # Brown
```

**Example usage in your config:**
```bash
general {
    col.active_border = $base0D $base0C 45deg  # Blue to Cyan gradient
    col.inactive_border = $base01  # Subtle dim border
}

decoration {
    col.shadow = rgba(0b0d0daa)  # Dark shadow with alpha
    # Or use: rgba($base00 aa) in newer Hyprland versions
}

group {
    col.border_active = $base0D
    col.border_inactive = $base01
    col.border_locked_active = $base09
    col.border_locked_inactive = $base03
    
    groupbar {
        col.active = $base0D
        col.inactive = $base03
        text_color = $base07
    }
}

misc {
    background_color = $base00
    col.splash = $base0E
}
```

**Tips:**
- Use `$base0D` (blue) for active/focused elements
- Use `$base01`-`$base03` for subtle backgrounds and borders
- Create gradients: `$base0D $base0C 45deg` (blue to cyan)
- The theme file is regenerated each time you run `coat apply hyprland`
- Your main config remains untouched - just source the theme file

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

### Foot Terminal

**Permanent activation (add to `~/.config/foot/foot.ini`):**
```ini
include=~/.config/foot/coat-theme.ini
```

**Then reload:**
- Restart foot, or open a new window

**What's themed:**
- Foreground, background, cursor, selection colors
- All 16 terminal colors (regular + bright)
- Font and size (if configured in coat.yaml)
- Terminal opacity (via `alpha=` in `[colors-dark]`)

### Fuzzel Launcher

Fuzzel's config is written **in full** to `~/.config/fuzzel/fuzzel.ini` — no include needed, and no manual activation step required. The next time you launch fuzzel it will pick up the new config.

**Note:** coat writes the complete fuzzel.ini including layout settings (width, lines, prompt). Edits made directly to that file will be overwritten on the next `coat apply`.

**What's themed:**
- Background, text, match highlight colors
- Selection and selection-match colors
- Border color
- Font and size (uses `sizes.popups` from coat.yaml)

### Mako Notification Daemon

Mako's config is written **in full** to `~/.config/mako/config` and `makoctl reload` is run automatically.

**If mako doesn't reload automatically:**
```bash
makoctl reload
```

**What's themed:**
- Background, text, border, and progress bar colors
- Urgency-level overrides (low/normal/high)
- Font from coat.yaml
- Layout defaults (width, margin, padding, border-radius, anchor)

### Waybar

**Permanent activation (add to `~/.config/waybar/style.css`):**
```css
@import "coat-theme.css";
```

**Then reload:**
```bash
killall -SIGUSR2 waybar
```

**What's provided:**
- CSS custom properties (`@define-color`) for all 16 Base16 colors
- Styled selectors for common modules: workspaces, clock, battery, network, audio, CPU/memory, tray, MPD, idle inhibitor, tooltips
- Font family and size from coat.yaml (uses `sizes.desktop`)

**Customization:** The generated `~/.config/waybar/coat-theme.css` uses `@define-color` so you can reference `@base0D` etc. in your own `style.css`.

### Niri Compositor

**Permanent activation (add to `~/.config/niri/config.kdl`):**
```kdl
include "coat-theme.kdl"
```

**Then reload:**
```bash
niri msg action do-screen-transition
```
or simply save your config.kdl — niri reloads automatically.

**What's provided:**
- `layout {}` block with background-color, focus-ring, border, shadow, tab-indicator, and insert-hint settings
- Full Base16 color reference as comments

### Swaylock

Swaylock's config is written to `~/.config/swaylock/config` directly — no include needed. The theme takes effect the next time you run `swaylock`.

**What's themed:**
- Ring colors for all states (default, cleared, verifying, wrong, caps-lock)
- Inside fill colors per state
- Text colors per state
- Key highlight and backspace highlight colors
- Separator and layout indicator colors

### Labwc

Labwc's `themerc` is written to `~/.config/labwc/themerc` and `labwc --reconfigure` is run automatically.

**If labwc doesn't reconfigure automatically:**
```bash
labwc --reconfigure
```

**What's themed:**
- Active and inactive window titlebar backgrounds and text
- Window border colors (active = blue accent, inactive = subtle)
- Button hover colors
- Right-click menu colors

### Vesktop / Vencord (Discord)

coat writes a CSS theme file to both `~/.config/vesktop/themes/coat.theme.css` and `~/.config/Vencord/themes/coat.theme.css` (whichever directories already exist).

**To activate in Vesktop:**
1. Open Vesktop settings → Themes
2. Enable **coat**

**To activate in Vencord (standalone):**
1. Open Discord → Vencord settings → Themes
2. Enable **coat**

**What's themed:**
- All background surfaces (primary, secondary, tertiary, modals)
- Text colors (normal, muted, brand, links)
- Button colors (brand, positive, danger, secondary)
- Status indicators (online, idle, DND, offline)
- Input fields, checkboxes, scrollbars, tooltips

### Qt Applications (qt5ct / qt6ct)

coat writes color schemes to `~/.config/qt5ct/colors/coat.conf` and `~/.config/qt6ct/colors/coat.conf`, and updates the main qt5ct/qt6ct config to select the coat palette.

**Requirements:**
- `qt5ct` and/or `qt6ct` must be installed
- Set `QT_QPA_PLATFORMTHEME=qt5ct` (or `qt6ct`) in your environment

**To activate:**
```bash
# Set environment variable (add to ~/.profile or shell config)
export QT_QPA_PLATFORMTHEME=qt5ct   # or qt6ct

# Open qt5ct and verify "coat" is selected under Colors
qt5ct
```

**What's themed:**
- Full QPalette (active, disabled, inactive color groups)
- Window, button, text, base, highlight, link, tooltip colors

### Xresources

coat writes to `~/.Xresources` and runs `xrdb -merge ~/.Xresources` automatically.

**What's provided:**
- `#define` macros for all 16 Base16 colors
- URxvt color settings (color0-15, background, foreground, cursor)
- XTerm color settings
- Generic `*.color*` X11 resource settings
- Font settings (if configured in coat.yaml)

**Note:** This overwrites `~/.Xresources`. If you have custom Xresources settings, back them up first.

### Ranger File Manager

coat writes a Python colorscheme to `~/.config/ranger/colorschemes/coat.py`.

**One-time activation (add to `~/.config/ranger/rc.conf`):**
```
set colorscheme coat
```

**What's themed:**
- Directory, executable, symlink, socket, FIFO, and media file colors
- Titlebar and statusbar colors
- VCS (git) status colors in the file browser
- Selected/marked/tagged item highlighting

---

## Quick Start

1. **Edit your config:**
   ```bash
   vim ~/.config/coat/coat.yaml
   ```

2. **Set your scheme and enabled apps:**
   ```yaml
   scheme: gruvbox-dark-hard
   prefer_base24: false
   material_you: false  # Enable Material You color transformation (optional)
   enabled:
     - fish
     - kitty
     - i3
   font:
     monospace: "JetBrains Mono"
   ```
   
   **Material You Transformation:**
   - When `material_you: true`, colors are transformed to be more vibrant and harmonious
   - Increases saturation by 25% for more vivid accent colors
   - Enhances contrast by 15% for better readability
   - Applies color harmonization to create a cohesive palette
   - Neutrals (backgrounds/foregrounds) remain balanced
   - Perfect for modern, colorful desktop environments

3. **Apply the theme:**
   ```bash
   coat apply
   ```

4. **Activate in each application** (see sections above)

---

## Wallpaper Color Extraction

**Automatic theming from your wallpaper** - Extract colors from your current wallpaper and generate a custom theme automatically.

### Setup

Set `scheme: wallpaper` in `~/.config/coat/coat.yaml`:

```yaml
scheme: wallpaper           # Extract colors from wallpaper
material_you: true          # Apply Material You color science (recommended)
enabled:
  - fish
  - kitty
  - hyprland
  # ... other apps
```

### Requirements

- **swww** wallpaper daemon must be running
- Current wallpaper must be accessible

### How It Works

When you run `coat apply` with `scheme: wallpaper`:

1. **Query wallpaper**: Runs `swww query` to get current wallpaper path
2. **Load image**: Reads the wallpaper file (PNG, JPG, etc.)
3. **Color extraction**: Uses k-means clustering to find 16 dominant colors
4. **Color scoring**: Ranks colors by saturation, lightness, and frequency
5. **Palette generation**: 
   - **base00-07** (neutrals): Generated from darkest to lightest colors
   - **base08-0F** (accents): Assigned to most vibrant/saturated colors
6. **Material You**: Applies harmonization and color science (if enabled)
7. **Apply themes**: Updates all enabled applications

### One-Time Usage

You can also extract wallpaper colors without changing your config:

```bash
coat wallpaper                    # Extract and apply with Material You
coat wallpaper --no-material-you  # Extract without Material You
```

### Tips

- **Material You recommended**: Ensures colors are vibrant and harmonious
- **Automatic updates**: Re-run `coat apply` when you change your wallpaper
- **Works with any wallpaper**: PNG, JPG, WebP, etc.
- **Smart color selection**: Avoids dull/extreme colors, prefers mid-tones

---

## Available Commands

- `coat list` - List all available color schemes (with preview)
- `coat list --dark` - Show only dark schemes
- `coat list --light` - Show only light schemes
- `coat wallpaper` - Extract colors from wallpaper and apply (one-time)
- `coat search gruvbox` - Search for schemes
- `coat apply` - Generate theme files for enabled applications
- `coat update` - Update the schemes repository
- `coat help` - Show help message

---

## Supported Applications

**Terminals**
- **fish** - Fish shell syntax highlighting and pager colors
- **foot** - Foot terminal emulator colors, font, and opacity
- **kitty** - Kitty terminal emulator colors and theme
- **tty** - Linux TTY console colors

**Editors**
- **helix** - Helix text editor theme
- **vscode** - Visual Studio Code color theme

**Window Managers / Compositors**
- **hyprland** - Hyprland compositor (color variables)
- **i3** - i3 window manager colors (borders, bar, workspaces)
- **labwc** - Labwc window manager (themerc)
- **niri** - Niri compositor (layout colors, focus ring)
- **sway** - Sway window manager colors (borders, client theming)

**Bars / Panels**
- **waybar** - Waybar CSS theme (modules, workspaces, battery, etc.)
- swaybar (configured via the **sway** module)

**Screen Locker**
- **swaylock** - Swaylock screen locker for Wayland

**Launchers**
- **bemenu** - Bemenu dynamic menu theme
- **fuzzel** - Fuzzel launcher (full config)
- **rofi** - Rofi application launcher theme

**Notifications**
- **dunst** - Dunst notification daemon theme
- **mako** - Mako notification daemon (full config)

**System Theming**
- **avizo** - Avizo notification overlay theme
- **gtk** - GTK 3.0/4.0 applications (GNOME, Nautilus, etc.)
- **qt** - Qt5/Qt6 applications via qt5ct/qt6ct
- **xresources** - X11 Xresources (URxvt, XTerm, generic X11)

**Utilities / Viewers**
- **bat** - Bat syntax highlighter theme
- **btop** - Btop system monitor theme
- **cava** - CAVA audio visualizer theme
- **ranger** - Ranger file manager colorscheme
- **yazi** - Yazi file manager theme
- **zathura** - Zathura document viewer theme

**Other**
- **conky** - Conky system monitor
- **mangowc** - MangoHud / mangowc overlay
- **vesktop** - Vesktop/Vencord (Discord) CSS theme

Coming soon:
- alacritty
- vim/neovim
- tmux
- And more!
