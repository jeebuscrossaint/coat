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

/// The eight accent slots, in order. Names only — base16 fixes WHICH slots exist
/// and what each is for, and that part is a contract with every template
/// downstream. What each one should LOOK like is a measurement, and lives in
/// `Corpus`: `hue`, `hue_spread`, `min_sep` and the per-slot `accent_l`.
///
/// The hue numbers that used to sit here (25 red, 55 orange, 95 yellow, …) were
/// somebody's recollection of the convention. The convention has 403 dark
/// schemes' worth of evidence sitting in ~/.config/coat/schemes, and the corpus
/// median is that convention, stated by the people who actually ship schemes
/// rather than by a comment. base0F's "brown, ten degrees off red BY CONVENTION,
/// separated by lightness" is a good example: the corpus says exactly how many
/// degrees and exactly how much darker, so neither has to be asserted.
const SLOTS: [&str; 8] = [
    "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
];

/// Where a real scheme puts its colours, measured from the library on disk.
///
/// Everything in here used to be a literal — a ladder and a chroma ramp copied
/// out of a one-off analysis of the tinted-theming repo, with a comment saying
/// so. The numbers were right when they were taken and they rot: that comment
/// claimed "406 dark schemes and 127 light ones" against a library that is now
/// 403 and 130. coat already ships that library, already parses it
/// (`load_all_schemes`, which is itself cached) and already has the Oklch
/// conversion. A measurement of data we hold has no business being a literal.
///
/// So the ladder is measured per run, per polarity, and `coat match` tracks the
/// library instead of a snapshot of it. What is NOT measured: the accent hues in
/// `ACCENTS` (base16 says base08 is red — a contract with every template
/// downstream, not an observation) and the knobs further down, which are choices
/// about how this generator behaves and would only be dressed up as science by
/// fitting them to a sample.
struct Corpus {
    /// Lightness per neutral slot: the corpus MEDIAN. A generator that invents
    /// its own ladder lands somewhere no real scheme sits — the first cut of this
    /// file put base00 at L 0.14, a near-black void, where the corpus says 0.23
    /// and the schemes people actually name (catppuccin-mocha, rose-pine,
    /// tokyo-night, gruvbox-dark-hard) agree within a hair.
    ramp: [f64; 8],
    /// Chroma per neutral slot, and the shape matters: it HUMPS at base02/base03
    /// rather than decaying from the background. Taken at p75 rather than the
    /// median, because half the library is deliberately greyscale and this
    /// generator exists to carry a wallpaper's cast.
    ///
    /// A CEILING, not a fixed amount — see `neutral_chroma`.
    chroma: [f64; 8],
    /// Median lightness PER accent slot, as an absolute — see `follow` for why it
    /// is not a gap from the background. Per-slot rather than pooled because the
    /// corpus holds base0F lower than the rest, which the old code asserted as a
    /// hardcoded `accent_l - 0.12`.
    accent_l: [f64; 8],
    /// Circular-mean hue per accent slot: what "base08 is red" actually means,
    /// measured. Replaces a hand-written table of canonical hues.
    hue: [f64; 8],
    /// How far that slot's hue wanders in practice — the p90 absolute angular
    /// deviation from its own mean.
    ///
    /// Does two jobs that used to be two guesses. A cluster further than this
    /// from the slot's hue is not that colour (`HUE_TOLERANCE`, formerly a flat
    /// 45° for every slot, though yellow is a far narrower band than blue), and
    /// it is also the furthest an unmatched slot may lean toward the image's own
    /// hues (`FALLBACK_PULL_MAX`, formerly a flat 30°). Leaning is what stops
    /// every photo without a red in it producing the SAME red; bounding the lean
    /// by the slot's own measured spread is what keeps it recognisably red.
    hue_spread: [f64; 8],
    /// The closest two accents in one scheme are allowed to get, at the corpus
    /// p05. Two accents nearer than this are the same colour to the eye and the
    /// scheme has lost a slot rather than gained a matched one.
    min_sep: f64,
    /// Accent chroma at p25..p75 — the band an image's own saturation moves
    /// within, so a washed-out photo gives muted accents and a vivid one does not.
    accent_band: (f64, f64),
    /// How much each neutral slot moves when the BACKGROUND moves: the regression
    /// slope of that slot's lightness against base00's, across the corpus.
    ///
    /// This is the shape of a scheme, and it is the thing that lets `coat match`
    /// hand the wallpaper the background without wrecking the ramp. Dark schemes
    /// measure [1.000, 0.309, 0.347, 0.331, 0.084, 0.111, 0.209, 0.059]: base00
    /// is the background by definition, base01..base03 follow it about a third of
    /// the way, and base04 upward barely move at all. A near-black scheme and a
    /// median one differ by 0.24 at base00, by 0.004 at base05, and not at all at
    /// base07 — the ramp is pinned at the top and floats at the bottom.
    ///
    /// The constant this replaced guessed [1.0, 0.8, 0.5, 0.25, 0, 0, 0, 0] and
    /// then scaled the whole thing down by 0.06, which is why the wallpaper only
    /// ever moved the background by 0.024.
    follow: [f64; 8],
    /// Same slope for the accents, and it comes out at 0.126 — accents hold their
    /// ABSOLUTE lightness almost regardless of how dark the background is. Real
    /// near-black schemes put accents at L 0.676 and median-background ones at
    /// 0.699, a 0.02 spread, while the gap between accent and background swings
    /// from 0.676 to 0.395. Holding the gap constant instead would have dragged
    /// accents down to 0.469 on a black background, which no real scheme does.
    accent_follow: f64,
    /// The base00..base01 lightness gap at the corpus p25 — the separation three
    /// quarters of real schemes keep between the background and the surface
    /// directly above it. Used as a floor, not a target.
    bg_gap: f64,
    /// The base00 lightness range the corpus actually occupies, min..p95. An
    /// image anchor outside it is not a scheme of this polarity any more, and for
    /// dark schemes the floor is a true 0.000 — pure black backgrounds are
    /// ordinary, so the wallpaper is allowed all the way down.
    bg_range: (f64, f64),
}

