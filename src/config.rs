use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FontSizes {
    pub terminal: Option<u32>,
    pub desktop: Option<u32>,
    pub popups: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FontConfig {
    pub monospace: Option<String>,
    pub sansserif: Option<String>,
    pub serif: Option<String>,
    pub emoji: Option<String>,
    pub sizes: Option<FontSizes>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct OpacityConfig {
    pub terminal: Option<f64>,
    pub applications: Option<f64>,
    pub desktop: Option<f64>,
    pub popups: Option<f64>,
}

/// Material-You-style perceptual regularization (see `normalize.rs`).
/// Off by default — enabling it changes every colour coat writes.
#[derive(Debug, Deserialize, Clone)]
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

/// Drop NUL bytes before the YAML parser sees them.
///
/// A file that was open when the machine lost power comes back from NTFS
/// zero-padded to its allocated length — `scheme: gruvbox\n` followed by forty
/// NULs. YAML rejects those outright ("control characters are not allowed"),
/// and because every command loads the config first, one unclean shutdown took
/// coat out entirely until the file was edited by hand. A NUL is never
/// meaningful in a config, so dropping it is always the right repair.
///
/// `update_scheme_in_config` sanitizes on the same path, so the next `coat set`
/// also writes the padding back out of existence rather than preserving it as
/// an unrecognized line.
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
pub struct ModuleOverride {
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub opacity: OpacityConfig,
}

#[derive(Debug, Deserialize)]
pub struct CoatConfig {
    pub scheme: String,
    #[serde(default)]
    pub prefer_base24: bool,
    #[serde(default)]
    pub normalize: NormalizeConfig,
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub opacity: OpacityConfig,
    /// Keyed by module name, exactly as it appears under `enabled:`.
    #[serde(default)]
    pub overrides: HashMap<String, ModuleOverride>,
}

impl CoatConfig {
    pub fn path() -> Result<std::path::PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".config/coat/coat.yaml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        serde_yaml::from_str(&sanitize(&content))
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
        let Some(ov) = self.overrides.get(module) else {
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

        out
    }

    fn shallow_clone(&self) -> CoatConfig {
        CoatConfig {
            scheme: self.scheme.clone(),
            prefer_base24: self.prefer_base24,
            normalize: self.normalize.clone(),
            enabled: self.enabled.clone(),
            font: self.font.clone(),
            opacity: self.opacity.clone(),
            overrides: HashMap::new(),
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
    pub fn font_size_terminal(&self) -> u32 {
        self.font.sizes.as_ref().and_then(|s| s.terminal).unwrap_or(12)
    }
    pub fn font_size_desktop(&self) -> u32 {
        self.font.sizes.as_ref().and_then(|s| s.desktop).unwrap_or(10)
    }
    pub fn font_size_popups(&self) -> u32 {
        self.font.sizes.as_ref().and_then(|s| s.popups).unwrap_or(11)
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
        let alpha = (self.opacity_popups() * 255.0).round() as u8;
        format!("{:02X}", alpha)
    }
}
