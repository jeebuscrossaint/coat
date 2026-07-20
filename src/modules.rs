use anyhow::{bail, Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use tera::{Tera, Value};

use crate::config::CoatConfig;
use crate::scheme::Scheme;

/// When true, the per-file/per-note chatter below is swallowed — the caller
/// (a progress spinner) is showing a single clean status line per app instead.
pub static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Like `println!`, but suppressed while a progress spinner owns the terminal.
#[macro_export]
macro_rules! detail {
    ($($arg:tt)*) => {
        if !$crate::modules::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    };
}

// ── Template source (embedded at compile time) ─────────────────────────────

macro_rules! tpl {
    ($name:expr, $file:expr) => {
        ($name, include_str!(concat!("../templates/", $file)))
    };
}

static TEMPLATES: &[(&str, &str)] = &[
    tpl!("ashell",    "ashell.tera"),
    tpl!("bat",       "bat.tera"),
    tpl!("btop",      "btop.tera"),
    tpl!("dunst",     "dunst.tera"),
    tpl!("firefox",   "firefox.tera"),
    tpl!("firefox_content", "firefox_content.tera"),
    tpl!("fish",      "fish.tera"),
    tpl!("foot",      "foot.tera"),
    tpl!("fuzzel",    "fuzzel.tera"),
    tpl!("gtk",       "gtk.tera"),
    tpl!("helix",     "helix.tera"),
    tpl!("hyprland",  "hyprland.tera"),
    tpl!("i3",        "i3.tera"),
    tpl!("kde",       "kde.tera"),
    tpl!("kitty",     "kitty.tera"),
    tpl!("konsole",   "konsole.tera"),
    tpl!("labwc",     "labwc.tera"),
    tpl!("lf",        "lf.tera"),
    tpl!("mpv",       "mpv.tera"),
    tpl!("neovim",    "neovim.tera"),
    tpl!("ranger",    "ranger.tera"),
    tpl!("rofi",      "rofi.tera"),
    tpl!("sway",      "sway.tera"),
    tpl!("swaylock",  "swaylock.tera"),
    tpl!("vesktop",   "vesktop.tera"),
    tpl!("waybar",    "waybar.tera"),
    tpl!("xresources","xresources.tera"),
    tpl!("zathura",   "zathura.tera"),
    tpl!("qt_colors", "qt_colors.tera"),
];

// ── Tera setup ─────────────────────────────────────────────────────────────

pub fn make_tera() -> Result<Tera> {
    let mut tera = Tera::default();
    for (name, src) in TEMPLATES {
        tera.add_raw_template(name, src)
            .with_context(|| format!("Failed to parse template '{}'", name))?;
    }

    // nohash: strips '#' from a hex color string
    tera.register_filter(
        "nohash",
        |val: &Value, _: &HashMap<String, Value>| {
            let s = val.as_str().unwrap_or("");
            Ok(Value::String(s.trim_start_matches('#').to_string()))
        },
    );

    // r / g / b: extract R, G, B integer from 6-char uppercase hex
    tera.register_filter(
        "r",
        |val: &Value, _: &HashMap<String, Value>| {
            let (r, _, _) = Scheme::hex_to_rgb(val.as_str().unwrap_or("000000"));
            Ok(Value::Number(r.into()))
        },
    );
    tera.register_filter(
        "g",
        |val: &Value, _: &HashMap<String, Value>| {
            let (_, g, _) = Scheme::hex_to_rgb(val.as_str().unwrap_or("000000"));
            Ok(Value::Number(g.into()))
        },
    );
    tera.register_filter(
        "b",
        |val: &Value, _: &HashMap<String, Value>| {
            let (_, _, b) = Scheme::hex_to_rgb(val.as_str().unwrap_or("000000"));
            Ok(Value::Number(b.into()))
        },
    );

    // lower: lowercase a string (useful for hex that's uppercase in our store)
    tera.register_filter(
        "lower_hex",
        |val: &Value, _: &HashMap<String, Value>| {
            Ok(Value::String(val.as_str().unwrap_or("").to_lowercase()))
        },
    );

    Ok(tera)
}

// ── Context builder ────────────────────────────────────────────────────────

fn build_context(scheme: &Scheme, config: &CoatConfig) -> tera::Context {
    let mut ctx = tera::Context::new();
    // Colors — uppercase 6-char hex without '#'
    ctx.insert("base00", &scheme.base00);
    ctx.insert("base01", &scheme.base01);
    ctx.insert("base02", &scheme.base02);
    ctx.insert("base03", &scheme.base03);
    ctx.insert("base04", &scheme.base04);
    ctx.insert("base05", &scheme.base05);
    ctx.insert("base06", &scheme.base06);
    ctx.insert("base07", &scheme.base07);
    ctx.insert("base08", &scheme.base08);
    ctx.insert("base09", &scheme.base09);
    ctx.insert("base0A", &scheme.base0a);
    ctx.insert("base0B", &scheme.base0b);
    ctx.insert("base0C", &scheme.base0c);
    ctx.insert("base0D", &scheme.base0d);
    ctx.insert("base0E", &scheme.base0e);
    ctx.insert("base0F", &scheme.base0f);
    // Base24 extended colors (empty strings for Base16 schemes)
    ctx.insert("base10", &scheme.base10);
    ctx.insert("base11", &scheme.base11);
    ctx.insert("base12", &scheme.base12);
    ctx.insert("base13", &scheme.base13);
    ctx.insert("base14", &scheme.base14);
    ctx.insert("base15", &scheme.base15);
    ctx.insert("base16", &scheme.base16_color);
    ctx.insert("base17", &scheme.base17);
    ctx.insert("is_base24", &scheme.is_base24);
    // Scheme metadata
    ctx.insert("scheme_name", &scheme.name);
    ctx.insert("scheme_author", &scheme.author);
    ctx.insert("scheme_variant", &scheme.variant);
    ctx.insert("is_dark", &scheme.is_dark());
    // Font
    ctx.insert("font_monospace", config.font_monospace());
    ctx.insert("font_sansserif", config.font_sansserif());
    ctx.insert("font_serif", config.font_serif());
    ctx.insert("font_emoji", config.font_emoji());
    ctx.insert("font_size_terminal", &config.font_size_terminal());
    ctx.insert("font_size_desktop", &config.font_size_desktop());
    ctx.insert("font_size_popups", &config.font_size_popups());
    // Opacity
    ctx.insert("opacity_terminal", &config.opacity_terminal());
    ctx.insert("opacity_desktop", &config.opacity_desktop());
    ctx.insert("opacity_popups", &config.opacity_popups());
    ctx.insert("opacity_applications", &config.opacity_applications());
    ctx.insert("opacity_popups_hex", &config.opacity_popups_hex());
    ctx
}

// ── Helper utilities ───────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("Cannot determine home directory")
}

fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {}", path.display()))?;
    }
    Ok(())
}

fn render_to(tera: &Tera, name: &str, ctx: &tera::Context, dest: &Path) -> Result<()> {
    ensure_dir(dest.parent().unwrap_or(Path::new("/")))?;
    let content = tera
        .render(name, ctx)
        .with_context(|| format!("Failed to render template '{}'", name))?;
    fs::write(dest, &content)
        .with_context(|| format!("Failed to write {}", dest.display()))?;
    detail!("  ✓ {}", dest.display());
    Ok(())
}

