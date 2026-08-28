//! `coat match` — derive a scheme from the wallpaper that is already on screen.
//!
//! Nothing here picks colours by taste. The image decides the HUES, the base16
//! slot contract decides where they land, and lightness/chroma are pinned to
//! per-polarity constants so a muddy photo cannot produce an illegible theme.
//! That split is deliberate: a generator that also honoured the image's own
//! lightness would hand you base05 at 20% L over a base00 at 15% and call it a
//! scheme.
//!
//! The output is written out as an ordinary scheme file and then read back
//! through `Scheme::load_file`, so a generated scheme goes through exactly the
//! same funnel — base24 fallbacks, normalization — as one from the schemes repo.
//! `coat set <slug>` works on it afterwards like any other.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::normalize::{oklch_to_hex, rgb_to_oklch, Oklch};
use crate::scheme::{schemes_dir, Scheme};

/// Longest edge of the image we actually cluster. 160px is ~25k samples, which
/// is far more than 12 clusters need and keeps a 4K wallpaper under ~80ms.
const SAMPLE_EDGE: u32 = 160;
const CLUSTERS: usize = 12;
const ITERATIONS: usize = 12;

/// Conventional base16 accent hues, as Oklch degrees. The image's own hues are
/// snapped to whichever of these they are nearest, so `base0B` holds something
/// green even when the wallpaper is mostly teal — the slots keep meaning what
/// they say they mean.
const ACCENTS: [(&str, f64); 8] = [
    ("base08", 25.0),  // red
    ("base09", 55.0),  // orange
    ("base0A", 95.0),  // yellow
    ("base0B", 145.0), // green
    ("base0C", 195.0), // cyan
    ("base0D", 255.0), // blue
    ("base0E", 320.0), // magenta
    ("base0F", 35.0),  // brown — orange's family, held darker
];

/// A cluster hue this far from a slot's target hue is not that colour, and the
/// slot takes its canonical hue instead of a bad match.
const HUE_TOLERANCE: f64 = 45.0;

/// Two accents closer than this are the same colour to the eye, and a scheme
/// where base0D and base0E are indistinguishable has lost two slots rather than
/// gained a matched one. A slot whose best hue collides falls back to its
/// canonical target. base0F is exempt: it is brown, ten degrees off red BY
/// CONVENTION, and separated by lightness rather than hue.
const MIN_ACCENT_SEP: f64 = 20.0;

/// Lightness ladder for base00..base07. Fixed, not sampled: this is the
/// contrast structure every consumer relies on.
const RAMP_DARK: [f64; 8] = [0.14, 0.21, 0.29, 0.45, 0.63, 0.80, 0.88, 0.95];
const RAMP_LIGHT: [f64; 8] = [0.97, 0.93, 0.87, 0.70, 0.52, 0.34, 0.26, 0.18];

/// Chroma across that ramp: the background keeps a visible cast of the
/// wallpaper, the foreground ends up near-neutral so text does not read tinted.
const RAMP_CHROMA: (f64, f64) = (0.024, 0.008);

/// Where accents sit once their hue is chosen.
const ACCENT_DARK: (f64, f64) = (0.78, 0.135);
const ACCENT_LIGHT: (f64, f64) = (0.52, 0.150);

#[derive(Clone, Copy, PartialEq)]
pub enum Polarity {
    Dark,
    Light,
}

struct Cluster {
    l: f64,
    a: f64,
    b: f64,
    weight: f64,
}

impl Cluster {
    fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }
    fn hue(&self) -> f64 {
        self.b.atan2(self.a).to_degrees().rem_euclid(360.0)
    }
}

