use anyhow::{bail, Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tera::{Tera, Value};

use crate::config::CoatConfig;
use crate::scheme::Scheme;

// ── Template source (embedded at compile time) ─────────────────────────────

macro_rules! tpl {
    ($name:expr, $file:expr) => {
        ($name, include_str!(concat!("../templates/", $file)))
    };
}

static TEMPLATES: &[(&str, &str)] = &[
    tpl!("bat",       "bat.tera"),
    tpl!("btop",      "btop.tera"),
    tpl!("dunst",     "dunst.tera"),
    tpl!("fish",      "fish.tera"),
    tpl!("foot",      "foot.tera"),
    tpl!("fuzzel",    "fuzzel.tera"),
    tpl!("gtk",       "gtk.tera"),
    tpl!("helix",     "helix.tera"),
    tpl!("i3",        "i3.tera"),
    tpl!("kitty",     "kitty.tera"),
    tpl!("labwc",     "labwc.tera"),
    tpl!("lf",        "lf.tera"),
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

fn make_tera() -> Result<Tera> {
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
    println!("  ✓ {}", dest.display());
    Ok(())
}

fn run(cmd: &str) {
    let _ = Command::new("sh").args(["-c", cmd]).status();
}

// ── Module dispatch ────────────────────────────────────────────────────────

pub const ALL_MODULES: &[&str] = &[
    "bat", "btop", "dunst", "fish", "foot", "fuzzel", "gtk", "helix",
    "i3", "kitty", "labwc", "lf", "qt", "ranger", "rofi", "sway",
    "swaylock", "vesktop", "vscode", "waybar", "xresources", "zathura",
];

pub fn module_aliases(name: &str) -> Option<&'static str> {
    match name {
        "vencord" | "discord" => Some("vesktop"),
        _ => None,
    }
}

pub fn apply_module(name: &str, scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let name = module_aliases(name).unwrap_or(name);
    let tera = make_tera()?;
    let ctx = build_context(scheme, config);

    match name {
        "bat"        => apply_bat(&tera, &ctx, scheme, config),
        "btop"       => apply_btop(&tera, &ctx, scheme, config),
        "dunst"      => apply_dunst(&tera, &ctx, scheme, config),
        "fish"       => apply_fish(&tera, &ctx, scheme, config),
        "foot"       => apply_foot(&tera, &ctx, scheme, config),
        "fuzzel"     => apply_fuzzel(&tera, &ctx, scheme, config),
        "gtk"        => apply_gtk(&tera, &ctx, scheme, config),
        "helix"      => apply_helix(&tera, &ctx, scheme, config),
        "i3"         => apply_i3(&tera, &ctx, scheme, config),
        "kitty"      => apply_kitty(&tera, &ctx, scheme, config),
        "labwc"      => apply_labwc(&tera, &ctx, scheme, config),
        "lf"         => apply_lf(&tera, &ctx, scheme, config),
        "qt"         => apply_qt(scheme, config),
        "ranger"     => apply_ranger(&tera, &ctx, scheme, config),
        "rofi"       => apply_rofi(&tera, &ctx, scheme, config),
        "sway"       => apply_sway(&tera, &ctx, scheme, config),
        "swaylock"   => apply_swaylock(&tera, &ctx, scheme, config),
        "vesktop"    => apply_vesktop(&tera, &ctx, scheme, config),
        "vscode"     => apply_vscode(scheme, config),
        "waybar"     => apply_waybar(&tera, &ctx, scheme, config),
        "xresources" => apply_xresources(&tera, &ctx, scheme, config),
        "zathura"    => apply_zathura(&tera, &ctx, scheme, config),
        other        => bail!("Unknown module: {}", other),
    }
}

// ── Individual module functions ────────────────────────────────────────────

fn apply_bat(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/bat/themes/coat.tmTheme");
    render_to(tera, "bat", ctx, &dest)?;
    println!("    Run: bat cache --build");
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

fn apply_i3(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/i3/coat-theme.conf");
    render_to(tera, "i3", ctx, &dest)?;
    run("i3-msg reload 2>/dev/null");
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
    if Command::new("sh").args(["-c", &live_cmd]).status().map(|s| !s.success()).unwrap_or(true) {
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

fn apply_lf(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/lf/colors");
    render_to(tera, "lf", ctx, &dest)?;
    println!("    Add to ~/.config/lf/lfrc: set color true");
    Ok(())
}

fn apply_ranger(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/ranger/colorschemes/coat.py");
    render_to(tera, "ranger", ctx, &dest)?;
    println!("    Add to ~/.config/ranger/rc.conf: set colorscheme coat");
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

/// Render the vesktop CSS theme to any path — used by both the Linux module
/// and the Windows `apply_discord` function in windows.rs.
pub fn apply_vesktop_shared(scheme: &Scheme, path: &Path) -> Result<()> {
    let tera = make_tera()?;
    let mut ctx = tera::Context::new();
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
    ctx.insert("scheme_name",   &scheme.name);
    ctx.insert("scheme_author", &scheme.author);
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
        println!("  ✓ {}", scheme_path.display());

        // Write main conf
        let conf_path = dir.join(conf_name);
        let conf_content = build_qt_conf(&scheme_path, config);
        fs::write(&conf_path, &conf_content)
            .with_context(|| format!("Failed to write {}", conf_path.display()))?;
        println!("  ✓ {}", conf_path.display());
        applied += 1;
    }

    if applied == 0 {
        eprintln!("  ✗ Neither qt5ct nor qt6ct config directory found.");
        eprintln!("    Install qt5ct or qt6ct and run it once to initialise.");
    } else {
        println!("    Set QT_QPA_PLATFORMTHEME=qt5ct (or qt6ct) in your environment.");
    }
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

// ── VSCode — handled directly (JSON merge) ────────────────────────────────

/// Shared VSCode apply logic. Works on any `path` (Linux or Windows).
/// `font` is optional; pass `None` to skip font injection.
pub fn apply_vscode_shared(scheme: &Scheme, path: &Path, font: Option<&str>) -> Result<()> {
    // Read existing settings or start fresh
    let mut settings: serde_json::Map<String, JsonValue> = if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        serde_json::from_str(&content).unwrap_or_default()
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
    println!("  ✓ {}", path.display());
    Ok(())
}

fn apply_vscode(scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let settings_path = home.join(".config/Code/User/settings.json");
    let font = Some(config.font_monospace()).filter(|f| !f.is_empty());
    apply_vscode_shared(scheme, &settings_path, font)
}

// ── Docs strings ───────────────────────────────────────────────────────────

pub fn module_docs(name: &str) {
    let name = module_aliases(name).unwrap_or(name);
    println!("=== {} Setup Instructions ===\n", name);
    match name {
        "fish" => {
            println!("To activate the fish theme:\n");
            println!("  fish_config theme save coat\n");
            println!("Or add to ~/.config/fish/config.fish:\n");
            println!("  fish_config theme choose coat");
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
