//! The one place an apply touches the disk.
//!
//! A module (modules.rs) only DESCRIBES its theme, as a `Plan`:
//!
//! - the files it renders
//! - the include line it needs in somebody else's config
//! - the ini keys it sets
//! - the command that makes a running app re-read all of that
//!
//! This module carries the plan out. It compares it to what is on disk and
//! writes only what differs. It records what coat now owns. It hands back the
//! reloads only for a module where something actually changed.
//!
//! Keeping every write here, rather than in thirty apply functions, buys three
//! things:
//!
//! - `--dry` shows exactly what a real apply would do, because it is the same
//!   code with the writes held in memory.
//! - Every module can plan at once, because a plan touches nothing.
//! - The manifest `coat remove` works from is complete by construction: a plan
//!   is the list of what coat owns.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::CoatConfig;
use crate::manifest::Entry;
use crate::scheme::Scheme;

/// A comment's delimiters in the target syntax: ("#", "") for ini/conf, ("/*",
/// " */") for CSS. Getting this wrong writes a file the app refuses to parse.
pub type Comment = (&'static str, &'static str);

/// Runs after the files are in place, with whether any of them changed. For
/// the few steps that are not a file: the qt watcher nudge, its session env.
pub type After = Box<dyn FnOnce(bool) -> Result<()> + Send>;

/// (section, key, value) to set in an ini file.
pub type IniEdit = (String, String, String);

/// What one module wants. Built by the planners in modules.rs, never by IO.
#[derive(Default)]
pub struct Plan {
    /// Files coat owns outright. `coat remove` deletes them.
    pub files: Vec<(PathBuf, String)>,
    /// Somebody else's file, rewritten whole after coat merged its keys in
    /// (VS Code's settings.json). Written when it changed; never deleted.
    pub merged: Vec<(PathBuf, String)>,
    /// Written only when missing: a config the app needs to exist before it
    /// reads coat's fragment. `owned` ones are coat's to delete on remove.
    pub seeds: Vec<(PathBuf, String, bool)>,
    /// (host config, include line). Prepended once, at the top.
    pub includes: Vec<(PathBuf, String, Comment)>,
    /// (ini file, [(section, key, value)]) in a file another program writes.
    pub ini: Vec<(PathBuf, Vec<IniEdit>)>,
    /// Shell to make a running app re-read the above. Only run when something
    /// above changed, or under `--force`.
    pub reloads: Vec<String>,
    /// Shell run on every apply, changed or not. For what no file here records
    /// (gsettings, which compares for itself) and copies whose far side may
    /// have missed the last one (colors.send).
    pub always: Vec<String>,
    /// Things the user has to do or know, shown with the module.
    pub notes: Vec<String>,
    pub after: Vec<After>,
}

impl Plan {
    pub fn file(&mut self, path: PathBuf, content: String) {
        self.files.push((path, content));
    }
    pub fn include(&mut self, config: PathBuf, line: impl Into<String>, comment: Comment) {
        self.includes.push((config, line.into(), comment));
    }
    pub fn reload(&mut self, cmd: impl Into<String>) {
        self.reloads.push(cmd.into());
    }
    pub fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }
}

#[derive(Default, Clone, Copy)]
pub struct Options {
    /// Reload every app whether or not its files changed -- for one that was
    /// not running, or not listening, when they last did.
    pub force: bool,
    /// Work everything out, write nothing, run nothing.
    pub dry: bool,
}

/// One file an apply changed (or, under `--dry`, would change).
pub struct Change {
    pub path: PathBuf,
    /// None when the file did not exist.
    pub old: Option<String>,
    pub new: String,
}

pub struct Outcome {
    pub changed: Vec<Change>,
    /// Whether anything changed. `!changed.is_empty()`, except on the Windows
    /// paths that do their own IO and report true without file details.
    pub touched: bool,
    pub notes: Vec<String>,
}

/// File reads and writes for one apply. Writes land in `pending` too, so a later
/// step that reads the same path (an include going on top of a file a seed just
/// created) sees it -- which is what keeps `--dry` exact, since there the
/// pending map is the only place the write goes.
struct Disk {
    dry: bool,
    pending: HashMap<PathBuf, String>,
}

