use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;

use crate::scheme::{Scheme, SLOTS};

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct FontSizes {
    pub terminal: Option<f64>,
    pub desktop: Option<f64>,
    pub popups: Option<f64>,
}

/// A size as a JSON/template value: whole sizes stay integers ("13", not
/// "13.0"), so only a fractional size like 9.75 changes what gets written.
pub fn size_value(size: f64) -> serde_json::Value {
    if size.fract() == 0.0 && size >= 0.0 {
        (size as u64).into()
    } else {
        serde_json::Number::from_f64(size).map_or(serde_json::Value::Null, Into::into)
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct FontConfig {
    pub monospace: Option<String>,
    pub sansserif: Option<String>,
    pub serif: Option<String>,
    pub emoji: Option<String>,
    pub sizes: Option<FontSizes>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct OpacityConfig {
    pub terminal: Option<f64>,
    pub applications: Option<f64>,
    pub desktop: Option<f64>,
    pub popups: Option<f64>,
}

/// Material-You-style perceptual regularization (see `normalize.rs`).
/// Off by default — enabling it changes every colour coat writes.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NormalizeConfig {
    #[serde(default)]
    pub enabled: bool,
    /// 0.0 = untouched scheme, 1.0 = fully standardized. Drives hue and chroma —
    /// the part of a palette that reads as the scheme's identity.
    #[serde(default = "default_strength")]
    pub strength: f64,
    /// Same scale, but for lightness only. Defaults to 1.0: full tonal
    /// uniformity (even brightness, legible text) while `strength` leaves
    /// saturation and hue closer to whatever the scheme author picked.
    /// Explicit `null` makes it follow `strength` instead.
    #[serde(default = "default_lightness_strength")]
    pub lightness_strength: Option<f64>,
    /// Also retone the base00..base07 greyscale ramp, not just the accents.
    #[serde(default = "default_true")]
    pub ramp: bool,
    /// Stretches the ramp away from mid-grey. 1.0 = tuned baseline; above that
    /// pushes base00 blacker and base07 whiter.
    #[serde(default = "default_ramp_contrast")]
    pub ramp_contrast: f64,
    /// Target lightness for base08..base0E, 0..1. Unset picks a sensible value
    /// from the scheme's polarity (0.72 dark / 0.52 light).
    #[serde(default)]
    pub accent_lightness: Option<f64>,
    /// Chroma (saturation) ceiling for accents. Higher = more vivid; anything
    /// past the sRGB gamut is mapped back down per-hue.
    #[serde(default = "default_accent_chroma")]
    pub accent_chroma: f64,
    /// Minimum hue separation between accents, in degrees. 0 disables.
    #[serde(default = "default_min_hue_sep")]
    pub min_hue_sep: f64,
    /// WCAG contrast ratio base05 must clear against base00. <= 1.0 disables.
    #[serde(default = "default_contrast_floor")]
    pub contrast_floor: f64,
}

// These defaults are the tuned result of working through real schemes — they
// are meant to be good enough that `normalize: {enabled: true}` is the whole
// config. Everything below is an escape hatch, not a required decision.

/// Partial, so each scheme keeps its own saturation and hue character.
fn default_strength() -> f64 {
    0.45
}
fn default_true() -> bool {
    true
}
/// Full, so tone is uniform regardless of scheme — this is what makes accents
/// evenly legible and is the main reason to normalize at all.
fn default_lightness_strength() -> Option<f64> {
    Some(1.0)
}
fn default_min_hue_sep() -> f64 {
    20.0
}
/// Dark schemes naturally land near 9:1 while light ones hit ~18:1; 12 closes
/// that gap so a dark theme doesn't feel washed out next to a light one.
fn default_contrast_floor() -> f64 {
    12.0
}
/// Above 1.0 flattens tinted backgrounds into plain black/white — rarely wanted.
fn default_ramp_contrast() -> f64 {
    1.0
}
fn default_accent_chroma() -> f64 {
    0.14
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: default_strength(),
            lightness_strength: default_lightness_strength(),
            ramp: default_true(),
            ramp_contrast: default_ramp_contrast(),
            accent_lightness: None,
            accent_chroma: default_accent_chroma(),
            min_hue_sep: default_min_hue_sep(),
            contrast_floor: default_contrast_floor(),
        }
    }
}