/// Fire off a shell command with its stdio fully silenced and DON'T wait for it —
/// these are best-effort reload hooks (bat cache rebuilds, dunst restarts, ...)
/// whose own chatter isn't ours to show inside a clean per-app status line, and
/// which can each take hundreds of ms. Spawning instead of blocking lets the
/// reloads finish in the background (reparented to init once coat exits) so
/// `apply` returns as soon as the config files are written.
fn run(cmd: &str) {
    let result = Command::new("sh")
        .args(["-c", cmd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(e) = result {
        detail!("  warning: {}", e);
    }
}

// ── Module dispatch ────────────────────────────────────────────────────────

pub const ALL_MODULES: &[&str] = &[
    "ashell", "bat", "btop", "code-oss", "dunst", "firefox", "fish", "foot", "fuzzel", "gtk",
    "helix", "hyprland", "i3", "kde", "kitty", "labwc", "lf", "mpv", "neovim", "qt", "ranger", "rofi",
    "sway", "swaylock", "vesktop", "vscode", "waybar", "xresources", "zathura", "zed",
];

pub fn module_aliases(name: &str) -> Option<&'static str> {
    match name {
        "vencord" | "discord" => Some("vesktop"),
        "plasma" | "kde-plasma" => Some("kde"),
        "codeoss" | "code_oss" | "vscode-oss" => Some("code-oss"),
        "nvim" | "vim" => Some("neovim"),
        "hypr" => Some("hyprland"),
        _ => None,
    }
}

pub fn apply_module(name: &str, scheme: &Scheme, config: &CoatConfig, tera: &Tera) -> Result<()> {
    let name = module_aliases(name).unwrap_or(name);
    let ctx = build_context(scheme, config);

    // On Windows, cross-platform apps live at native locations (%APPDATA%, …)
    // rather than the XDG paths the Linux module functions assume. Route them
    // to the Windows-specific apply functions so `coat apply <app>` works too.
    #[cfg(windows)]
    {
        match name {
            "vscode"   => return crate::windows::apply_vscode(scheme),
            "zed"      => return crate::windows::apply_zed(scheme),
            "vesktop"  => return crate::windows::apply_discord(scheme, config),
            _ => {}
        }
    }

    match name {
        "ashell"     => apply_ashell(tera, &ctx, scheme, config),
        "bat"        => apply_bat(tera, &ctx, scheme, config),
        "btop"       => apply_btop(tera, &ctx, scheme, config),
        "code-oss"   => apply_code_oss(scheme, config),
        "dunst"      => apply_dunst(tera, &ctx, scheme, config),
        "firefox"    => apply_firefox(tera, &ctx, scheme, config),
        "fish"       => apply_fish(tera, &ctx, scheme, config),
        "foot"       => apply_foot(tera, &ctx, scheme, config),
        "fuzzel"     => apply_fuzzel(tera, &ctx, scheme, config),
        "gtk"        => apply_gtk(tera, &ctx, scheme, config),
        "helix"      => apply_helix(tera, &ctx, scheme, config),
        "hyprland"   => apply_hyprland(tera, &ctx, scheme, config),
        "i3"         => apply_i3(tera, &ctx, scheme, config),
        "kde"        => apply_kde(tera, &ctx, scheme, config),
        "kitty"      => apply_kitty(tera, &ctx, scheme, config),
        "labwc"      => apply_labwc(tera, &ctx, scheme, config),
        "lf"         => apply_lf(tera, &ctx, scheme, config),
        "mpv"        => apply_mpv(tera, &ctx, scheme, config),
        "neovim"     => apply_neovim(tera, &ctx, scheme, config),
        "qt"         => apply_qt(scheme, config),
        "ranger"     => apply_ranger(tera, &ctx, scheme, config),
        "rofi"       => apply_rofi(tera, &ctx, scheme, config),
        "sway"       => apply_sway(tera, &ctx, scheme, config),
        "swaylock"   => apply_swaylock(tera, &ctx, scheme, config),
        "vesktop"    => apply_vesktop(tera, &ctx, scheme, config),
        "vscode"     => apply_vscode(scheme, config),
        "waybar"     => apply_waybar(tera, &ctx, scheme, config),
        "xresources" => apply_xresources(tera, &ctx, scheme, config),
        "zathura"    => apply_zathura(tera, &ctx, scheme, config),
        "zed"        => apply_zed(scheme, config),
        other        => bail!("Unknown module: {}", other),
    }
}

// ── Individual module functions ────────────────────────────────────────────

fn apply_ashell(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/ashell/config.toml");
    render_to(tera, "ashell", ctx, &dest)?;
    // ashell watches its config and hot-reloads, so no restart needed.
    Ok(())
}

fn apply_bat(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/bat/themes/coat.tmTheme");
    render_to(tera, "bat", ctx, &dest)?;
    detail!("    Run: bat cache --build");
    run("bat cache --build 2>/dev/null");
    Ok(())
}

fn apply_btop(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/btop/themes/coat.theme");
    render_to(tera, "btop", ctx, &dest)
}

fn apply_dunst(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/dunst/dunstrc");
    render_to(tera, "dunst", ctx, &dest)?;
    run("killall dunst 2>/dev/null; dunst &");
    Ok(())
}

fn apply_fish(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/fish/themes/coat.theme");
    render_to(tera, "fish", ctx, &dest)
}

fn apply_foot(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/foot/coat-theme.ini");
    render_to(tera, "foot", ctx, &dest)
}

fn apply_fuzzel(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/fuzzel/fuzzel.ini");
    render_to(tera, "fuzzel", ctx, &dest)
}

fn apply_gtk(tera: &Tera, ctx: &tera::Context, scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // Write same template to both gtk-3.0 and gtk-4.0
    for ver in &["gtk-3.0", "gtk-4.0"] {
        let dest = home.join(format!(".config/{}/gtk.css", ver));
        render_to(tera, "gtk", ctx, &dest)?;
    }
    // gsettings calls
    let theme = if scheme.is_dark() { "adw-gtk3-dark" } else { "adw-gtk3" };
    run(&format!("gsettings set org.gnome.desktop.interface gtk-theme '{}'", theme));
    if !config.font_sansserif().is_empty() {
        run(&format!(
            "gsettings set org.gnome.desktop.interface font-name '{} {}'",
            config.font_sansserif(),
            config.font_size_desktop()
        ));
    }
    if !config.font_monospace().is_empty() {
        run(&format!(
            "gsettings set org.gnome.desktop.interface monospace-font-name '{} {}'",
            config.font_monospace(),
            config.font_size_terminal()
        ));
    }
    if !config.font_emoji().is_empty() {
        run(&format!(
            "gsettings set org.gnome.desktop.interface document-font-name '{} {}'",
            config.font_emoji(),
            config.font_size_desktop()
        ));
    }
    run("pkill -HUP -f 'gtk' 2>/dev/null; true");
    Ok(())
}

fn apply_helix(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/helix/themes/coat.toml");
    render_to(tera, "helix", ctx, &dest)
}

fn apply_hyprland(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/hypr/coat-theme.conf");
    render_to(tera, "hyprland", ctx, &dest)?;
    // Live-reload a running Hyprland session (best-effort, silenced).
    run("hyprctl reload 2>/dev/null");
    Ok(())
}

fn apply_i3(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/i3/coat-theme.conf");
    render_to(tera, "i3", ctx, &dest)?;
    run("i3-msg reload 2>/dev/null");
    Ok(())
}

fn rgb(hex: &str) -> String {
    let (r, g, b) = Scheme::hex_to_rgb(hex);
    format!("{},{},{}", r, g, b)
}

fn kde_color_sections(s: &Scheme) -> String {
    let mut o = String::new();

    for sec in &["Colors:Button", "Colors:Complementary", "Colors:Header", "Colors:Tooltip"] {
        o += &format!("[{}]\n", sec);
        o += &format!("BackgroundAlternate={}\n", rgb(&s.base02));
        o += &format!("BackgroundNormal={}\n",    rgb(&s.base01));
        o += &format!("DecorationFocus={}\n",     rgb(&s.base0d));
        o += &format!("DecorationHover={}\n",     rgb(&s.base0c));
        o += &format!("ForegroundActive={}\n",    rgb(&s.base06));
        o += &format!("ForegroundInactive={}\n",  rgb(&s.base03));
        o += &format!("ForegroundLink={}\n",      rgb(&s.base0d));
        o += &format!("ForegroundNegative={}\n",  rgb(&s.base08));
        o += &format!("ForegroundNeutral={}\n",   rgb(&s.base09));
        o += &format!("ForegroundNormal={}\n",    rgb(&s.base05));
        o += &format!("ForegroundPositive={}\n",  rgb(&s.base0b));
        o += &format!("ForegroundVisited={}\n",   rgb(&s.base0e));
        o += "\n";
    }

    o += &format!("[Colors:Selection]\n\
        BackgroundAlternate={}\nBackgroundNormal={}\n\
        DecorationFocus={}\nDecorationHover={}\n\
        ForegroundActive={}\nForegroundInactive={}\n\
        ForegroundLink={}\nForegroundNegative={}\n\
        ForegroundNeutral={}\nForegroundNormal={}\n\
        ForegroundPositive={}\nForegroundVisited={}\n\n",
        rgb(&s.base0e), rgb(&s.base0d),
        rgb(&s.base0d), rgb(&s.base0c),
        rgb(&s.base07), rgb(&s.base01),
        rgb(&s.base07), rgb(&s.base08),
        rgb(&s.base09), rgb(&s.base00),
        rgb(&s.base0b), rgb(&s.base06));

    for (sec, bg, bg_alt, fg_active) in &[
        ("Colors:View",   &s.base00, &s.base01, &s.base06),
        ("Colors:Window", &s.base00, &s.base01, &s.base07),
    ] {
        o += &format!("[{}]\n", sec);
        o += &format!("BackgroundAlternate={}\n", rgb(bg_alt));
        o += &format!("BackgroundNormal={}\n",    rgb(bg));
        o += &format!("DecorationFocus={}\n",     rgb(&s.base0d));
        o += &format!("DecorationHover={}\n",     rgb(&s.base0c));
        o += &format!("ForegroundActive={}\n",    rgb(fg_active));
        o += &format!("ForegroundInactive={}\n",  rgb(&s.base03));
        o += &format!("ForegroundLink={}\n",      rgb(&s.base0d));
        o += &format!("ForegroundNegative={}\n",  rgb(&s.base08));
        o += &format!("ForegroundNeutral={}\n",   rgb(&s.base09));
        o += &format!("ForegroundNormal={}\n",    rgb(&s.base05));
        o += &format!("ForegroundPositive={}\n",  rgb(&s.base0b));
        o += &format!("ForegroundVisited={}\n",   rgb(&s.base0e));
        o += "\n";
    }

    o += &format!("[WM]\n\
        activeBackground={}\nactiveBlend={}\nactiveForeground={}\n\
        inactiveBackground={}\ninactiveBlend={}\ninactiveForeground={}\n\n",
        rgb(&s.base01), rgb(&s.base05), rgb(&s.base05),
        rgb(&s.base00), rgb(&s.base03), rgb(&s.base03));

    o += "[ColorEffects:Disabled]\n\
        ChangeSelectionColor=true\nColor=56,56,56\nColorAmount=0\n\
        ColorEffect=0\nContrastAmount=0.65\nContrastEffect=1\n\
        Enable=true\nIntensityAmount=0.1\nIntensityEffect=2\n\n";

    o += "[ColorEffects:Inactive]\n\
        ChangeSelectionColor=true\nColor=112,111,110\nColorAmount=0.025\n\
        ColorEffect=2\nContrastAmount=0.1\nContrastEffect=0\n\
        Enable=false\nIntensityAmount=0\nIntensityEffect=0\n\n";

    o
}

// Write color sections directly into kdeglobals, bypassing plasma-apply-colorscheme.
// That tool silently skips re-application when the scheme name already matches
// (it checks ColorScheme= and ColorSchemeHash= in [General]), so it never updates
// colors when you switch between schemes while keeping the "coat" name.
fn update_kdeglobals(scheme: &Scheme) -> Result<()> {
    let home = home_dir()?;
    let path = home.join(".config/kdeglobals");

    let owned: &[&str] = &[
        "Colors:Button", "Colors:Complementary", "Colors:Header",
        "Colors:Selection", "Colors:Tooltip", "Colors:View", "Colors:Window",
        "WM", "ColorEffects:Disabled", "ColorEffects:Inactive",
    ];

    let existing = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut out = String::new();
    let mut in_owned = false;
    let mut in_general = false;
    let mut wrote_colorscheme = false;
    let mut saw_general = false;

    for line in existing.lines() {
        if line.starts_with('[') && line.ends_with(']') {
            let sec = &line[1..line.len() - 1];
            if in_general && !wrote_colorscheme {
                out.push_str("ColorScheme=coat\n");
                wrote_colorscheme = true;
            }
            in_owned   = owned.contains(&sec);
            in_general = sec == "General";
            if in_general { saw_general = true; }
        }
        if in_owned { continue; }
        if in_general {
            if line.starts_with("ColorScheme=") {
                out.push_str("ColorScheme=coat\n");
                wrote_colorscheme = true;
                continue;
            }
            if line.starts_with("ColorSchemeHash=") { continue; }
        }
        out.push_str(line);
        out.push('\n');
    }

    if in_general && !wrote_colorscheme {
        out.push_str("ColorScheme=coat\n");
        wrote_colorscheme = true;
    }
    if !saw_general {
        out.push_str("\n[General]\nColorScheme=coat\n");
        wrote_colorscheme = true;
    }
    let _ = wrote_colorscheme;

    out.push('\n');
    out.push_str(&kde_color_sections(scheme));

    let out = out.trim_end().to_string() + "\n";
    ensure_dir(path.parent().unwrap_or(std::path::Path::new("/")))?;
    fs::write(&path, &out)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    detail!("  ✓ {}", path.display());
    Ok(())
}

fn apply_kde(tera: &Tera, ctx: &tera::Context, scheme: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;

    // coat.colors — available in System Settings colour picker
    let colors_dir = home.join(".local/share/color-schemes");
    ensure_dir(&colors_dir)?;
    render_to(tera, "kde", ctx, &colors_dir.join("coat.colors"))?;

    // Konsole colour scheme
    let konsole_dir = home.join(".local/share/konsole");
    ensure_dir(&konsole_dir)?;
    render_to(tera, "konsole", ctx, &konsole_dir.join("coat.colorscheme"))?;

    // Write colours directly into kdeglobals (ensures correct values even if
    // plasma-apply-colorscheme is unavailable)
    update_kdeglobals(scheme)?;

    // plasma-apply-colorscheme silently skips when the scheme name in kdeglobals
    // already matches. Reset it first so it always does a full live apply.
    run("kwriteconfig6 --file kdeglobals --group General --key ColorScheme _coat_reset 2>/dev/null");
    run("plasma-apply-colorscheme coat 2>/dev/null");

    Ok(())
}

fn apply_kitty(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/kitty/coat-theme.conf");
    render_to(tera, "kitty", ctx, &dest)?;
    let path_str = dest.to_string_lossy();
    let live_cmd = format!(
        "kitty @ --to unix:/tmp/kitty set-colors --all --configured {} 2>/dev/null",
        path_str
    );
    let live_ok = Command::new("sh")
        .args(["-c", &live_cmd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !live_ok {
        run("pkill -SIGUSR1 kitty 2>/dev/null");
    }
    Ok(())
}

fn apply_labwc(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/labwc/themerc");
    render_to(tera, "labwc", ctx, &dest)?;
    run("labwc --reconfigure 2>/dev/null; true");
    Ok(())
}

fn apply_mpv(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/mpv/coat-theme.conf");
    render_to(tera, "mpv", ctx, &dest)?;
    detail!("    Add to ~/.config/mpv/mpv.conf: include ~/.config/mpv/coat-theme.conf");
    Ok(())
}

fn apply_neovim(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // Write a standard colorscheme onto Neovim's default runtimepath
    // ($XDG_DATA_HOME/nvim/site is always on 'rtp'), so any config can do
    // `:colorscheme coat` regardless of where its own files live.
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let dest = data.join("nvim/site/colors/coat.lua");
    render_to(tera, "neovim", ctx, &dest)?;
    detail!("    Set in your Neovim config: vim.cmd.colorscheme(\"coat\")");
    detail!("    (Running Neovim instances: run  :colorscheme coat  to reload.)");
    Ok(())
}

fn firefox_profile_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    // Linux: XDG path first, then legacy ~/.mozilla.
    let mut candidates = vec![
        home.join(".config/mozilla/firefox/profiles.ini"),
        home.join(".mozilla/firefox/profiles.ini"),
    ];
    // Windows: Firefox stores profiles under %APPDATA%\Mozilla\Firefox.
    if let Ok(appdata) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(appdata).join(r"Mozilla\Firefox\profiles.ini"));
    }
    let ini_path = candidates.into_iter().find(|p| p.exists())?;
    let content = fs::read_to_string(&ini_path).ok()?;

    // Parse all sections into (name, key→value) pairs
    let mut sections: Vec<(String, HashMap<String, String>)> = Vec::new();
    let mut cur_name = String::new();
    let mut cur_map: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if !cur_name.is_empty() {
                sections.push((cur_name.clone(), cur_map.clone()));
                cur_map.clear();
            }
            cur_name = line[1..line.len() - 1].to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            cur_map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if !cur_name.is_empty() {
        sections.push((cur_name, cur_map));
    }

    // [Install*] Default= is the most reliable pointer to the default profile
    let mut path_str: Option<String> = None;
    let mut relative = true;
    for (sec, map) in &sections {
        if sec.starts_with("Install") {
            if let Some(p) = map.get("Default") {
                path_str = Some(p.clone());
                break;
            }
        }
    }
    // Fall back to [Profile*] with Default=1
    if path_str.is_none() {
        for (sec, map) in &sections {
            if sec.starts_with("Profile") && map.get("Default").map(|s| s == "1").unwrap_or(false) {
                path_str = map.get("Path").cloned();
                relative = map.get("IsRelative").map(|s| s == "1").unwrap_or(true);
                break;
            }
        }
    }
    // Fall back to first profile
    if path_str.is_none() {
        for (sec, map) in &sections {
            if sec.starts_with("Profile") {
                path_str = map.get("Path").cloned();
                relative = map.get("IsRelative").map(|s| s == "1").unwrap_or(true);
                break;
            }
        }
    }

    let p = path_str?;
    if relative {
        Some(ini_path.parent()?.join(&p))
    } else {
        Some(PathBuf::from(&p))
    }
}

