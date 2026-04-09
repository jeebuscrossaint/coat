use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct RawScheme {
    name: String,
    #[serde(default)]
    slug: String,
    author: String,
    #[serde(default)]
    variant: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    palette: HashMap<String, String>,
}

/// A loaded Base16/Base24 color scheme.
/// All color fields are uppercase 6-char hex WITHOUT '#'.
#[derive(Debug, Clone)]
pub struct Scheme {
    pub name: String,
    pub slug: String,
    pub author: String,
    pub variant: String,
    #[allow(dead_code)]
    pub description: String,
    pub is_base24: bool,
    pub base00: String,
    pub base01: String,
    pub base02: String,
    pub base03: String,
    pub base04: String,
    pub base05: String,
    pub base06: String,
    pub base07: String,
    pub base08: String,
    pub base09: String,
    pub base0a: String,
    pub base0b: String,
    pub base0c: String,
    pub base0d: String,
    pub base0e: String,
    pub base0f: String,
    pub base10: String,
    pub base11: String,
    pub base12: String,
    pub base13: String,
    pub base14: String,
    pub base15: String,
    pub base16_color: String,
    pub base17: String,
}

impl Scheme {
    fn norm(s: &str) -> String {
        s.trim_start_matches('#').to_uppercase()
    }

    fn derive_slug(name: &str) -> String {
        name.to_lowercase()
            .replace(' ', "-")
            .replace('_', "-")
    }

    fn from_raw(raw: RawScheme) -> Self {
        let get = |key: &str| -> String {
            raw.palette.get(key).map(|s| Self::norm(s)).unwrap_or_default()
        };
        let base10 = get("base10");
        let is_base24 = !base10.is_empty();
        let slug = if raw.slug.is_empty() {
            Self::derive_slug(&raw.name)
        } else {
            raw.slug
        };
        Scheme {
            name: raw.name,
            slug,
            author: raw.author,
            variant: raw.variant,
            description: raw.description,
            is_base24,
            base00: get("base00"),
            base01: get("base01"),
            base02: get("base02"),
            base03: get("base03"),
            base04: get("base04"),
            base05: get("base05"),
            base06: get("base06"),
            base07: get("base07"),
            base08: get("base08"),
            base09: get("base09"),
            base0a: get("base0A"),
            base0b: get("base0B"),
            base0c: get("base0C"),
            base0d: get("base0D"),
            base0e: get("base0E"),
            base0f: get("base0F"),
            base10,
            base11: get("base11"),
            base12: get("base12"),
            base13: get("base13"),
            base14: get("base14"),
            base15: get("base15"),
            base16_color: get("base16"),
            base17: get("base17"),
        }
    }

    pub fn load_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let raw: RawScheme = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Self::from_raw(raw))
    }

    pub fn is_dark(&self) -> bool {
        !self.variant.to_lowercase().contains("light")
    }

    pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
        let h = hex.trim_start_matches('#');
        let n = u32::from_str_radix(h, 16).unwrap_or(0);
        (((n >> 16) & 0xFF) as u8, ((n >> 8) & 0xFF) as u8, (n & 0xFF) as u8)
    }
}

pub fn schemes_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".config/coat/schemes"))
}

pub fn schemes_exists() -> bool {
    schemes_dir().map(|d| d.join(".git").is_dir()).unwrap_or(false)
}

pub fn schemes_clone() -> Result<()> {
    println!("Cloning tinted-theming/schemes repository...");
    println!("This may take a moment...\n");
    let dir = schemes_dir()?;
    let url = "https://github.com/tinted-theming/schemes.git";
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&dir)
        .status()
        .context("Failed to run git")?;
    if !status.success() {
        bail!("git clone failed");
    }
    println!("Successfully cloned schemes repository!");
    Ok(())
}

pub fn schemes_update() -> Result<()> {
    if !schemes_exists() {
        return schemes_clone();
    }
    println!("Updating schemes repository...");
    let dir = schemes_dir()?;
    let status = std::process::Command::new("git")
        .args(["-C"])
        .arg(&dir)
        .arg("pull")
        .status()
        .context("Failed to run git")?;
    if !status.success() {
        bail!("git pull failed");
    }
    println!("Successfully updated schemes repository!");
    Ok(())
}

