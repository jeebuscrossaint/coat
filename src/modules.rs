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
    tpl!("bat",       "bat.tera"),
    tpl!("btop",      "btop.tera"),
    tpl!("dunst",     "dunst.tera"),
    tpl!("dwl",       "dwl.tera"),
    tpl!("firefox",   "firefox.tera"),
    tpl!("firefox_content", "firefox_content.tera"),
    tpl!("fish",      "fish.tera"),
    tpl!("foot",      "foot.tera"),
    tpl!("gtk",       "gtk.tera"),
    tpl!("gtklock",   "gtklock.tera"),
    tpl!("labwc",     "labwc.tera"),
    tpl!("mew",       "mew.tera"),
    tpl!("mpv",       "mpv.tera"),
    tpl!("neovim",    "neovim.tera"),
    tpl!("sway",      "sway.tera"),
    tpl!("swaybar",   "swaybar.tera"),
    tpl!("swayosd",   "swayosd.tera"),
    tpl!("tsubu",    "tsubu.tera"),
    tpl!("tofi",      "tofi.tera"),
    tpl!("vesktop",   "vesktop.tera"),
    tpl!("wlock",    "wlock.tera"),
    tpl!("xresources","xresources.tera"),
    tpl!("zathura",   "zathura.tera"),
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

/// Idempotently make sure the user's own config pulls in coat's generated theme
/// fragment.
///
/// coat's job stops at colours/fonts/sizes/opacity: it writes a fragment to a
/// `coat-*` filename and never owns the app's primary config. This single
/// include line is the seam between the two. It is written once, at the top of
/// the file — top, because every app whose include we use here (tofi, zathura,
/// GTK's CSS `@import`) applies it at the point it appears, so anything the user
/// writes below wins. Already present → nothing happens, so `coat apply` stays
/// safe to re-run.
///
/// A missing config is created holding just the include; that is the one case
/// where coat authors the file, and it is still only a pointer at a fragment.
///
/// `comment` is the target syntax's comment delimiters — ("#", "") for ini/conf,
/// ("!", "") for Xresources, ("/*", " */") for CSS. Getting this wrong writes a
/// file the app refuses to parse, which is worse than no theme at all.
fn ensure_include(config: &Path, include_line: &str, comment: (&str, &str)) -> Result<()> {
    let existing = fs::read_to_string(config).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == include_line) {
        return Ok(());
    }
    ensure_dir(config.parent().unwrap_or(Path::new("/")))?;
    let (open, close) = comment;
    let mut out = String::new();
    out.push_str(&format!(
        "{} Added by coat — pulls in the generated colour/font fragment.{}\n",
        open, close
    ));
    out.push_str(&format!(
        "{} Everything below is yours: coat rewrites the fragment, never this file.{}\n",
        open, close
    ));
    out.push_str(include_line);
    out.push('\n');
    if !existing.is_empty() {
        out.push('\n');
        out.push_str(&existing);
    }
    fs::write(config, out)
        .with_context(|| format!("Failed to write {}", config.display()))?;
    detail!("  ✓ {} (include added)", config.display());
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
    "bat", "btop", "dunst", "dwl", "firefox", "fish", "foot", "gtk", "gtklock", "labwc",
    "mew", "mpv",
    "tsubu", "wlock",
    "neovim", "sway", "swaybar", "swayosd", "tofi", "vesktop", "vscode", "xresources",
    "zathura",
];

