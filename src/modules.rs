use anyhow::{bail, Context, Result};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tera::{Kwargs, State, Tera};

use crate::apply::Plan;
use crate::config::{size_value, CoatConfig};
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
    tpl!("hyprland",  "hyprland.tera"),
    tpl!("gtk",       "gtk.tera"),
    tpl!("imv",       "imv.tera"),
    tpl!("kitty",     "kitty.tera"),
    tpl!("micro",     "micro.tera"),
    tpl!("lsd",       "lsd.tera"),
    tpl!("mango",     "mango.tera"),
    tpl!("mpv",       "mpv.tera"),
    tpl!("qtct",      "qtct.tera"),
    tpl!("qutebrowser","qutebrowser.tera"),
    tpl!("satty",     "satty.tera"),
    tpl!("swaylock",  "swaylock.tera"),
    tpl!("vesktop",   "vesktop.tera"),
    tpl!("xresources","xresources.tera"),
    tpl!("zathura",   "zathura.tera"),
];

// ── Tera setup ─────────────────────────────────────────────────────────────

/// Every template, parsed. For the tests and the Windows paths; an apply
/// parses only what its modules render (see `Ctx::render`).
pub fn make_tera() -> Result<Tera> {
    let mut tera = engine();
    for (name, src) in TEMPLATES {
        tera.add_raw_template(name, src)
            .with_context(|| format!("Failed to parse template '{}'", name))?;
    }
    Ok(tera)
}

/// A template engine with coat's filters and no templates yet.
///
/// Templates are added AFTER the filters, and that order is load-bearing in
/// tera 2: it resolves filter names while PARSING, so a template using
/// lower_hex fails with "Unknown filter" if it is added first. tera 1 resolved
/// at render time and did not care.
fn engine() -> Tera {
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

    tera
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
    neutral_end(s, std::cmp::Ordering::Less).unwrap_or_else(|| "000000".to_string())
}

/// The other end of the ramp: what reads against `darkest_neutral`. Qt calls it
/// BrightText -- text drawn on the Dark role -- and like the shadow it does not
/// flip with the variant.
fn brightest_neutral(s: &Scheme) -> String {
    neutral_end(s, std::cmp::Ordering::Greater).unwrap_or_else(|| "FFFFFF".to_string())
}

fn neutral_end(s: &Scheme, end: std::cmp::Ordering) -> Option<String> {
    [
        &s.base00, &s.base01, &s.base02, &s.base03, &s.base04, &s.base05, &s.base06, &s.base07,
    ]
    .into_iter()
    .filter(|c| c.trim_start_matches('#').len() >= 6)
    .min_by(|a, b| {
        let o = luminance(a)
            .partial_cmp(&luminance(b))
            .unwrap_or(std::cmp::Ordering::Equal);
        if end == std::cmp::Ordering::Less { o } else { o.reverse() }
    })
    .cloned()
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
    ctx.insert("bright", &brightest_neutral(scheme));
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
    ctx.insert("font_size_terminal", &size_value(config.font_size_terminal()));
    ctx.insert("font_size_desktop", &size_value(config.font_size_desktop()));
    ctx.insert("font_size_popups", &size_value(config.font_size_popups()));
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
    crate::paths::home_dir().context("Cannot determine home directory")
}

// ── Planning ───────────────────────────────────────────────────────────────

/// Everything a planner reads. Built once per module, with that module's
/// overrides already folded into `config` and its palette painted onto
/// `scheme`, so no planner can forget either.
pub struct Ctx {
    /// This module's own engine, holding only the templates it has rendered.
    /// Parsing all 22 up front was a sixth of an apply; parsed here, each
    /// module's templates are parsed on its own planning thread, and a
    /// one-app apply parses one.
    tera: RefCell<(Tera, Vec<&'static str>)>,
    vars: tera::Context,
    pub scheme: Scheme,
    pub config: CoatConfig,
    pub home: PathBuf,
}

impl Ctx {
    fn render(&self, template: &str) -> Result<String> {
        let mut guard = self.tera.borrow_mut();
        let (tera, parsed) = &mut *guard;
        if !parsed.contains(&template) {
            let (name, src) = TEMPLATES
                .iter()
                .find(|(n, _)| *n == template)
                .with_context(|| format!("No template '{}'", template))?;
            tera.add_raw_template(name, src)
                .with_context(|| format!("Failed to parse template '{}'", name))?;
            parsed.push(name);
        }
        tera.render(template, &self.vars)
            .with_context(|| format!("Failed to render template '{}'", template))
    }

    /// `~/<rel>`.
    fn path(&self, rel: &str) -> PathBuf {
        self.home.join(rel)
    }

    /// Render `template` to `~/<rel>`, a file coat owns.
    fn render_to(&self, plan: &mut Plan, template: &str, rel: &str) -> Result<PathBuf> {
        let dest = self.path(rel);
        plan.file(dest.clone(), self.render(template)?);
        Ok(dest)
    }
}

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

// ── Module registry ────────────────────────────────────────────────────────

/// One app coat themes. The single table every list of modules comes from:
/// dispatch, `coat docs`, validation, and shell completion all read it.
pub struct Module {
    pub name: &'static str,
    /// Other names it answers to, in `enabled:`, `overrides:` and on the
    /// command line.
    pub aliases: &'static [&'static str],
    plan: fn(&Ctx, &mut Plan) -> Result<()>,
    /// `coat docs <name>`.
    pub docs: &'static str,
    /// A one-time step that needs root, offered after the first apply. See
    /// setup.rs.
    pub setup: Option<&'static crate::setup::Setup>,
}

macro_rules! module {
    ($name:literal, $plan:ident, $docs:expr) => {
        module!($name, [], $plan, $docs)
    };
    ($name:literal, [$($alias:literal),*], $plan:ident, $docs:expr) => {
        module!($name, [$($alias),*], $plan, $docs, None)
    };
    ($name:literal, [$($alias:literal),*], $plan:ident, $docs:expr, $setup:expr) => {
        Module { name: $name, aliases: &[$($alias),*], plan: $plan, docs: $docs, setup: $setup }
    };
}