fn apply_firefox(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let Some(profile) = firefox_profile_dir() else {
        bail!("Firefox profile not found — is Firefox installed?");
    };

    // Write userChrome.css (browser UI) and userContent.css (about:/new-tab pages)
    let chrome_dir = profile.join("chrome");
    ensure_dir(&chrome_dir)?;
    render_to(tera, "firefox", ctx, &chrome_dir.join("userChrome.css"))?;
    render_to(tera, "firefox_content", ctx, &chrome_dir.join("userContent.css"))?;

    // Ensure toolkit.legacyUserProfileCustomizations.stylesheets is enabled
    let user_js = profile.join("user.js");
    let existing = if user_js.exists() {
        fs::read_to_string(&user_js).unwrap_or_default()
    } else {
        String::new()
    };
    let pref = "toolkit.legacyUserProfileCustomizations.stylesheets";
    if !existing.contains(pref) {
        let appended = format!(
            "{}user_pref(\"{}\", true);\n",
            if existing.ends_with('\n') || existing.is_empty() { existing } else { existing + "\n" },
            pref
        );
        fs::write(&user_js, appended)
            .with_context(|| format!("Failed to write {}", user_js.display()))?;
        detail!("  ✓ {}", user_js.display());
    }

    detail!("    Restart Firefox for changes to take effect.");
    Ok(())
}

