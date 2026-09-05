//! `coat match` — derive a scheme from the wallpaper that is already on screen.
//!
//! Nothing here picks colours by taste. The image supplies the colours; the
//! lightness/chroma ladders are the measured medians of the tinted-theming
//! library, so a generated scheme sits where real schemes sit instead of
//! somewhere nobody ships.
//!
//! By default the image's colours go into the accent slots as they are,
//! pywal-style, and a wallpaper with one strong hue gives eight shades of it.
//! `--slots` is the other bargain: every accent holds the colour it is named
//! after, at the cost of accents that vary less from wallpaper to wallpaper.
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

/// Conventional base16 accent hues, as Oklch degrees. Under `--slots` an image
/// hue is snapped to whichever of these it is nearest, so `base0B` holds
/// something green even when the wallpaper is mostly teal. By default they only
/// order the assignment: each of the image's colours goes to the slot whose hue
/// it is closest to, and is then kept as it is.
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

/// How far an unmatched slot is allowed to lean toward the image's own hues, and
/// how much of the way it leans. Without this every photo without a red in it
/// produced the SAME red — canonical hue, canonical chroma — and eight slots of
/// identical accents across completely different wallpapers. Leaning keeps red
/// recognisably red while letting a cold image have colder reds than a warm one.
const FALLBACK_PULL_MAX: f64 = 30.0;
const FALLBACK_PULL: f64 = 0.45;

/// A cluster hue this far from a slot's target hue is not that colour, and the
/// slot takes its canonical hue instead of a bad match.
const HUE_TOLERANCE: f64 = 45.0;

/// Two accents closer than this are the same colour to the eye, and a scheme
/// where base0D and base0E are indistinguishable has lost two slots rather than
/// gained a matched one. A slot whose best hue collides falls back to its
/// canonical target. base0F is exempt: it is brown, ten degrees off red BY
/// CONVENTION, and separated by lightness rather than hue.
const MIN_ACCENT_SEP: f64 = 20.0;

/// Lightness ladder for base00..base07, and the chroma that goes with it.
///
/// These are not taste. They are the MEDIAN of every scheme in the tinted-theming
/// library, measured in Oklch — 406 dark schemes and 127 light ones. A generator
/// that invents its own ladder lands somewhere no real scheme sits: the first cut
/// of this file put base00 at L 0.14, a near-black void, when the corpus median is
/// 0.229 and the schemes people actually name — catppuccin-mocha 0.24, rose-pine
/// 0.21, tokyo-night 0.23, gruvbox-dark-hard 0.24 — agree within a hair.
const RAMP_DARK: [f64; 8] = [0.229, 0.277, 0.404, 0.537, 0.670, 0.810, 0.894, 0.967];
const RAMP_LIGHT: [f64; 8] = [0.974, 0.915, 0.840, 0.684, 0.559, 0.411, 0.311, 0.231];

/// Chroma per slot, and the shape matters: it HUMPS at base02/base03 rather than
/// decaying from the background, in both the corpus (0.010 → 0.017 → 0.003) and in
/// the tinted schemes specifically. Values sit near the corpus p75 rather than its
/// median, because half that library is deliberately greyscale and this generator
/// exists to carry a wallpaper's cast — catppuccin-mocha runs 0.030 at base00 and
/// 0.032 at base02, which is the territory being aimed at.
const CHROMA_DARK: [f64; 8] = [0.028, 0.026, 0.032, 0.030, 0.024, 0.020, 0.016, 0.010];
const CHROMA_LIGHT: [f64; 8] = [0.016, 0.020, 0.028, 0.030, 0.026, 0.022, 0.018, 0.012];