/// A scheme `coat match` wrote. Measuring these would feed generated schemes back
/// into the ladder that generates them, so a week of wallpapers would drag the
/// corpus toward itself.
const GENERATED_AUTHOR: &str = "coat match";

/// Below this many schemes the percentiles are noise, and the shipped fallback is
/// the better answer. A fresh install before `coat clone` has none at all.
const CORPUS_MIN: usize = 24;

/// The last measurement taken from the library, in case it cannot be read. These
/// ARE the old literals: a snapshot of tinted-theming, kept only so that
/// `coat match` works before `coat clone` has ever run.
const FALLBACK_RAMP_DARK: [f64; 8] = [0.229, 0.277, 0.404, 0.537, 0.670, 0.810, 0.894, 0.967];
const FALLBACK_RAMP_LIGHT: [f64; 8] = [0.974, 0.915, 0.840, 0.684, 0.559, 0.411, 0.311, 0.231];
const FALLBACK_CHROMA_DARK: [f64; 8] = [0.028, 0.026, 0.032, 0.030, 0.024, 0.020, 0.016, 0.010];
const FALLBACK_CHROMA_LIGHT: [f64; 8] = [0.016, 0.020, 0.028, 0.030, 0.026, 0.022, 0.018, 0.012];
const FALLBACK_ACCENT_DARK_L: [f64; 8] = [0.699; 8];
const FALLBACK_ACCENT_LIGHT_L: [f64; 8] = [0.581; 8];
const FALLBACK_HUE: [f64; 8] = [25.0, 55.0, 95.0, 145.0, 195.0, 255.0, 320.0, 35.0];
const FALLBACK_HUE_SPREAD: [f64; 8] = [45.0; 8];
const FALLBACK_MIN_SEP: f64 = 20.0;
const FALLBACK_ACCENT_BAND: (f64, f64) = (0.085, 0.165);
const FALLBACK_FOLLOW_DARK: [f64; 8] = [1.0, 0.309, 0.347, 0.331, 0.084, 0.111, 0.209, 0.059];
const FALLBACK_FOLLOW_LIGHT: [f64; 8] = [1.0, 0.936, 0.677, 0.771, 0.952, 0.170, 0.0, 0.0];
const FALLBACK_ACCENT_FOLLOW_DARK: f64 = 0.126;
const FALLBACK_ACCENT_FOLLOW_LIGHT: f64 = 0.539;
const FALLBACK_BG_RANGE_DARK: (f64, f64) = (0.000, 0.326);
const FALLBACK_BG_RANGE_LIGHT: (f64, f64) = (0.761, 1.000);
const FALLBACK_BG_GAP_DARK: f64 = 0.0210;
const FALLBACK_BG_GAP_LIGHT: f64 = -0.0790;