fn apply_lf(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/lf/colors");
    render_to(tera, "lf", ctx, &dest)?;
    detail!("    Add to ~/.config/lf/lfrc: set color true");
    Ok(())
}

fn apply_ranger(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/ranger/colorschemes/coat.py");
    render_to(tera, "ranger", ctx, &dest)?;
    detail!("    Add to ~/.config/ranger/rc.conf: set colorscheme coat");
    Ok(())
}

fn apply_rofi(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/rofi/coat.rasi");
    render_to(tera, "rofi", ctx, &dest)
}

fn apply_sway(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/sway/coat-theme");
    render_to(tera, "sway", ctx, &dest)
}

fn apply_swaylock(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/swaylock/config");
    render_to(tera, "swaylock", ctx, &dest)
}

/// Render the vesktop CSS theme to any path — used by the Windows `apply_discord` function.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn apply_vesktop_shared(scheme: &Scheme, config: &CoatConfig, path: &Path) -> Result<()> {
    let tera = make_tera()?;
    let ctx = build_context(scheme, config);
    render_to(&tera, "vesktop", &ctx, path)
}

fn apply_vesktop(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let paths = [
        home.join(".config/vesktop/themes"),
        home.join(".config/Vencord/themes"),
    ];
    let mut wrote = false;
    for dir in &paths {
        if dir.is_dir() {
            let dest = dir.join("coat.theme.css");
            render_to(tera, "vesktop", ctx, &dest)?;
            wrote = true;
        }
    }
    if !wrote {
        // Write to vesktop path anyway (create it)
        let dest = paths[0].join("coat.theme.css");
        render_to(tera, "vesktop", ctx, &dest)?;
    }
    Ok(())
}

fn apply_waybar(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/waybar/coat-theme.css");
    render_to(tera, "waybar", ctx, &dest)
}

fn apply_xresources(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".Xresources");
    render_to(tera, "xresources", ctx, &dest)?;
    run(&format!("xrdb -merge {} 2>/dev/null", dest.display()));
    Ok(())
}

fn apply_zathura(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/zathura/zathurarc");
    render_to(tera, "zathura", ctx, &dest)
}

// ── Qt — handled directly (conditional directory detection) ───────────────

fn apply_qt(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let qt5_dir = home.join(".config/qt5ct");
    let qt6_dir = home.join(".config/qt6ct");

    let mut applied = 0;
    for (dir, conf_name) in &[(&qt5_dir, "qt5ct.conf"), (&qt6_dir, "qt6ct.conf")] {
        if !dir.is_dir() {
            continue;
        }
        let colors_dir = dir.join("colors");
        ensure_dir(&colors_dir)?;

        // Write color scheme file
        let scheme_path = colors_dir.join("coat.conf");
        let colors_content = build_qt_colors(scheme, config);
        fs::write(&scheme_path, &colors_content)
            .with_context(|| format!("Failed to write {}", scheme_path.display()))?;
        detail!("  ✓ {}", scheme_path.display());

        // Write main conf
        let conf_path = dir.join(conf_name);
        let conf_content = build_qt_conf(&scheme_path, config);
        fs::write(&conf_path, &conf_content)
            .with_context(|| format!("Failed to write {}", conf_path.display()))?;
        detail!("  ✓ {}", conf_path.display());
        applied += 1;
    }

    if applied == 0 {
        bail!("Neither qt5ct nor qt6ct config directory found — install and run one once to initialise");
    }
    detail!("    Set QT_QPA_PLATFORMTHEME=qt5ct (or qt6ct) in your environment.");
    Ok(())
}

fn build_qt_colors(scheme: &Scheme, _config: &CoatConfig) -> String {
    let c = |color: &str| format!("#{}", color);
    format!(
        "# coat Qt color scheme: {}\n# {}\n\n[ColorScheme]\n\
active_colors=\
{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}\n\
disabled_colors=\
{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}\n\
inactive_colors=\
{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}\n",
        scheme.name,
        scheme.author,
        // active
        c(&scheme.base05), c(&scheme.base02), c(&scheme.base03), c(&scheme.base02),
        c(&scheme.base01), c(&scheme.base02), c(&scheme.base05), c(&scheme.base07),
        c(&scheme.base05), c(&scheme.base00), c(&scheme.base01), c(&scheme.base00),
        c(&scheme.base0d), c(&scheme.base07), c(&scheme.base0d), c(&scheme.base0e),
        c(&scheme.base01), c(&scheme.base01), c(&scheme.base05), c(&scheme.base04),
        // disabled
        c(&scheme.base03), c(&scheme.base01), c(&scheme.base02), c(&scheme.base02),
        c(&scheme.base01), c(&scheme.base01), c(&scheme.base03), c(&scheme.base04),
        c(&scheme.base03), c(&scheme.base00), c(&scheme.base00), c(&scheme.base00),
        c(&scheme.base02), c(&scheme.base03), c(&scheme.base03), c(&scheme.base03),
        c(&scheme.base00), c(&scheme.base00), c(&scheme.base03), c(&scheme.base03),
        // inactive
        c(&scheme.base04), c(&scheme.base01), c(&scheme.base02), c(&scheme.base02),
        c(&scheme.base01), c(&scheme.base01), c(&scheme.base04), c(&scheme.base05),
        c(&scheme.base04), c(&scheme.base00), c(&scheme.base01), c(&scheme.base00),
        c(&scheme.base03), c(&scheme.base06), c(&scheme.base0d), c(&scheme.base0e),
        c(&scheme.base00), c(&scheme.base01), c(&scheme.base04), c(&scheme.base03),
    )
}

