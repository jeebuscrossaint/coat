use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Default)]
pub struct FontSizes {
    pub terminal: Option<u32>,
    pub desktop: Option<u32>,
    pub popups: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FontConfig {
    pub monospace: Option<String>,
    pub sansserif: Option<String>,
    pub serif: Option<String>,
    pub emoji: Option<String>,
    pub sizes: Option<FontSizes>,
}

#[derive(Debug, Deserialize, Default)]
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
    /// 0.0 = untouched scheme, 1.0 = fully standardized.
    #[serde(default = "default_strength")]
    pub strength: f64,
    /// Also retone the base00..base07 greyscale ramp, not just the accents.
    #[serde(default = "default_true")]
    pub ramp: bool,
    /// Minimum hue separation between accents, in degrees. 0 disables.
    #[serde(default = "default_min_hue_sep")]
    pub min_hue_sep: f64,
    /// WCAG contrast ratio base05 must clear against base00. <= 1.0 disables.
    #[serde(default = "default_contrast_floor")]
    pub contrast_floor: f64,
}

fn default_strength() -> f64 {
    0.6
}
fn default_true() -> bool {
    true
}
fn default_min_hue_sep() -> f64 {
    20.0
}
fn default_contrast_floor() -> f64 {
    7.0
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: default_strength(),
            ramp: default_true(),
            min_hue_sep: default_min_hue_sep(),
            contrast_floor: default_contrast_floor(),
        }
    }
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
        serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
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
