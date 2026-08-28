#![recursion_limit = "512"]

mod browse;
mod config;
mod dynamic;
mod manifest;
mod modules;
mod normalize;
mod scheme;
#[cfg(windows)]
mod windows;

use anyhow::{Context, Result};
use config::CoatConfig;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use modules::{apply_module, make_tera, module_docs, module_aliases, ALL_MODULES};
use scheme::{
    find_scheme, list_schemes, pick_random_scheme, print_color_preview, schemes_clone,
    schemes_exists, schemes_update, Scheme,
};
use std::fs;
use std::time::Duration;
use tera::Tera;

/// A spinner that ticks in place, then freezes into a static ✓/✗ result line.
fn spinner(label: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb.set_message(label.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Apply `scheme` to every named app under a single spinner that names the app
/// currently being written, then clears itself. Only failures are printed —
/// successes are left to the caller's one-line summary, so a 20-app apply stays
/// two lines instead of twenty. Returns the number that applied successfully.
fn apply_with_spinner(apps: &[String], scheme: &Scheme, config: &CoatConfig, tera: &Tera) -> usize {
    modules::set_quiet(true);
    let pb = spinner("Applying...");
    let mut applied = 0;
    let mut failures: Vec<(String, String)> = Vec::new();
    for app in apps {
        pb.set_message(format!("Applying to {}...", app));
        match apply_module(app, scheme, config, tera) {
            Ok(_) => applied += 1,
            Err(e) => failures.push((app.clone(), e.to_string())),
        }
    }
    pb.finish_and_clear();
    modules::set_quiet(false);
    for (app, err) in &failures {
        println!("{} {}  {}", style("✗").red(), app, style(err).red());
    }
    applied
}

fn print_usage(prog: &str) {
    println!("coat - color scheme configurator\n");
    println!("Usage: {} [COMMAND] [OPTIONS]\n", prog);
    println!("Commands:");
    println!("  clone               Clone the schemes repository");
    println!("  update              Update the schemes repository");
    println!("  list [OPTIONS]      List available color schemes");
    println!("  browse [OPTIONS]    Scroll through schemes interactively, apply on Enter");
    println!("  search <term>       Search schemes by name or author");
    println!("  set <scheme>        Switch to a scheme and apply everywhere");
    println!("  random [OPTIONS]    Pick a random scheme, preview it, and apply");
    println!("  match [image]       Build a scheme from the current wallpaper, and apply");
    println!("  apply [app]         Apply current scheme to all apps or one specific app");
    println!("  remove <app>        Undo an app's theming and disable its module");
    println!("  docs <app>          Show setup instructions for an app");
    println!("  completions <shell> Install shell completions (fish, powershell)");
    println!("  help                Show this help message");
    println!();
    println!("List/Search/Browse/Random Options:");
    println!("  --dark              Show/pick only dark variant schemes");
    println!("  --light             Show/pick only light variant schemes");
    println!("  --no-preview        Skip color swatches (list/search only)");
    println!("  --dry               Preview a random pick without applying (random only)");
    println!("  --yes, -y           Apply a random pick without prompting (random only)");
    if cfg!(windows) {
        println!();
        println!("Set/Browse/Random Options (Windows):");
        println!("  --elevate           Prompt for admin to theme the logon screen");
    }
    println!();
    println!("Examples:");
    println!("  {} set gruvbox       Switch to gruvbox and apply everywhere", prog);
    println!("  {} browse            Scroll schemes interactively and apply one", prog);
    println!("  {} browse --dark     Browse only dark schemes", prog);
    println!("  {} random            Pick a random scheme, preview, then confirm", prog);
    println!("  {} random --dark     Pick a random dark scheme", prog);
    println!("  {} random -y         Pick and apply a random scheme, no prompt", prog);
    println!("  {} random --dry      Preview a random scheme without applying", prog);
    println!("  {} apply             Apply current scheme to all enabled apps", prog);
    println!("  {} apply foot        Apply current scheme only to foot", prog);
    println!("  {} list --dark       List dark schemes", prog);
}

fn ensure_schemes() -> Result<()> {
    if !schemes_exists() {
        println!("Schemes repository not found. Cloning...");
        schemes_clone()?;
        println!();
    }
    Ok(())
}

fn cmd_apply(args: &[String], _prog: &str) -> Result<()> {
    let specific_app = args.first().map(|s| s.as_str());

    // Validate module name up front
    if let Some(app) = specific_app {
        let canonical = module_aliases(app).unwrap_or(app);
        if !ALL_MODULES.contains(&canonical) {
            eprintln!("Error: No module found for '{}'\n", app);
            eprintln!("Available modules:");
            for m in ALL_MODULES {
                eprintln!("  - {}", m);
            }
            std::process::exit(1);
        }
    }

    ensure_schemes()?;

    let config = CoatConfig::load().context("Failed to load coat.yaml")?;

    println!("Loading scheme: {}", config.scheme);
    let scheme = find_scheme(&config.scheme, config.prefer_base24)
        .with_context(|| format!("Failed to load scheme '{}'", config.scheme))?;
    println!("Applying scheme: {}", scheme.name);

    let tera = make_tera().context("Failed to build template engine")?;

    let apps: Vec<String> = match specific_app {
        Some(app) => vec![app.to_string()],
        None => {
            if config.enabled.is_empty() {
                println!("No modules enabled in coat.yaml.");
                return Ok(());
            }
            config.enabled.clone()
        }
    };

    let total = apps.len();
    let applied = apply_with_spinner(&apps, &scheme, &config, &tera);

    println!();
    println!(
        "{} scheme to {}/{} application{}!",
        if applied == total { "Applied" } else { "Partially applied" },
        applied,
        total,
        if total == 1 { "" } else { "s" }
    );

    Ok(())
}

/// Whether coat.yaml's enabled list mentions this app.
fn module_is_enabled(app: &str) -> Result<bool> {
    let canonical = module_aliases(app).unwrap_or(app);
    Ok(CoatConfig::load()
        .map(|c| c.enabled.iter().any(|m| m == canonical || m == app))
        .unwrap_or(false))
}

/// Drop `  - <app>` from the enabled list in coat.yaml, leaving every comment
/// and every other key exactly as it was. Returns whether a line was removed.
fn disable_module_in_config(app: &str) -> Result<bool> {
    let path = CoatConfig::path()?;
    if !path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let canonical = modules::module_aliases(app).unwrap_or(app);
    let mut removed = false;
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| {
            let t = line.trim();
            let Some(item) = t.strip_prefix("- ") else {
                return true;
            };
            // Only inside a list, and only an exact name — `- fish` must never
            // take out `- fishy` or a `scheme: fish` line.
            let item = item.trim();
            if item == canonical || item == app {
                removed = true;
                return false;
            }
            true
        })
        .collect();

    if removed {
        let mut out = kept.join("\n");
        if content.ends_with('\n') {
            out.push('\n');
        }
        fs::write(&path, out)
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    Ok(removed)
}

/// Rewrite the `scheme:` line in coat.yaml in-place, preserving everything else.
/// Creates a minimal coat.yaml if the file doesn't exist yet.
fn update_scheme_in_config(name: &str) -> Result<()> {
    let path = CoatConfig::path()?;

    if path.exists() {
        let content = config::sanitize(
            &fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?,
        );

        let mut found = false;
        let mut new_lines: Vec<String> = content
            .lines()
            .map(|line| {
                if !found && line.trim_start().starts_with("scheme:") {
                    found = true;
                    format!("scheme: {}", name)
                } else {
                    line.to_string()
                }
            })
            .collect();
        if !found {
            new_lines.insert(0, format!("scheme: {}", name));
        }

        let mut out = new_lines.join("\n");
        if content.ends_with('\n') {
            out.push('\n');
        }
        fs::write(&path, out)
            .with_context(|| format!("Failed to write {}", path.display()))?;
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        fs::write(&path, format!("scheme: {}\nenabled: []\n", name))
            .with_context(|| format!("Failed to create {}", path.display()))?;
        println!("  Created {}", path.display());
    }

    println!("  coat.yaml → scheme: {}", name);
    Ok(())
}

/// Did the user pass `--elevate`? Windows-only; ignored elsewhere.
fn wants_elevation(args: &[String]) -> bool {
    args.iter().any(|a| a == "--elevate")
}

/// Persist `scheme` as the current scheme and apply it everywhere.
/// Shared by `set`, `random`, and `browse`.
///
/// `elevate` only means anything on Windows, where it authorizes a UAC prompt
/// for the logon-screen and HKLM registry keys.
fn set_and_apply(scheme: &Scheme, elevate: bool) -> Result<()> {
    // Load *before* the rewrite. Only fonts, opacity and the enabled list are
    // read from here — the scheme itself arrives as an argument — so this is
    // the same config we'd get afterwards, and a coat.yaml that doesn't parse
    // now stops us before the file has been touched. Loading afterwards meant
    // a bad config left coat.yaml naming a scheme that was never applied.
    let config = CoatConfig::load().context("Failed to load coat.yaml")?;

    // Persist the new scheme name into coat.yaml
    update_scheme_in_config(&scheme.slug)?;
    println!();

    // Platform-specific theming
    #[cfg(windows)]
    {
        windows::apply_all(scheme, &config, elevate)?;
    }

    #[cfg(not(windows))]
    {
        let _ = elevate; // Windows-only flag
        if config.enabled.is_empty() {
            println!("No modules enabled. Add apps to coat.yaml and run 'coat apply'.");
        } else {
            let tera = make_tera().context("Failed to build template engine")?;
            let total = config.enabled.len();
            let applied = apply_with_spinner(&config.enabled, scheme, &config, &tera);
            println!(
                "Applied scheme to {}/{} application{}!",
                applied,
                total,
                if total == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}

/// `coat remove <app> [--dry] [--keep-enabled]`
///
/// The inverse of `coat apply <app>`, driven off the manifest the apply wrote.
fn cmd_remove(args: &[String]) -> Result<()> {
    let Some(app) = args.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("Error: remove requires an app name\n");
        eprintln!("Usage: coat remove <app> [--dry] [--keep-enabled]");
        eprintln!("\nApplied modules coat can remove:");
        let m = manifest::load();
        if m.modules.is_empty() {
            eprintln!("  (none recorded yet — they are recorded on apply)");
        } else {
            for name in m.modules.keys() {
                eprintln!("  {}", name);
            }
        }
        std::process::exit(1);
    };
    let dry = args.iter().any(|a| a == "--dry" || a == "--dry-run");
    let keep_enabled = args.iter().any(|a| a == "--keep-enabled");

    let done = modules::remove_module(app, dry)?;

    if done.is_empty() {
        println!("Nothing left to undo for {}.", app);
    } else {
        println!("{} {}:\n", if dry { "Would clean up" } else { "Cleaned up" }, app);
        for line in &done {
            println!("  {}", line);
        }
        println!();
    }

    if keep_enabled {
        println!("Left enabled in coat.yaml — the next apply will write it again.");
    } else if dry {
        if module_is_enabled(app)? {
            println!("Would also drop '{}' from the enabled list in coat.yaml.", app);
        }
    } else if disable_module_in_config(app)? {
        println!("Disabled '{}' in coat.yaml.", app);
    }
    Ok(())
}

/// `coat match [image] [--light|--dark] [--dry]`
fn cmd_match(args: &[String]) -> Result<()> {
    use dynamic::Polarity;

    let polarity = if args.iter().any(|a| a == "--light") {
        Some(Polarity::Light)
    } else if args.iter().any(|a| a == "--dark") {
        Some(Polarity::Dark)
    } else {
        None
    };
    let dry = args.iter().any(|a| a == "--dry" || a == "--dry-run");

    let image = match args.iter().find(|a| !a.starts_with('-')) {
        Some(p) => std::path::PathBuf::from(p),
        None => dynamic::current_wallpaper()?,
    };
    if !image.is_file() {
        eprintln!("No such image: {}", image.display());
        std::process::exit(1);
    }

    println!("Sampling: {}\n", image.display());
    let (scheme, path) = dynamic::scheme_from_image(&image, polarity)?;

    println!("{}", scheme::header_string(&scheme));
    scheme::print_color_preview(&scheme);
    println!();

    if dry {
        println!("Written to {} — not applied.", path.display());
        println!("Apply it later with: coat set {}", scheme.slug);
        return Ok(());
    }

    set_and_apply(&scheme, wants_elevation(args))
}

fn cmd_set(args: &[String]) -> Result<()> {
    // First non-flag argument, so `coat set --elevate gruvbox` works too.
    let Some(name) = args.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("Error: set requires a scheme name\n");
        eprintln!("Usage: coat set <scheme> [--elevate]");
        std::process::exit(1);
    };

    ensure_schemes()?;

    // Validate the scheme exists before touching anything
    let prefer_base24 = CoatConfig::load().map(|c| c.prefer_base24).unwrap_or(false);
    let scheme = match find_scheme(name, prefer_base24) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Scheme '{}' not found.", name);
            let suggestions = scheme::suggest_schemes(name, 5);
            if !suggestions.is_empty() {
                eprintln!("\nDid you mean:");
                for s in &suggestions {
                    eprintln!("  {}", s);
                }
            } else {
                eprintln!("\nRun 'coat browse' or 'coat list' to see available schemes.");
            }
            std::process::exit(1);
        }
    };

    println!("Setting scheme: {}\n", scheme.name);

    set_and_apply(&scheme, wants_elevation(args))
}

/// What the user chose at the `random` apply prompt.
enum RandomChoice {
    Apply,
    Reroll,
    Quit,
}

/// Prompt after previewing a random pick. Single keypress, no Enter needed.
/// Falls back to Apply when stdin isn't a terminal (e.g. piped/scripted).
fn prompt_random() -> Result<RandomChoice> {
    use console::{Key, Term};
    let term = Term::stdout();
    if !term.is_term() {
        return Ok(RandomChoice::Apply);
    }
    print!("Apply? [Y]es · [r]eroll · [n]o: ");
    std::io::Write::flush(&mut std::io::stdout())?;
    loop {
        let c = match term.read_key()? {
            Key::Enter => 'y',
            Key::Escape => 'n',
            Key::Char(c) => c.to_ascii_lowercase(),
            _ => continue,
        };
        match c {
            'y' => { println!("yes\n"); return Ok(RandomChoice::Apply); }
            'r' => { println!("reroll\n"); return Ok(RandomChoice::Reroll); }
            'n' | 'q' => { println!("no"); return Ok(RandomChoice::Quit); }
            _ => continue,
        }
    }
}

fn cmd_random(args: &[String]) -> Result<()> {
    // Optional variant filter, mirroring list/search flags.
    let variant_filter = args.iter().find_map(|a| match a.as_str() {
        "--dark"  => Some("dark"),
        "--light" => Some("light"),
        _         => None,
    });
    // --dry previews the pick without persisting or applying it.
    let dry = args.iter().any(|a| a == "--dry" || a == "--dry-run");
    // --yes / -y skips the confirmation prompt and applies immediately.
    let auto = args.iter().any(|a| a == "--yes" || a == "-y");
    let elevate = wants_elevation(args);

    ensure_schemes()?;

    let prefer_base24 = CoatConfig::load().map(|c| c.prefer_base24).unwrap_or(false);

    loop {
        let scheme = pick_random_scheme(variant_filter, prefer_base24)
            .context("Failed to pick a random scheme")?;

        println!("🎲 Randomly selected: {} by {}", scheme.name, scheme.author);
        if !scheme.variant.is_empty() {
            println!("   Variant: {}", scheme.variant);
        }
        print_color_preview(&scheme);
        println!();

        if dry {
            println!("(preview only — not applied)");
            return Ok(());
        }
        if auto {
            return set_and_apply(&scheme, elevate);
        }
        match prompt_random()? {
            RandomChoice::Apply => return set_and_apply(&scheme, elevate),
            RandomChoice::Reroll => continue,
            RandomChoice::Quit => { println!("Not applied."); return Ok(()); }
        }
    }
}

fn cmd_browse(args: &[String]) -> Result<()> {
    let variant_filter = args.iter().find_map(|a| match a.as_str() {
        "--dark"  => Some("dark"),
        "--light" => Some("light"),
        _         => None,
    });

    ensure_schemes()?;

    match browse::browse(variant_filter)? {
        Some(scheme) => {
            println!("Setting scheme: {}\n", scheme.name);
            set_and_apply(&scheme, wants_elevation(args))
        }
        None => {
            println!("No scheme selected.");
            Ok(())
        }
    }
}

fn cmd_list(args: &[String]) -> Result<()> {
    ensure_schemes()?;
    let mut variant_filter = None;
    let mut show_preview = true;
    for arg in args {
        match arg.as_str() {
            "--dark"       => variant_filter = Some("dark"),
            "--light"      => variant_filter = Some("light"),
            "--no-preview" => show_preview = false,
            _              => {}
        }
    }
    list_schemes(variant_filter, None, show_preview)
}

fn cmd_search(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: search requires a search term\n");
        eprintln!("Usage: coat search <term>");
        std::process::exit(1);
    }
    ensure_schemes()?;
    let term = &args[0];
    let mut variant_filter = None;
    let mut show_preview = true;
    for arg in &args[1..] {
        match arg.as_str() {
            "--dark"       => variant_filter = Some("dark"),
            "--light"      => variant_filter = Some("light"),
            "--no-preview" => show_preview = false,
            _              => {}
        }
    }
    list_schemes(variant_filter, Some(term), show_preview)
}