pub static MODULES: &[Module] = &[
    module!("bat",         plan_bat,         docs::BAT),
    module!("btop",        plan_btop,        docs::BTOP),
    module!("colors",      plan_colors,      docs::COLORS),
    module!("conky",       plan_conky,       docs::CONKY),
    module!("dunst",       plan_dunst,       docs::DUNST),
    module!("fastfetch",   plan_fastfetch,   docs::FASTFETCH),
    module!("fish",        plan_fish,        docs::FISH),
    module!("fuzzel",      plan_fuzzel,      docs::FUZZEL),
    module!("gtk",         plan_gtk,         docs::GTK),
    module!("hyprland",    plan_hyprland,    docs::HYPRLAND),
    module!("imv",         plan_imv,         docs::IMV),
    module!("kitty",       plan_kitty,       docs::KITTY),
    module!("lsd",         plan_lsd,         docs::LSD),
    module!("mango",       plan_mango,       docs::MANGO),
    module!("micro",       plan_micro,       docs::MICRO),
    module!("mpv",         plan_mpv,         docs::MPV),
    module!("qt", ["qt5ct", "qt6ct"], plan_qt, docs::QT),
    module!("qutebrowser", plan_qutebrowser, docs::QUTEBROWSER),
    module!("satty",       plan_satty,       docs::SATTY),
    module!("swaylock",    plan_swaylock,    docs::SWAYLOCK),
    module!("tty", ["console", "vt"], plan_tty, docs::TTY, Some(&crate::setup::TTY)),
    module!("vesktop", ["discord", "vencord"], plan_vesktop, docs::VESKTOP),
    module!("vscode",      plan_vscode,      docs::VSCODE),
    module!("xresources",  plan_xresources,  docs::XRESOURCES),
    module!("zathura",     plan_zathura,     docs::ZATHURA),
];

/// The module `name` names, by its own name or an alias.
pub fn find(name: &str) -> Option<&'static Module> {
    MODULES.iter().find(|m| m.name == name || m.aliases.contains(&name))
}

/// What one module would write, without writing it.
pub fn plan(name: &str, scheme: &Scheme, config: &CoatConfig) -> Result<Plan> {
    let module = find(name).with_context(|| format!("Unknown module: {}", name))?;

    // Per-module overrides are folded in HERE, once, so nothing downstream has
    // to know they exist. That includes the palette, so the scheme is painted
    // per module too.
    let config = config.merged_for(module.name);
    let scheme = config.paint(scheme)?;
    let ctx = Ctx {
        tera: RefCell::new((engine(), Vec::new())),
        vars: build_context(&scheme, &config),
        home: home_dir()?,
        scheme,
        config,
    };
    let mut plan = Plan::default();
    (module.plan)(&ctx, &mut plan)?;
    Ok(plan)
}

/// Print `coat docs <name>`.
pub fn module_docs(name: &str) {
    let Some(module) = find(name) else { return };
    println!("=== {} Setup Instructions ===\n", module.name);
    println!("{}", module.docs.trim());
    println!();
}

/// Undo what the last apply recorded for `name`: delete the files coat
/// generated, strip the include lines it patched into files it does not own, and
/// drop the ini keys it merged. Returns a human-readable list of what it did.
///
/// Nothing here guesses. If the manifest has no entry for the module, there is
/// nothing coat can prove it wrote, and it says so rather than deleting a path
/// it merely believes it owns.
pub fn remove_module(name: &str, dry: bool) -> Result<Vec<String>> {
    let module = find(name);
    let name = module.map_or(name, |m| m.name);
    let mut done = Vec::new();

    // The root-installed part first: it is the one that can still fail (a
    // cancelled sudo), and nothing else should be gone if it does.
    if let (Some(setup), Some(home)) = (module.and_then(|m| m.setup), crate::paths::home_dir()) {
        if (setup.installed)(&home) {
            if !dry {
                (setup.uninstall)(&home)?;
            }
            done.push(format!("uninstalled  the setup step (sudo): {}", setup.what));
        }
    }

    let manifest = crate::manifest::load();
    let Some(entry) = manifest.modules.get(name) else {
        if !done.is_empty() {
            return Ok(done);
        }
        bail!(
            "coat has no record of applying '{}'.\n\
             The record is written on apply, so a module last applied by an older\n\
             coat has none. Run `coat apply {}` once, then remove it.",
            name,
            name
        );
    };

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

    for (file, keys) in &entry.ini_keys {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let mut section = String::new();
        let mut out = String::new();
        for l in text.lines() {
            let t = l.trim();
            if t.starts_with('[') && t.ends_with(']') {
                section = t[1..t.len() - 1].to_string();
            } else if let Some((k, _)) = l.split_once('=') {
                if keys.contains(&format!("{}/{}", section, k.trim())) {
                    continue;
                }
            }
            out.push_str(l);
            out.push('\n');
        }
        if out == text {
            continue;
        }
        if !dry {
            fs::write(file, &out).with_context(|| format!("Failed to rewrite {}", file.display()))?;
        }
        done.push(format!("dropped {} key(s) from  {}", keys.len(), file.display()));
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

// ── Individual module planners ─────────────────────────────────────────────

fn plan_bat(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "bat", ".config/bat/themes/coat.tmTheme")?;
    p.reload("bat cache --build");
    Ok(())
}

fn plan_btop(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "btop", ".config/btop/themes/coat.theme")?;
    Ok(())
}

fn plan_dunst(c: &Ctx, p: &mut Plan) -> Result<()> {
    // dunst reads drop-ins only from the `dunstrc.d` sitting next to the FIRST
    // dunstrc it finds, searching $XDG_CONFIG_HOME before $XDG_CONFIG_DIRS. With
    // no ~/.config/dunst/dunstrc the base config resolves to /etc/dunst/dunstrc
    // and our drop-in in ~/.config is never looked at — silently unthemed. So
    // make sure a user dunstrc exists, seeded from the packaged one. Only read
    // when it will be used: the packaged file is 20K, read on every apply.
    let base = c.path(".config/dunst/dunstrc");
    if !base.exists() {
        let seed = fs::read_to_string("/etc/dunst/dunstrc").unwrap_or_else(|_| {
            "# Your dunst config. coat writes only dunstrc.d/50-coat.conf\n\
             # (colours + font); geometry, timeouts and actions belong here.\n\
             [global]\n"
                .to_string()
        });
        p.seeds.push((base, seed, false));
    }

    c.render_to(p, "dunst", ".config/dunst/dunstrc.d/50-coat.conf")?;
    // Reload rather than kill/respawn: this re-reads dunstrc plus its drop-ins in
    // place, so the running daemon repaints without dropping its queue.
    p.reload("dunstctl reload");
    Ok(())
}