pub fn find_scheme(name: &str, prefer_base24: bool) -> Result<Scheme> {
    let sdir = schemes_dir()?;

    let dirs_to_try: Vec<PathBuf> = if prefer_base24 {
        vec![sdir.join("base24"), sdir.join("base16")]
    } else {
        vec![sdir.join("base16"), sdir.join("base24")]
    };

    // First pass: match by filename stem
    for dir in &dirs_to_try {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.eq_ignore_ascii_case(name) {
                return Scheme::load_file(path);
            }
        }
    }

    // Second pass: match by scheme name or slug field
    for dir in &dirs_to_try {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            if let Ok(scheme) = Scheme::load_file(path) {
                if scheme.name.eq_ignore_ascii_case(name) || scheme.slug.eq_ignore_ascii_case(name) {
                    return Ok(scheme);
                }
            }
        }
    }

    bail!("Scheme '{}' not found in {}", name, sdir.display())
}

pub fn list_schemes(
    variant_filter: Option<&str>,
    search_term: Option<&str>,
    show_preview: bool,
) -> Result<()> {
    let sdir = schemes_dir()?;
    let dirs_to_scan = [sdir.join("base16"), sdir.join("base24")];

    let mut schemes: Vec<Scheme> = Vec::new();

    for dir in &dirs_to_scan {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let Ok(scheme) = Scheme::load_file(path) else {
                continue;
            };
            if let Some(vf) = variant_filter {
                if !scheme.variant.to_lowercase().contains(vf) {
                    continue;
                }
            }
            if let Some(term) = search_term {
                let t = term.to_lowercase();
                if !scheme.name.to_lowercase().contains(&t)
                    && !scheme.author.to_lowercase().contains(&t)
                    && !scheme.slug.to_lowercase().contains(&t)
                {
                    continue;
                }
            }
            schemes.push(scheme);
        }
    }

    schemes.sort_by(|a, b| a.name.cmp(&b.name));

    if schemes.is_empty() {
        println!("No schemes found matching criteria.");
        return Ok(());
    }

    println!(
        "Found {} scheme{}:\n",
        schemes.len(),
        if schemes.len() == 1 { "" } else { "s" }
    );

    for scheme in &schemes {
        print!("\x1b[1m{:<30}\x1b[0m", scheme.name);
        if !scheme.variant.is_empty() {
            print!(" [{}]", scheme.variant);
        }
        if scheme.is_base24 {
            print!(" [Base24]");
        }
        println!();
        if !scheme.author.is_empty() {
            println!("  Author: {}", scheme.author);
        }
        if !scheme.slug.is_empty() {
            println!("  Slug:   {}", scheme.slug);
        }
        if show_preview {
            print_color_preview(scheme);
        }
        println!();
    }

    Ok(())
}

fn print_color_block(hex: &str) {
    let (r, g, b) = Scheme::hex_to_rgb(hex);
    print!("\x1b[48;2;{};{};{}m  \x1b[0m {:<7} ", r, g, b, hex);
}

fn print_color_preview(scheme: &Scheme) {
    println!("\nColor Preview:");
    println!("─────────────────────────────────────────────────────");
    println!("Backgrounds & Foregrounds:");
    print_color_block(&scheme.base00);
    print_color_block(&scheme.base01);
    print_color_block(&scheme.base02);
    print_color_block(&scheme.base03);
    println!();
    print_color_block(&scheme.base04);
    print_color_block(&scheme.base05);
    print_color_block(&scheme.base06);
    print_color_block(&scheme.base07);
    println!("\n");
    println!("Accent Colors:");
    print_color_block(&scheme.base08);
    print_color_block(&scheme.base09);
    print_color_block(&scheme.base0a);
    print_color_block(&scheme.base0b);
    println!();
    print_color_block(&scheme.base0c);
    print_color_block(&scheme.base0d);
    print_color_block(&scheme.base0e);
    print_color_block(&scheme.base0f);
    println!();
    if scheme.is_base24 && !scheme.base10.is_empty() {
        println!("\nBase24 Extended Colors:");
        print_color_block(&scheme.base10);
        print_color_block(&scheme.base11);
        print_color_block(&scheme.base12);
        print_color_block(&scheme.base13);
        println!();
        print_color_block(&scheme.base14);
        print_color_block(&scheme.base15);
        print_color_block(&scheme.base16_color);
        print_color_block(&scheme.base17);
        println!();
    }
}