/// The completion script, kept as a static file so packagers can install it
/// too; embedded here so `coat completions` works from a bare `cargo install`.
const FISH_COMPLETIONS: &str = include_str!("../completions/coat.fish");
const PS_COMPLETIONS: &str = include_str!("../completions/coat.ps1");

/// Ask PowerShell where `$PROFILE` lives. Worth spawning a process for:
/// the path differs between Windows PowerShell 5.1 (`Documents\WindowsPowerShell`)
/// and PowerShell 7 (`Documents\PowerShell`), and Documents itself is often
/// redirected into OneDrive — guessing gets it wrong on most real machines.
/// Prefers `pwsh` and falls back to the in-box `powershell`.
fn powershell_profile_path() -> Result<std::path::PathBuf> {
    for exe in ["pwsh", "powershell"] {
        let out = std::process::Command::new(exe)
            .args(["-NoProfile", "-NonInteractive", "-Command", "$PROFILE"])
            .output();
        if let Ok(out) = out {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(std::path::PathBuf::from(path));
                }
            }
        }
    }
    anyhow::bail!("Could not locate $PROFILE — is PowerShell on PATH?")
}

/// Write the completion script beside `$PROFILE` and dot-source it from there.
/// PowerShell has no autoloaded completions directory, so the profile line is
/// what actually makes them live. Both steps are idempotent.
fn install_powershell_completions() -> Result<()> {
    let profile = powershell_profile_path()?;
    let dir = profile
        .parent()
        .context("$PROFILE has no parent directory")?
        .to_path_buf();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create {}", dir.display()))?;

    let script = dir.join("coat.completion.ps1");
    fs::write(&script, PS_COMPLETIONS)
        .with_context(|| format!("Failed to write {}", script.display()))?;
    println!("Installed PowerShell completions → {}", script.display());

    // Match on the filename, not the full line, so a profile edited by hand
    // (different quoting, absolute vs. $PSScriptRoot) isn't duplicated.
    let existing = fs::read_to_string(&profile).unwrap_or_default();
    if existing.contains("coat.completion.ps1") {
        println!("$PROFILE already sources it — nothing to add.");
    } else {
        let mut updated = existing;
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&format!(
            "\n# coat completions\n. '{}'\n",
            script.display()
        ));
        fs::write(&profile, updated)
            .with_context(|| format!("Failed to write {}", profile.display()))?;
        println!("Added a source line to {}", profile.display());
    }

    println!("Start a new shell (or run '. $PROFILE') to pick them up.");
    Ok(())
}