pub fn module_aliases(name: &str) -> Option<&'static str> {
    match name {
        "vencord" | "discord" => Some("vesktop"),
        "nvim" | "vim" => Some("neovim"),
        "bar" | "swaybar-colors" => Some("swaybar"),
        "dwm" => Some("dwl"),
        "dmenu" | "menu" => Some("mew"),
        "notifications" => Some("tsubu"),
        "lock" | "lockscreen" => Some("wlock"),
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
            "vscode"   => return crate::windows::apply_vscode(scheme, config),
            "vesktop"  => return crate::windows::apply_discord(scheme, config),
            _ => {}
        }
    }

    match name {
        "bat"        => apply_bat(tera, &ctx, scheme, config),
        "btop"       => apply_btop(tera, &ctx, scheme, config),
        "dunst"      => apply_dunst(tera, &ctx, scheme, config),
        "firefox"    => apply_firefox(tera, &ctx, scheme, config),
        "fish"       => apply_fish(tera, &ctx, scheme, config),
        "foot"       => apply_foot(tera, &ctx, scheme, config),
        "gtk"        => apply_gtk(tera, &ctx, scheme, config),
        "gtklock"    => apply_gtklock(tera, &ctx, scheme, config),
        "dwl"        => apply_dwl(tera, &ctx, scheme, config),
        "labwc"      => apply_labwc(tera, &ctx, scheme, config),
        "mew"        => apply_mew(tera, &ctx, scheme, config),
        "tsubu"      => apply_tsubu(tera, &ctx, scheme, config),
        "wlock"      => apply_wlock(tera, &ctx, scheme, config),
        "mpv"        => apply_mpv(tera, &ctx, scheme, config),
        "neovim"     => apply_neovim(tera, &ctx, scheme, config),
        "sway"       => apply_sway(tera, &ctx, scheme, config),
        "swaybar"    => apply_swaybar(tera, &ctx, scheme, config),
        "swayosd"    => apply_swayosd(tera, &ctx, scheme, config),
        "tofi"       => apply_tofi(tera, &ctx, scheme, config),
        "vesktop"    => apply_vesktop(tera, &ctx, scheme, config),
        "vscode"     => apply_vscode(scheme, config),
        "xresources" => apply_xresources(tera, &ctx, scheme, config),
        "zathura"    => apply_zathura(tera, &ctx, scheme, config),
        other        => bail!("Unknown module: {}", other),
    }
}

// ── Individual module functions ────────────────────────────────────────────

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
    let dir = home.join(".config/dunst");

    // dunst reads drop-ins only from the `dunstrc.d` sitting next to the FIRST
    // dunstrc it finds, searching $XDG_CONFIG_HOME before $XDG_CONFIG_DIRS. With
    // no ~/.config/dunst/dunstrc the base config resolves to /etc/dunst/dunstrc
    // and our drop-in in ~/.config is never looked at — silently unthemed. So
    // make sure a user dunstrc exists, seeded from the packaged one.
    let base = dir.join("dunstrc");
    if !base.exists() {
        ensure_dir(&dir)?;
        let seed = fs::read_to_string("/etc/dunst/dunstrc").unwrap_or_else(|_| {
            "# Your dunst config. coat writes only dunstrc.d/50-coat.conf\n\
             # (colours + font); geometry, timeouts and actions belong here.\n\
             [global]\n"
                .to_string()
        });
        fs::write(&base, seed)
            .with_context(|| format!("Failed to write {}", base.display()))?;
        detail!(
            "  ✓ {} (created — drop-ins are only read next to a dunstrc)",
            base.display()
        );
    }

    render_to(tera, "dunst", ctx, &dir.join("dunstrc.d/50-coat.conf"))?;
    // Reload rather than kill/respawn: this re-reads dunstrc plus its drop-ins in
    // place, so the running daemon repaints without dropping its queue.
    run("dunstctl reload 2>/dev/null");
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