fn build_qt_conf(colors_path: &Path, config: &CoatConfig) -> String {
    let mut out = String::new();
    out.push_str("[Appearance]\n");
    out.push_str(&format!("color_scheme_path={}\n", colors_path.display()));
    out.push_str("custom_palette=true\n");
    out.push_str("standard_dialogs=default\n");
    out.push_str("style=Fusion\n\n");

    let has_font = !config.font_monospace().is_empty() || !config.font_sansserif().is_empty();
    if has_font {
        out.push_str("[Fonts]\n");
        if !config.font_monospace().is_empty() {
            out.push_str(&format!(
                "fixed=\"{},-1,5,50,0,0,0,1,0\"\n",
                config.font_monospace()
            ));
        }
        if !config.font_sansserif().is_empty() {
            out.push_str(&format!(
                "general=\"{},-1,5,50,0,0,0,0,0\"\n",
                config.font_sansserif()
            ));
        }
        out.push('\n');
    }
    out
}

// ── JSONC-safe settings reading ───────────────────────────────────────────

/// Strip `//` line comments, `/* */` block comments, and trailing commas from a
/// JSONC document so `serde_json` can parse it. String literals are preserved
/// verbatim. Editors like VSCode and Zed use JSONC for their settings files.
fn strip_jsonc(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                out.push(c);
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2; // skip the closing */
            }
            b'}' | b']' => {
                // Drop any trailing comma (and intervening whitespace) before a
                // closing bracket — JSONC allows it, strict JSON does not.
                while matches!(out.last(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                    out.pop();
                }
                if out.last() == Some(&b',') {
                    out.pop();
                }
                out.push(c);
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }

    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

/// Read an existing settings file as a JSON object, tolerating JSONC.
/// Returns an error (rather than an empty map) if the file can't be parsed,
/// so a malformed file is never silently overwritten and its contents lost.
pub fn read_json_settings(path: &Path) -> Result<serde_json::Map<String, JsonValue>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(serde_json::Map::new());
    }
    let cleaned = strip_jsonc(&content);
    let value: JsonValue = serde_json::from_str(&cleaned).with_context(|| {
        format!(
            "Failed to parse {} as JSON — refusing to overwrite it",
            path.display()
        )
    })?;
    match value {
        JsonValue::Object(m) => Ok(m),
        _ => bail!("{} is not a JSON object", path.display()),
    }
}

// ── VSCode — handled directly (JSON merge) ────────────────────────────────

/// Shared VSCode apply logic. Works on any `path` (Linux or Windows).
/// `font` is optional; pass `None` to skip font injection.
pub fn apply_vscode_shared(scheme: &Scheme, path: &Path, font: Option<&str>) -> Result<()> {
    // Read existing settings or start fresh (JSONC-tolerant; never clobbers
    // a file it can't parse).
    let mut settings: serde_json::Map<String, JsonValue> = if path.exists() {
        read_json_settings(path)?
    } else {
        if let Some(parent) = path.parent() {
            ensure_dir(parent)?;
        }
        serde_json::Map::new()
    };

    let s = scheme;
    let h = |c: &str| format!("#{}", c);
    let ha = |c: &str, a: &str| format!("#{}{}", c, a);

    let ui_colors = serde_json::json!({
        "editor.background":                  h(&s.base00),
        "editor.foreground":                  h(&s.base05),
        "editor.lineHighlightBackground":     h(&s.base01),
        "editor.selectionBackground":         h(&s.base02),
        "editor.selectionHighlightBackground":ha(&s.base02, "80"),
        "editor.inactiveSelectionBackground": ha(&s.base02, "60"),
        "editor.findMatchBackground":         h(&s.base0a),
        "editor.findMatchHighlightBackground":ha(&s.base0a, "80"),
        "editorCursor.foreground":            h(&s.base05),
        "editorLineNumber.foreground":        h(&s.base03),
        "editorLineNumber.activeForeground":  h(&s.base04),
        "editorGutter.background":            h(&s.base00),
        "editorGroup.border":                 h(&s.base02),
        "editorGroupHeader.tabsBackground":   h(&s.base01),
        "editorIndentGuide.background1":      h(&s.base02),
        "editorIndentGuide.activeBackground1":h(&s.base03),
        "editorWhitespace.foreground":        h(&s.base03),
        "editorBracketMatch.background":      ha(&s.base02, "80"),
        "editorBracketMatch.border":          h(&s.base0d),
        "activityBar.background":             h(&s.base01),
        "activityBar.foreground":             h(&s.base05),
        "activityBar.inactiveForeground":     h(&s.base03),
        "activityBar.border":                 h(&s.base01),
        "activityBarBadge.background":        h(&s.base0d),
        "activityBarBadge.foreground":        h(&s.base00),
        "sideBar.background":                 h(&s.base01),
        "sideBar.foreground":                 h(&s.base05),
        "sideBarSectionHeader.background":    h(&s.base02),
        "sideBarSectionHeader.foreground":    h(&s.base05),
        "list.activeSelectionBackground":     h(&s.base02),
        "list.activeSelectionForeground":     h(&s.base05),
        "list.inactiveSelectionBackground":   h(&s.base01),
        "list.hoverBackground":               h(&s.base01),
        "list.hoverForeground":               h(&s.base05),
        "list.focusBackground":               h(&s.base02),
        "list.highlightForeground":           h(&s.base0d),
        "statusBar.background":               h(&s.base01),
        "statusBar.foreground":               h(&s.base05),
        "statusBar.noFolderBackground":       h(&s.base01),
        "statusBar.debuggingBackground":      h(&s.base09),
        "statusBarItem.hoverBackground":      h(&s.base02),
        "titleBar.activeBackground":          h(&s.base01),
        "titleBar.activeForeground":          h(&s.base05),
        "titleBar.inactiveBackground":        h(&s.base00),
        "titleBar.inactiveForeground":        h(&s.base03),
        "tab.activeBackground":               h(&s.base00),
        "tab.activeForeground":               h(&s.base05),
        "tab.inactiveBackground":             h(&s.base01),
        "tab.inactiveForeground":             h(&s.base03),
        "tab.border":                         h(&s.base01),
        "tab.activeBorderTop":                h(&s.base0d),
        "breadcrumb.background":              h(&s.base00),
        "breadcrumb.foreground":              h(&s.base04),
        "breadcrumb.focusForeground":         h(&s.base05),
        "panel.background":                   h(&s.base01),
        "panel.border":                       h(&s.base02),
        "terminal.background":                h(&s.base00),
        "terminal.foreground":                h(&s.base05),
        "terminal.ansiBlack":                 h(&s.base00),
        "terminal.ansiRed":                   h(&s.base08),
        "terminal.ansiGreen":                 h(&s.base0b),
        "terminal.ansiYellow":                h(&s.base0a),
        "terminal.ansiBlue":                  h(&s.base0d),
        "terminal.ansiMagenta":               h(&s.base0e),
        "terminal.ansiCyan":                  h(&s.base0c),
        "terminal.ansiWhite":                 h(&s.base05),
        "terminal.ansiBrightBlack":           h(&s.base03),
        "terminal.ansiBrightRed":             h(&s.base08),
        "terminal.ansiBrightGreen":           h(&s.base0b),
        "terminal.ansiBrightYellow":          h(&s.base0a),
        "terminal.ansiBrightBlue":            h(&s.base0d),
        "terminal.ansiBrightMagenta":         h(&s.base0e),
        "terminal.ansiBrightCyan":            h(&s.base0c),
        "terminal.ansiBrightWhite":           h(&s.base07),
        "inputValidation.errorBackground":    h(&s.base08),
        "inputValidation.errorBorder":        h(&s.base08),
        "inputValidation.warningBackground":  h(&s.base09),
        "inputValidation.warningBorder":      h(&s.base09),
        "focusBorder":                        h(&s.base0d),
        "button.background":                  h(&s.base0d),
        "button.foreground":                  h(&s.base00),
        "button.hoverBackground":             h(&s.base0e),
        "dropdown.background":                h(&s.base01),
        "dropdown.border":                    h(&s.base02),
        "input.background":                   h(&s.base01),
        "input.border":                       h(&s.base02),
        "input.foreground":                   h(&s.base05),
        "scrollbar.shadow":                   h(&s.base00),
        "scrollbarSlider.background":         ha(&s.base03, "80"),
        "scrollbarSlider.hoverBackground":    ha(&s.base04, "80"),
        "scrollbarSlider.activeBackground":   ha(&s.base05, "80"),
        "badge.background":                   h(&s.base0d),
        "badge.foreground":                   h(&s.base00),
        "progressBar.background":             h(&s.base0d),
        "notifications.background":           h(&s.base01),
        "notificationLink.foreground":        h(&s.base0d),
    });
    settings.insert("workbench.colorCustomizations".to_string(), ui_colors);

    let token_colors = serde_json::json!({
        "comments":   { "foreground": h(&s.base03), "fontStyle": "italic" },
        "strings":    { "foreground": h(&s.base0b) },
        "keywords":   { "foreground": h(&s.base0e), "fontStyle": "bold" },
        "numbers":    { "foreground": h(&s.base09) },
        "types":      { "foreground": h(&s.base0a) },
        "functions":  { "foreground": h(&s.base0d) },
        "variables":  { "foreground": h(&s.base08) },
        "textMateRules": [
            { "scope": "comment", "settings": { "foreground": h(&s.base03), "fontStyle": "italic" } },
            { "scope": "constant", "settings": { "foreground": h(&s.base09) } },
            { "scope": "entity.name", "settings": { "foreground": h(&s.base0a) } },
            { "scope": "entity.name.function", "settings": { "foreground": h(&s.base0d) } },
            { "scope": "keyword", "settings": { "foreground": h(&s.base0e) } },
            { "scope": "storage", "settings": { "foreground": h(&s.base0e) } },
            { "scope": "string", "settings": { "foreground": h(&s.base0b) } },
            { "scope": "variable", "settings": { "foreground": h(&s.base08) } },
            { "scope": "support", "settings": { "foreground": h(&s.base0c) } },
            { "scope": "markup.heading", "settings": { "foreground": h(&s.base0d), "fontStyle": "bold" } },
            { "scope": "markup.bold", "settings": { "fontStyle": "bold" } },
            { "scope": "markup.italic", "settings": { "fontStyle": "italic" } },
            { "scope": "markup.underline.link", "settings": { "foreground": h(&s.base0c) } },
        ]
    });
    settings.insert("editor.tokenColorCustomizations".to_string(), token_colors);

    if let Some(f) = font {
        if !f.is_empty() {
            settings.insert("editor.fontFamily".to_string(), JsonValue::String(f.to_string()));
        }
    }

    let json_str = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize settings.json")?;
    fs::write(path, json_str)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    detail!("  ✓ {}", path.display());
    Ok(())
}

