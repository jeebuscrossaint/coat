use anyhow::{bail, Context, Result};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use tera::{Kwargs, State, Tera};

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
    tpl!("conky",     "conky.tera"),
    tpl!("fastfetch", "fastfetch.tera"),
    tpl!("fish",      "fish.tera"),
    tpl!("fuzzel",    "fuzzel.tera"),
    tpl!("gtk",       "gtk.tera"),
    tpl!("imv",       "imv.tera"),
    tpl!("kitty",     "kitty.tera"),
    tpl!("micro",     "micro.tera"),
    tpl!("lsd",       "lsd.tera"),
    tpl!("mango",     "mango.tera"),
    tpl!("mpv",       "mpv.tera"),
    tpl!("qutebrowser","qutebrowser.tera"),
    tpl!("satty",     "satty.tera"),
    tpl!("swaylock",  "swaylock.tera"),
    tpl!("vesktop",   "vesktop.tera"),
    tpl!("xresources","xresources.tera"),
    tpl!("zathura",   "zathura.tera"),
];

// ── Tera setup ─────────────────────────────────────────────────────────────

pub fn make_tera() -> Result<Tera> {
    let mut tera = Tera::default();
    // tera 2 filters: |value, kwargs, state| returning a plain value. In tera 1
    // these took (&Value, &HashMap) and returned Result<Value>, and Value was
    // serde_json's -- Value::String/Value::Number no longer exist.
    tera.register_filter("nohash", |v: &str, _: Kwargs, _: &State| {
        v.trim_start_matches('#').to_string()
    });

    // r / g / b: one channel of a 6-char hex, as an integer.
    tera.register_filter("r", |v: &str, _: Kwargs, _: &State| Scheme::hex_to_rgb(v).0);
    tera.register_filter("g", |v: &str, _: Kwargs, _: &State| Scheme::hex_to_rgb(v).1);
    tera.register_filter("b", |v: &str, _: Kwargs, _: &State| Scheme::hex_to_rgb(v).2);

    // hue / sat / lum: Discord's palette is authored in HSL, so recolouring it
    // means speaking HSL back. Three decimals, matching how it writes its own.
    for (name, idx) in [("hue", 0usize), ("sat", 1), ("lum", 2)] {
        tera.register_filter(name, move |v: &str, _: Kwargs, _: &State| {
            let hsl = hex_to_hsl(v);
            format!("{:.3}", [hsl.0, hsl.1, hsl.2][idx])
        });
    }

    tera.register_filter("lower_hex", |v: &str, _: Kwargs, _: &State| v.to_lowercase());

    // Templates are added AFTER the filters, and that order is load-bearing in
    // tera 2: it resolves filter names while PARSING, so a template using
    // lower_hex fails with "Unknown filter" if it is added first. tera 1 resolved
    // at render time and did not care.
    for (name, src) in TEMPLATES {
        tera.add_raw_template(name, src)
            .with_context(|| format!("Failed to parse template '{}'", name))?;
    }

    Ok(tera)
}

// ── Context builder ────────────────────────────────────────────────────────

/// Relative luminance of an `RRGGBB` string, sRGB coefficients, no gamma step --
/// this only ever has to ORDER colours, and gamma is monotonic.
/// sRGB hex -> HSL. Discord's palette is authored in HSL and every one of its
/// theme tokens bottoms out in an `hsl(var(--x-hsl)/a)`, so recolouring it means
/// speaking HSL back at it.
fn hex_to_hsl(hex: &str) -> (f32, f32, f32) {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return (0.0, 0.0, 0.0);
    }
    let ch = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
    let (r, g, b) = (ch(0), ch(2), ch(4));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f32::EPSILON {
        return (0.0, 0.0, l * 100.0);
    }
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let hue = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    (hue, s * 100.0, l * 100.0)
}

fn luminance(hex: &str) -> f32 {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return 1.0;
    }
    let ch = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
    0.2126 * ch(0) + 0.7152 * ch(2) + 0.0722 * ch(4)
}