fn apply_gtk(tera: &Tera, ctx: &tera::Context, scheme: &Scheme, config: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // Same fragment for gtk-3.0 and gtk-4.0, each pulled in by that version's
    // gtk.css. @import has to come first in a CSS file, which is where
    // ensure_include puts it.
    for ver in &["gtk-3.0", "gtk-4.0"] {
        let dir = home.join(format!(".config/{}", ver));
        render_to(tera, "gtk", ctx, &dir.join("coat-theme.css"))?;
        ensure_include(&dir.join("gtk.css"), "@import url(\"coat-theme.css\");", ("/*", " */"))?;
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

fn apply_gtklock(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/gtklock");
    render_to(tera, "gtklock", ctx, &dir.join("coat-theme.css"))?;
    // gtklock's whole appearance is GTK CSS, so the seam is a plain @import —
    // which GTK resolves relative to the importing stylesheet.
    ensure_include(
        &dir.join("style.css"),
        "@import url(\"coat-theme.css\");",
        ("/*", " */"),
    )?;
    detail!("    Point gtklock at it: style=~/.config/gtklock/style.css in config.ini");
    Ok(())
}

fn apply_dwl(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/dwl/coat-colors.h");
    render_to(tera, "dwl", ctx, &dest)?;
    // dwl is configured at compile time, so a colour change is a rebuild. The
    // tree picks this header up via -I$(HOME)/.config/dwl in config.mk, and
    // config.def.h falls back to built-in colours if it is missing.
    //
    // We deliberately do NOT restart dwl here. With the wayland-socket-handover
    // patch plus a wl-restart wrapper a restart is lossless, but without that
    // wrapper killing dwl would take the whole session down -- far too rude for
    // a theme change. Rebuild, and let the user restart when they choose.
    // dwl.o depends on config.h, NOT on this header, so make would otherwise see
    // nothing to do after a colour-only change. Touch config.h to force it.
    run("d=\"$HOME/dotfiles/linux/pkg/dwl\"; [ -d \"$d\" ] || exit 0; \
         [ -f \"$d/config.h\" ] || cp \"$d/config.def.h\" \"$d/config.h\"; \
         touch \"$d/config.h\"; \
         make -C \"$d\" >/dev/null 2>&1 || true");
    Ok(())
}
fn apply_labwc(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/labwc/themerc");
    render_to(tera, "labwc", ctx, &dest)?;
    run("labwc --reconfigure 2>/dev/null; true");
    Ok(())
}

fn apply_mew(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/mew/coat-colors.h");
    render_to(tera, "mew", ctx, &dest)?;
    // Same compile-time story as dwl: touch config.h so make does not skip the
    // rebuild (mew.o depends on config.h, not on this header). Cheap -- mew is
    // one translation unit -- and the next launcher invocation picks it up, so
    // unlike dwl there is nothing to restart.
    run("d=\"$HOME/dotfiles/linux/pkg/mew\"; [ -d \"$d\" ] || exit 0; \
         [ -f \"$d/config.h\" ] || cp \"$d/config.def.h\" \"$d/config.h\"; \
         touch \"$d/config.h\"; \
         make -C \"$d\" >/dev/null 2>&1 || true");
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

/// All default profile directories coat should theme.
///
/// A machine can have several Firefox installs (stable, Developer Edition,
/// Nightly), each pinned to its own profile via an `[Install*] Default=` entry
/// in profiles.ini. We theme *every* such profile so whichever Firefox you open
/// picks up the current scheme. Falls back to the `Default=1` profile, then the
/// first profile, when no `[Install*]` sections exist.
fn firefox_profile_dirs() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    // Linux: XDG path first, then legacy ~/.mozilla.
    let mut candidates = vec![
        home.join(".config/mozilla/firefox/profiles.ini"),
        home.join(".mozilla/firefox/profiles.ini"),
    ];
    // Windows: Firefox stores profiles under %APPDATA%\Mozilla\Firefox.
    if let Ok(appdata) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(appdata).join(r"Mozilla\Firefox\profiles.ini"));
    }
    let Some(ini_path) = candidates.into_iter().find(|p| p.exists()) else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&ini_path) else {
        return Vec::new();
    };

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

    // Resolve a profile path relative to profiles.ini (or absolute as-is).
    let resolve = |path: &str, relative: bool| -> Option<PathBuf> {
        if relative {
            ini_path.parent().map(|d| d.join(path))
        } else {
            Some(PathBuf::from(path))
        }
    };

    // Prefer every [Install*] Default= — one per installed Firefox edition.
    // Install paths are always relative to profiles.ini.
    let mut dirs: Vec<PathBuf> = Vec::new();
    for (sec, map) in &sections {
        if sec.starts_with("Install") {
            if let Some(p) = map.get("Default") {
                if let Some(dir) = resolve(p, true) {
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            }
        }
    }
    if !dirs.is_empty() {
        return dirs;
    }

    // Fall back to [Profile*] with Default=1
    for (sec, map) in &sections {
        if sec.starts_with("Profile") && map.get("Default").map(|s| s == "1").unwrap_or(false) {
            if let Some(p) = map.get("Path") {
                let rel = map.get("IsRelative").map(|s| s == "1").unwrap_or(true);
                if let Some(dir) = resolve(p, rel) {
                    return vec![dir];
                }
            }
        }
    }
    // Fall back to first profile
    for (sec, map) in &sections {
        if sec.starts_with("Profile") {
            if let Some(p) = map.get("Path") {
                let rel = map.get("IsRelative").map(|s| s == "1").unwrap_or(true);
                if let Some(dir) = resolve(p, rel) {
                    return vec![dir];
                }
            }
        }
    }

    Vec::new()
}