impl Disk {
    fn read(&self, path: &Path) -> Option<String> {
        self.pending
            .get(path)
            .cloned()
            .or_else(|| fs::read_to_string(path).ok())
    }

    fn exists(&self, path: &Path) -> bool {
        self.pending.contains_key(path) || path.exists()
    }

    /// Write `content` unless the file already holds exactly that. Rewriting an
    /// identical file still bumps its mtime, and file watchers (conky's
    /// self-reload, VS Code) treat that as a change.
    fn put(&mut self, path: &Path, content: &str, changes: &mut Vec<Change>) -> Result<()> {
        let old = self.read(path);
        if old.as_deref() == Some(content) {
            return Ok(());
        }
        if !self.dry {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)
                    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
            }
            fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
        }
        self.pending.insert(path.to_path_buf(), content.to_string());
        match changes.iter_mut().find(|c| c.path == path) {
            Some(c) => c.new = content.to_string(),
            None => changes.push(Change { path: path.to_path_buf(), old, new: content.to_string() }),
        }
        Ok(())
    }
}

/// `existing` with `line` added at the top, or None when it is already there.
///
/// Top, because every app whose include coat uses (kitty, zathura, GTK's CSS
/// `@import`) applies it where it appears, so anything the user writes below
/// wins. A missing config is created holding just the include; that is the one
/// case where coat authors the file, and it is still only a pointer.
fn with_include(existing: &str, line: &str, (open, close): Comment) -> Option<String> {
    if existing.lines().any(|l| l.trim() == line) {
        return None;
    }
    let mut out = format!(
        "{open} Added by coat — pulls in the generated colour/font fragment.{close}\n\
         {open} Everything below is yours: coat rewrites the fragment, never this file.{close}\n\
         {line}\n"
    );
    if !existing.is_empty() {
        out.push('\n');
        out.push_str(existing);
    }
    Some(out)
}