/// How far the background may drift from that median with the image's overall
/// brightness, and how fast the drift fades up the ramp.
///
/// Anchored on MEAN lightness, not on the image's darkest colour: every photo has
/// something near-black in it, so a darkest-colour anchor pins itself to the
/// bottom of the clamp on every wallpaper and stops being character at all. Mean
/// brightness actually varies, and the range is narrow by design — a bright
/// wallpaper lands around catppuccin's 0.24, a dark one around rose-pine's 0.21,
/// and neither end reaches the void this replaced.
const DRIFT_SCALE: f64 = 0.06;
const DRIFT_RANGE: (f64, f64) = (-0.025, 0.035);
const DRIFT_FADE: [f64; 8] = [1.0, 0.8, 0.5, 0.25, 0.0, 0.0, 0.0, 0.0];

/// Where accents sit once their hue is chosen. Lightness is fixed — that is the
/// legibility guarantee — but chroma is only a MIDPOINT: the image's own
/// saturation scales it within the band below, so a washed-out photo gives muted
/// accents instead of eight candy-bright canonical hues stapled onto it.
///
/// Same provenance: dark-scheme accents in the library run L 0.712 median
/// (p25 0.62, p75 0.77) and chroma 0.123 median (p25 0.086, p75 0.157).
const ACCENT_DARK_L: f64 = 0.740;
const ACCENT_LIGHT_L: f64 = 0.545;
const ACCENT_CHROMA_BAND: (f64, f64) = (0.085, 0.165);

/// Image chroma that maps to the top of that band. Anything more saturated than
/// this is already vivid enough to clip.
const CHROMA_REFERENCE: f64 = 0.11;

/// How to decide light vs dark.
///
/// DARK IS THE DEFAULT, and deliberately not inferred from the image. Inferring
/// it means a snowy wallpaper turns the whole desktop white, which is not what
/// anyone asks for when they point a themer at a photo — pywal has been dark by
/// default for the same reason. `Auto` is still there for when the image really
/// should decide.
#[derive(Clone, Copy, PartialEq)]
pub enum Polarity {
    Dark,
    Light,
    Auto,
}

