#![recursion_limit = "512"]

mod config;
mod modules;
mod scheme;
#[cfg(windows)]
mod windows;

use anyhow::{Context, Result};
use config::CoatConfig;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use modules::{apply_module, make_tera, module_docs, module_aliases, ALL_MODULES};
use scheme::{
    find_scheme, list_schemes, pick_random_scheme, preview_scheme, schemes_clone, schemes_exists,
    schemes_update, Scheme,
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

/// Apply `scheme` to each named app, showing one spinner line per app that
/// freezes into a ✓/✗ result once that app is done. Returns the number
/// that applied successfully.
fn apply_with_spinner(apps: &[String], scheme: &Scheme, config: &CoatConfig, tera: &Tera) -> usize {
    modules::set_quiet(true);
    let mut applied = 0;
    for app in apps {
        let pb = spinner(&format!("Applying to {}...", app));
        let result = apply_module(app, scheme, config, tera);
        pb.finish_and_clear();
        match result {
            Ok(_) => {
                println!("{} {}", style("✓").green(), app);
                applied += 1;
            }
            Err(e) => {
                println!("{} {}  {}", style("✗").red(), app, style(e.to_string()).red());
            }
        }
    }
    modules::set_quiet(false);
    applied
}

fn print_usage(prog: &str) {
    println!("coat - color scheme configurator\n");
    println!("Usage: {} [COMMAND] [OPTIONS]\n", prog);
    println!("Commands:");
    println!("  clone               Clone the schemes repository");
    println!("  update              Update the schemes repository");
    println!("  list [OPTIONS]      List available color schemes");
    println!("  search <term>       Search schemes by name or author");
    println!("  set <scheme>        Switch to a scheme and apply everywhere");
    println!("  random [OPTIONS]    Switch to a random scheme and apply everywhere");
    println!("  apply [app]         Apply current scheme to all apps or one specific app");
    println!("  docs <app>          Show setup instructions for an app");
    println!("  help                Show this help message");
    println!();
    println!("List/Search/Random Options:");
    println!("  --dark              Show/pick only dark variant schemes");
    println!("  --light             Show/pick only light variant schemes");
    println!("  --no-preview        Skip color swatches (list/search only)");
    println!("  --dry               Preview a random pick without applying (random only)");
    println!();
    println!("Examples:");
    println!("  {} set gruvbox       Switch to gruvbox and apply everywhere", prog);
    println!("  {} random            Switch to a random scheme and apply everywhere", prog);
    println!("  {} random --dark     Switch to a random dark scheme", prog);
    println!("  {} random --dry      Preview a random scheme without applying", prog);
    println!("  {} apply             Apply current scheme to all enabled apps", prog);
    println!("  {} apply kitty       Apply current scheme only to kitty", prog);
    println!("  {} list --dark       List dark schemes", prog);
    println!("  {} search gruvbox    Search for gruvbox schemes", prog);
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
    println!("Applying scheme: {}\n", scheme.name);

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

/// Rewrite the `scheme:` line in coat.yaml in-place, preserving everything else.
/// Creates a minimal coat.yaml if the file doesn't exist yet.
fn update_scheme_in_config(name: &str) -> Result<()> {
    let path = CoatConfig::path()?;

    if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

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

/// Persist `scheme` as the current scheme and apply it everywhere.
/// Shared by `set` and `random`.
fn set_and_apply(scheme: &Scheme) -> Result<()> {
    // Persist the new scheme name into coat.yaml
    update_scheme_in_config(&scheme.slug)?;
    println!();

    let config = CoatConfig::load().context("Failed to load coat.yaml")?;

    // Platform-specific theming
    #[cfg(windows)]
    {
        windows::apply_all(scheme, &config)?;
    }

    #[cfg(not(windows))]
    {
        if config.enabled.is_empty() {
            println!("No modules enabled. Add apps to coat.yaml and run 'coat apply'.");
        } else {
            let tera = make_tera().context("Failed to build template engine")?;
            let total = config.enabled.len();
            let applied = apply_with_spinner(&config.enabled, scheme, &config, &tera);
            println!();
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

fn cmd_set(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: set requires a scheme name\n");
        eprintln!("Usage: coat set <scheme>");
        std::process::exit(1);
    }
    let name = &args[0];

    ensure_schemes()?;

    // Validate the scheme exists before touching anything
    let prefer_base24 = CoatConfig::load().map(|c| c.prefer_base24).unwrap_or(false);
    let scheme = find_scheme(name, prefer_base24)
        .with_context(|| format!("Scheme '{}' not found", name))?;

    println!("Setting scheme: {}\n", scheme.name);

    set_and_apply(&scheme)
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

    ensure_schemes()?;

    let prefer_base24 = CoatConfig::load().map(|c| c.prefer_base24).unwrap_or(false);
    let scheme = pick_random_scheme(variant_filter, prefer_base24)
        .context("Failed to pick a random scheme")?;

    if dry {
        println!("🎲 Random preview (not applied — run 'coat random' to apply):\n");
        preview_scheme(&scheme);
        return Ok(());
    }

    println!("🎲 Randomly selected: {} by {}", scheme.name, scheme.author);
    if !scheme.variant.is_empty() {
        println!("   Variant: {}", scheme.variant);
    }
    println!();

    set_and_apply(&scheme)
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
            "random"          => cmd_random(&args[2..]),
            "apply"           => cmd_apply(&args[2..], prog),
            "docs"            => cmd_docs(&args[2..], prog),
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