/// The darkest colour on the scheme's neutral ramp.
///
/// A shadow is cast by a light source, so it is dark in EVERY scheme -- but it
/// should still be the scheme's own dark rather than a hardcoded black, or coat
/// is not theming it at all. base00 is the answer on a dark variant and the wrong
/// end entirely on a light one, where it is the page. Picking by luminance gets
/// both without branching on the variant field, which schemes are free to leave
/// unset.
fn darkest_neutral(s: &Scheme) -> String {
    [
        &s.base00, &s.base01, &s.base02, &s.base03, &s.base04, &s.base05, &s.base06, &s.base07,
    ]
    .into_iter()
    .filter(|c| c.trim_start_matches('#').len() >= 6)
    .min_by(|a, b| {
        luminance(a)
            .partial_cmp(&luminance(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
    .cloned()
    .unwrap_or_else(|| "000000".to_string())
}

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
    // The scheme's dark end, for shadows -- dark on light schemes too.
    ctx.insert("shadow", &darkest_neutral(scheme));
    // Endpoints of the scheme's neutral ramp, as lightness percentages. Discord's
    // neutral ramp runs light (--neutral-1) to dark (--neutral-100) and coat
    // remaps it onto these, so the recolour works from either variant without
    // asking which one it is.
    let neutrals = [
        &scheme.base00, &scheme.base01, &scheme.base02, &scheme.base03,
        &scheme.base04, &scheme.base05, &scheme.base06, &scheme.base07,
    ];
    let ls: Vec<f32> = neutrals.iter().map(|c| hex_to_hsl(c).2).collect();
    let lmax = ls.iter().cloned().fold(f32::MIN, f32::max);
    let lmin = ls.iter().cloned().fold(f32::MAX, f32::min);
    ctx.insert("neutral_l_light", &lmax);
    ctx.insert("neutral_l_dark", &lmin);
    // Hue and saturation for the neutral ramp, taken from base03 and NOT base00.
    // HSL saturation is meaningless at the ends of the lightness range: a cream
    // white like #FFFCF0 reports S=100%, and applying that across the whole ramp
    // turns the mid-tones bright orange. base03 sits mid-ramp where the number
    // means something. Clamped anyway -- Discord's own neutrals run 0-7%.
    let (nh, ns, _) = hex_to_hsl(&scheme.base03);
    ctx.insert("neutral_h", &format!("{:.3}", nh));
    ctx.insert("neutral_s", &format!("{:.3}", ns.min(10.0)));
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
    // Leave an identical file alone: rewriting it bumps the mtime, which file
    // watchers (conky's self-reload, for one) treat as a change.
    if fs::read(dest).ok().as_deref() != Some(content.as_bytes()) {
        fs::write(dest, &content)
            .with_context(|| format!("Failed to write {}", dest.display()))?;
    }
    crate::manifest::record_write(dest);
    detail!("  ✓ {}", dest.display());
    Ok(())
}

/// Idempotently make sure the user's own config pulls in coat's generated theme
/// fragment.
///
/// coat's job stops at colours/fonts/sizes/opacity: it writes a fragment to a
/// `coat-*` filename and never owns the app's primary config. This single
/// include line is the seam between the two. It is written once, at the top of
/// the file — top, because every app whose include we use here (kitty, zathura,
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
    // Recorded whether or not it has to be added: the line is coat's either way,
    // and `coat remove` has to know to strip it.
    crate::manifest::record_include(config, include_line);
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
    "bat", "btop", "conky", "dunst", "fastfetch", "fish", "fuzzel", "gtk", "imv", "kitty",
    "lsd", "mango", "micro", "mpv", "qutebrowser", "satty", "swaylock", "vesktop", "vscode", "xresources",
    "zathura",
];

/// (alias, module) — other names a module answers to. A table rather than a
/// match so shell completion can list them.
pub const MODULE_ALIASES: &[(&str, &str)] = &[
    ("discord", "vesktop"),
    ("vencord", "vesktop"),
];

pub fn module_aliases(name: &str) -> Option<&'static str> {
    MODULE_ALIASES.iter().find(|(alias, _)| *alias == name).map(|(_, module)| *module)
}

pub fn apply_module(name: &str, scheme: &Scheme, config: &CoatConfig, tera: &Tera) -> Result<()> {
    let name = module_aliases(name).unwrap_or(name);
    crate::manifest::begin(name);

    // Per-module overrides are folded in HERE, once, so nothing downstream has
    // to know they exist.
    let merged = config.merged_for(name);
    let config = &merged;

    let ctx = build_context(scheme, config);

    // On Windows, cross-platform apps live at native locations (%APPDATA%, …)
    // rather than the XDG paths the Linux module functions assume. Route them
    // to the Windows-specific apply functions so `coat apply <app>` works too.
    #[cfg(windows)]
    {
        match name {
            // These return a step Outcome for the `coat set` summary; from
            // here only success or failure matters.
            "vscode"   => return crate::windows::apply_vscode(scheme, config).map(|_| ()),
            "vesktop"  => return crate::windows::apply_discord(scheme, config).map(|_| ()),
            _ => {}
        }
    }

    let result = match name {
        "bat"        => apply_bat(tera, &ctx, scheme, config),
        "btop"       => apply_btop(tera, &ctx, scheme, config),
        "fastfetch"  => apply_fastfetch(tera, &ctx, scheme, config),
        "imv"        => apply_imv(tera, &ctx, scheme, config),
        "lsd"        => apply_lsd(tera, &ctx, scheme, config),
        "satty"      => apply_satty(tera, &ctx, scheme, config),
        "dunst"      => apply_dunst(tera, &ctx, scheme, config),
        "fish"       => apply_fish(tera, &ctx, scheme, config),
        "gtk"        => apply_gtk(tera, &ctx, scheme, config),
        "kitty"      => apply_kitty(tera, &ctx, scheme, config),
        "conky"      => apply_conky(tera, &ctx, scheme, config),
        "micro"      => apply_micro(tera, &ctx, scheme, config),
        "mango"      => apply_mango(tera, &ctx, scheme, config),
        "fuzzel"     => apply_fuzzel(tera, &ctx, scheme, config),
        "swaylock"   => apply_swaylock(tera, &ctx, scheme, config),
        "mpv"        => apply_mpv(tera, &ctx, scheme, config),
        "qutebrowser"=> apply_qutebrowser(tera, &ctx, scheme, config),
        "vesktop"    => apply_vesktop(tera, &ctx, scheme, config),
        "vscode"     => apply_vscode(scheme, config),
        "xresources" => apply_xresources(tera, &ctx, scheme, config),
        "zathura"    => apply_zathura(tera, &ctx, scheme, config),
        other        => bail!("Unknown module: {}", other),
    };

    // Committed even on failure: the files written before the error exist, and
    // `coat remove` should be able to clean up a half-finished apply.
    crate::manifest::commit();
    result
}

/// Undo what the last apply recorded for `name`: delete the files coat
/// generated, strip the include lines it patched into files it does not own, and
/// drop the ini keys it merged. Returns a human-readable list of what it did.
///
/// Nothing here guesses. If the manifest has no entry for the module, there is
/// nothing coat can prove it wrote, and it says so rather than deleting a path
/// it merely believes it owns.
pub fn remove_module(name: &str, dry: bool) -> Result<Vec<String>> {
    let name = module_aliases(name).unwrap_or(name);
    let manifest = crate::manifest::load();
    let Some(entry) = manifest.modules.get(name) else {
        bail!(
            "coat has no record of applying '{}'.\n\
             The record is written on apply, so a module last applied by an older\n\
             coat has none. Run `coat apply {}` once, then remove it.",
            name,
            name
        );
    };

    let mut done = Vec::new();

    for file in &entry.written {
        if !file.exists() {
            continue;
        }
        if !dry {
            fs::remove_file(file).with_context(|| format!("Failed to delete {}", file.display()))?;
        }
        done.push(format!("deleted  {}", file.display()));
    }

    for (config, line) in &entry.includes {
        let Ok(text) = fs::read_to_string(config) else {
            continue;
        };
        let stripped = strip_include(&text, line);
        if stripped == text {
            continue;
        }
        if !dry {
            fs::write(config, &stripped)
                .with_context(|| format!("Failed to rewrite {}", config.display()))?;
        }
        done.push(format!("un-included  {}", config.display()));
    }

    if !dry {
        crate::manifest::forget(name);
    }
    Ok(done)
}

/// Drop `line` from `text`, along with the comment block `ensure_include`
/// prepends above it — two comment lines and the blank line after, all of which
/// are coat's and none of which mean anything once the include is gone.
fn strip_include(text: &str, line: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(idx) = lines.iter().position(|l| l.trim() == line.trim()) else {
        return text.to_string();
    };

    // Walk back over the comment header coat wrote (it always mentions coat).
    let mut start = idx;
    while start > 0 {
        let prev = lines[start - 1].trim();
        let is_coat_comment = prev.contains("Added by coat")
            || prev.contains("coat rewrites the fragment")
            || prev.contains("never this file");
        if is_coat_comment {
            start -= 1;
        } else {
            break;
        }
    }

    let mut end = idx + 1;
    if lines.get(end).map(|l| l.trim().is_empty()).unwrap_or(false) {
        end += 1;
    }

    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    kept.extend_from_slice(&lines[..start]);
    kept.extend_from_slice(&lines[end..]);
    let mut out = kept.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
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

fn apply_kitty(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dest = home.join(".config/kitty/coat-theme.conf");
    render_to(tera, "kitty", ctx, &dest)?;
    ensure_include(
        &home.join(".config/kitty/kitty.conf"),
        "include coat-theme.conf",
        ("#", ""),
    )?;

    // Recolour running windows without a restart. `kitty @ set-colors` needs remote
    // control enabled (allow_remote_control + listen_on in kitty.conf) and talks over the
    // socket that listen_on names. If that fails -- remote control off, no instance
    // running -- fall back to SIGUSR1, which makes kitty re-read its config.
    //
    // The socket is globbed, not named directly. kitty APPENDS its pid to whatever
    // listen_on says, so `listen_on unix:/tmp/kitty` produces /tmp/kitty-1234 and
    // talking to /tmp/kitty always failed -- this path never once ran, and every
    // apply silently took the SIGUSR1 fallback. Stale sockets from dead instances
    // are left behind too, so a failure on one must not stop the others.
    //
    // The fallback lives in the same backgrounded script: each socket round-trip
    // costs ~50ms, and waiting on them held up every `coat set`.
    run(&format!(
        "ok=1; for s in /tmp/kitty /tmp/kitty-*; do [ -S \"$s\" ] || continue; \
         kitty @ --to \"unix:$s\" set-colors --all --configured {} 2>/dev/null && ok=0; \
         done; [ $ok = 0 ] || pkill -SIGUSR1 -x kitty 2>/dev/null; true",
        dest.to_string_lossy()
    ));
    Ok(())
}

fn apply_fuzzel(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/fuzzel");
    render_to(tera, "fuzzel", ctx, &dir.join("coat-theme.ini"))?;
    // fuzzel resolves an include where the line appears, so this belongs at the
    // top of fuzzel.ini and anything after it wins. Nothing to reload: fuzzel is
    // not a daemon, it reads its config on each launch.
    ensure_include(
        &dir.join("fuzzel.ini"),
        &format!("include={}", dir.join("coat-theme.ini").display()),
        ("#", ""),
    )
}

fn apply_mpv(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/mpv");
    render_to(tera, "mpv", ctx, &dir.join("coat-theme.conf"))?;
    // Was a printed hint telling the user to add this line themselves, which meant
    // that on any machine where they had not, coat wrote a theme nothing read.
    // Every other module with an include mechanism wires itself up; this one now
    // does too.
    ensure_include(
        &dir.join("mpv.conf"),
        &format!("include={}", dir.join("coat-theme.conf").display()),
        ("#", ""),
    )
}

fn apply_conky(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    render_to(tera, "conky", ctx, &home.join(".config/conky/coat-colors.lua"))?;

    // conky watches conky.conf for changes and reloads itself, but this writes a
    // file conky.conf only `dofile`s -- which that watch does not cover. So the
    // reload has to be asked for. SIGUSR1 is conky's documented reload signal and
    // it re-executes the config, picking the new table up.
    run("pkill -USR1 -x conky 2>/dev/null || true");
    Ok(())
}

fn apply_micro(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // A standard micro colourscheme on the search path, so settings.json only
    // ever names it: "colorscheme": "coat". The file holds nothing but
    // colour-links, which is why coat owns all of it rather than a fragment.
    render_to(
        tera,
        "micro",
        ctx,
        &home.join(".config/micro/colorschemes/coat.micro"),
    )?;

    // No reload hook on purpose. micro reads its colourscheme once at startup and
    // has no config-reload signal; the in-editor `> set colorscheme coat` re-reads
    // it for a running instance, and there is nothing to send from out here.
    Ok(())
}

fn apply_mango(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/mango");
    render_to(tera, "mango", ctx, &dir.join("coat-colors.conf"))?;

    // mango's `source=` takes a path, not a relative include, and it does not
    // expand ~ -- so the absolute path has to be written out.
    let line = format!("source={}/coat-colors.conf", dir.display());
    ensure_include(&dir.join("config.conf"), &line, ("#", ""))?;

    // The payoff for moving off dwl: mango's config is runtime, so a scheme
    // change recolours a LIVE session. dwl had to be rebuilt and restarted, which
    // meant losing every window and (because its autostart re-ran) its audio.
    //
    // reload_config re-reads the whole config, which re-runs `exec=` lines but
    // NOT `exec-once=` -- daemons are left alone by design.
    run("mmsg dispatch reload_config 2>/dev/null || true");
    Ok(())
}

fn apply_swaylock(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    // A FRAGMENT beside the config, never the config itself.
    //
    // swaylock reads exactly one file and has no include mechanism, so coat used
    // to patch the colour keys into ~/.config/swaylock/config in place. That
    // works, but the file is normally a dotfiles symlink, so every `coat set`
    // landed as a diff in the user's repo -- the one module left doing it.
    //
    // The way out is not an include, it is the command line: every swaylock
    // config key is also a long flag, and swaylock parses argv AFTER the config
    // file specifically so flags win (main.c: load_config, then parse_options).
    // So coat writes `key=value` lines here and the caller turns them into flags:
    //
    //     swaylock $(grep -v '^#' ~/.config/swaylock/coat-theme.conf | sed 's/^/--/')
    //
    // Nothing to reload: swaylock reads both fresh at every invocation, so the
    // next lock is already themed.
    render_to(tera, "swaylock", ctx, &home.join(".config/swaylock/coat-theme.conf"))
}

/// Render the vesktop CSS theme to each of `paths` — used by the Windows
/// `apply_discord` function. Builds the template set once for all of them.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn apply_vesktop_shared(scheme: &Scheme, config: &CoatConfig, paths: &[PathBuf]) -> Result<()> {
    let tera = make_tera()?;
    let ctx = build_context(scheme, config);
    paths.iter().try_for_each(|path| render_to(&tera, "vesktop", &ctx, path))
}

fn apply_qutebrowser(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/qutebrowser");
    render_to(tera, "qutebrowser", ctx, &dir.join("coat-theme.py"))?;

    // Once a config.py exists, qutebrowser stops reading autoconfig.yml unless the
    // file says to -- so a config.py holding nothing but coat's include would
    // silently drop every setting made with :set. Seed it with the load call
    // before the include goes on top, which also leaves autoconfig.yml winning.
    let config_py = dir.join("config.py");
    if !config_py.exists() {
        ensure_dir(&dir)?;
        fs::write(&config_py, "config.load_autoconfig()\n")
            .with_context(|| format!("Failed to write {}", config_py.display()))?;
    }
    // config.source resolves a relative path against the config directory.
    ensure_include(&config_py, "config.source('coat-theme.py')", ("#", ""))?;

    // qutebrowser does not watch its config, but a second `qutebrowser <cmd>`
    // hands the command to the running instance over IPC. Guarded by pgrep,
    // because with nothing running that same call opens a browser.
    run("pgrep -u \"$(id -u)\" -x qutebrowser >/dev/null \
         && qutebrowser ':config-source' >/dev/null 2>&1; true");
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
    ensure_include(&dir.join("zathurarc"), "include coat-theme", ("#", ""))?;

    // zathura re-reads its config over D-Bus, so a document that is already open
    // recolours in place instead of waiting to be reopened. Each instance owns a
    // well-known name of the form org.pwmt.zathura.PID-<pid>.
    run(concat!(
        r#"busctl --user list --no-legend 2>/dev/null "#,
        r#"| awk '/^org\.pwmt\.zathura\.PID-/{print $1}' "#,
        r#"| while read -r n; do busctl --user call "$n" /org/pwmt/zathura "#,
        r#"org.pwmt.zathura SourceConfig >/dev/null 2>&1; done; true"#,
    ));
    Ok(())
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

fn apply_fastfetch(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    render_to(tera, "fastfetch", ctx, &home.join(".config/fastfetch/config.jsonc"))
}

fn apply_imv(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    render_to(tera, "imv", ctx, &home.join(".config/imv/config"))
}

fn apply_satty(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    render_to(tera, "satty", ctx, &home.join(".config/satty/config.toml"))
}

fn apply_lsd(tera: &Tera, ctx: &tera::Context, _s: &Scheme, _c: &CoatConfig) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".config/lsd");
    render_to(tera, "lsd", ctx, &dir.join("colors.yaml"))?;

    // colors.yaml is read ONLY when config.yaml opts in. Without it lsd falls
    // back to its built-in 256-colour theme and says nothing -- the file parses,
    // it is simply never consulted -- so a module that wrote colours alone would
    // look like it worked and change nothing.
    let cfg = dir.join("config.yaml");
    if !cfg.exists() {
        ensure_dir(&dir)?;
        fs::write(&cfg, "color:\n  theme: custom\n")
            .with_context(|| format!("Failed to write {}", cfg.display()))?;
        crate::manifest::record_write(&cfg);
        detail!("  ✓ {}", cfg.display());
    } else {
        let existing = fs::read_to_string(&cfg).unwrap_or_default();
        if !existing.contains("theme: custom") {
            detail!(
                "  note: add `color:\n    theme: custom` to {} or the colours are ignored",
                cfg.display()
            );
        }
    }
    Ok(())
}