fn plan_fish(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "fish", ".config/fish/themes/coat.theme")?;
    Ok(())
}

fn plan_gtk(c: &Ctx, p: &mut Plan) -> Result<()> {
    // Same fragment for gtk-3.0 and gtk-4.0, each pulled in by that version's
    // gtk.css. @import has to come first in a CSS file, which is where
    // the include goes.
    for ver in ["gtk-3.0", "gtk-4.0"] {
        c.render_to(p, "gtk", &format!(".config/{}/coat-theme.css", ver))?;
        p.include(
            c.path(&format!(".config/{}/gtk.css", ver)),
            "@import url(\"coat-theme.css\");",
            ("/*", " */"),
        );
    }
    let config = &c.config;
    let theme = if c.scheme.is_dark() { "adw-gtk3-dark" } else { "adw-gtk3" };
    p.always.push(gsettings_set("gtk-theme", theme));
    if !config.font_sansserif().is_empty() {
        p.always.push(gsettings_set(
            "font-name",
            &format!("{} {}", config.font_sansserif(), config.font_size_desktop()),
        ));
    }
    if !config.font_monospace().is_empty() {
        p.always.push(gsettings_set(
            "monospace-font-name",
            &format!("{} {}", config.font_monospace(), config.font_size_terminal()),
        ));
    }
    if !config.font_emoji().is_empty() {
        p.always.push(gsettings_set(
            "document-font-name",
            &format!("{} {}", config.font_emoji(), config.font_size_desktop()),
        ));
    }
    Ok(())
}

/// Shell to set an org.gnome.desktop.interface key, unless it already holds
/// `value`.
///
/// Run on every apply rather than gated on file changes: half of what goes
/// through here (the sans and emoji fonts) is in no file coat writes. dconf
/// signals every set, same value or not, and each GTK app restyles on it -- so
/// ask first.
fn gsettings_set(key: &str, value: &str) -> String {
    let k = format!("org.gnome.desktop.interface {}", key);
    let v = format!("'{}'", value);
    format!("[ \"$(gsettings get {k} 2>/dev/null)\" = \"{v}\" ] || gsettings set {k} \"{v}\"")
}

fn plan_kitty(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "kitty", ".config/kitty/coat-theme.conf")?;
    p.include(c.path(".config/kitty/kitty.conf"), "include coat-theme.conf", ("#", ""));

    // Reload running instances without a restart. `kitty @ load-config` needs
    // remote control enabled (allow_remote_control + listen_on in kitty.conf)
    // and talks over the socket that listen_on names. If that fails -- remote
    // control off, no instance running -- fall back to SIGUSR1, which makes kitty
    // re-read its config the same way.
    //
    // load-config, not `set-colors`: set-colors only ever changed colours, so
    // an opacity or font change was written and never shown.
    //
    // The socket is globbed, not named directly. kitty APPENDS its pid to whatever
    // listen_on says, so `listen_on unix:/tmp/kitty` produces /tmp/kitty-1234 and
    // talking to /tmp/kitty always failed. Stale sockets from dead instances are
    // left behind too, so a failure on one must not stop the others.
    p.reload(
        "ok=1; for s in /tmp/kitty /tmp/kitty-*; do [ -S \"$s\" ] || continue; \
         kitty @ --to \"unix:$s\" load-config 2>/dev/null && ok=0; \
         done; [ $ok = 0 ] || pkill -SIGUSR1 -x kitty",
    );
    Ok(())
}

/// The scheme as plain data, for anything coat has no module for: every colour
/// with its `#`, the scheme's name and variant, and the fonts. Readers map the
/// slots themselves, so a new consumer never needs a coat change.
fn plan_colors(c: &Ctx, p: &mut Plan) -> Result<()> {
    let (s, cfg) = (&c.scheme, &c.config);
    let dest = c.path(".config/coat/colors.json");
    let mut colors = serde_json::Map::new();
    let mut slots = vec![
        ("base00", &s.base00), ("base01", &s.base01), ("base02", &s.base02), ("base03", &s.base03),
        ("base04", &s.base04), ("base05", &s.base05), ("base06", &s.base06), ("base07", &s.base07),
        ("base08", &s.base08), ("base09", &s.base09), ("base0A", &s.base0a), ("base0B", &s.base0b),
        ("base0C", &s.base0c), ("base0D", &s.base0d), ("base0E", &s.base0e), ("base0F", &s.base0f),
    ];
    if s.is_base24 {
        slots.extend([
            ("base10", &s.base10), ("base11", &s.base11), ("base12", &s.base12), ("base13", &s.base13),
            ("base14", &s.base14), ("base15", &s.base15), ("base16", &s.base16_color), ("base17", &s.base17),
        ]);
    }
    for (slot, hex) in slots {
        colors.insert(slot.into(), format!("#{}", hex).into());
    }
    let doc = serde_json::json!({
        "scheme": s.name,
        "author": s.author,
        "variant": if s.is_dark() { "dark" } else { "light" },
        "base24": s.is_base24,
        "colors": colors,
        "font": {
            "monospace": cfg.font_monospace(),
            "sansserif": cfg.font_sansserif(),
            "serif": cfg.font_serif(),
            "size": size_value(cfg.font_size_terminal()),
        },
    });
    p.file(dest.clone(), serde_json::to_string_pretty(&doc)? + "\n");

    // Copies for other machines, each `host:/dir`. Written under a temp name and
    // moved into place so a reader never sees half a file, and left alone when
    // unchanged, so whatever watches it only wakes up for a real change.
    //
    // `sh -c '...'` because the login shell there need not be POSIX (the homelab's
    // is fish). BatchMode: if SSH would prompt (a key passphrase, Tailscale SSH's
    // periodic check), give up rather than hang in the background where nobody
    // can answer it.
    //
    // Not gated on a local change the way reloads are: the far side compares
    // for itself, and a copy that failed while the host was down has to be
    // retried by the next apply, not wait for the next scheme.
    for target in &cfg.colors.send {
        let Some((host, dir)) = target.split_once(':') else {
            p.note(format!("colors.send entry '{}' should look like host:/dir", target));
            continue;
        };
        p.always.push(format!(
            "ssh -o BatchMode=yes -o ConnectTimeout=5 {host} \
             \"sh -c 'f={dir}/colors.json; cat > \\$f.new && \
             if cmp -s \\$f.new \\$f; then rm \\$f.new; else mv \\$f.new \\$f; fi'\" < '{}'",
            dest.to_string_lossy()
        ));
    }
    Ok(())
}