/// Apply to a VSCode-family editor whose user config lives at
/// `~/.config/<dir>/User/settings.json` (e.g. "Code", "Code - OSS").
fn apply_vscode_variant(scheme: &Scheme, config: &CoatConfig, dir: &str) -> Result<()> {
    let home = home_dir()?;
    let settings_path = home.join(".config").join(dir).join("User/settings.json");
    let font = Some(config.font_monospace()).filter(|f| !f.is_empty());
    apply_vscode_shared(scheme, &settings_path, font)
}

fn apply_vscode(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    apply_vscode_variant(scheme, config, "Code")
}

fn apply_code_oss(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    apply_vscode_variant(scheme, config, "Code - OSS")
}

// ── Zed — handled directly (theme JSON + settings.json merge) ──────────────

/// Build a Zed theme-family JSON document for the given scheme.
/// Produces a single inner theme named "coat" following the v0.2.0 schema.
// `drop(put)` / `drop(puts)` below release each closure's `&mut` borrow of the
// map it fills, so the map can be moved/inserted afterward — intentional, not a
// no-op, despite clippy flagging the type as non-Drop.
#[allow(clippy::drop_non_drop)]
fn build_zed_theme(s: &Scheme) -> JsonValue {
    let h = |c: &str| format!("#{}", c);
    let ha = |c: &str, a: &str| format!("#{}{}", c, a);

    // Build the style object programmatically — a single json! literal with
    // this many keys blows past the macro recursion limit.
    let mut style = serde_json::Map::new();
    let mut put = |k: &str, v: String| {
        style.insert(k.to_string(), JsonValue::String(v));
    };

    put("border",                 h(&s.base02));
    put("border.variant",         h(&s.base01));
    put("border.focused",         h(&s.base0d));
    put("border.selected",        h(&s.base0d));
    put("border.transparent",     ha(&s.base02, "00"));
    put("border.disabled",        h(&s.base01));
    put("elevated_surface.background", h(&s.base01));
    put("surface.background",     h(&s.base01));
    put("background",             h(&s.base00));
    put("element.background",     h(&s.base01));
    put("element.hover",          h(&s.base02));
    put("element.active",         h(&s.base02));
    put("element.selected",       h(&s.base02));
    put("element.disabled",       h(&s.base01));
    put("drop_target.background", ha(&s.base0d, "40"));
    put("ghost_element.background", ha(&s.base00, "00"));
    put("ghost_element.hover",    h(&s.base01));
    put("ghost_element.active",   h(&s.base02));
    put("ghost_element.selected", h(&s.base02));
    put("ghost_element.disabled", h(&s.base01));
    put("text",                   h(&s.base05));
    put("text.muted",             h(&s.base04));
    put("text.placeholder",       h(&s.base03));
    put("text.disabled",          h(&s.base03));
    put("text.accent",            h(&s.base0d));
    put("icon",                   h(&s.base05));
    put("icon.muted",             h(&s.base04));
    put("icon.disabled",          h(&s.base03));
    put("icon.placeholder",       h(&s.base04));
    put("icon.accent",            h(&s.base0d));
    put("status_bar.background",  h(&s.base01));
    put("title_bar.background",   h(&s.base01));
    put("title_bar.inactive_background", h(&s.base00));
    put("toolbar.background",     h(&s.base00));
    put("tab_bar.background",     h(&s.base01));
    put("tab.inactive_background", h(&s.base01));
    put("tab.active_background",  h(&s.base00));
    put("search.match_background", ha(&s.base0a, "66"));
    put("panel.background",       h(&s.base01));
    put("panel.focused_border",   h(&s.base0d));
    put("pane.focused_border",    h(&s.base0d));
    put("scrollbar.thumb.background", ha(&s.base03, "80"));
    put("scrollbar.thumb.hover_background", ha(&s.base04, "80"));
    put("scrollbar.thumb.border", ha(&s.base03, "00"));
    put("scrollbar.track.background", ha(&s.base00, "00"));
    put("scrollbar.track.border", h(&s.base01));
    put("editor.foreground",      h(&s.base05));
    put("editor.background",      h(&s.base00));
    put("editor.gutter.background", h(&s.base00));
    put("editor.subheader.background", h(&s.base01));
    put("editor.active_line.background", ha(&s.base01, "bb"));
    put("editor.highlighted_line.background", h(&s.base01));
    put("editor.line_number",     h(&s.base03));
    put("editor.active_line_number", h(&s.base05));
    put("editor.invisible",       h(&s.base03));
    put("editor.wrap_guide",      ha(&s.base02, "80"));
    put("editor.active_wrap_guide", h(&s.base02));
    put("editor.document_highlight.read_background", ha(&s.base0d, "33"));
    put("editor.document_highlight.write_background", ha(&s.base04, "33"));
    put("terminal.background",    h(&s.base00));
    put("terminal.foreground",    h(&s.base05));
    put("terminal.bright_foreground", h(&s.base07));
    put("terminal.dim_foreground", h(&s.base04));
    put("terminal.ansi.black",           h(&s.base00));
    put("terminal.ansi.bright_black",    h(&s.base03));
    put("terminal.ansi.dim_black",       h(&s.base00));
    put("terminal.ansi.red",             h(&s.base08));
    put("terminal.ansi.bright_red",      h(&s.base08));
    put("terminal.ansi.dim_red",         h(&s.base08));
    put("terminal.ansi.green",           h(&s.base0b));
    put("terminal.ansi.bright_green",    h(&s.base0b));
    put("terminal.ansi.dim_green",       h(&s.base0b));
    put("terminal.ansi.yellow",          h(&s.base0a));
    put("terminal.ansi.bright_yellow",   h(&s.base0a));
    put("terminal.ansi.dim_yellow",      h(&s.base0a));
    put("terminal.ansi.blue",            h(&s.base0d));
    put("terminal.ansi.bright_blue",     h(&s.base0d));
    put("terminal.ansi.dim_blue",        h(&s.base0d));
    put("terminal.ansi.magenta",         h(&s.base0e));
    put("terminal.ansi.bright_magenta",  h(&s.base0e));
    put("terminal.ansi.dim_magenta",     h(&s.base0e));
    put("terminal.ansi.cyan",            h(&s.base0c));
    put("terminal.ansi.bright_cyan",     h(&s.base0c));
    put("terminal.ansi.dim_cyan",        h(&s.base0c));
    put("terminal.ansi.white",           h(&s.base05));
    put("terminal.ansi.bright_white",    h(&s.base07));
    put("terminal.ansi.dim_white",       h(&s.base04));
    put("link_text.hover",        h(&s.base0c));
    put("conflict",               h(&s.base09));
    put("conflict.background",    h(&s.base01));
    put("conflict.border",        h(&s.base09));
    put("created",                h(&s.base0b));
    put("created.background",     h(&s.base01));
    put("created.border",         h(&s.base0b));
    put("deleted",                h(&s.base08));
    put("deleted.background",     h(&s.base01));
    put("deleted.border",         h(&s.base08));
    put("error",                  h(&s.base08));
    put("error.background",       h(&s.base01));
    put("error.border",           h(&s.base08));
    put("hidden",                 h(&s.base03));
    put("hidden.background",      h(&s.base01));
    put("hidden.border",          h(&s.base02));
    put("hint",                   h(&s.base04));
    put("hint.background",        h(&s.base01));
    put("hint.border",            h(&s.base02));
    put("ignored",                h(&s.base03));
    put("ignored.background",     h(&s.base01));
    put("ignored.border",         h(&s.base02));
    put("info",                   h(&s.base0d));
    put("info.background",        h(&s.base01));
    put("info.border",            h(&s.base0d));
    put("modified",               h(&s.base0a));
    put("modified.background",    h(&s.base01));
    put("modified.border",        h(&s.base0a));
    put("predictive",             h(&s.base03));
    put("predictive.background",  h(&s.base01));
    put("predictive.border",      h(&s.base02));
    put("renamed",                h(&s.base0d));
    put("renamed.background",     h(&s.base01));
    put("renamed.border",         h(&s.base0d));
    put("success",                h(&s.base0b));
    put("success.background",     h(&s.base01));
    put("success.border",         h(&s.base0b));
    put("unreachable",            h(&s.base03));
    put("unreachable.background", h(&s.base01));
    put("unreachable.border",     h(&s.base02));
    put("warning",                h(&s.base09));
    put("warning.background",     h(&s.base01));
    put("warning.border",         h(&s.base09));
    drop(put);

    // Players (cursor / block selection colours).
    let player = |cur: &str, bg: &str, sel: String| {
        serde_json::json!({ "cursor": h(cur), "background": h(bg), "selection": sel })
    };
    style.insert("players".to_string(), JsonValue::Array(vec![
        player(&s.base05, &s.base0d, ha(&s.base02, "80")),
        player(&s.base0b, &s.base0b, ha(&s.base0b, "40")),
        player(&s.base0e, &s.base0e, ha(&s.base0e, "40")),
        player(&s.base09, &s.base09, ha(&s.base09, "40")),
    ]));

    // Syntax — each entry is {"color": "#..."} with optional style/weight.
    let mut syntax = serde_json::Map::new();
    let syn = |c: &str| serde_json::json!({ "color": h(c) });
    let syn_italic = |c: &str| serde_json::json!({ "color": h(c), "font_style": "italic" });
    let syn_bold = |c: &str| serde_json::json!({ "color": h(c), "font_weight": 700 });
    let mut puts = |k: &str, v: JsonValue| { syntax.insert(k.to_string(), v); };
    puts("attribute",            syn(&s.base0a));
    puts("boolean",              syn(&s.base09));
    puts("comment",              syn_italic(&s.base03));
    puts("comment.doc",          syn_italic(&s.base04));
    puts("constant",             syn(&s.base09));
    puts("constructor",          syn(&s.base0a));
    puts("embedded",             syn(&s.base05));
    puts("emphasis",             syn_italic(&s.base0d));
    puts("emphasis.strong",      syn_bold(&s.base0d));
    puts("enum",                 syn(&s.base0a));
    puts("function",             syn(&s.base0d));
    puts("hint",                 syn(&s.base04));
    puts("keyword",              syn(&s.base0e));
    puts("label",                syn(&s.base0d));
    puts("link_text",            syn_italic(&s.base0c));
    puts("link_uri",             syn(&s.base0c));
    puts("number",               syn(&s.base09));
    puts("operator",             syn(&s.base05));
    puts("predictive",           syn(&s.base03));
    puts("preproc",              syn(&s.base0e));
    puts("primary",              syn(&s.base05));
    puts("property",             syn(&s.base0d));
    puts("punctuation",          syn(&s.base05));
    puts("punctuation.bracket",  syn(&s.base05));
    puts("punctuation.delimiter", syn(&s.base05));
    puts("punctuation.list_marker", syn(&s.base0c));
    puts("punctuation.special",  syn(&s.base0f));
    puts("string",               syn(&s.base0b));
    puts("string.escape",        syn(&s.base0c));
    puts("string.regex",         syn(&s.base0c));
    puts("string.special",       syn(&s.base0f));
    puts("string.special.symbol", syn(&s.base0f));
    puts("tag",                  syn(&s.base08));
    puts("text.literal",         syn(&s.base0b));
    puts("title",                syn_bold(&s.base0d));
    puts("type",                 syn(&s.base0a));
    puts("variable",             syn(&s.base05));
    puts("variable.special",     syn(&s.base08));
    puts("variant",              syn(&s.base0a));
    drop(puts);
    style.insert("syntax".to_string(), JsonValue::Object(syntax));

    serde_json::json!({
        "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
        "name": "coat",
        "author": s.author,
        "themes": [
            {
                "name": "coat",
                "appearance": if s.is_dark() { "dark" } else { "light" },
                "style": JsonValue::Object(style),
            }
        ]
    })
}