fn apply_firefox(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let profiles = firefox_profile_dirs();
    if profiles.is_empty() {
        bail!("Firefox profile not found — is Firefox installed?");
    }

    // Theme every default profile (stable, Developer Edition, Nightly, …) so
    // whichever Firefox you launch shows the current scheme.
    for profile in &profiles {
        // Write userChrome.css (browser UI) and userContent.css (about:/new-tab)
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
    }

    detail!("    Restart Firefox for changes to take effect.");
    Ok(())
}

fn apply_sway(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/sway/coat-theme");
    render_to(tera, "sway", ctx, &dest)
}

fn apply_swaybar(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/sway/coat-bar");
    render_to(tera, "swaybar", ctx, &dest)?;
    // swaybar re-reads its config only when sway reloads, and a reload is the
    // only way to pick up the `sway` module's client.* colors too — so this is
    // where a running sway session gets repainted (best-effort, silenced).
    run("swaymsg reload 2>/dev/null");
    Ok(())
}

fn apply_swayosd(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/swayosd/style.css");
    render_to(tera, "swayosd", ctx, &dest)?;
    // swayosd-server parses its stylesheet once at startup, so restart it if
    // it's running (pkill succeeds only when it killed something → no
    // duplicate server on a TTY).
    //
    // The 2s grace period before respawning is for the supervised case: the
    // sway session runs the server under `swayosd-supervise`, which brings it
    // back ~1s after the kill. Respawning unconditionally would leave two
    // servers racing for the DBus name, and the loser exiting instantly puts
    // the supervisor into a restart spin. So: kill, wait, and only start one
    // ourselves if nothing else did. Unsupervised sessions still get a server
    // back, just 2s later.
    run("pkill -x swayosd-server 2>/dev/null || exit 0; \
         sleep 2; \
         pgrep -x swayosd-server >/dev/null || exec swayosd-server");
    Ok(())
}

fn apply_tofi(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/tofi");
    render_to(tera, "tofi", ctx, &dir.join("coat-theme"))?;
    // Relative include: tofi resolves it against the including file's directory.
    ensure_include(&dir.join("config"), "include=coat-theme", ("#", ""))
}

/// Render the vesktop CSS theme to any path — used by the Windows `apply_discord` function.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn apply_vesktop_shared(scheme: &Scheme, config: &CoatConfig, path: &Path) -> Result<()> {
    let tera = make_tera()?;
    let ctx = build_context(scheme, config);
    render_to(&tera, "vesktop", &ctx, path)
}