impl Corpus {
    fn fallback(dark: bool) -> Self {
        Corpus {
            ramp: if dark { FALLBACK_RAMP_DARK } else { FALLBACK_RAMP_LIGHT },
            chroma: if dark { FALLBACK_CHROMA_DARK } else { FALLBACK_CHROMA_LIGHT },
            accent_l: if dark { FALLBACK_ACCENT_DARK_L } else { FALLBACK_ACCENT_LIGHT_L },
            hue: FALLBACK_HUE,
            hue_spread: FALLBACK_HUE_SPREAD,
            min_sep: FALLBACK_MIN_SEP,
            accent_band: FALLBACK_ACCENT_BAND,
            follow: if dark { FALLBACK_FOLLOW_DARK } else { FALLBACK_FOLLOW_LIGHT },
            accent_follow: if dark {
                FALLBACK_ACCENT_FOLLOW_DARK
            } else {
                FALLBACK_ACCENT_FOLLOW_LIGHT
            },
            bg_gap: if dark { FALLBACK_BG_GAP_DARK } else { FALLBACK_BG_GAP_LIGHT },
            bg_range: if dark { FALLBACK_BG_RANGE_DARK } else { FALLBACK_BG_RANGE_LIGHT },
        }
    }

    /// Measure the library, or fall back to the snapshot above.
    fn measure(dark: bool) -> Self {
        Self::try_measure(dark).unwrap_or_else(|| Self::fallback(dark))
    }