/// Shared Zed apply logic. Works on any platform.
/// Writes the theme family to `themes_dir/coat.json` and merges
/// `settings.json` to select the "coat" theme (and font, if given).
pub fn apply_zed_shared(
    scheme: &Scheme,
    settings_path: &Path,
    themes_dir: &Path,
    font: Option<&str>,
) -> Result<()> {
    // 1. Write the theme family JSON.
    ensure_dir(themes_dir)?;
    let theme = build_zed_theme(scheme);
    let theme_path = themes_dir.join("coat.json");
    let theme_str = serde_json::to_string_pretty(&theme)
        .context("Failed to serialize Zed theme")?;
    fs::write(&theme_path, theme_str)
        .with_context(|| format!("Failed to write {}", theme_path.display()))?;
    detail!("  ✓ {}", theme_path.display());

    // 2. Merge settings.json — select the theme and (optionally) set the font.
    //    JSONC-tolerant; never clobbers a file it can't parse.
    let mut settings: serde_json::Map<String, JsonValue> = if settings_path.exists() {
        read_json_settings(settings_path)?
    } else {
        if let Some(parent) = settings_path.parent() {
            ensure_dir(parent)?;
        }
        serde_json::Map::new()
    };

    settings.insert("theme".to_string(), JsonValue::String("coat".to_string()));
    if let Some(f) = font {
        if !f.is_empty() {
            settings.insert("buffer_font_family".to_string(), JsonValue::String(f.to_string()));
        }
    }

    let json_str = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize Zed settings.json")?;
    fs::write(settings_path, json_str)
        .with_context(|| format!("Failed to write {}", settings_path.display()))?;
    detail!("  ✓ {}", settings_path.display());
    Ok(())
}