fn plan_fuzzel(c: &Ctx, p: &mut Plan) -> Result<()> {
    let theme = c.render_to(p, "fuzzel", ".config/fuzzel/coat-theme.ini")?;
    // fuzzel resolves an include where the line appears, so this belongs at the
    // top of fuzzel.ini and anything after it wins. Nothing to reload: fuzzel is
    // not a daemon, it reads its config on each launch.
    p.include(
        c.path(".config/fuzzel/fuzzel.ini"),
        format!("include={}", theme.display()),
        ("#", ""),
    );
    Ok(())
}

fn plan_mpv(c: &Ctx, p: &mut Plan) -> Result<()> {
    let theme = c.render_to(p, "mpv", ".config/mpv/coat-theme.conf")?;
    // Was a printed hint telling the user to add this line themselves, which meant
    // that on any machine where they had not, coat wrote a theme nothing read.
    // Every other module with an include mechanism wires itself up; this one now
    // does too.
    p.include(
        c.path(".config/mpv/mpv.conf"),
        format!("include={}", theme.display()),
        ("#", ""),
    );
    Ok(())
}

fn plan_conky(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "conky", ".config/conky/coat-colors.lua")?;
    // conky watches conky.conf for changes and reloads itself, but this writes a
    // file conky.conf only `dofile`s -- which that watch does not cover. So the
    // reload has to be asked for. SIGUSR1 is conky's documented reload signal and
    // it re-executes the config, picking the new table up.
    p.reload("pkill -USR1 -x conky");
    Ok(())
}

fn plan_micro(c: &Ctx, p: &mut Plan) -> Result<()> {
    // A standard micro colourscheme on the search path, so settings.json only
    // ever names it: "colorscheme": "coat". The file holds nothing but
    // colour-links, which is why coat owns all of it rather than a fragment.
    //
    // No reload on purpose. micro reads its colourscheme once at startup and
    // has no config-reload signal; the in-editor `> set colorscheme coat` re-reads
    // it for a running instance, and there is nothing to send from out here.
    c.render_to(p, "micro", ".config/micro/colorschemes/coat.micro")?;
    Ok(())
}

fn plan_mango(c: &Ctx, p: &mut Plan) -> Result<()> {
    let colors = c.render_to(p, "mango", ".config/mango/coat-colors.conf")?;

    // mango's `source=` takes a path, not a relative include, and it does not
    // expand ~ -- so the absolute path has to be written out.
    p.include(
        c.path(".config/mango/config.conf"),
        format!("source={}", colors.display()),
        ("#", ""),
    );

    // The payoff for moving off dwl: mango's config is runtime, so a scheme
    // change recolours a LIVE session. dwl had to be rebuilt and restarted, which
    // meant losing every window and (because its autostart re-ran) its audio.
    //
    // reload_config re-reads the whole config, which re-runs `exec=` lines but
    // NOT `exec-once=` -- daemons are left alone by design.
    p.reload("mmsg dispatch reload_config");
    Ok(())
}

fn plan_hyprland(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "hyprland", ".config/hypr/coat-colors.lua")?;
    // Recolours a running session; a no-op when Hyprland is not up.
    p.reload("hyprctl reload");
    Ok(())
}

fn plan_swaylock(c: &Ctx, p: &mut Plan) -> Result<()> {
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
    c.render_to(p, "swaylock", ".config/swaylock/coat-theme.conf")?;
    Ok(())
}

/// Render the vesktop CSS theme to each of `paths` — used by the Windows
/// `apply_discord` function.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn apply_vesktop_shared(scheme: &Scheme, config: &CoatConfig, paths: &[PathBuf]) -> Result<()> {
    let content = make_tera()?.render("vesktop", &build_context(scheme, config))?;
    let mut plan = Plan::default();
    for path in paths {
        plan.file(path.clone(), content.clone());
    }
    crate::apply::execute_now(plan)
}

