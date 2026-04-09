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

#[derive(Debug, Deserialize)]
pub struct CoatConfig {
    pub scheme: String,
    #[serde(default)]
    pub prefer_base24: bool,
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