fn apply_tsubu(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/tsubu/coat-colors.h");
    render_to(tera, "tsubu", ctx, &dest)?;
    // Compile-time configured, same as dwl and mew: touch config.h so make does
    // not skip the rebuild (tsubu.o depends on config.h, not on this header).
    // Nothing to restart: tsubu is daemonless, so the next notification is themed.
    run("d=\"$HOME/dotfiles/linux/pkg/tsubu\"; [ -d \"$d\" ] || exit 0; \
         [ -f \"$d/config.h\" ] || cp \"$d/config.def.h\" \"$d/config.h\"; \
         touch \"$d/config.h\"; \
         make -C \"$d\" >/dev/null 2>&1 || true");
    Ok(())
}
fn apply_wlock(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/wlock/coat-colors.h");
    render_to(tera, "wlock", ctx, &dest)?;
    // Compile-time configured, same as dwl and mew: touch config.h so make does
    // not skip the rebuild (wlock.o depends on config.h, not on this header).
    // Nothing to restart: wlock only runs while the screen is locked.
    run("d=\"$HOME/dotfiles/linux/pkg/wlock\"; [ -d \"$d\" ] || exit 0; \
         [ -f \"$d/config.h\" ] || cp \"$d/config.def.h\" \"$d/config.h\"; \
         touch \"$d/config.h\"; \
         make -C \"$d\" >/dev/null 2>&1 || true");
    Ok(())
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

fn apply_xresources(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // ~/.Xresources is a general-purpose file — DPI, cursor, per-app tweaks — so
    // coat writes a fragment beside it and adds one #include rather than replacing
    // whatever was there. The cpp-style include gets an absolute path so it holds
    // regardless of the directory xrdb runs in.
    let dest = home.join(".Xresources.coat");
    render_to(tera, "xresources", ctx, &dest)?;
    let main = home.join(".Xresources");
    ensure_include(&main, &format!("#include \"{}\"", dest.display()), ("!", ""))?;
    run(&format!("xrdb -merge {} 2>/dev/null", main.display()));
    Ok(())
}

fn apply_zathura(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/zathura");
    render_to(tera, "zathura", ctx, &dir.join("coat-theme"))?;
    // girara resolves a relative include against the config being processed.
    ensure_include(&dir.join("zathurarc"), "include coat-theme", ("#", ""))
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
        "neovim" => {
            println!("A colorscheme is written to:");
            println!("  $XDG_DATA_HOME/nvim/site/colors/coat.lua (default ~/.local/share/nvim/site/...)\n");
            println!("It sits on Neovim's runtimepath automatically, so just add to your config:\n");
            println!("  vim.cmd.colorscheme(\"coat\")   -- or in Vimscript:  colorscheme coat\n");
            println!("In an already-open Neovim, reload it with:  :colorscheme coat");
        }
        "bat" => {
            println!("Add to ~/.config/bat/config:\n");
            println!("  --theme=\"coat\"\n");
            println!("Or use temporarily: bat --theme=coat <file>");
        }
        "sway" => {
            println!("Add to ~/.config/sway/config:\n");
            println!("  include ~/.config/sway/coat-theme\n");
            println!("This sets window/border colors only. For sway's built-in bar,");
            println!("enable the 'swaybar' module instead of writing a bar {{ }} block.\n");
            println!("Then reload: swaymsg reload");
        }
        "swaybar" => {
            println!("Themes sway's built-in bar (swaybar).\n");
            println!("Add to ~/.config/sway/config:\n");
            println!("  include ~/.config/sway/coat-bar\n");
            println!("IMPORTANT: remove every other 'bar {{ }}' block from your config —");
            println!("each one creates an additional bar, and this file provides a full");
            println!("block. The whole block is generated (not just the colors) because");
            println!("sway rejects 'include' inside 'bar {{ }}'.\n");
            println!("The status line is swaybar's status_command, set to 'swayrbar'.");
            println!("Configure its modules in ~/.config/swayrbar/config.toml.\n");
            println!("coat runs 'swaymsg reload' automatically.");
        }
        "swayosd" => {
            println!("Writes ~/.config/swayosd/style.css (GTK CSS).\n");
            println!("No include needed — swayosd-server picks the file up at startup,");
            println!("and coat restarts the server so a theme switch applies at once.\n");
            println!("Run the server from your compositor config:\n");
            println!("  exec swayosd-server\n");
            println!("Then bind the keys, e.g. in sway:\n");
            println!("  bindsym XF86AudioRaiseVolume exec swayosd-client --output-volume raise");
            println!("  bindsym --release Caps_Lock exec swayosd-client --caps-lock\n");
            println!("The caps/num/scroll-lock OSD works from a compositor bind like the");
            println!("one above. swayosd-libinput-backend is only needed to catch those");
            println!("keys without a bind, and it wants root plus a service manager unit.");
        }
        "tofi" => {
            println!("Writes ~/.config/tofi/coat-theme (font + colours) and adds");
            println!("  include=coat-theme");
            println!("to ~/.config/tofi/config on first apply. Geometry and behaviour");
            println!("keys stay yours; anything set after the include overrides it.\n");
            println!("tofi re-reads both on every launch, so there is nothing to reload.\n");
            println!("Test with:  tofi-drun");
            println!("Bind in sway:  set $menu tofi-drun");
        }
        "vscode" => {
            println!("The theme is automatically activated.\n");
            println!("If it doesn't appear, reload VSCode:");
            println!("  Ctrl+Shift+P → Reload Window");
        }
        "gtk" => {
            println!("Writes coat-theme.css into ~/.config/gtk-3.0 and gtk-4.0, and adds");
            println!("  @import url(\"coat-theme.css\");");
            println!("at the top of each gtk.css on first apply — your own CSS below it");
            println!("is preserved.\n");
            println!("Theme is applied via gsettings automatically.\n");
            println!("Ensure 'adw-gtk3' and 'adw-gtk3-dark' are installed.");
            println!("Some apps may require a restart.");
        }
        "gtklock" => {
            println!("Writes ~/.config/gtklock/coat-theme.css and adds");
            println!("  @import url(\"coat-theme.css\");");
            println!("to ~/.config/gtklock/style.css on first apply.\n");
            println!("Point gtklock at it in ~/.config/gtklock/config.ini:");
            println!("  [main]");
            println!("  style=/home/you/.config/gtklock/style.css\n");
            println!("Use an absolute path — a relative `style` resolves against");
            println!("gtklock's working directory, whatever spawned it.\n");
            println!("Note: gtklock's builder ids (#clock-label, #input-field,");
            println!("#unlock-button) are NOT CSS selectors and match nothing. GTK3");
            println!("doesn't turn a builder id into a CSS name and gtklock names only");
            println!("the window, so style widget types (window, label, entry, button)");
            println!("plus .suggested-action; it also sets .focused and .hidden.");
        }
        "dunst" => {
            println!("Writes the drop-in ~/.config/dunst/dunstrc.d/50-coat.conf");
            println!("(colours + font) and runs `dunstctl reload`. Your dunstrc —");
            println!("geometry, timeouts, icons, mouse actions — is left alone.\n");
            println!("Drop-ins apply after the base config in lexical order, last");
            println!("winning, so override coat with e.g. 99-mine.conf.\n");
            println!("Note: dunst only reads the dunstrc.d next to the FIRST dunstrc");
            println!("it finds, so coat creates ~/.config/dunst/dunstrc if missing —");
            println!("otherwise the base would resolve to /etc and the drop-in would");
            println!("never be read.\n");
            println!("If it doesn't reload, run manually:");
            println!("  dunstctl reload");
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
            println!("Writes ~/.config/zathura/coat-theme (colours + font) and adds");
            println!("  include coat-theme");
            println!("to ~/.config/zathura/zathurarc on first apply. Bindings and");
            println!("behaviour settings stay yours.\n");
            println!("Restart zathura or open a new PDF to see the changes.");
        }
        "vesktop" => {
            println!("Enable the theme in Discord/Vesktop:\n");
            println!("  Settings → Vencord → Themes → coat.theme.css\n");
            println!("Or restart if auto-loading is enabled.");
        }
        "xresources" => {
            println!("Writes ~/.Xresources.coat and adds an #include for it to");
            println!("~/.Xresources on first apply, so your own DPI, cursor and per-app");
            println!("settings there are preserved. Then merges automatically.\n");
            println!("To make permanent, add to ~/.xinitrc or ~/.xprofile:");
            println!("  xrdb -merge ~/.Xresources");
        }
        "dwl" => {
            println!("Writes ~/.config/dwl/coat-colors.h and rebuilds the dwl tree.\n");
            println!("dwl is configured at compile time, so the colours are a header the");
            println!("source includes; config.def.h uses __has_include and falls back to");
            println!("its built-in colours when the header is absent.\n");
            println!("A running dwl is NOT restarted -- restart it yourself to see the");
            println!("change. Under wl-restart (wayland-socket-handover patch) that keeps");
            println!("your clients alive.");
        }
        "tsubu" => {
            println!("Writes ~/.config/tsubu/coat-colors.h and rebuilds the tsubu tree.\n");
            println!("tsubu is daemonless -- nothing to restart, the next notification");
            println!("is drawn with the new colours.");
        }
        "wlock" => {
            println!("Writes ~/.config/wlock/coat-colors.h and rebuilds the wlock tree.\n");
            println!("Nothing to restart; wlock only runs while the screen is locked.");
        }
        "mew" => {
            println!("Writes ~/.config/mew/coat-colors.h and rebuilds the mew tree.\n");
            println!("mew is compile-time configured, so colours are a header the source");
            println!("includes. No restart needed -- the next launch picks it up.");
        }
        "labwc" => {
            println!("Theme is applied automatically via labwc --reconfigure.\n");
            println!("If labwc is not running, start it and the theme will load.");
        }
        other => {
            println!("The {} theme has been applied.", other);
            println!("See USAGE.md for detailed information.");
        }
    }
    println!();
}