fn default_scheme() -> String {
    "default".to_string()
}

/// Drop NUL bytes before the YAML parser sees them.
///
/// A file that was open when the machine lost power comes back from NTFS
/// zero-padded to its allocated length — `scheme: gruvbox\n` followed by forty
/// NULs. YAML rejects those outright ("control characters are not allowed"),
/// and because every command loads the config first, one unclean shutdown took
/// coat out entirely until the file was edited by hand. A NUL is never
/// meaningful in a config, so dropping it is always the right repair.
///
/// The state file is read through here too, for the same reason.
pub fn sanitize(content: &str) -> String {
    content.replace('\0', "")
}

/// Per-app overrides of the global font and opacity, in the spirit of stylix's
/// per-target settings.
///
/// The global block stays the answer for "what does this desktop look like";
/// this is for the places where one app genuinely needs something else. The
/// motivating case: the shell wants SF Pro, because a UI drawn in a monospace
/// font never reads right, while the terminal obviously must stay on SF Mono.
/// Expressing that globally is impossible — there is one `sansserif` key.
///
/// Only the keys present are overridden; everything else falls through to the
/// global block, so an override is a patch and not a replacement.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct ModuleOverride {
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub opacity: OpacityConfig,
    /// Slots repainted for this app only, over the global `palette:`.
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
}

impl ModuleOverride {
    /// Every key this override sets, each paired with a copy of the override in
    /// which only that key holds some other value. Planning the module with both
    /// and getting the same files back proves the app has nowhere to put it --
    /// asked of the module's own output, so no list of who-reads-what is kept.
    pub fn probes(&self) -> Vec<(String, ModuleOverride)> {
        type Set<T> = fn(&mut ModuleOverride, T);
        let sizes = self.font.sizes.clone().unwrap_or_default();

        let fonts: [(&str, &Option<String>, Set<String>); 4] = [
            ("font.monospace", &self.font.monospace, |o, v| o.font.monospace = Some(v)),
            ("font.sansserif", &self.font.sansserif, |o, v| o.font.sansserif = Some(v)),
            ("font.serif", &self.font.serif, |o, v| o.font.serif = Some(v)),
            ("font.emoji", &self.font.emoji, |o, v| o.font.emoji = Some(v)),
        ];
        let numbers: [(&str, Option<f64>, Set<f64>); 7] = [
            ("font.sizes.terminal", sizes.terminal, |o, v| {
                o.font.sizes.get_or_insert_with(Default::default).terminal = Some(v)
            }),
            ("font.sizes.desktop", sizes.desktop, |o, v| {
                o.font.sizes.get_or_insert_with(Default::default).desktop = Some(v)
            }),
            ("font.sizes.popups", sizes.popups, |o, v| {
                o.font.sizes.get_or_insert_with(Default::default).popups = Some(v)
            }),
            ("opacity.terminal", self.opacity.terminal, |o, v| o.opacity.terminal = Some(v)),
            ("opacity.applications", self.opacity.applications, |o, v| o.opacity.applications = Some(v)),
            ("opacity.desktop", self.opacity.desktop, |o, v| o.opacity.desktop = Some(v)),
            ("opacity.popups", self.opacity.popups, |o, v| o.opacity.popups = Some(v)),
        ];

        let mut out = Vec::new();
        for (key, value, set) in fonts {
            if let Some(v) = value {
                let mut ov = self.clone();
                set(&mut ov, format!("{} probe", v));
                out.push((key.to_string(), ov));
            }
        }
        for (key, value, set) in numbers {
            if let Some(v) = value {
                // A step big enough to survive any app's rounding: a whole point
                // of size, a quarter of opacity (towards the middle, so in range).
                let other = match key.starts_with("opacity") {
                    true if v > 0.5 => v - 0.25,
                    true => v + 0.25,
                    false => v + 1.0,
                };
                let mut ov = self.clone();
                set(&mut ov, other);
                out.push((key.to_string(), ov));
            }
        }
        for (slot, value) in &self.palette {
            // Two candidates, so one of them always differs from what was set.
            let other = if value.trim_start_matches('#').eq_ignore_ascii_case("000000") {
                "#FFFFFF"
            } else {
                "#000000"
            };
            let mut ov = self.clone();
            ov.palette.insert(slot.clone(), other.to_string());
            out.push((format!("palette.{}", slot), ov));
        }
        out
    }
}