/// Qt through qt6ct: the platform theme hands every Qt app coat's palette and
/// fonts, and Fusion draws the widgets from nothing but that palette.
///
/// Fusion, not Kvantum (Stylix's choice), because Kvantum cannot be live. Its
/// polish() overwrites any palette with the colours in its own theme file and
/// it never re-reads that file, so a running app keeps the old scheme until it
/// restarts. Fusion takes qt6ct's new palette in place -- measured, not assumed.
fn plan_qt(c: &Ctx, p: &mut Plan) -> Result<()> {
    // qt5ct only when it is there: Qt 5 apps need it and Qt 6 ones do not, and
    // coat writing its directory would not make it installed.
    let mut tools = vec!["qt6ct"];
    if c.path(".config/qt5ct").is_dir() || on_path("qt5ct") {
        tools.push("qt5ct");
    }
    let mut dirs = Vec::new();
    for tool in tools {
        let dir = c.path(".config").join(tool);
        let colors = c.render_to(p, "qtct", &format!(".config/{}/colors/coat.conf", tool))?;
        let mut keys = vec![
            ("Appearance", "custom_palette", "true".to_string()),
            ("Appearance", "color_scheme_path", colors.display().to_string()),
            ("Appearance", "style", "Fusion".to_string()),
        ];
        // Quoted: unquoted, QSettings splits "Family,10" at the comma into a
        // list, and qt6ct's toString() on a list is empty -- no font at all.
        let size = c.config.font_size_desktop();
        if !c.config.font_sansserif().is_empty() {
            keys.push(("Fonts", "general", format!("\"{},{}\"", c.config.font_sansserif(), size)));
        }
        if !c.config.font_monospace().is_empty() {
            keys.push(("Fonts", "fixed", format!("\"{},{}\"", c.config.font_monospace(), size)));
        }
        p.ini.push((
            dir.join(format!("{}.conf", tool)),
            keys.into_iter().map(|(s, k, v)| (s.to_string(), k.to_string(), v)).collect(),
        ));
        dirs.push(dir);
    }

    // Running apps re-read all of the above when qt6ct's watcher fires -- but it
    // watches the DIRECTORY, which only signals when an entry comes or goes.
    // Rewriting a file in place, or anything under colors/, never trips it. A
    // file created and removed does, unconditionally.
    p.after.push(Box::new(move |changed| {
        if changed {
            for dir in dirs {
                let nudge = dir.join(".coat-reload");
                fs::write(&nudge, "")
                    .with_context(|| format!("Failed to write {}", nudge.display()))?;
                let _ = fs::remove_file(&nudge);
            }
        }
        Ok(())
    }));

    // Nothing above is read unless Qt loads qt6ct (its plugin answers to qt5ct
    // too). environment.d reaches the whole session: the systemd user manager
    // reads it, and a uwsm or systemd-launched compositor starts from that.
    let env = "QT_QPA_PLATFORMTHEME=qt6ct";
    p.file(
        c.path(".config/environment.d/90-coat-qt.conf"),
        format!("# Written by coat (`coat remove qt` deletes it).\n{}\n", env),
    );

    if !on_path("qt6ct") {
        p.note("qt6ct is not installed — install `qt6ct`.");
        return Ok(());
    }

    // A platform theme is chosen once, at app start, so an app already running
    // without the variable cannot be brought round -- only restarted. That is a
    // first-apply problem, and the app that hits it is the portal: a service
    // started at login that spawns the Qt screen-share picker with its own
    // environment. So the first time the variable lands, the session gets it
    // and the portal restarts into it. Only then, so a later apply cannot cut
    // off a share in progress. Not before qt6ct exists either, or every Qt app
    // launched meanwhile warns about a missing plugin.
    //
    // Checked in the background with the reloads: asking systemd took 6ms,
    // half of an apply, and nothing waits on the answer. Qt apps already
    // running need one restart to pick the variable up.
    p.always.push(format!(
        "systemctl --user show-environment | grep -qx '{env}' \
         || {{ systemctl --user set-environment '{env}' \
         && systemctl --user try-restart xdg-desktop-portal-hyprland; }}"
    ));
    Ok(())
}

fn plan_qutebrowser(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "qutebrowser", ".config/qutebrowser/coat-theme.py")?;

    // Once a config.py exists, qutebrowser stops reading autoconfig.yml unless the
    // file says to -- so a config.py holding nothing but coat's include would
    // silently drop every setting made with :set. Seed it with the load call
    // before the include goes on top, which also leaves autoconfig.yml winning.
    let config_py = c.path(".config/qutebrowser/config.py");
    p.seeds.push((config_py.clone(), "config.load_autoconfig()\n".into(), false));
    // config.source resolves a relative path against the config directory.
    p.include(config_py, "config.source('coat-theme.py')", ("#", ""));

    // qutebrowser does not watch its config, but a second `qutebrowser <cmd>`
    // hands the command to the running instance over IPC. Guarded by pgrep,
    // because with nothing running that same call opens a browser.
    p.reload(
        "pgrep -u \"$(id -u)\" -x qutebrowser >/dev/null \
         && qutebrowser ':config-source' >/dev/null 2>&1",
    );
    Ok(())
}

fn plan_vesktop(c: &Ctx, p: &mut Plan) -> Result<()> {
    let dirs = [".config/vesktop/themes", ".config/Vencord/themes"];
    let existing: Vec<&str> = dirs.into_iter().filter(|d| c.path(d).is_dir()).collect();
    // Neither installed yet: write to vesktop's, creating it.
    let targets = if existing.is_empty() { vec![dirs[0]] } else { existing };
    for dir in targets {
        c.render_to(p, "vesktop", &format!("{}/coat.theme.css", dir))?;
    }
    Ok(())
}

fn plan_xresources(c: &Ctx, p: &mut Plan) -> Result<()> {
    // ~/.Xresources is a general-purpose file — DPI, cursor, per-app tweaks — so
    // coat writes a fragment beside it and adds one #include rather than replacing
    // whatever was there. The cpp-style include gets an absolute path so it holds
    // regardless of the directory xrdb runs in.
    let dest = c.render_to(p, "xresources", ".Xresources.coat")?;
    let main = c.path(".Xresources");
    p.include(main.clone(), format!("#include \"{}\"", dest.display()), ("!", ""));
    p.reload(format!("xrdb -merge {}", main.display()));
    Ok(())
}

fn plan_zathura(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "zathura", ".config/zathura/coat-theme")?;
    // girara resolves a relative include against the config being processed.
    p.include(c.path(".config/zathura/zathurarc"), "include coat-theme", ("#", ""));

    // zathura re-reads its config over D-Bus, so a document that is already open
    // recolours in place instead of waiting to be reopened. Each instance owns a
    // well-known name of the form org.pwmt.zathura.PID-<pid>.
    p.reload(concat!(
        r#"busctl --user list --no-legend 2>/dev/null "#,
        r#"| awk '/^org\.pwmt\.zathura\.PID-/{print $1}' "#,
        r#"| while read -r n; do busctl --user call "$n" /org/pwmt/zathura "#,
        r#"org.pwmt.zathura SourceConfig >/dev/null 2>&1; done"#,
    ));
    Ok(())
}