fn cmd_completions(args: &[String]) -> Result<()> {
    // `--print`/`--stdout` emits the script instead of installing it, for
    // packagers or anyone routing it elsewhere.
    let print_only = args.iter().any(|a| a == "--print" || a == "--stdout");
    let shell = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(|s| s.as_str());

    match shell {
        Some("fish") => {
            if print_only {
                print!("{}", FISH_COMPLETIONS);
                return Ok(());
            }
            let dir = dirs::config_dir()
                .context("Cannot determine config directory")?
                .join("fish/completions");
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create {}", dir.display()))?;
            let path = dir.join("coat.fish");
            fs::write(&path, FISH_COMPLETIONS)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            println!("Installed fish completions → {}", path.display());
            println!("Start a new shell (or run 'exec fish') to pick them up.");
            Ok(())
        }
        Some("powershell") | Some("pwsh") => {
            if print_only {
                print!("{}", PS_COMPLETIONS);
                return Ok(());
            }
            install_powershell_completions()
        }
        Some(other) => {
            eprintln!("Unsupported shell: {}\n", other);
            eprintln!("Supported shells: fish, powershell");
            std::process::exit(1);
        }
        None => {
            eprintln!("Usage: coat completions <shell> [--print]\n");
            eprintln!("Supported shells: fish, powershell");
            std::process::exit(1);
        }
    }
}