    fn try_measure(dark: bool) -> Option<Self> {
        let schemes = crate::scheme::load_all_schemes().ok()?;

        // Per neutral slot, every lightness and every chroma in the corpus.
        let mut neutral_l: Vec<Vec<f64>> = vec![Vec::new(); 8];
        let mut neutral_c: Vec<Vec<f64>> = vec![Vec::new(); 8];
        // Accents are pooled across all eight slots: the question is where an
        // accent sits, not where base0B specifically sits.
        let mut accent_l: Vec<f64> = Vec::new();
        let mut accent_c: Vec<f64> = Vec::new();
        let mut slot_l: Vec<Vec<f64>> = vec![Vec::new(); 8];
        let mut slot_hues: Vec<Vec<f64>> = vec![Vec::new(); 8];
        let mut hue_vec: Vec<(f64, f64)> = vec![(0.0, 0.0); 8];
        let mut hue_n: Vec<f64> = vec![0.0; 8];
        let mut sep_samples: Vec<f64> = Vec::new();
        // Accent chroma floor for the hue statistics only: the corpus p25, i.e.
        // the bottom of the band this generator will ever emit. Below it a slot
        // is grey and its hue is noise.
        let accent_hue_floor = FALLBACK_ACCENT_BAND.0;
        // Paired with the scheme's own base00, for the regressions below: how a
        // slot moves WHEN THE BACKGROUND MOVES is a different question from where
        // the slot sits on average, and only the pairs can answer it.
        let mut bg_l: Vec<f64> = Vec::new();
        let mut slot_by_bg: Vec<Vec<f64>> = vec![Vec::new(); 8];
        let mut accent_by_bg: Vec<f64> = Vec::new();
        let mut counted = 0usize;

        for s in &schemes {
            if s.is_dark() != dark || s.author == GENERATED_AUTHOR {
                continue;
            }
            counted += 1;
            for (i, hex) in neutrals(s).iter().enumerate() {
                if let Some(col) = parse_oklch(hex) {
                    neutral_l[i].push(col.l);
                    neutral_c[i].push(col.c);
                }
            }
            // Per slot, not pooled: base08 being red is a fact about base08.
            // Hues accumulate as unit vectors — an average of 350 and 10 is 0,
            // not 180, and hue is the one axis where the arithmetic mean lies.
            let mut present: Vec<(usize, f64)> = Vec::with_capacity(8);
            for (i, hex) in accents(s).iter().enumerate() {
                if let Some(col) = parse_oklch(hex) {
                    accent_l.push(col.l);
                    accent_c.push(col.c);
                    slot_l[i].push(col.l);
                    // A near-grey accent has no meaningful hue and would drag the
                    // mean toward whatever its rounding noise points at.
                    if col.c >= accent_hue_floor {
                        let r = col.h.to_radians();
                        hue_vec[i].0 += r.cos();
                        hue_vec[i].1 += r.sin();
                        hue_n[i] += 1.0;
                        slot_hues[i].push(col.h);
                        present.push((i, col.h));
                    }
                }
            }
            // How close this scheme lets any two of its own accents get. base0F
            // is excluded: it is brown, a near-neighbour of red BY CONVENTION and
            // separated by lightness rather than hue, so counting it would drag
            // the measured minimum down to brown-vs-red on every scheme.
            let mut closest = f64::MAX;
            for (ai, (i, ha)) in present.iter().enumerate() {
                for (j, hb) in present.iter().skip(ai + 1) {
                    if *i == 7 || *j == 7 {
                        continue;
                    }
                    closest = closest.min(hue_delta(*ha, *hb));
                }
            }
            if closest.is_finite() {
                sep_samples.push(closest);
            }

            // A scheme only joins the regression if it is complete: a slope built
            // from rows of differing length is not a slope.
            let neutral: Option<Vec<f64>> =
                neutrals(s).iter().map(|h| parse_oklch(h).map(|c| c.l)).collect();
            let accent: Option<Vec<f64>> =
                accents(s).iter().map(|h| parse_oklch(h).map(|c| c.l)).collect();
            if let (Some(neutral), Some(accent)) = (neutral, accent) {
                bg_l.push(neutral[0]);
                for (i, l) in neutral.iter().enumerate() {
                    slot_by_bg[i].push(*l);
                }
                accent_by_bg.push(accent.iter().sum::<f64>() / accent.len() as f64);
            }
        }

        if counted < CORPUS_MIN || accent_c.is_empty() || bg_l.len() < CORPUS_MIN {
            return None;
        }

        // A slot missing everywhere would leave a hole in the ladder; there is
        // nothing to measure, so keep the snapshot's value for that slot.
        let fb = Self::fallback(dark);
        let mut ramp = [0.0; 8];
        let mut chroma = [0.0; 8];
        let mut follow = [0.0; 8];
        for i in 0..8 {
            ramp[i] = percentile(&mut neutral_l[i], 0.50).unwrap_or(fb.ramp[i]);
            chroma[i] = percentile(&mut neutral_c[i], 0.75).unwrap_or(fb.chroma[i]);
            follow[i] = slope(&bg_l, &slot_by_bg[i]).unwrap_or(fb.follow[i]);
        }

        // Circular mean per slot, and the p90 absolute deviation from it.
        let mut hue = fb.hue;
        let mut hue_spread = fb.hue_spread;
        let mut slot_lightness = fb.accent_l;
        for i in 0..8 {
            if hue_n[i] >= CORPUS_MIN as f64 {
                let h = hue_vec[i].1.atan2(hue_vec[i].0).to_degrees().rem_euclid(360.0);
                hue[i] = h;
                let mut devs: Vec<f64> =
                    slot_hues[i].iter().map(|x| hue_delta(*x, h)).collect();
                if let Some(p90) = percentile(&mut devs, 0.90) {
                    hue_spread[i] = p90;
                }
            }
            if let Some(l) = percentile(&mut slot_l[i], 0.50) {
                slot_lightness[i] = l;
            }
        }

        let mut bg_sorted = bg_l.clone();
        let mut bg_gaps: Vec<f64> = bg_l
            .iter()
            .zip(slot_by_bg[1].iter())
            .map(|(a, b)| b - a)
            .collect();
        Some(Corpus {
            ramp,
            chroma,
            accent_l: slot_lightness,
            hue,
            hue_spread,
            min_sep: percentile(&mut sep_samples, 0.05).unwrap_or(fb.min_sep),
            accent_band: (
                percentile(&mut accent_c, 0.25)?,
                percentile(&mut accent_c, 0.75)?,
            ),
            follow,
            accent_follow: slope(&bg_l, &accent_by_bg).unwrap_or(fb.accent_follow),
            // p25 from the dark end, p75 from the light end: both are "the
            // quartile most schemes are past", just measured in the direction
            // that polarity's gap runs.
            bg_gap: percentile(&mut bg_gaps, if dark { 0.25 } else { 0.75 })
                .unwrap_or(fb.bg_gap),
            bg_range: (
                percentile(&mut bg_sorted, 0.0)?,
                percentile(&mut bg_sorted, 0.95)?,
            ),
        })
    }
}