/// The Linux virtual console: Ctrl+Alt+F2 and the rest.
///
/// The console takes its 16 colours from `ESC ] P n rrggbb`, per VT, at
/// runtime and without root. Text already on screen recolours at once, since
/// the console stores palette indices, not colours. The palette lives as those
/// bytes in tty.esc, and two things write it:
///
/// - a fish conf.d snippet, when a shell starts on a VT (TERM=linux), so every
///   new login gets it
/// - the reload, to the VTs this user is logged in on right now
///
/// From boot -- the getty prompt, every VT -- takes root: setvtrgb, or the
/// vt.default_* kernel parameters. coat writes tty.vtrgb in the format both
/// read, and `coat docs tty` has the command.
///
/// No fonts: the console draws PSF bitmap fonts (vconsole.conf), not the TTF
/// families in `font:`.
fn plan_tty(c: &Ctx, p: &mut Plan) -> Result<()> {
    let s = &c.scheme;
    // The slots templates/kitty.tera uses, so a VT and a terminal window agree.
    // The console draws text in 7 on 0: base05 on base00, as everywhere else.
    let palette = [
        &s.base00, &s.base08, &s.base0b, &s.base0a, &s.base0d, &s.base0e, &s.base0c, &s.base05,
        &s.base03, &s.base08, &s.base0b, &s.base0a, &s.base0d, &s.base0e, &s.base0c, &s.base07,
    ];

    let esc_path = c.path(".config/coat/tty.esc");
    let esc: String = palette
        .iter()
        .enumerate()
        .map(|(i, hex)| format!("\x1b]P{:X}{}", i, hex))
        .collect();
    p.file(esc_path.clone(), esc);

    // Red, green and blue, one line each: setvtrgb's decimal format, and the
    // value of each vt.default_* kernel parameter as it stands.
    let channel = |pick: fn((u8, u8, u8)) -> u8| {
        palette.iter().map(|h| pick(Scheme::hex_to_rgb(h)).to_string()).collect::<Vec<_>>().join(",")
    };
    p.file(
        c.path(".config/coat/tty.vtrgb"),
        format!("{}\n{}\n{}\n", channel(|c| c.0), channel(|c| c.1), channel(|c| c.2)),
    );

    // fish sources conf.d by itself, so this needs no include line anywhere.
    if c.path(".config/fish").is_dir() {
        p.file(
            c.path(".config/fish/conf.d/coat-tty.fish"),
            format!(
                "# Written by coat (`coat remove tty` deletes it): the scheme's palette\n\
                 # on a Linux virtual console. Terminal windows are left alone.\n\
                 if test \"$TERM\" = linux\n    cat '{}'\nend\n",
                esc_path.display()
            ),
        );
    }

    // VTs this user is logged in on recolour now; the rest get it at their next
    // login. Owned AND writable, so a getty prompt or another user's VT is never
    // written to. The VT a compositor runs on is in graphics mode, where the
    // kernel keeps the palette for later without drawing anything.
    p.reload(format!(
        "for t in /dev/tty[0-9]*; do [ -O \"$t\" ] && [ -w \"$t\" ] && cat '{}' > \"$t\"; done",
        esc_path.display()
    ));
    Ok(())
}

fn plan_fastfetch(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "fastfetch", ".config/fastfetch/config.jsonc")?;
    Ok(())
}

fn plan_imv(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "imv", ".config/imv/config")?;
    Ok(())
}

fn plan_satty(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "satty", ".config/satty/config.toml")?;
    Ok(())
}

