//! What the last apply actually touched, per module.
//!
//! `coat remove` needs to know which files coat wrote, which include lines it
//! patched into somebody else's config, and which ini keys it merged. That
//! information already exists at apply time — every `render_to`,
//! `ensure_include` and `apply_ini_edits` call has it in hand — so it is
//! recorded there rather than restated in a table beside the apply functions.
//! A hand-maintained table is a second source of truth, and it would go stale
//! the first time a module grew a file.
//!
//! Recording is best-effort by design: a manifest that cannot be written must
//! never fail an apply, and a missing manifest only means `remove` has nothing
//! to undo yet.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Files coat generated in full. Removing the module deletes these.
    #[serde(default)]
    pub written: Vec<PathBuf>,
    /// (host config, exact include line) coat prepended to a file it does not
    /// own. Removing strips the line and the comment block coat added above it.
    #[serde(default)]
    pub includes: Vec<(PathBuf, String)>,
    /// (ini file, keys) coat merged into a file the user also edits.
    #[serde(default)]
    pub ini_keys: Vec<(PathBuf, Vec<String>)>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub modules: BTreeMap<String, Entry>,
}

/// The module currently being applied, and what it has touched so far.
static CURRENT: Mutex<Option<(String, Entry)>> = Mutex::new(None);

pub fn path() -> Result<PathBuf> {
    let base = dirs::state_dir()
        .or_else(dirs::data_local_dir)
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

/// Start recording for `module`. Anything the previous module recorded and did
/// not commit is dropped.
pub fn begin(module: &str) {
    if let Ok(mut cur) = CURRENT.lock() {
        *cur = Some((module.to_string(), Entry::default()));
    }
}

/// Write the recorded entry out, REPLACING whatever that module recorded before
/// — the current apply is the truth about what exists now.
pub fn commit() {
    let Ok(mut cur) = CURRENT.lock() else { return };
    let Some((module, entry)) = cur.take() else {
        return;
    };
    let mut m = load();
    m.modules.insert(module, entry);
    let _ = store(&m);
}

/// Forget a module entirely (after a successful remove).
pub fn forget(module: &str) {
    let mut m = load();
    m.modules.remove(module);
    let _ = store(&m);
}

fn with<F: FnOnce(&mut Entry)>(f: F) {
    if let Ok(mut cur) = CURRENT.lock() {
        if let Some((_, entry)) = cur.as_mut() {
            f(entry);
        }
    }
}

pub fn record_write(dest: &Path) {
    let dest = dest.to_path_buf();
    with(|e| {
        if !e.written.contains(&dest) {
            e.written.push(dest);
        }
    });
}

pub fn record_include(config: &Path, line: &str) {
    let pair = (config.to_path_buf(), line.to_string());
    with(|e| {
        if !e.includes.contains(&pair) {
            e.includes.push(pair);
        }
    });
}

pub fn record_ini_keys(dest: &Path, keys: &[String]) {
    let pair = (dest.to_path_buf(), keys.to_vec());
    with(|e| {
        if !e.ini_keys.iter().any(|(p, _)| p == &pair.0) {
            e.ini_keys.push(pair);
        }
    });
}