pub fn module_docs(name: &str) {
    let name = module_aliases(name).unwrap_or(name);
    println!("=== {} Setup Instructions ===\n", name);
    match name {
        "mpv" => {
            println!("Add to ~/.config/mpv/mpv.conf:\n");
            println!("  include ~/.config/mpv/coat-theme.conf");
        }
        "micro" => {
            println!("coat writes ~/.config/micro/colorschemes/coat.micro.\n");
            println!("Select it in ~/.config/micro/settings.json:\n");
            println!("  \"colorscheme\": \"coat\"\n");
            println!("Colours are '#RRGGBB' and need truecolor: micro's `truecolor`");
            println!("option is 'auto', which follows $COLORTERM -- set it to 'on' if a");
            println!("terminal does not advertise it, or the scheme degrades to 256.\n");
            println!("A running micro does not pick up a reapply: `> set colorscheme coat`");
            println!("re-reads the file, and there is no signal coat can send it.");
        }
        "conky" => {
            println!("coat writes ~/.config/conky/coat-colors.lua as a Lua table.\n");
            println!("Load it at the top of ~/.config/conky/conky.conf:\n");
            println!("  local c = dofile(os.getenv(\"HOME\") .. \"/.config/conky/coat-colors.lua\")\n");
            println!("Then reference it in the config table and the text block:\n");
            println!("  conky.config = {{ default_color = c.base05, color1 = c.base0D }}");
            println!("  conky.text = [[ ${{color1}}label${{color}} ]]\n");
            println!("Colours are bare hex, no leading '#'; conky rejects the '#' form.");
            println!("coat sends SIGUSR1 after writing, which is conky's reload signal.");
        }
        "fish" => {
            println!("To activate the fish theme:\n");
            println!("  fish_config theme save coat\n");
            println!("Or add to ~/.config/fish/config.fish:\n");
            println!("  fish_config theme choose coat");
        }
        "bat" => {
            println!("Add to ~/.config/bat/config:\n");
            println!("  --theme=\"coat\"\n");
            println!("Or use temporarily: bat --theme=coat <file>");
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
        "fastfetch" => {
            println!("config.jsonc is written whole. Just run `fastfetch`.");
        }
        "lsd" => {
            println!("Colours live in ~/.config/lsd/colors.yaml, which lsd reads");
            println!("ONLY when ~/.config/lsd/config.yaml contains:\n");
            println!("  color:");
            println!("    theme: custom\n");
            println!("coat writes that file when lsd has none of its own.");
        }
        "imv" => {
            println!("~/.config/imv/config is written whole.");
        }
        "qutebrowser" => {
            println!("Writes ~/.config/qutebrowser/coat-theme.py (colours + font) and");
            println!("adds\n");
            println!("  config.source('coat-theme.py')\n");
            println!("to the top of config.py on first apply. A running qutebrowser is");
            println!("reloaded with :config-source; anything set later in config.py or");
            println!("autoconfig.yml still wins.\n");
            println!("preferred_color_scheme (what sites see) may need a restart.");
        }
        "satty" => {
            println!("~/.config/satty/config.toml is written whole.");
            println!("The annotation palette becomes the scheme's accent wheel.");
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
        "fuzzel" => {
            println!("Patches colour keys into ~/.config/fuzzel/fuzzel.ini in place.\n");
            println!("fuzzel colours are bare 8-digit hex (rrggbbaa) with NO leading");
            println!("'#'. Nothing to reload: fuzzel reads its config on each launch.");
        }
        other => {
            println!("The {} theme has been applied.", other);
            println!("See USAGE.md for detailed information.");
        }
    }
    println!();
}