fn plan_lsd(c: &Ctx, p: &mut Plan) -> Result<()> {
    c.render_to(p, "lsd", ".config/lsd/colors.yaml")?;

    // colors.yaml is read ONLY when config.yaml opts in. Without it lsd falls
    // back to its built-in 256-colour theme and says nothing -- the file parses,
    // it is simply never consulted -- so a module that wrote colours alone would
    // look like it worked and change nothing.
    let cfg = c.path(".config/lsd/config.yaml");
    if let Ok(existing) = fs::read_to_string(&cfg) {
        if !existing.contains("theme: custom") {
            p.note(format!(
                "add `color:\n    theme: custom` to {} or the colours are ignored",
                cfg.display()
            ));
        }
    }
    p.seeds.push((cfg, "color:\n  theme: custom\n".into(), true));
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
#[cfg_attr(not(windows), allow(dead_code))]
pub fn apply_vscode_shared(scheme: &Scheme, path: &Path, font: Option<&str>, sizes: VscodeSizes) -> Result<()> {
    let mut plan = Plan::default();
    plan.merged.push((path.to_path_buf(), vscode_settings(scheme, path, font, sizes)?));
    crate::apply::execute_now(plan)
}

fn plan_vscode(c: &Ctx, p: &mut Plan) -> Result<()> {
    let path = c.path(".config/Code/User/settings.json");
    let font = Some(c.config.font_monospace()).filter(|f| !f.is_empty());
    let sizes = VscodeSizes::from_config(&c.config);
    p.merged.push((path.clone(), vscode_settings(&c.scheme, &path, font, sizes)?));
    Ok(())
}

/// coat's sizes are points; VSCode's are CSS pixels, and its UI is a fixed
/// 13px that only `window.zoomLevel` (×1.2 per step) can grow.
#[derive(Clone, Copy)]
pub struct VscodeSizes {
    /// `font.sizes.terminal`, pt: the editor and integrated terminal.
    pub text: f64,
    /// `font.sizes.desktop`, pt: the UI chrome.
    pub ui: f64,
}

impl VscodeSizes {
    pub fn from_config(config: &crate::config::CoatConfig) -> Self {
        Self { text: config.font_size_terminal(), ui: config.font_size_desktop() }
    }

    /// (zoomLevel, editor/terminal fontSize). The zoom also scales the
    /// editor, so the font size is divided back out of it.
    fn settings(self) -> (f64, f64) {
        let px = |pt: f64| pt * 4.0 / 3.0;
        let zoom = px(self.ui) / 13.0;
        let round = |v: f64, k: f64| (v * k).round() / k;
        (round(zoom.ln() / 1.2f64.ln(), 100.0), round(px(self.text) / zoom, 2.0))
    }
}

/// `path`'s settings with coat's colours (and `font`) merged in, as the text to
/// write back. Every other setting is kept.
fn vscode_settings(scheme: &Scheme, path: &Path, font: Option<&str>, sizes: VscodeSizes) -> Result<String> {
    // Read existing settings or start fresh (JSONC-tolerant; never clobbers
    // a file it can't parse).
    let mut settings: serde_json::Map<String, JsonValue> = if path.exists() {
        read_json_settings(path)?
    } else {
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

    let (zoom, size) = sizes.settings();
    settings.insert("window.zoomLevel".to_string(), serde_json::json!(zoom));
    settings.insert("editor.fontSize".to_string(), serde_json::json!(size));
    settings.insert("terminal.integrated.fontSize".to_string(), serde_json::json!(size));

    serde_json::to_string_pretty(&settings).context("Failed to serialize settings.json")
}

// ── Docs strings ───────────────────────────────────────────────────────────

/// `coat docs <module>`, one per entry in `MODULES`.
mod docs {
    pub const BAT: &str = r##"
Writes ~/.config/bat/themes/coat.tmTheme and rebuilds bat's cache.
Select it in ~/.config/bat/config:

  --theme="coat"

Or use temporarily: bat --theme=coat <file>
"##;

    pub const BTOP: &str = r##"
To activate:

1. Open btop
2. Press ESC to open menu
3. Navigate to 'Options' > 'Color theme'
4. Select 'coat'

Or set in ~/.config/btop/btop.conf:
  color_theme = "coat"
"##;

    pub const COLORS: &str = r##"
Writes ~/.config/coat/colors.json: the scheme as plain data, for
anything coat has no module for. Each colour is "#RRGGBB" under
colors.base00 ... base0F (and base10 ... base17 for Base24), plus
scheme, author, variant (dark/light), base24, and font.

To keep a copy on other machines too, in coat.yaml:

  colors:
    send:
      - me@server:/some/dir   # ssh, needs a login that doesn't prompt

The copy is only replaced when it changed, so a reader can watch it.
e.g. jq -r .colors.base00 ~/.config/coat/colors.json
"##;

    pub const CONKY: &str = r##"
coat writes ~/.config/conky/coat-colors.lua as a Lua table.

Load it at the top of ~/.config/conky/conky.conf:

  local c = dofile(os.getenv("HOME") .. "/.config/conky/coat-colors.lua")

Then reference it in the config table and the text block:

  conky.config = { default_color = c.base05, color1 = c.base0D }
  conky.text = [[ ${color1}label${color} ]]

Colours are bare hex, no leading '#'; conky rejects the '#' form.
coat sends SIGUSR1 after writing, which is conky's reload signal.
"##;

    pub const DUNST: &str = r##"
Writes the drop-in ~/.config/dunst/dunstrc.d/50-coat.conf
(colours + font) and runs `dunstctl reload`. Your dunstrc —
geometry, timeouts, icons, mouse actions — is left alone.

Drop-ins apply after the base config in lexical order, last
winning, so override coat with e.g. 99-mine.conf.

Note: dunst only reads the dunstrc.d next to the FIRST dunstrc
it finds, so coat creates ~/.config/dunst/dunstrc if missing —
otherwise the base would resolve to /etc and the drop-in would
never be read.

If it doesn't reload, run manually:
  dunstctl reload
"##;

    pub const FASTFETCH: &str = r##"
config.jsonc is written whole. Just run `fastfetch`.
"##;

    pub const FISH: &str = r##"
To activate the fish theme:

  fish_config theme save coat

Or add to ~/.config/fish/config.fish:

  fish_config theme choose coat
"##;

    pub const FUZZEL: &str = r##"
Writes ~/.config/fuzzel/coat-theme.ini (colours + font) and adds an
include= line for it to the top of fuzzel.ini on first apply, so
anything set below it in fuzzel.ini wins.

fuzzel colours are bare 8-digit hex (rrggbbaa) with NO leading
'#'. Nothing to reload: fuzzel reads its config on each launch.
"##;

    pub const GTK: &str = r##"
Writes coat-theme.css into ~/.config/gtk-3.0 and gtk-4.0, and adds
  @import url("coat-theme.css");
at the top of each gtk.css on first apply — your own CSS below it
is preserved.

Theme and fonts are set via gsettings automatically.

Ensure 'adw-gtk3' and 'adw-gtk3-dark' are installed.
Some apps may require a restart.
"##;

    pub const HYPRLAND: &str = r##"
Writes ~/.config/hypr/coat-colors.lua and runs `hyprctl reload`, so a
running session recolours in place. Load the file from your Hyprland
config to use its colours.
"##;

    pub const IMV: &str = r##"
~/.config/imv/config is written whole.
"##;

    pub const KITTY: &str = r##"
Writes ~/.config/kitty/coat-theme.conf (colours, font, opacity) and adds
  include coat-theme.conf
to the top of kitty.conf on first apply.

Running windows reload in place when kitty.conf has:

  allow_remote_control yes
  listen_on unix:/tmp/kitty

Without remote control coat falls back to SIGUSR1, which reloads too.
Opacity changes take effect live only in a kitty started after coat
first wrote `dynamic_background_opacity yes`.
"##;

    pub const LSD: &str = r##"
Colours live in ~/.config/lsd/colors.yaml, which lsd reads
ONLY when ~/.config/lsd/config.yaml contains:

  color:
    theme: custom

coat writes that file when lsd has none of its own.
"##;

    pub const MANGO: &str = r##"
Writes ~/.config/mango/coat-colors.conf and adds a source= line for it
to config.conf on first apply. `mmsg dispatch reload_config` recolours
the running session.
"##;

    pub const MICRO: &str = r##"
coat writes ~/.config/micro/colorschemes/coat.micro.

Select it in ~/.config/micro/settings.json:

  "colorscheme": "coat"

Colours are '#RRGGBB' and need truecolor: micro's `truecolor`
option is 'auto', which follows $COLORTERM -- set it to 'on' if a
terminal does not advertise it, or the scheme degrades to 256.

A running micro does not pick up a reapply: `> set colorscheme coat`
re-reads the file, and there is no signal coat can send it.
"##;

    pub const MPV: &str = r##"
Writes ~/.config/mpv/coat-theme.conf and adds an include= line for it
to mpv.conf on first apply. Takes effect on mpv's next start.
"##;

    pub const QT: &str = r##"
Qt through qt6ct, drawn by Fusion. Needs:  pacman -S qt6ct

Writes:
  ~/.config/qt6ct/colors/coat.conf          the palette
  ~/.config/qt6ct/qt6ct.conf                custom_palette, color_scheme_path,
                                            style=Fusion, fonts — only those keys
  ~/.config/environment.d/90-coat-qt.conf   QT_QPA_PLATFORMTHEME=qt6ct

qt5ct gets the same when it is installed.

Live: running Qt apps change scheme a few seconds after `coat set`.
Apps started before the first apply need one restart.
"##;

    pub const QUTEBROWSER: &str = r##"
Writes ~/.config/qutebrowser/coat-theme.py (colours + font) and
adds

  config.source('coat-theme.py')

to the top of config.py on first apply. A running qutebrowser is
reloaded with :config-source; anything set later in config.py or
autoconfig.yml still wins.

preferred_color_scheme (what sites see) may need a restart.
"##;

    pub const SATTY: &str = r##"
~/.config/satty/config.toml is written whole.
The annotation palette becomes the scheme's accent wheel.
"##;

    pub const SWAYLOCK: &str = r##"
Writes ~/.config/swaylock/coat-theme.conf as key=value lines; your
swaylock config is never touched. Pass them as flags, which swaylock
reads after its config so they win:

  swaylock $(grep -v '^#' ~/.config/swaylock/coat-theme.conf | sed 's/^/--/')
"##;

    pub const TTY: &str = r##"
The Linux virtual console (Ctrl+Alt+F2 …), in the scheme's 16 colours.
Fonts stay as they are: the console draws PSF bitmap fonts, set in
/etc/vconsole.conf, not the families in `font:`.

Writes:
  ~/.config/coat/tty.esc                 the palette, as console escapes
  ~/.config/coat/tty.vtrgb               the same, for setvtrgb / kernel params
  ~/.config/fish/conf.d/coat-tty.fish    applies tty.esc when fish starts on a VT

VTs you are logged in on recolour on apply; every new login gets it from
the fish snippet. For bash or zsh, add to ~/.bashrc or ~/.zshrc:

  [ "$TERM" = linux ] && cat ~/.config/coat/tty.esc

The login prompt itself, and every VT nobody is logged in on, take root.
The first `coat apply` with tty enabled offers to set that up (one sudo):
two systemd units that run setvtrgb on tty.vtrgb at boot and again each
time coat rewrites it. After that every scheme change recolours every
console, with no password.

  coat setup tty     set them up now, or again after answering no
  coat remove tty    takes them out again and restores the stock palette

The palette as kernel parameters too, for the boot messages before any
unit runs -- add to /etc/kernel/cmdline (unified kernel images) or your
bootloader's command line, then rebuild the image:

  echo vt.default_red=$(sed -n 1p ~/.config/coat/tty.vtrgb) \
       vt.default_grn=$(sed -n 2p ~/.config/coat/tty.vtrgb) \
       vt.default_blu=$(sed -n 3p ~/.config/coat/tty.vtrgb)

Those are fixed at the scheme they were made from; the units follow it.
"##;

    pub const VESKTOP: &str = r##"
Enable the theme in Discord/Vesktop:

  Settings → Vencord → Themes → coat.theme.css

Or restart if auto-loading is enabled.
"##;

    pub const VSCODE: &str = r##"
The theme is automatically activated.

If it doesn't appear, reload VSCode:
  Ctrl+Shift+P → Reload Window
"##;

    pub const XRESOURCES: &str = r##"
Writes ~/.Xresources.coat and adds an #include for it to
~/.Xresources on first apply, so your own DPI, cursor and per-app
settings there are preserved. Then merges automatically.

To make permanent, add to ~/.xinitrc or ~/.xprofile:
  xrdb -merge ~/.Xresources
"##;

    pub const ZATHURA: &str = r##"
Writes ~/.config/zathura/coat-theme (colours + font) and adds
  include coat-theme
to ~/.config/zathura/zathurarc on first apply. Bindings and
behaviour settings stay yours.

Open documents recolour in place over D-Bus.
"##;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FontConfig, FontSizes, OpacityConfig};

    fn scheme(base24: bool, variant: &str) -> Scheme {
        let mut yaml = format!(
            "system: base{}\nname: Fixture\nauthor: coat\nvariant: {}\npalette:\n",
            if base24 { 24 } else { 16 },
            variant
        );
        let slots = if base24 { crate::scheme::SLOTS.len() } else { 16 };
        for (i, slot) in crate::scheme::SLOTS[..slots].iter().enumerate() {
            yaml.push_str(&format!("  {}: \"#{:02x}{:02x}{:02x}\"\n", slot, i * 10, 255 - i * 10, i * 7));
        }
        Scheme::from_yaml(&yaml).unwrap()
    }

    fn config(full: bool) -> CoatConfig {
        let mut c: CoatConfig = serde_norway::from_str("{}").unwrap();
        if full {
            c.font = FontConfig {
                monospace: Some("Mono".into()),
                sansserif: Some("Sans".into()),
                serif: Some("Serif".into()),
                emoji: Some("Emoji".into()),
                sizes: Some(FontSizes { terminal: Some(12.5), desktop: Some(10.0), popups: Some(9.75) }),
            };
            c.opacity = OpacityConfig {
                terminal: Some(0.9),
                applications: Some(0.85),
                desktop: Some(1.0),
                popups: Some(0.8),
            };
        }
        c
    }

    /// Every template renders for every kind of scheme and config, so a tera
    /// mistake fails here and not halfway through somebody's apply.
    #[test]
    fn every_template_renders() {
        let tera = make_tera().unwrap();
        for base24 in [false, true] {
            for variant in ["dark", "light"] {
                for full in [false, true] {
                    let ctx = build_context(&scheme(base24, variant), &config(full));
                    for (name, _) in TEMPLATES {
                        let out = tera.render(name, &ctx).unwrap_or_else(|e| {
                            panic!("{} (base24={} {} full={}): {:?}", name, base24, variant, full, e)
                        });
                        assert!(!out.trim().is_empty(), "{} rendered empty", name);
                    }
                }
            }
        }
    }

    #[test]
    fn every_module_has_a_template_or_needs_none() {
        for m in MODULES {
            assert!(!m.docs.trim().is_empty(), "{} has no docs", m.name);
            assert!(find(m.name).is_some());
            for a in m.aliases {
                assert_eq!(find(a).map(|f| f.name), Some(m.name));
            }
        }
    }
}