/// Shortest angular distance between two hues, in degrees.
fn hue_delta(a: f64, b: f64) -> f64 {
    let d = (a - b).abs().rem_euclid(360.0);
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

/// The wallpaper the daemon is displaying right now.
///
/// `awww query` (and swww's, which it forked from) prints one line per output:
///
///     : eDP-1: 2560x1600, scale: 1, currently displaying: image: /path/to.jpg
///
/// The first line carrying an image wins — with two monitors showing different
/// wallpapers there is no single right answer, and asking is worse than picking.
pub fn current_wallpaper() -> Result<PathBuf> {
    let mut tried = Vec::new();
    for daemon in ["awww", "swww"] {
        let out = match Command::new(daemon).arg("query").output() {
            Ok(o) => o,
            Err(_) => continue,
        };
        tried.push(daemon);
        if !out.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Some((_, path)) = line.rsplit_once("image: ") {
                let path = PathBuf::from(path.trim());
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
    }

    if tried.is_empty() {
        bail!("no wallpaper daemon found (looked for awww, swww) — pass an image path instead");
    }
    bail!(
        "{} is installed but reported no image — set a wallpaper first, or pass a path",
        tried.join("/")
    )
}

/// Decode, downscale, and cluster the image in Oklab.
///
/// Seeding is farthest-point rather than random, so the same wallpaper always
/// produces the same scheme. A generator you cannot reproduce is a generator you
/// cannot debug.
fn cluster_image(path: &Path) -> Result<Vec<Cluster>> {
    let img = image::open(path)
        .with_context(|| format!("cannot decode {} as an image", path.display()))?;
    let small = img.thumbnail(SAMPLE_EDGE, SAMPLE_EDGE).to_rgb8();

    let points: Vec<[f64; 3]> = small
        .pixels()
        .map(|p| {
            let c = rgb_to_oklch(
                p[0] as f64 / 255.0,
                p[1] as f64 / 255.0,
                p[2] as f64 / 255.0,
            );
            let h = c.h.to_radians();
            [c.l, c.c * h.cos(), c.c * h.sin()]
        })
        .collect();

    if points.is_empty() {
        bail!("{} decoded to zero pixels", path.display());
    }

    let dist2 = |p: &[f64; 3], q: &[f64; 3]| {
        // Chroma weighted up against lightness: hue/saturation is what we are
        // actually clustering for, and raw Oklab distance is dominated by L.
        let dl = p[0] - q[0];
        let da = (p[1] - q[1]) * 2.5;
        let db = (p[2] - q[2]) * 2.5;
        dl * dl + da * da + db * db
    };

    // Seed 0: the point nearest the mean. Seeds 1..K: farthest from all chosen.
    let mut mean = [0.0f64; 3];
    for p in &points {
        for i in 0..3 {
            mean[i] += p[i] / points.len() as f64;
        }
    }
    let mut centroids: Vec<[f64; 3]> = Vec::with_capacity(CLUSTERS);
    let first = points
        .iter()
        .min_by(|a, b| dist2(a, &mean).total_cmp(&dist2(b, &mean)))
        .copied()
        .unwrap();
    centroids.push(first);
    while centroids.len() < CLUSTERS.min(points.len()) {
        let next = points
            .iter()
            .max_by(|a, b| {
                let da = centroids.iter().map(|c| dist2(a, c)).fold(f64::MAX, f64::min);
                let db = centroids.iter().map(|c| dist2(b, c)).fold(f64::MAX, f64::min);
                da.total_cmp(&db)
            })
            .copied()
            .unwrap();
        centroids.push(next);
    }

    let mut counts = vec![0usize; centroids.len()];
    for _ in 0..ITERATIONS {
        let mut sums = vec![[0.0f64; 3]; centroids.len()];
        counts = vec![0usize; centroids.len()];
        for p in &points {
            let (idx, _) = centroids
                .iter()
                .enumerate()
                .map(|(i, c)| (i, dist2(p, c)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            for i in 0..3 {
                sums[idx][i] += p[i];
            }
            counts[idx] += 1;
        }
        for (i, c) in centroids.iter_mut().enumerate() {
            if counts[i] > 0 {
                for k in 0..3 {
                    c[k] = sums[i][k] / counts[i] as f64;
                }
            }
        }
    }

    Ok(centroids
        .iter()
        .zip(counts.iter())
        .filter(|(_, n)| **n > 0)
        .map(|(c, n)| Cluster {
            l: c[0],
            a: c[1],
            b: c[2],
            weight: *n as f64 / points.len() as f64,
        })
        .collect())
}

/// The hue the neutrals get tinted with: the chroma-weighted circular mean of
/// the image, which reads as "what colour is this picture".
fn tint_hue(clusters: &[Cluster]) -> f64 {
    let (mut a, mut b) = (0.0, 0.0);
    for c in clusters {
        let w = c.weight * c.chroma();
        a += c.a * w;
        b += c.b * w;
    }
    if a == 0.0 && b == 0.0 {
        // A genuinely greyscale wallpaper. Any hue is as right as any other and
        // the ramp chroma is low enough that it barely shows.
        return 250.0;
    }
    b.atan2(a).to_degrees().rem_euclid(360.0)
}

fn hex(l: f64, c: f64, h: f64) -> String {
    oklch_to_hex(Oklch { l, c, h })
}

/// Build a scheme from an image, write it into the schemes directory, and hand
/// back what `Scheme::load_file` makes of it.
pub fn scheme_from_image(path: &Path, forced: Option<Polarity>) -> Result<(Scheme, PathBuf)> {
    let clusters = cluster_image(path)?;

    let mean_l: f64 = clusters.iter().map(|c| c.l * c.weight).sum();
    let dark = match forced {
        Some(Polarity::Dark) => true,
        Some(Polarity::Light) => false,
        None => mean_l < 0.55,
    };

    let tint = tint_hue(&clusters);
    let ramp = if dark { RAMP_DARK } else { RAMP_LIGHT };
    let (accent_l, accent_c) = if dark { ACCENT_DARK } else { ACCENT_LIGHT };

    let mut palette: Vec<(String, String)> = Vec::with_capacity(16);
    for (i, l) in ramp.iter().enumerate() {
        let t = i as f64 / (ramp.len() - 1) as f64;
        let c = RAMP_CHROMA.0 + (RAMP_CHROMA.1 - RAMP_CHROMA.0) * t;
        palette.push((format!("base0{:X}", i), hex(*l, c, tint)));
    }

    let mut taken: Vec<f64> = Vec::with_capacity(ACCENTS.len());
    for (slot, target) in ACCENTS {
        // Best match = nearest hue, broken by how much of the image it is and
        // how saturated it is. Grey clusters are not candidates for an accent.
        let best = clusters
            .iter()
            .filter(|c| c.chroma() >= 0.035 && hue_delta(c.hue(), target) <= HUE_TOLERANCE)
            .max_by(|a, b| {
                let sa = a.weight * a.chroma() / (1.0 + hue_delta(a.hue(), target) / 90.0);
                let sb = b.weight * b.chroma() / (1.0 + hue_delta(b.hue(), target) / 90.0);
                sa.total_cmp(&sb)
            });
        let mut hue = best.map(|c| c.hue()).unwrap_or(target);
        if slot != "base0F" {
            if taken.iter().any(|h| hue_delta(*h, hue) < MIN_ACCENT_SEP) {
                hue = target;
            }
            taken.push(hue);
        }
        let (l, c) = if slot == "base0F" {
            (accent_l - 0.12, accent_c)
        } else {
            (accent_l, accent_c)
        };
        palette.push((slot.to_string(), hex(l, c, hue)));
    }

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "wallpaper".into());
    let slug = format!("wall-{}", slugify(&stem));
    let name = format!("Wall {}", stem.replace(['_', '-'], " "));

    let mut yaml = String::new();
    yaml.push_str("# Generated by `coat match` — regenerated on every run, edit at your peril.\n");
    yaml.push_str(&format!("# Source: {}\n", path.display()));
    yaml.push_str(&format!("name: \"{}\"\n", name.replace('"', "")));
    yaml.push_str(&format!("slug: \"{}\"\n", slug));
    yaml.push_str("author: \"coat match\"\n");
    yaml.push_str(&format!(
        "variant: \"{}\"\n",
        if dark { "dark" } else { "light" }
    ));
    yaml.push_str(&format!(
        "description: \"Sampled from {}\"\n",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    yaml.push_str("palette:\n");
    for (slot, value) in &palette {
        yaml.push_str(&format!("  {}: \"#{}\"\n", slot, value));
    }

    let dir = generated_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("cannot create {}", dir.display()))?;
    let dest = dir.join(format!("{}.yaml", slug));
    std::fs::write(&dest, &yaml).with_context(|| format!("cannot write {}", dest.display()))?;

    let scheme = Scheme::load_file(&dest)?;
    Ok((scheme, dest))
}

/// Generated schemes live under the schemes directory so `find_scheme`,
/// `coat list` and `coat browse` pick them up with no special-casing — but in
/// their own subdirectory, so they are obviously not from the upstream repo.
pub fn generated_dir() -> Result<PathBuf> {
    Ok(schemes_dir()?.join("generated"))
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dash = false;
    for ch in s.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            out.push(ch);
            dash = false;
        } else {
            dash = true;
        }
    }
    // Wallpaper filenames run long ("a_snowy_mountain_tops_with_blue_sky").
    if out.len() > 40 {
        out.truncate(40);
        out = out.trim_end_matches('-').to_string();
    }
    out
}
