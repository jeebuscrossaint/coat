//! What the last apply actually touched, per module.
//!
//! `coat remove` needs to know which files coat wrote and which include lines
//! it patched into somebody else's config, and which ini keys it set. That
//! information already exists at apply time — it is exactly the module's
//! `apply::Plan` — so it is recorded from there rather than restated in a
//! table beside the modules.
//! A hand-maintained table is a second source of truth, and it would go stale
//! the first time a module grew a file.
//!
//! Recording is best-effort by design: a manifest that cannot be written must
//! never fail an apply, and a missing manifest only means `remove` has nothing
//! to undo yet.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Files coat generated in full. Removing the module deletes these.
    #[serde(default)]
    pub written: Vec<PathBuf>,
    /// (host config, exact include line) coat prepended to a file it does not
    /// own. Removing strips the line and the comment block coat added above it.
    #[serde(default)]
    pub includes: Vec<(PathBuf, String)>,
    /// (ini file, "section/key"s) coat set in a file another program also
    /// writes. Removing drops exactly those keys.
    #[serde(default)]
    pub ini_keys: Vec<(PathBuf, Vec<String>)>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub modules: BTreeMap<String, Entry>,
}

pub fn path() -> Result<PathBuf> {
    let base = crate::paths::state_dir()
        .or_else(crate::paths::data_local_dir)
        .context("Cannot determine a state directory")?;
    Ok(base.join("coat/manifest.json"))
}

pub fn load() -> Manifest {
    let Ok(p) = path() else {
        return Manifest::default();
    };
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn store(m: &Manifest) -> Result<()> {
    let p = path()?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&p, serde_json::to_string_pretty(m)?)?;
    Ok(())
}

/// Write what an apply recorded, REPLACING whatever each of those modules
/// recorded before -- the current apply is the truth about what exists now.
/// One read and one write for the whole apply, not one per module.
pub fn record_all(entries: Vec<(String, Entry)>) {
    if entries.is_empty() {
        return;
    }
    let mut m = load();
    m.modules.extend(entries);
    let _ = store(&m);
}

/// Forget a module entirely (after a successful remove).
pub fn forget(module: &str) {
    let mut m = load();
    m.modules.remove(module);
    let _ = store(&m);
}