impl CoatConfig {
    /// This config with `module`'s override replaced by `ov`, under every name
    /// it was written as.
    pub fn with_override(&self, module: &str, ov: ModuleOverride) -> CoatConfig {
        let mut out = self.shallow_clone();
        out.overrides = self.overrides.clone();
        out.overrides.retain(|k, _| crate::modules::find(k).map_or(k.as_str(), |m| m.name) != module);
        out.overrides.insert(module.to_string(), ov);
        out
    }
}

/// Where the current scheme is remembered.
///
/// NOT coat.yaml. The scheme in use is state, not configuration: it changes
/// every time you press the theme key, and coat.yaml is a file people keep in a
/// dotfiles repo. Writing it back there made the busiest file in that repo a
/// config nobody had edited -- 102 commits of `scheme:` churn, and a merge
/// conflict between two machines that had simply picked different colours.
///
/// coat.yaml still carries a `scheme:` line and it still means something: it is
/// the seed, what a fresh checkout comes up as before anything has been picked.
/// State wins once it exists, and coat never writes the config file again.
pub fn state_path() -> Result<std::path::PathBuf> {
    let dir = match std::env::var_os("XDG_STATE_HOME") {
        Some(d) if !d.is_empty() => std::path::PathBuf::from(d),
        _ => crate::paths::home_dir()
            .context("Cannot determine home directory")?
            .join(".local/state"),
    };
    Ok(dir.join("coat/state.yaml"))
}

/// `key: value` and nothing cleverer, so a shell script can read it with grep
/// the same way it used to read coat.yaml.
pub fn load_state() -> Option<(String, bool)> {
    let content = fs::read_to_string(state_path().ok()?).ok()?;
    let mut scheme = None;
    let mut base24 = false;
    for line in sanitize(&content).lines() {
        match line.split_once(':') {
            Some((k, v)) if k.trim() == "scheme" => scheme = Some(v.trim().to_string()),
            Some((k, v)) if k.trim() == "base24" => base24 = v.trim() == "true",
            _ => {}
        }
    }
    scheme.filter(|s| !s.is_empty()).map(|s| (s, base24))
}

pub fn save_state(scheme: &str, base24: bool) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create state directory")?;
    }
    fs::write(&path, format!("scheme: {}\nbase24: {}\n", scheme, base24))
        .with_context(|| format!("Failed to write {}", path.display()))
}

/// `prefer_base24:` — `true`, `false`, or `any`. `any` lets `coat random` deal
/// from both flavours; everywhere else it behaves like `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Flavour {
    #[default]
    Base16,
    Base24,
    Any,
}

impl Flavour {
    pub fn from_bool(b: bool) -> Self {
        if b { Flavour::Base24 } else { Flavour::Base16 }
    }

    pub fn prefers_base24(self) -> bool {
        self == Flavour::Base24
    }
}