/// `original` with each (section, key, value) set. Every other line stays. A key
/// missing from its section goes after the section's last key, a missing section
/// at the end -- for qt6ct.conf, which qt6ct's GUI rewrites whole, so it can
/// neither take an include nor be owned by coat.
fn with_ini_keys(original: &str, edits: &[IniEdit]) -> String {
    let mut lines: Vec<String> = original.lines().map(str::to_string).collect();

    for (section, key, value) in edits {
        let header = format!("[{}]", section);
        let line = format!("{}={}", key, value);
        let Some(start) = lines.iter().position(|l| l.trim() == header) else {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(header);
            lines.push(line);
            continue;
        };
        let end = lines[start + 1..]
            .iter()
            .position(|l| l.trim_start().starts_with('['))
            .map_or(lines.len(), |i| start + 1 + i);
        let existing = (start + 1..end).find(|&i| {
            lines[i].split_once('=').is_some_and(|(k, _)| k.trim() == *key)
        });
        match existing {
            Some(i) => lines[i] = line,
            None => {
                let after = (start + 1..end)
                    .rev()
                    .find(|&i| lines[i].contains('='))
                    .unwrap_or(start);
                lines.insert(after + 1, line);
            }
        }
    }

    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Carry out one module's plan. Returns what to record in the manifest
/// alongside the result: on an error part-way through, the files written before
/// it still exist and `coat remove` should still be able to clean them up.
///
/// The reloads and `always` commands are handed back, not run, so the caller
/// can start every module's in one go.
fn execute(
    plan: Plan,
    prev: Option<&Entry>,
    opts: Options,
    commands: &mut Vec<String>,
) -> (Entry, Result<Outcome>) {
    let mut entry = Entry::default();
    let mut disk = Disk { dry: opts.dry, pending: HashMap::new() };
    let mut changes = Vec::new();

    let result = (|| -> Result<()> {
        for (path, content, owned) in &plan.seeds {
            // An owned seed stays coat's on the applies after the one that made
            // it, when it already exists.
            let was_ours = prev.is_some_and(|e| e.written.contains(path));
            if !disk.exists(path) {
                disk.put(path, content, &mut changes)?;
                if *owned {
                    entry.written.push(path.clone());
                }
            } else if *owned && was_ours {
                entry.written.push(path.clone());
            }
        }
        for (path, content) in &plan.files {
            disk.put(path, content, &mut changes)?;
            if !entry.written.contains(path) {
                entry.written.push(path.clone());
            }
        }
        for (path, content) in &plan.merged {
            disk.put(path, content, &mut changes)?;
        }
        for (path, edits) in &plan.ini {
            let original = disk.read(path).unwrap_or_default();
            disk.put(path, &with_ini_keys(&original, edits), &mut changes)?;
            let keys: Vec<String> = edits.iter().map(|(s, k, _)| format!("{}/{}", s, k)).collect();
            entry.ini_keys.push((path.clone(), keys));
        }
        for (path, line, comment) in &plan.includes {
            // Recorded whether or not it had to be added: the line is coat's
            // either way, and `coat remove` has to know to strip it.
            entry.includes.push((path.clone(), line.clone()));
            if let Some(text) = with_include(&disk.read(path).unwrap_or_default(), line, *comment) {
                disk.put(path, &text, &mut changes)?;
            }
        }
        Ok(())
    })();

    let changed = !changes.is_empty();
    let reloaded = changed || opts.force;
    let result = result.and_then(|()| {
        if opts.dry {
            return Ok(());
        }
        if reloaded {
            commands.extend(plan.reloads);
        }
        commands.extend(plan.always);
        plan.after.into_iter().try_for_each(|f| f(reloaded))
    });

    let outcome = result.map(|()| Outcome { changed: changes, touched: changed, notes: plan.notes });
    (entry, outcome)
}

/// Carry out a plan on its own, outside a full apply: no manifest, nothing
/// batched. For the Windows paths that render one of coat's themes to a place
/// of their own choosing.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn execute_now(plan: Plan) -> Result<()> {
    let mut commands = Vec::new();
    let (_, outcome) = execute(plan, None, Options::default(), &mut commands);
    spawn(&commands);
    outcome.map(|_| ())
}

/// Fire off every command in one backgrounded shell, each in a subshell of its
/// own so they run side by side, and DON'T wait. They are best-effort reload
/// hooks whose chatter is not ours to show, and some take hundreds of ms --
/// `apply` returns as soon as the files are written. One shell for the lot
/// rather than one per command: a full `coat set` used to fork fifteen.
pub fn spawn(commands: &[String]) {
    if commands.is_empty() {
        return;
    }
    let script = commands
        .iter()
        .map(|c| format!("( {} ) &", c))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nwait";
    let result = Command::new("sh")
        .args(["-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(e) = result {
        crate::detail!("  warning: {}", e);
    }
}

/// Apply `scheme` to every module in `names`.
///
/// Every module plans at once, on a thread each -- a plan does no writes, and
/// some read (VS Code's settings, qt6ct.conf). The plans are then carried out in
/// order, the manifest is written once, and every reload goes out in one spawn.
///
/// Returns each module's outcome in `names` order.
pub fn apply_all(
    names: &[String],
    scheme: &Scheme,
    config: &CoatConfig,
    opts: Options,
) -> Vec<(String, Result<Outcome>)> {
    let plans: Vec<Result<Plan>> = std::thread::scope(|s| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| s.spawn(move || crate::modules::plan(name, scheme, config)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err(anyhow::anyhow!("planning panicked"))))
            .collect()
    });

    let manifest = crate::manifest::load();
    let mut entries = Vec::new();
    let mut commands = Vec::new();
    let mut out = Vec::new();

    for (name, plan) in names.iter().zip(plans) {
        let canonical = crate::modules::find(name).map_or(name.as_str(), |m| m.name);

        // On Windows, VS Code and Discord live at native locations, handled by
        // windows.rs with its own IO. From here only success or failure shows.
        #[cfg(windows)]
        if !opts.dry && matches!(canonical, "vscode" | "vesktop") {
            let painted = config.merged_for(canonical).paint(scheme);
            let r = painted.and_then(|s| {
                let c = config.merged_for(canonical);
                if canonical == "vscode" {
                    crate::windows::apply_vscode(&s, &c).map(|_| ())
                } else {
                    crate::windows::apply_discord(&s, &c).map(|_| ())
                }
            });
            out.push((
                name.clone(),
                r.map(|()| Outcome { changed: Vec::new(), touched: true, notes: Vec::new() }),
            ));
            continue;
        }

        let plan = match plan {
            Ok(p) => p,
            Err(e) => {
                out.push((name.clone(), Err(e)));
                continue;
            }
        };
        let (entry, outcome) = execute(plan, manifest.modules.get(canonical), opts, &mut commands);
        entries.push((canonical.to_string(), entry));
        out.push((name.clone(), outcome));
    }

    if !opts.dry {
        crate::manifest::record_all(entries);
        spawn(&commands);
    }
    out
}