fn neutrals(s: &crate::scheme::Scheme) -> [&str; 8] {
    [
        &s.base00, &s.base01, &s.base02, &s.base03, &s.base04, &s.base05, &s.base06, &s.base07,
    ]
}

fn accents(s: &crate::scheme::Scheme) -> [&str; 8] {
    [
        &s.base08, &s.base09, &s.base0a, &s.base0b, &s.base0c, &s.base0d, &s.base0e, &s.base0f,
    ]
}

/// Least-squares slope of `ys` against `xs`: how far a slot moves when base00
/// moves by one.
///
/// Clamped to 0..=1, which is structural rather than cosmetic. Below 0 a slot
/// would move OPPOSITE its own background and above 1 it would outrun it; the
/// corpus produces both on light schemes, where every base00 is crammed into
/// 0.76..1.00 and the regression has almost no spread to work with. A slope from
/// that is noise, and the clamp keeps noise from inverting a ramp.
fn slope(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    let n = xs.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let sxy: f64 = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    let syy: f64 = ys.iter().map(|y| (y - my) * (y - my)).sum();
    if sxx <= f64::EPSILON || syy <= f64::EPSILON {
        return None;
    }
    // Shrink by R². A slope is only worth applying to the extent the data
    // supports it, and here the two polarities differ enormously: dark base00
    // spans 0.000..0.467 and the fit is real, while light base00 is crammed into
    // 0.761..1.000 and the regression has almost no leverage — it returned 0.936
    // for base01 and 0.952 for base04, which dragged the whole light ramp down
    // with the background and compressed it. Multiplying by R² leaves a
    // well-supported slope essentially untouched and collapses an unsupported one
    // toward zero, where "zero" means the honest thing: the background moves and
    // the rest of the ramp stays put.
    let r2 = (sxy * sxy) / (sxx * syy);
    Some((sxy / sxx).clamp(0.0, 1.0) * r2)
}

/// Linear-interpolated percentile. Sorts in place; `None` on an empty sample.
fn percentile(xs: &mut Vec<f64>, p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(f64::total_cmp);
    let pos = p * (xs.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    Some(if lo == hi {
        xs[lo]
    } else {
        xs[lo] + (xs[hi] - xs[lo]) * (pos - lo as f64)
    })
}

/// A scheme's stored hex (no leading `#`, already normalized by `load_file`) in
/// Oklch. `None` on anything that is not six hex digits.
fn parse_oklch(hex: &str) -> Option<Oklch> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(rgb_to_oklch(
        ((v >> 16) & 0xFF) as f64 / 255.0,
        ((v >> 8) & 0xFF) as f64 / 255.0,
        (v & 0xFF) as f64 / 255.0,
    ))
}