fn apply_zed(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let settings_path = home.join(".config/zed/settings.json");
    let themes_dir = home.join(".config/zed/themes");
    let font = Some(config.font_monospace()).filter(|f| !f.is_empty());
    apply_zed_shared(scheme, &settings_path, &themes_dir, font)
}

// ── Docs strings ───────────────────────────────────────────────────────────

pub fn module_docs(name: &str) {
    let name = module_aliases(name).unwrap_or(name);
    println!("=== {} Setup Instructions ===\n", name);
    match name {
        "mpv" => {
            println!("Add to ~/.config/mpv/mpv.conf:\n");
            println!("  include ~/.config/mpv/coat-theme.conf");
        }
        "firefox" => {
            println!("userChrome.css and userContent.css are written automatically.\n");
            println!("If colors don't appear, enable custom CSS in about:config:\n");
            println!("  toolkit.legacyUserProfileCustomizations.stylesheets = true\n");
            println!("Then restart Firefox (fully quit — check no firefox.exe lingers).");
        }
        "fish" => {
            println!("To activate the fish theme:\n");
            println!("  fish_config theme save coat\n");
            println!("Or add to ~/.config/fish/config.fish:\n");
            println!("  fish_config theme choose coat");
        }
        "kde" => {
            println!("The KDE color scheme is applied automatically via plasma-apply-colorscheme.\n");
            println!("A Konsole color scheme is also written. To activate it in Konsole:\n");
            println!("  Settings → Edit Current Profile → Appearance → coat\n");
            println!("Or from within a Konsole session:");
            println!("  konsoleprofile ColorScheme=coat");
        }
        "kitty" => {
            println!("Add to ~/.config/kitty/kitty.conf:\n");
            println!("  include coat-theme.conf\n");
            println!("Then reload: Ctrl+Shift+F5");
        }
        "helix" => {
            println!("Add to ~/.config/helix/config.toml:\n");
            println!("  theme = \"coat\"\n");
            println!("Then restart helix or run :config-reload");
        }
        "hyprland" => {
            println!("Add to ~/.config/hypr/hyprland.conf:\n");
            println!("  source = ~/.config/hypr/coat-theme.conf\n");
            println!("Sets border + shadow colors. coat runs `hyprctl reload` automatically.");
        }
        "neovim" => {
            println!("A colorscheme is written to:");
            println!("  $XDG_DATA_HOME/nvim/site/colors/coat.lua (default ~/.local/share/nvim/site/...)\n");
            println!("It sits on Neovim's runtimepath automatically, so just add to your config:\n");
            println!("  vim.cmd.colorscheme(\"coat\")   -- or in Vimscript:  colorscheme coat\n");
            println!("In an already-open Neovim, reload it with:  :colorscheme coat");
        }
        "i3" => {
            println!("Add to ~/.config/i3/config:\n");
            println!("  include coat-theme.conf\n");
            println!("Then reload: $mod+Shift+r");
        }
        "rofi" => {
            println!("Add to ~/.config/rofi/config.rasi:\n");
            println!("  @theme \"coat\"\n");
            println!("Or test with: rofi -show drun -theme coat");
        }
        "ashell" => {
            println!("coat writes ~/.config/ashell/config.toml (colors + module layout).");
            println!("ashell hot-reloads it automatically — no restart needed.");
            println!("Edit module layout in coat's templates/ashell.tera, then `coat apply ashell`.");
        }
        "bat" => {
            println!("Add to ~/.config/bat/config:\n");
            println!("  --theme=\"coat\"\n");
            println!("Or use temporarily: bat --theme=coat <file>");
        }
        "sway" => {
            println!("Add to ~/.config/sway/config:\n");
            println!("  include ~/.config/sway/coat-theme\n");
            println!("IMPORTANT: Remove any 'bar {{ }}' blocks from your config!\n");
            println!("Then reload: swaymsg reload");
        }
        "lf" => {
            println!("Add to ~/.config/lf/lfrc:\n");
            println!("  set color true\n");
            println!("lf picks up ~/.config/lf/colors automatically on next launch.");
        }
        "vscode" => {
            println!("The theme is automatically activated.\n");
            println!("If it doesn't appear, reload VSCode:");
            println!("  Ctrl+Shift+P → Reload Window");
        }
        "code-oss" => {
            println!("Colors are written to ~/.config/Code - OSS/User/settings.json.\n");
            println!("The theme is automatically activated.\n");
            println!("If it doesn't appear, reload the window:");
            println!("  Ctrl+Shift+P → Reload Window");
        }
        "zed" => {
            println!("The 'coat' theme is written to ~/.config/zed/themes/coat.json");
            println!("and selected automatically in settings.json.\n");
            println!("Zed hot-reloads themes — no restart needed.\n");
            println!("If it doesn't switch, set it manually:");
            println!("  Ctrl+Shift+P → \"theme selector: toggle\" → coat");
        }
        "gtk" => {
            println!("Theme is applied via gsettings automatically.\n");
            println!("Ensure 'adw-gtk3' and 'adw-gtk3-dark' are installed.");
            println!("Some apps may require a restart.");
        }
        "swaylock" => {
            println!("Theme is applied automatically.\n");
            println!("Test with: swaylock\n");
            println!("To bind to a key, add to Sway config:");
            println!("  bindsym $mod+l exec swaylock");
        }
        "dunst" => {
            println!("Dunst is restarted automatically to apply the theme.\n");
            println!("If it doesn't restart, run manually:");
            println!("  killall dunst && dunst &");
        }
        "btop" => {
            println!("To activate:\n");
            println!("1. Open btop");
            println!("2. Press ESC to open menu");
            println!("3. Navigate to 'Options' > 'Color theme'");
            println!("4. Select 'coat'\n");
            println!("Or set in ~/.config/btop/btop.conf:");
            println!("  color_theme = \"coat\"");
        }
        "zathura" => {
            println!("Restart zathura or open a new PDF to see the changes.");
        }
        "vesktop" => {
            println!("Enable the theme in Discord/Vesktop:\n");
            println!("  Settings → Vencord → Themes → coat.theme.css\n");
            println!("Or restart if auto-loading is enabled.");
        }
        "waybar" => {
            println!("Add to the top of ~/.config/waybar/style.css:\n");
            println!("  @import \"coat-theme.css\";\n");
            println!("Or replace your entire style.css with coat-theme.css.\n");
            println!("Then reload: pkill -SIGUSR2 waybar");
        }
        "xresources" => {
            println!("Theme is automatically merged.\n");
            println!("To make permanent, add to ~/.xinitrc or ~/.xprofile:");
            println!("  xrdb -merge ~/.Xresources");
        }
        "qt" => {
            println!("Qt5ct/Qt6ct color schemes created.\n");
            println!("To enable, ensure QT_QPA_PLATFORMTHEME is set:\n");
            println!("  export QT_QPA_PLATFORMTHEME=qt5ct  # For Qt5");
            println!("  export QT_QPA_PLATFORMTHEME=qt6ct  # For Qt6\n");
            println!("Add to ~/.profile, ~/.bash_profile, or ~/.config/fish/config.fish\n");
            println!("Then launch qt5ct or qt6ct to verify the 'coat' color scheme.");
        }
        "labwc" => {
            println!("Theme is applied automatically via labwc --reconfigure.\n");
            println!("If labwc is not running, start it and the theme will load.");
        }
        "ranger" => {
            println!("Add to ~/.config/ranger/rc.conf:\n");
            println!("  set colorscheme coat\n");
            println!("Restart ranger to apply.");
        }
        other => {
            println!("The {} theme has been applied.", other);
            println!("See USAGE.md for detailed information.");
        }
    }
    println!();
}