impl Default for Polarity {
    fn default() -> Self {
        Polarity::Dark
    }
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

/// Signed shortest rotation from `from` to `to`, in degrees (-180..180].
fn hue_signed(from: f64, to: f64) -> f64 {
    let d = (to - from).rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
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
pub fn scheme_from_image(path: &Path, polarity: Polarity, raw: bool) -> Result<(Scheme, PathBuf)> {
    let clusters = cluster_image(path)?;

    let mean_l: f64 = clusters.iter().map(|c| c.l * c.weight).sum();
    let dark = match polarity {
        Polarity::Dark => true,
        Polarity::Light => false,
        Polarity::Auto => mean_l < 0.55,
    };

    let tint = tint_hue(&clusters);
    let ramp = if dark { RAMP_DARK } else { RAMP_LIGHT };
    let accent_l = if dark { ACCENT_DARK_L } else { ACCENT_LIGHT_L };

    // How colourful the image actually is, weighted by area: a grey photo should
    // not produce a neon scheme.
    let image_chroma: f64 = clusters.iter().map(|c| c.chroma() * c.weight).sum();
    let (lo, hi) = ACCENT_CHROMA_BAND;
    let accent_c = lo + (hi - lo) * (image_chroma / CHROMA_REFERENCE).clamp(0.0, 1.0);

    // A bright wallpaper lifts the background slightly, a dark one sinks it.
    // Mirrored for light schemes, where "lifting" means going the other way.
    let drift = {
        let d = ((mean_l - 0.5) * DRIFT_SCALE).clamp(DRIFT_RANGE.0, DRIFT_RANGE.1);
        if dark {
            d
        } else {
            -d
        }
    };

    let chroma_ramp = if dark { CHROMA_DARK } else { CHROMA_LIGHT };
    let mut palette: Vec<(String, String)> = Vec::with_capacity(16);
    for (i, l) in ramp.iter().enumerate() {
        palette.push((
            format!("base0{:X}", i),
            hex(l + drift * DRIFT_FADE[i], chroma_ramp[i], tint),
        ));
    }

    // The default: spread the image's actual colours across the eight accents,
    // giving up the guarantee that a slot holds the colour it is named after.
    //
    // It is a one-cluster-per-slot assignment, NOT "each slot takes its nearest
    // cluster" -- that scores every slot against the same dominant colour and
    // hands back eight identical accents, which is worse than useless on a
    // wallpaper with one strong hue.
    let mut assigned: Vec<Option<&Cluster>> = vec![None; ACCENTS.len()];
    if raw {
        let mut pool: Vec<&Cluster> = clusters.iter().filter(|c| c.chroma() >= 0.02).collect();
        pool.sort_by(|a, b| (b.weight * b.chroma()).total_cmp(&(a.weight * a.chroma())));
        pool.truncate(ACCENTS.len());

        // Greedy over the globally closest (slot, cluster) pair, so the best
        // match wins the slot it fits rather than the first slot that asks.
        let mut free: Vec<usize> = (0..ACCENTS.len()).collect();
        while !pool.is_empty() && !free.is_empty() {
            let mut best = (0usize, 0usize, f64::MAX);
            for (pi, c) in pool.iter().enumerate() {
                for (fi, si) in free.iter().enumerate() {
                    let d = hue_delta(c.hue(), ACCENTS[*si].1);
                    if d < best.2 {
                        best = (pi, fi, d);
                    }
                }
            }
            let slot = free.remove(best.1);
            assigned[slot] = Some(pool.remove(best.0));
        }
    }

    let mut taken: Vec<f64> = Vec::with_capacity(ACCENTS.len());
    for (idx, (slot, target)) in ACCENTS.iter().copied().enumerate() {
        // Best match = nearest hue, broken by how much of the image it is and
        // how saturated it is. Grey clusters are not candidates for an accent.
        let best = if raw {
            assigned[idx]
        } else {
            clusters
                .iter()
                .filter(|c| c.chroma() >= 0.035 && hue_delta(c.hue(), target) <= HUE_TOLERANCE)
                .max_by(|a, b| {
                    let sa = a.weight * a.chroma() / (1.0 + hue_delta(a.hue(), target) / 90.0);
                    let sb = b.weight * b.chroma() / (1.0 + hue_delta(b.hue(), target) / 90.0);
                    sa.total_cmp(&sb)
                })
        };

        let mut hue = match best {
            Some(c) => c.hue(),
            // Nothing that colour in the image: keep the slot's identity, but
            // lean it toward whatever the image's nearest hue actually is.
            None => {
                let nearest = clusters
                    .iter()
                    .filter(|c| c.chroma() >= 0.035)
                    .min_by(|a, b| {
                        hue_delta(a.hue(), target).total_cmp(&hue_delta(b.hue(), target))
                    })
                    .map(|c| c.hue());
                match nearest {
                    Some(h) => {
                        let pull =
                            hue_signed(target, h).clamp(-FALLBACK_PULL_MAX, FALLBACK_PULL_MAX);
                        (target + pull * FALLBACK_PULL).rem_euclid(360.0)
                    }
                    None => target,
                }
            }
        };

        if !raw && slot != "base0F" {
            if taken.iter().any(|h| hue_delta(*h, hue) < MIN_ACCENT_SEP) {
                hue = target;
            }
            taken.push(hue);
        }

        // By default the matched colour's own saturation carries through, only
        // lifted into the legible band; under --slots every accent shares the
        // image-wide chroma so the row reads as one family.
        let (lo, hi) = ACCENT_CHROMA_BAND;
        let chroma = match (raw, best) {
            (true, Some(c)) => (c.chroma() * 1.2).clamp(lo, hi),
            _ => accent_c,
        };
        let l = if slot == "base0F" { accent_l - 0.12 } else { accent_l };
        palette.push((slot.to_string(), hex(l, chroma, hue)));
    }

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "wallpaper".into());
    let slug = format!(
        "wall-{}{}{}",
        slugify(&stem),
        if raw { "" } else { "-slots" },
        if dark { "" } else { "-light" }
    );
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