/// Scale the neutral ladder by how colourful the image actually is.
///
/// The ladder above is what a neutral ramp looks like in a scheme a person sat
/// down and designed. Applied flat it says something about the IMAGE that may be
/// false: a greyscale wallpaper whose only non-grey pixels are quantisation
/// noise — 0.3% of the frame, invisible on screen — still drove `tint_hue` to a
/// definite hue, and the flat ladder then painted base00..base07 with 0.028
/// chroma of it. Every surface and every glyph came out tinted by an artefact,
/// roughly ninety times more colour than the image contained.
///
/// The accents already refused to do this (`accent_c` scales with the same
/// measurement); the neutrals just never got the same treatment. So: the ramp is
/// the most colour the neutrals may carry, and the image says how much of it is
/// earned. The reference for "colourful enough to fill the ladder" is the
/// ladder's own peak — no threshold to tune, no cliff to sit on the wrong side
/// of, and an image at or above it is themed exactly as before.
///
/// This also defuses the hue question rather than answering it. A hue derived
/// from 0.3% of an image is meaningless, but at the chroma such an image now
/// earns it is also invisible, so there is nothing to get wrong.
fn neutral_chroma(ladder: &[f64; 8], image_chroma: f64) -> [f64; 8] {
    let peak = ladder.iter().copied().fold(f64::MIN, f64::max);
    let fill = (image_chroma / peak).clamp(0.0, 1.0);
    let mut out = [0.0; 8];
    for (o, l) in out.iter_mut().zip(ladder.iter()) {
        *o = l * fill;
    }
    out
}

/// Which of the image's colours becomes the background.
///
/// The largest cluster by area — literally "what colour is most of this
/// wallpaper". `coat match` says it derives a scheme from the wallpaper, and the
/// background is the single most visible thing it produces, so the wallpaper had
/// better get to choose it.
///
/// This replaced a `DRIFT_SCALE` of 0.06 applied to the image's MEAN lightness,
/// which let the wallpaper move base00 by at most 0.025 — a background that was
/// 96% corpus median and 4% wallpaper. A user running `coat match` on a
/// three-quarters-black wallpaper and getting #171717 was right to call that
/// not matching anything.
///
/// Mean lightness was the wrong measurement for this and its own comment said
/// why, in the negative: it was chosen over the image's darkest colour because
/// "every photo has something near-black in it", which is an argument against
/// darkest, not an argument for mean. A mean blends the subject into the
/// backdrop and lands between them, on a colour that may not be in the image at
/// all. The dominant cluster is a colour the wallpaper actually contains.
fn background_anchor(clusters: &[Cluster]) -> Option<f64> {
    clusters
        .iter()
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .map(|c| c.l)
}