/// Hidden helper used by shell completions to enumerate dynamic candidates.
/// Deliberately silent and side-effect free — never clones the library.
fn cmd_complete(args: &[String]) -> Result<()> {
    match args.first().map(|s| s.as_str()) {
        Some("schemes") => {
            if schemes_exists() {
                for slug in scheme::all_slugs().unwrap_or_default() {
                    println!("{}", slug);
                }
            }
        }
        Some("modules") => {
            for m in ALL_MODULES {
                println!("{}", m);
            }
        }
        _ => {}
    }
    Ok(())
}

fn cmd_docs(args: &[String], prog: &str) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: docs requires an app name\n");
        eprintln!("Usage: {} docs <app>", prog);
        eprintln!("\nAvailable modules:");
        for m in ALL_MODULES {
            eprintln!("  - {}", m);
        }
        std::process::exit(1);
    }
    let app = &args[0];
    let canonical = module_aliases(app).unwrap_or(app);
    if !ALL_MODULES.contains(&canonical) {
        eprintln!("Error: No module found for '{}'\n", app);
        eprintln!("Available modules:");
        for m in ALL_MODULES {
            eprintln!("  - {}", m);
        }
        std::process::exit(1);
    }
    module_docs(app);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = args[0].as_str();

    // Must run before any scheme is loaded — `Scheme::load_file` consults this.
    // A missing or unparseable config simply leaves normalization off.
    if let Ok(cfg) = CoatConfig::load() {
        normalize::init(cfg.normalize.clone());
    }

    let result = if args.len() < 2 {
        // No command: show config summary
        cmd_info()
    } else {
        match args[1].as_str() {
            "clone"           => schemes_clone(),
            "update"          => schemes_update(),
            "list"            => cmd_list(&args[2..]),
            "search"          => cmd_search(&args[2..]),
            "set"             => cmd_set(&args[2..]),
            // `dynamic` is an alias: it is what this reads as in conversation.
            "match" | "dynamic" => cmd_match(&args[2..]),
            "random"          => cmd_random(&args[2..]),
            "browse"          => cmd_browse(&args[2..]),
            "apply"           => cmd_apply(&args[2..], prog),
            "remove" | "rm"   => cmd_remove(&args[2..]),
            "docs"            => cmd_docs(&args[2..], prog),
            "completions"     => cmd_completions(&args[2..]),
            "__complete"      => cmd_complete(&args[2..]),
            // Hidden: the UAC-elevated child spawned by `--elevate`.
            #[cfg(windows)]
            "__winelevate"    => windows::cmd_elevated_keys(&args[2..]),
            "help" | "--help" => { print_usage(prog); Ok(()) }
            other             => {
                eprintln!("Unknown command: {}\n", other);
                print_usage(prog);
                std::process::exit(1);
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn cmd_info() -> Result<()> {
    println!("coat - color scheme configurator\n");

    let config = match CoatConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("No config found at ~/.config/coat/coat.yaml");
            println!("Error: {}\n", e);
            println!("Run 'coat help' to get started.");
            return Ok(());
        }
    };

    println!("Scheme: {}", config.scheme);
    println!("Enabled ({}):", config.enabled.len());
    for app in &config.enabled {
        println!("  - {}", app);
    }
    println!();
    if !config.font_monospace().is_empty() {
        println!("Font (monospace): {} {}pt", config.font_monospace(), config.font_size_terminal());
    }
    if !config.font_sansserif().is_empty() {
        println!("Font (sans-serif): {}", config.font_sansserif());
    }

    if schemes_exists() {
        println!("\nScheme library: installed");
        if !config.scheme.is_empty() {
            match find_scheme(&config.scheme, config.prefer_base24) {
                Ok(scheme) => {
                    println!("\nCurrent scheme: {} by {}", scheme.name, scheme.author);
                    if !scheme.variant.is_empty() {
                        println!("Variant: {}", scheme.variant);
                    }
                }
                Err(e) => println!("\nFailed to load scheme '{}': {}", config.scheme, e),
            }
        }
    } else {
        println!("\nScheme library: not installed — run 'coat clone'");
    }

    Ok(())
}