/// A line diff of `old` -> `new`: changed lines with `context` lines around
/// them, `-`/`+`/` ` prefixed, hunks separated by `…`. Longest common
/// subsequence, which is quadratic, but the files coat writes are a few hundred
/// lines at most.
pub fn diff(old: &str, new: &str, context: usize) -> Vec<String> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    // The full edit script first, then keep only what is near a change.
    let mut ops: Vec<(char, &str)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            ops.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] > lcs[i + 1][j]) {
            ops.push(('+', b[j]));
            j += 1;
        } else {
            ops.push(('-', a[i]));
            i += 1;
        }
    }

    let near = |k: usize| {
        let lo = k.saturating_sub(context);
        let hi = (k + context + 1).min(ops.len());
        ops[lo..hi].iter().any(|(op, _)| *op != ' ')
    };
    let mut out = Vec::new();
    let mut gap = false;
    for (k, (op, line)) in ops.iter().enumerate() {
        if near(k) {
            if gap && !out.is_empty() {
                out.push("…".to_string());
            }
            gap = false;
            out.push(format!("{}{}", op, line));
        } else {
            gap = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_goes_on_top_once() {
        let once = with_include("a=1\n", "include x", ("#", "")).unwrap();
        assert!(once.starts_with("# Added by coat"));
        assert!(once.ends_with("include x\n\na=1\n"));
        assert!(with_include(&once, "include x", ("#", "")).is_none());
    }

    #[test]
    fn ini_keys_merge_in_place() {
        let edits = vec![
            ("A".into(), "k".into(), "new".into()),
            ("A".into(), "added".into(), "1".into()),
            ("B".into(), "x".into(), "y".into()),
        ];
        let out = with_ini_keys("[A]\nk=old\nkeep=1\n\n[C]\nz=0\n", &edits);
        assert_eq!(out, "[A]\nk=new\nkeep=1\nadded=1\n\n[C]\nz=0\n\n[B]\nx=y\n");
    }

    #[test]
    fn dry_run_writes_nothing_but_reports_changes() {
        let dir = std::env::temp_dir().join(format!("coat-dry-{}", std::process::id()));
        let file = dir.join("theme.conf");
        let mut plan = Plan::default();
        plan.file(file.clone(), "x\n".into());
        plan.include(dir.join("main.conf"), "include theme.conf", ("#", ""));
        plan.reload("echo reload");
        let mut cmds = Vec::new();
        let (entry, out) = execute(plan, None, Options { dry: true, force: false }, &mut cmds);
        let out = out.unwrap();
        assert_eq!(out.changed.len(), 2);
        assert!(out.touched);
        assert!(cmds.is_empty(), "a dry run spawns nothing");
        assert!(!file.exists() && !dir.exists());
        assert_eq!(entry.written, vec![file]);
    }

    #[test]
    fn diff_shows_only_changes_with_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n";
        let new = "1\n2\n3\nFOUR\n5\n6\n7\n";
        assert_eq!(diff(old, new, 1), vec![" 3", "-4", "+FOUR", " 5"]);
    }
}