/// How saturated the image's ACCENT-WORTHY colours are — the chroma-weighted
/// chroma of the clusters that are actually candidates for a slot.
///
/// This replaced a `CHROMA_REFERENCE` of 0.11: the image's area-weighted chroma
/// over every cluster, greys included, divided by a hand-picked number to land
/// it somewhere in the accent band. That division existed only because the two
/// sides were not comparable — a mean that counts a black letterbox is an order
/// of magnitude below any accent chroma, so it needed a fudge factor to become
/// one, and the fudge factor was the thing nobody could justify.
///
/// Measuring only the chromatic clusters makes the two sides the same kind of
/// quantity, and then no conversion is needed at all: the image's colourfulness
/// IS an accent chroma, clamped into the band the corpus reports. A grey image
/// has no candidates, falls to the band floor, and gets the corpus p25 — muted,
/// legible, and not a number anybody chose.
fn image_accent_chroma(clusters: &[Cluster], floor: f64) -> Option<f64> {
    let (mut sum, mut weight) = (0.0, 0.0);
    for c in clusters.iter().filter(|c| c.chroma() >= floor) {
        let w = c.weight * c.chroma();
        sum += c.chroma() * w;
        weight += w;
    }
    (weight > 0.0).then(|| sum / weight)
}

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
        // A wallpaper that is greyscale to the last bit. Any hue is as right as
        // any other.
        //
        // This guard is close to unreachable and was never the protection it
        // looked like: a "black and white" image is almost never grey to f64
        // equality — quantisation noise, JPEG chroma, a stray palette entry — and
        // the ones that missed it by 0.3% of a frame came through here with a
        // confident, meaningless hue. What makes that harmless is `neutral_chroma`
        // downstream, which gives such an image almost no chroma to render the
        // hue with. The hue is not worth getting right; it is worth not showing.
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
    let corpus = Corpus::measure(dark);

    // How colourful the image actually is, weighted by area: a grey photo should
    // not produce a neon scheme.
    let image_chroma: f64 = clusters.iter().map(|c| c.chroma() * c.weight).sum();
    let (lo, hi) = corpus.accent_band;
    let accent_c = image_accent_chroma(&clusters, lo).unwrap_or(lo).clamp(lo, hi);

    // The wallpaper's own dominant colour becomes the background, clamped to the
    // lightness range schemes of this polarity actually occupy — outside it this
    // stops being a dark scheme at all. For dark that floor is a true 0.000, so a
    // black wallpaper is allowed a black background.
    let anchor = background_anchor(&clusters)
        .unwrap_or(corpus.ramp[0])
        .clamp(corpus.bg_range.0, corpus.bg_range.1);
    let shift = anchor - corpus.ramp[0];

    // Everything else moves by its measured share of that. base00 takes all of
    // it, base01..base03 about a third, the top of the ramp almost none — which
    // is how a real near-black scheme differs from a real median one, so the
    // ramp keeps its shape instead of collapsing toward the background.
    let ramp = {
        let mut r = [0.0; 8];
        for (i, slot) in r.iter_mut().enumerate() {
            *slot = corpus.ramp[i] + shift * corpus.follow[i];
        }
        // base00 moves with the anchor and base01 only follows a third of the
        // way, so a bright enough wallpaper drives the background past the
        // surface above it and inverts the bottom of the ramp — #31343B over
        // #2C3037, a "background" lighter than the panel sitting on it.
        //
        // A single regression slope is the wrong shape for this pair and the
        // corpus says so: base01 floors out around 0.23 as the background goes
        // black (slope ~0.23 at the dark end) but tracks it almost exactly when
        // the background is bright (~1.05 between the top two buckets). Rather
        // than fit a curve to 403 points, hold the floor the corpus keeps —
        // `bg_gap`, the p25 separation — and let the anchor have everything
        // above it. A wallpaper too bright to be a dark scheme's background
        // stops being honoured exactly where it would close that gap.
        //
        // Not monotonicity: only 211 of 403 dark schemes have a strictly
        // monotonic neutral ramp, so enforcing that would forbid what half the
        // corpus does. This is one pair, with one measured floor.
        let limit = r[1] - corpus.bg_gap;
        r[0] = if dark { r[0].min(limit) } else { r[0].max(limit) };
        r
    };

    let chroma_ramp = neutral_chroma(&corpus.chroma, image_chroma);
    let mut palette: Vec<(String, String)> = Vec::with_capacity(16);
    for (i, l) in ramp.iter().enumerate() {
        palette.push((
            format!("base0{:X}", i),
            hex(*l, chroma_ramp[i], tint),
        ));
    }

    // The default: spread the image's actual colours across the eight accents,
    // giving up the guarantee that a slot holds the colour it is named after.
    //
    // It is a one-cluster-per-slot assignment, NOT "each slot takes its nearest
    // cluster" -- that scores every slot against the same dominant colour and
    // hands back eight identical accents, which is worse than useless on a
    // wallpaper with one strong hue.
    let mut assigned: Vec<Option<&Cluster>> = vec![None; SLOTS.len()];
    if raw {
        let mut pool: Vec<&Cluster> = clusters.iter().filter(|c| c.chroma() >= 0.02).collect();
        pool.sort_by(|a, b| (b.weight * b.chroma()).total_cmp(&(a.weight * a.chroma())));
        pool.truncate(SLOTS.len());

        // Greedy over the globally closest (slot, cluster) pair, so the best
        // match wins the slot it fits rather than the first slot that asks.
        let mut free: Vec<usize> = (0..SLOTS.len()).collect();
        while !pool.is_empty() && !free.is_empty() {
            let mut best = (0usize, 0usize, f64::MAX);
            for (pi, c) in pool.iter().enumerate() {
                for (fi, si) in free.iter().enumerate() {
                    let d = hue_delta(c.hue(), corpus.hue[*si]);
                    if d < best.2 {
                        best = (pi, fi, d);
                    }
                }
            }
            let slot = free.remove(best.1);
            assigned[slot] = Some(pool.remove(best.0));
        }
    }

    let mut taken: Vec<f64> = Vec::with_capacity(SLOTS.len());
    for (idx, slot) in SLOTS.iter().copied().enumerate() {
        let target = corpus.hue[idx];
        // Per-slot rather than one flat tolerance: the corpus says yellow occupies
        // a far narrower band of hues than blue does.
        let tolerance = corpus.hue_spread[idx];
        // Best match = nearest hue, broken by how much of the image it is and
        // how saturated it is. Grey clusters are not candidates for an accent.
        let best = if raw {
            assigned[idx]
        } else {
            clusters
                .iter()
                .filter(|c| c.chroma() >= lo && hue_delta(c.hue(), target) <= tolerance)
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
                    .filter(|c| c.chroma() >= lo)
                    .min_by(|a, b| {
                        hue_delta(a.hue(), target).total_cmp(&hue_delta(b.hue(), target))
                    })
                    .map(|c| c.hue());
                match nearest {
                    Some(h) => {
                        // Lean toward the image, but never further than this
                        // slot's own hue wanders across the corpus — past that it
                        // stops being the colour the slot is named after.
                        let pull = hue_signed(target, h).clamp(-tolerance, tolerance);
                        (target + pull).rem_euclid(360.0)
                    }
                    None => target,
                }
            }
        };

        if !raw && slot != "base0F" {
            if taken.iter().any(|h| hue_delta(*h, hue) < corpus.min_sep) {
                hue = target;
            }
            taken.push(hue);
        }

        // By default the matched colour's own saturation carries through, only
        // lifted into the legible band; under --slots every accent shares the
        // image-wide chroma so the row reads as one family.
        let chroma = match (raw, best) {
            // The matched colour's own saturation, lifted into the legible band.
            (true, Some(c)) => c.chroma().clamp(lo, hi),
            _ => accent_c,
        };
        // Per-slot corpus lightness, shifted with the background. base0F comes out
        // darker than the rest because the corpus holds it darker, not because a
        // constant subtracted 0.12 from it.
        let l = corpus.accent_l[idx] + shift * corpus.accent_follow;
        palette.push((slot.to_string(), hex(l, chroma, hue)));
    }

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "wallpaper".into());
    // One slug per FLAVOUR, not per wallpaper. A wallpaper scheme is the
    // current wallpaper's scheme and nothing more -- it is replaced the next
    // time you change the wallpaper, so naming it after the image left a file
    // behind for every wallpaper ever set, all of them dead the moment the next
    // one landed, and all of them in `coat list` forever. Four names at most now
    // -- one per --slots/--light combination, so `coat set wall-light` still
    // means something -- and each is overwritten in place. Which image it came
    // from is still recorded, in `name` and `description` inside the file.
    let slug = format!(
        "wall{}{}",
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