impl<'de> Deserialize<'de> for Flavour {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Bool(bool),
            Str(String),
        }
        match Raw::deserialize(d)? {
            Raw::Bool(b) => Ok(Flavour::from_bool(b)),
            Raw::Str(s) => match s.to_lowercase().as_str() {
                "any" | "either" | "both" | "random" => Ok(Flavour::Any),
                "true" | "base24" => Ok(Flavour::Base24),
                "false" | "base16" => Ok(Flavour::Base16),
                _ => Err(serde::de::Error::custom(format!(
                    "prefer_base24: expected true, false or any, got '{}'", s
                ))),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CoatConfig {
    #[serde(default = "default_scheme")]
    pub scheme: String,
    #[serde(default)]
    pub prefer_base24: Flavour,
    #[serde(default)]
    pub normalize: NormalizeConfig,
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub opacity: OpacityConfig,
    /// Scheme slots to repaint, whatever scheme is loaded: `base0D: "#89b4fa"`,
    /// or another slot's name to borrow its colour (`base01: base00`).
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
    /// Keyed by module name, exactly as it appears under `enabled:`.
    #[serde(default)]
    pub overrides: HashMap<String, ModuleOverride>,
    /// Where the colors module also sends colors.json.
    #[serde(default)]
    pub colors: ColorsConfig,
    /// Top-level keys coat does not know. Kept rather than rejected, so a
    /// config from a newer coat still loads here, and reported by `check`.
    #[serde(flatten)]
    pub unknown: BTreeMap<String, serde_norway::Value>,
}

/// The keys `CoatConfig` reads, for "did you mean" on the ones it does not.
const TOP_LEVEL_KEYS: &[&str] = &[
    "scheme", "prefer_base24", "normalize", "enabled", "font", "opacity", "palette", "overrides",
    "colors",
];

/// The closest of `candidates` to `word`, if any is close enough to be a typo.
fn suggest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let max = (word.len() / 3).max(2);
    candidates
        .into_iter()
        .map(|c| (crate::scheme::levenshtein(&word.to_lowercase(), &c.to_lowercase()), c))
        .filter(|(d, _)| *d <= max)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn did_you_mean(s: Option<&str>) -> String {
    s.map(|s| format!(" — did you mean '{}'?", s)).unwrap_or_default()
}

/// One `palette:` entry is usable: a real slot, set to `#RRGGBB` or to another
/// slot's name.
fn check_palette_entry(slot: &str, value: &str) -> Result<()> {
    let is_slot = |s: &str| SLOTS.iter().any(|k| k.eq_ignore_ascii_case(s));
    if !is_slot(slot) {
        bail!(
            "palette: no slot '{}' (base00 … base17){}",
            slot,
            did_you_mean(suggest(slot, SLOTS))
        );
    }
    let v = value.trim();
    let h = v.trim_start_matches('#');
    if !is_slot(v) && (h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit())) {
        bail!("palette: {} = '{}' is neither #RRGGBB nor a slot name", slot, value);
    }
    Ok(())
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColorsConfig {
    /// `host:/dir` targets, copied to over ssh.
    #[serde(default)]
    pub send: Vec<String>,
}

impl CoatConfig {
    pub fn path() -> Result<std::path::PathBuf> {
        let home = crate::paths::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".config/coat/coat.yaml"))
    }

    pub fn load() -> Result<Self> {
        let mut config = Self::load_file()?;
        // What is actually on screen beats what the checkout came with.
        if let Some((scheme, base24)) = load_state() {
            config.scheme = scheme;
            config.prefer_base24 = Flavour::from_bool(base24);
        }
        Ok(config)
    }

    /// coat.yaml exactly as written, without the applied-scheme state over it.
    /// `prefer_base24` here is the user's choice, not the current scheme's flavour.
    pub fn load_file() -> Result<Self> {
        let path = Self::path()?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        serde_norway::from_str(&sanitize(&content))
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    /// The config as one module sees it: the global block with that module's
    /// override patched over the top.
    ///
    /// Returns a whole merged CoatConfig rather than threading a module name
    /// through every accessor, so the ~30 `apply_*` functions and every template
    /// keep reading plain `config.font_monospace()` and cannot forget to ask for
    /// the override.
    pub fn merged_for(&self, module: &str) -> CoatConfig {
        // By any of the module's names: `overrides: { discord: … }` is vesktop's.
        let owner = |k: &String| crate::modules::find(k).map_or(k.as_str(), |m| m.name) == module;
        let Some(ov) = self.overrides.iter().find(|(k, _)| owner(k)).map(|(_, ov)| ov) else {
            return self.shallow_clone();
        };

        let mut out = self.shallow_clone();

        if ov.font.monospace.is_some() {
            out.font.monospace = ov.font.monospace.clone();
        }
        if ov.font.sansserif.is_some() {
            out.font.sansserif = ov.font.sansserif.clone();
        }
        if ov.font.serif.is_some() {
            out.font.serif = ov.font.serif.clone();
        }
        if ov.font.emoji.is_some() {
            out.font.emoji = ov.font.emoji.clone();
        }

        // Sizes merge per-key too. An override that sets only `popups` must not
        // silently reset `terminal` and `desktop` to their defaults.
        if let Some(sizes) = &ov.font.sizes {
            let mut base = out.font.sizes.clone().unwrap_or_default();
            if sizes.terminal.is_some() {
                base.terminal = sizes.terminal;
            }
            if sizes.desktop.is_some() {
                base.desktop = sizes.desktop;
            }
            if sizes.popups.is_some() {
                base.popups = sizes.popups;
            }
            out.font.sizes = Some(base);
        }

        if ov.opacity.terminal.is_some() {
            out.opacity.terminal = ov.opacity.terminal;
        }
        if ov.opacity.applications.is_some() {
            out.opacity.applications = ov.opacity.applications;
        }
        if ov.opacity.desktop.is_some() {
            out.opacity.desktop = ov.opacity.desktop;
        }
        if ov.opacity.popups.is_some() {
            out.opacity.popups = ov.opacity.popups;
        }

        // Per slot, like everything else here: the app's entries win, the rest
        // of the global palette still applies.
        out.palette.extend(ov.palette.clone());

        out
    }

    /// `scheme` with this config's `palette:` painted over it. Call it on a
    /// `merged_for` config to get one app's colours.
    ///
    /// A slot named as a value is read from the scheme as loaded, not as
    /// repainted, so `base00: base01` + `base01: base00` swaps the two.
    pub fn paint(&self, scheme: &Scheme) -> Result<Scheme> {
        let mut loaded = scheme.clone();
        let mut out = scheme.clone();
        for (slot, value) in &self.palette {
            check_palette_entry(slot, value)?;
            let v = value.trim();
            let hex = match loaded.slot_mut(v) {
                Some(from) => from.clone(),
                None => v.trim_start_matches('#').to_uppercase(),
            };
            if let Some(dest) = out.slot_mut(slot) {
                *dest = hex;
            }
        }
        Ok(out)
    }

    /// Whether this config can be applied, and what in it is probably a typo.
    ///
    /// A bad `palette:` entry is an error, caught here once instead of failing
    /// every module that reads it. Names that match no module and keys coat
    /// does not read are warnings: they do nothing, which is exactly why they
    /// need saying -- `overrides: { kity: … }` used to be silently ignored.
    pub fn check(&self) -> Result<Vec<String>> {
        for (slot, value) in &self.palette {
            check_palette_entry(slot, value)?;
        }
        for (module, ov) in &self.overrides {
            for (slot, value) in &ov.palette {
                check_palette_entry(slot, value).with_context(|| format!("in overrides.{}", module))?;
            }
        }

        let modules = || {
            crate::modules::MODULES
                .iter()
                .flat_map(|m| std::iter::once(m.name).chain(m.aliases.iter().copied()))
        };
        let mut warnings = Vec::new();
        for name in &self.enabled {
            if crate::modules::find(name).is_none() {
                warnings.push(format!(
                    "enabled: no module '{}'{}",
                    name,
                    did_you_mean(suggest(name, modules()))
                ));
            }
        }
        let mut names: Vec<&String> = self.overrides.keys().collect();
        names.sort();
        for name in names {
            if crate::modules::find(name).is_none() {
                warnings.push(format!(
                    "overrides: no module '{}'{}",
                    name,
                    did_you_mean(suggest(name, modules()))
                ));
            }
        }
        // Two names for one module: only one of them is used, and which is not
        // something to leave to a HashMap.
        let mut owners: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
        for name in self.overrides.keys() {
            if let Some(m) = crate::modules::find(name) {
                owners.entry(m.name).or_default().push(name);
            }
        }
        for (module, mut keys) in owners {
            if keys.len() > 1 {
                keys.sort();
                let keys: Vec<String> = keys.iter().map(|k| format!("overrides.{}", k)).collect();
                warnings.push(format!(
                    "{} {} name {}; only one is used, so merge them into one",
                    keys.join(" and "),
                    if keys.len() == 2 { "both" } else { "all" },
                    module
                ));
            }
        }
        for key in self.unknown.keys() {
            warnings.push(format!(
                "coat.yaml: unknown key '{}'{}",
                key,
                did_you_mean(suggest(key, TOP_LEVEL_KEYS.iter().copied()))
            ));
        }
        Ok(warnings)
    }

    fn shallow_clone(&self) -> CoatConfig {
        CoatConfig {
            scheme: self.scheme.clone(),
            prefer_base24: self.prefer_base24,
            normalize: self.normalize.clone(),
            enabled: self.enabled.clone(),
            font: self.font.clone(),
            opacity: self.opacity.clone(),
            palette: self.palette.clone(),
            overrides: HashMap::new(),
            unknown: BTreeMap::new(),
            colors: self.colors.clone(),
        }
    }

    pub fn font_monospace(&self) -> &str {
        self.font.monospace.as_deref().unwrap_or("")
    }
    pub fn font_sansserif(&self) -> &str {
        self.font.sansserif.as_deref().unwrap_or("")
    }
    pub fn font_serif(&self) -> &str {
        self.font.serif.as_deref().unwrap_or("")
    }
    pub fn font_emoji(&self) -> &str {
        self.font.emoji.as_deref().unwrap_or("")
    }
    pub fn font_size_terminal(&self) -> f64 {
        self.font.sizes.as_ref().and_then(|s| s.terminal).unwrap_or(12.0)
    }
    pub fn font_size_desktop(&self) -> f64 {
        self.font.sizes.as_ref().and_then(|s| s.desktop).unwrap_or(10.0)
    }
    pub fn font_size_popups(&self) -> f64 {
        self.font.sizes.as_ref().and_then(|s| s.popups).unwrap_or(11.0)
    }
    pub fn opacity_terminal(&self) -> f64 {
        self.opacity.terminal.unwrap_or(1.0)
    }
    pub fn opacity_desktop(&self) -> f64 {
        self.opacity.desktop.unwrap_or(1.0)
    }
    pub fn opacity_popups(&self) -> f64 {
        self.opacity.popups.unwrap_or(1.0)
    }
    pub fn opacity_applications(&self) -> f64 {
        self.opacity.applications.unwrap_or(1.0)
    }
    pub fn opacity_popups_hex(&self) -> String {
        alpha_hex(self.opacity_popups())
    }
    pub fn opacity_applications_hex(&self) -> String {
        alpha_hex(self.opacity_applications())
    }
}

/// An opacity as the two hex digits of an 8-bit alpha channel.
fn alpha_hex(opacity: f64) -> String {
    format!("{:02X}", (opacity.clamp(0.0, 1.0) * 255.0).round() as u8)
}
