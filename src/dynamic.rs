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

use image::ImageDecoder;

use crate::icc::Profile;
use crate::cam16::{self, Conditions, Jmh};
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
    /// How far an unmatched slot may lean toward the image's own hues.
    ///
    /// Half the angular distance to its nearest neighbouring slot, so a leaned
    /// accent stays closer to its own hue than to anybody else's and remains the
    /// colour it is named after. Derived from the slot layout, which is itself
    /// measured — there is nothing here to pick.
    ///
    /// This is NOT `hue_spread`, and conflating them was a real bug. Spread says
    /// how widely the corpus draws this slot, which is the right test for whether
    /// an image cluster counts as that colour. Using it as the lean budget let
    /// cyan swing 45.9° and yellow 78.8° toward whatever the image had, so a
    /// wallpaper whose only saturated region was one warm sky pulled all eight
    /// accents into a single olive band.
    ///
    /// base0F is measured against everything except base08: it is brown, sitting
    /// 0.6° from red in the corpus BY CONVENTION and separated by lightness, so
    /// its true nearest neighbour would otherwise pin its budget to nothing.
    lean: [f64; 8],
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
const FALLBACK_RAMP_DARK: [f64; 8] =
    [14.161, 19.070, 33.269, 49.227, 65.061, 80.738, 89.551, 96.850];
const FALLBACK_RAMP_LIGHT: [f64; 8] =
    [97.473, 91.649, 83.701, 65.472, 50.994, 34.130, 23.403, 15.113];
const FALLBACK_CHROMA_DARK: [f64; 8] =
    [8.183, 9.319, 11.358, 10.280, 9.918, 8.992, 8.060, 6.096];
const FALLBACK_CHROMA_LIGHT: [f64; 8] =
    [4.146, 5.852, 7.462, 7.957, 7.781, 8.346, 6.548, 6.357];
const FALLBACK_ACCENT_DARK_L: [f64; 8] =
    [60.921, 73.987, 76.726, 73.706, 72.682, 64.193, 65.946, 52.440];
const FALLBACK_ACCENT_LIGHT_L: [f64; 8] =
    [50.611, 60.353, 63.530, 57.653, 58.025, 50.761, 50.724, 49.363];
const FALLBACK_ACCENT_BAND_DARK: (f64, f64) = (21.002, 31.432);
const FALLBACK_ACCENT_BAND_LIGHT: (f64, f64) = (22.814, 33.122);
const FALLBACK_FOLLOW_DARK: [f64; 8] =
    [1.0, 0.0283, 0.0461, 0.0378, 0.0018, 0.0020, 0.0062, 0.0002];
const FALLBACK_FOLLOW_LIGHT: [f64; 8] =
    [1.0, 0.0296, 0.0255, 0.0571, 0.0821, 0.0001, 0.0, 0.0];
const FALLBACK_HUE_DARK: [f64; 8] =
    [20.584, 66.411, 99.884, 139.519, 199.389, 251.815, 327.905, 23.725];
const FALLBACK_HUE_LIGHT: [f64; 8] =
    [22.649, 53.962, 85.052, 143.456, 199.849, 261.277, 327.351, 25.894];
const FALLBACK_SPREAD_DARK: [f64; 8] =
    [34.361, 43.776, 90.332, 35.531, 46.879, 39.100, 40.049, 40.346];
const FALLBACK_SPREAD_LIGHT: [f64; 8] =
    [18.044, 36.057, 45.306, 35.704, 42.596, 33.731, 33.310, 45.639];
const FALLBACK_LEAN_DARK: [f64; 8] =
    [22.914, 16.736, 16.736, 19.817, 26.213, 26.213, 26.339, 21.343];
const FALLBACK_LEAN_LIGHT: [f64; 8] =
    [15.656, 15.545, 15.545, 28.196, 28.196, 30.714, 27.649, 14.034];
const FALLBACK_ACCENT_FOLLOW_DARK: f64 = 0.126;
const FALLBACK_ACCENT_FOLLOW_LIGHT: f64 = 0.539;
const FALLBACK_BG_RANGE_DARK: (f64, f64) = (0.0, 24.313);
const FALLBACK_BG_RANGE_LIGHT: (f64, f64) = (74.995, 100.0);
const FALLBACK_BG_GAP_DARK: f64 = 1.970;
const FALLBACK_BG_GAP_LIGHT: f64 = -2.977;

impl Corpus {
    fn fallback(dark: bool) -> Self {
        Corpus {
            ramp: if dark { FALLBACK_RAMP_DARK } else { FALLBACK_RAMP_LIGHT },
            chroma: if dark { FALLBACK_CHROMA_DARK } else { FALLBACK_CHROMA_LIGHT },
            accent_l: if dark { FALLBACK_ACCENT_DARK_L } else { FALLBACK_ACCENT_LIGHT_L },
            hue: if dark { FALLBACK_HUE_DARK } else { FALLBACK_HUE_LIGHT },
            hue_spread: if dark { FALLBACK_SPREAD_DARK } else { FALLBACK_SPREAD_LIGHT },
            lean: if dark { FALLBACK_LEAN_DARK } else { FALLBACK_LEAN_LIGHT },
            accent_band: if dark { FALLBACK_ACCENT_BAND_DARK } else { FALLBACK_ACCENT_BAND_LIGHT },
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
        // Accent chroma floor for the hue statistics only: the corpus p25, i.e.
        // the bottom of the band this generator will ever emit. Below it a slot
        // is grey and its hue is noise.
        let accent_hue_floor = Self::fallback(dark).accent_band.0;
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
                    neutral_l[i].push(col.j);
                    neutral_c[i].push(col.m);
                }
            }
            // Per slot, not pooled: base08 being red is a fact about base08.
            // Hues accumulate as unit vectors — an average of 350 and 10 is 0,
            // not 180, and hue is the one axis where the arithmetic mean lies.
            for (i, hex) in accents(s).iter().enumerate() {
                if let Some(col) = parse_oklch(hex) {
                    accent_l.push(col.j);
                    accent_c.push(col.m);
                    slot_l[i].push(col.j);
                    // A near-grey accent has no meaningful hue and would drag the
                    // mean toward whatever its rounding noise points at.
                    if col.m >= accent_hue_floor {
                        let r = col.h.to_radians();
                        hue_vec[i].0 += r.cos();
                        hue_vec[i].1 += r.sin();
                        hue_n[i] += 1.0;
                        slot_hues[i].push(col.h);
                    }
                }
            }

            // A scheme only joins the regression if it is complete: a slope built
            // from rows of differing length is not a slope.
            let neutral: Option<Vec<f64>> =
                neutrals(s).iter().map(|h| parse_oklch(h).map(|c| c.j)).collect();
            let accent: Option<Vec<f64>> =
                accents(s).iter().map(|h| parse_oklch(h).map(|c| c.j)).collect();
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
            // p75 across the whole library, and this one resisted being derived.
            //
            // Two attempts, both measured, both worse. Otsu splits the library
            // into greyscale and tinted and takes the tinted median — but Otsu
            // lands where "exactly zero" ends, not where "greyscale-ish" ends,
            // so its upper class is nearly everything and its median halves the
            // tint (base02 chroma 0.0212 -> 0.0121). Taking that class's upper
            // fence instead, on the theory that a ceiling wants a top edge, was
            // worse again (0.0105): `neutral_chroma` normalises by the ladder's
            // PEAK, so only the ladder's shape survives, and the fence moves the
            // peak onto a different slot and flattens the base02/base03 hump
            // that the corpus and the named schemes both show.
            //
            // So the 75 stays, with its reason stated: half this library is
            // deliberately greyscale, and a median would report that half's
            // answer to a question this generator is not asking.
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
                if let Some(sd) = circular_sd(&slot_hues[i]) {
                    hue_spread[i] = sd;
                }
            }
            if let Some(l) = percentile(&mut slot_l[i], 0.50) {
                slot_lightness[i] = l;
            }
        }

        // Lean budgets fall out of the hues once they are known.
        let mut lean = fb.lean;
        for (i, l) in lean.iter_mut().enumerate() {
            let nearest = (0..8)
                .filter(|j| *j != i && *j != 7 && !(i == 7 && *j == 0))
                .map(|j| hue_delta(hue[i], hue[j]))
                .fold(f64::MAX, f64::min);
            if nearest.is_finite() {
                *l = nearest / 2.0;
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
            lean,
            // A low quantile, and deliberately not a Tukey fence. These three
            // are CONSTRAINTS — the least separation a scheme may have and still
            // work — not summaries of a population, and a fence answers the
            // opposite question. The gap distribution has a real negative tail
            // (schemes that genuinely invert base00/base01), so its lower fence
            // sits inside that tail and permits exactly the inversions the bound
            // exists to forbid: measured, 17 of 30 wallpapers inverted. A low
            // quantile says "what nearly every real scheme manages", which is
            // the question actually being asked.
            // Quartiles, and deliberately not Otsu. Accent chroma is NOT two
            // populations — it is one broad unimodal spread, so Otsu splits it
            // down the middle of a single mode and reports the midpoint as a
            // boundary. Measured: that floor lands at 0.134, which drags every
            // washed-out wallpaper up to vivid accents and destroys the muting
            // this band exists to provide. The IQR describes the bulk of one
            // population, which is what this is.
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

/// Euclidean distance in Oklab between two (L, chroma, hue) colours.
fn oklab_dist(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let p = |(l, c, h): (f64, f64, f64)| {
        let r = h.to_radians();
        [l, c * r.cos(), c * r.sin()]
    };
    let (p, q) = (p(a), p(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// Circular standard deviation of a set of hues, in degrees.
///
/// The textbook dispersion for angles, and it replaces a p90 of absolute
/// deviations. Both describe "how far this slot wanders", but the p90 needed
/// somebody to choose 90, while the circular SD falls out of the resultant
/// length with nothing to pick.
fn circular_sd(hues: &[f64]) -> Option<f64> {
    if hues.len() < 2 {
        return None;
    }
    let n = hues.len() as f64;
    let (mut c, mut s) = (0.0, 0.0);
    for h in hues {
        let r = h.to_radians();
        c += r.cos();
        s += r.sin();
    }
    let r_bar = (c * c + s * s).sqrt() / n;
    if r_bar <= 0.0 || r_bar >= 1.0 {
        return None;
    }
    Some((-2.0 * r_bar.ln()).sqrt().to_degrees())
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
/// CAM16-UCS. `None` on anything that is not six hex digits.
fn parse_oklch(hex: &str) -> Option<Jmh> {
    cam16::hex_to_ucs(hex, conditions())
}

/// The lightness below which an image is a dark scheme's, under `--auto`.
///
/// Halfway between where the two corpora actually put their backgrounds, rather
/// than halfway up the scale — light schemes cluster far nearer their end than
/// dark ones do, so the midpoint of the scale is not the midpoint of practice.
fn polarity_split() -> f64 {
    let dark = Corpus::measure(true).ramp[0];
    let light = Corpus::measure(false).ramp[0];
    (dark + light) / 2.0
}

/// The viewing conditions everything in this module is measured under. One set,
/// built once: two colours compared under different conditions are not comparable.
fn conditions() -> &'static Conditions {
    use std::sync::OnceLock;
    static VC: OnceLock<Conditions> = OnceLock::new();
    VC.get_or_init(Conditions::srgb)
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
fn background_anchor(clusters: &[Cluster]) -> Option<&Cluster> {
    clusters.iter().max_by(|a, b| a.weight.total_cmp(&b.weight))
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
    // Area-weighted, for the same reason as `tint_hue`: weighting a chroma
    // average by chroma reports the most saturated candidate rather than the
    // typical one, and the whole point of this number is to be typical.
    let (mut sum, mut weight) = (0.0, 0.0);
    for c in clusters.iter().filter(|c| c.chroma() >= floor) {
        sum += c.chroma() * c.weight;
        weight += c.weight;
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
///
/// swaybg is the fallback. It has no query command, so its image is read back
/// out of its own argv in /proc — see `swaybg_wallpaper`.
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

    let (found, image) = swaybg_wallpaper();
    if let Some(path) = image {
        return Ok(path);
    }
    if found {
        tried.push("swaybg");
    }

    if tried.is_empty() {
        bail!("no wallpaper daemon found (looked for awww, swww, swaybg) — pass an image path instead");
    }
    bail!(
        "{} is installed but reported no image — set a wallpaper first, or pass a path",
        tried.join("/")
    )
}

/// The image a running swaybg was started with: (any swaybg running, its image).
///
/// swaybg takes the wallpaper on the command line and never tells anyone again —
/// there is no IPC and no query. But its argv is still in /proc/<pid>/cmdline, so
/// that is the query: the first process whose comm is `swaybg` and whose
/// `-i`/`--image` names a file that exists. A relative path is resolved against
/// the process's own cwd, not ours. `-i` is repeatable (one per `-o` output);
/// the first one wins, for the same reason the first awww line does.
///
/// No /proc (not Linux) just means no swaybg found.
fn swaybg_wallpaper() -> (bool, Option<PathBuf>) {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return (false, None);
    };
    let mut found = false;
    for entry in entries.flatten() {
        let dir = entry.path();
        let is_swaybg = std::fs::read_to_string(dir.join("comm"))
            .map(|c| c.trim() == "swaybg")
            .unwrap_or(false);
        if !is_swaybg {
            continue;
        }
        found = true;
        let Ok(raw) = std::fs::read(dir.join("cmdline")) else {
            continue;
        };
        let argv: Vec<String> = raw
            .split(|&b| b == 0)
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        let mut args = argv.iter().skip(1);
        while let Some(arg) = args.next() {
            let value = match arg.as_str() {
                "-i" | "--image" => args.next().cloned(),
                a => a.strip_prefix("--image=").map(str::to_owned),
            };
            let Some(value) = value else { continue };
            let mut path = PathBuf::from(value);
            if path.is_relative() {
                if let Ok(cwd) = std::fs::read_link(dir.join("cwd")) {
                    path = cwd.join(path);
                }
            }
            if path.is_file() {
                return (true, Some(path));
            }
        }
    }
    (found, None)
}

/// Box-downsample to `edge` on the longest side, averaging in LINEAR LIGHT.
///
/// The averaging space is the whole point, and doing it in gamma-encoded sRGB —
/// which is what `image::thumbnail` and essentially every naive resize does — is
/// a real error, not a rounding one. sRGB bytes are perceptually spaced, not
/// linear in photons: averaging 0 and 255 as bytes gives 128, which is 0.216 in
/// linear light, where the true average of black and white is 0.500. Mid-tones
/// come out around half as bright as the image actually is.
///
/// It matters here more than in most resizes, because the images that trigger it
/// are exactly the ones this generator gets pointed at: anything dithered, hatched
/// or finely detailed — an engraving, pixel art, a halftone — is a field of pure
/// black and pure white pixels whose correct average is a mid grey. In gamma
/// space that field reads as near-black, which then drives the anchor, the mean
/// lightness and every cluster centroid.
///
/// So: linearise on the way in, average there, and hand Oklab the linear values
/// directly rather than re-encoding just to decode again.
fn downsample_linear(img: &image::RgbImage, edge: u32, profile: &Profile) -> Vec<[f64; 3]> {
    let lut = &profile.trc;

    let (w, h) = img.dimensions();
    let scale = (edge as f64 / w.max(h) as f64).min(1.0);
    let (tw, th) = (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    );

    let mut acc = vec![[0.0f64; 3]; (tw * th) as usize];
    let mut count = vec![0u32; (tw * th) as usize];
    for (x, y, px) in img.enumerate_pixels() {
        // Source pixel -> target cell. Every source pixel lands in exactly one
        // cell, so this is a true box filter with no pixel counted twice.
        let tx = ((x as u64 * tw as u64) / w as u64).min(tw as u64 - 1) as u32;
        let ty = ((y as u64 * th as u64) / h as u64).min(th as u64 - 1) as u32;
        let i = (ty * tw + tx) as usize;
        acc[i][0] += lut[0][px[0] as usize];
        acc[i][1] += lut[1][px[1] as usize];
        acc[i][2] += lut[2][px[2] as usize];
        count[i] += 1;
    }

    acc.iter()
        .zip(count.iter())
        .filter(|(_, n)| **n > 0)
        .map(|(a, n)| {
            let n = *n as f64;
            [a[0] / n, a[1] / n, a[2] / n]
        })
        .collect()
}

/// Decode an image and whatever colour profile it carries.
fn decode_with_profile(path: &Path) -> Result<(image::RgbImage, Profile)> {
    let reader = image::ImageReader::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot identify {}", path.display()))?;
    let mut decoder = reader
        .into_decoder()
        .with_context(|| format!("cannot decode {} as an image", path.display()))?;
    // Best-effort: a profile that is absent, unreadable, or not matrix/TRC just
    // means the image is treated as sRGB, which is what it was before.
    let profile = decoder
        .icc_profile()
        .ok()
        .flatten()
        .and_then(|raw| Profile::parse(&raw))
        .unwrap_or_else(Profile::srgb);
    let img = image::DynamicImage::from_decoder(decoder)
        .with_context(|| format!("cannot decode {} as an image", path.display()))?;
    Ok((img.to_rgb8(), profile))
}

/// Decode, downscale, and cluster the image in Oklab.
///
/// Seeding is farthest-point rather than random, so the same wallpaper always
/// produces the same scheme. A generator you cannot reproduce is a generator you
/// cannot debug.
fn cluster_image(path: &Path) -> Result<Vec<Cluster>> {
    // Read the image's own colour profile before reading its pixels. Averaging
    // and clustering are only meaningful once the bytes have been turned into
    // actual colour, and which colour a byte is depends on the profile.
    let (img, profile) = decode_with_profile(path)?;
    let small = downsample_linear(&img, SAMPLE_EDGE, &profile);

    let points: Vec<[f64; 3]> = small
        .iter()
        .map(|&[r, g, b]| {
            // Linear profile RGB -> XYZ D65. The matrix is linear, so applying it
            // after averaging is identical to applying it before, and this way it
            // runs once per sample instead of once per source pixel.
            let m = &profile.to_xyz_d65;
            // Profile RGB -> XYZ D65 -> appearance. XYZ carries Y in 0..100
            // for CAM16, so the matrix product is scaled to match.
            let c = cam16::xyz_to_ucs(
                100.0 * (m[0][0] * r + m[0][1] * g + m[0][2] * b),
                100.0 * (m[1][0] * r + m[1][1] * g + m[1][2] * b),
                100.0 * (m[2][0] * r + m[2][1] * g + m[2][2] * b),
                conditions(),
            );
            let h = c.h.to_radians();
            [c.j, c.m * h.cos(), c.m * h.sin()]
        })
        .collect();

    if points.is_empty() {
        bail!("{} decoded to zero pixels", path.display());
    }

    // True Oklab distance. Oklab exists so that Euclidean distance in it
    // approximates perceived difference; the previous version multiplied the a
    // and b differences by 2.5 to stop k-means splitting the image along
    // lightness alone, which bought more colourful palettes by throwing away the
    // one property the space was chosen for. It was also the last unexplained
    // number in the clustering path.
    //
    // It is no longer needed. The background now comes from the dominant
    // cluster rather than from whichever cluster happened to be most saturated,
    // and accents are filtered by a measured chroma floor and fall back to
    // corpus hues, so a palette of mostly-neutral clusters degrades correctly
    // instead of degrading into eight identical greys.
    let dist2 = |p: &[f64; 3], q: &[f64; 3]| {
        let dl = p[0] - q[0];
        let da = p[1] - q[1];
        let db = p[2] - q[2];
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

fn hex(l: f64, c: f64, h: f64) -> String {
    cam16::ucs_to_hex(Jmh { j: l, m: c, h }, conditions())
}

/// Build a scheme from an image, write it into the schemes directory, and hand
/// back what `Scheme::load_file` makes of it.
pub fn scheme_from_image(path: &Path, polarity: Polarity, raw: bool) -> Result<(Scheme, PathBuf)> {
    let clusters = cluster_image(path)?;

    let mean_l: f64 = clusters.iter().map(|c| c.l * c.weight).sum();
    let dark = match polarity {
        Polarity::Dark => true,
        Polarity::Light => false,
        Polarity::Auto => mean_l < polarity_split(),
    };

    let corpus = Corpus::measure(dark);
    let (lo, hi) = corpus.accent_band;
    let accent_c = image_accent_chroma(&clusters, lo).unwrap_or(lo).clamp(lo, hi);

    // The wallpaper's dominant colour becomes the background — ALL of it, hue and
    // chroma as well as lightness.
    //
    // These used to come from different places, and it showed. Lightness came
    // from the dominant cluster while hue came from `tint_hue`, a vector mean
    // over the whole image — and a vector mean is chroma-weighted whether you
    // want it or not, because a and b carry chroma as their magnitude. On a dusk
    // photograph whose largest region was 38% of the frame at chroma 0.012 and
    // whose brightest was 10% at chroma 0.10, the small bright region outvoted
    // the large dark one roughly eight to one, and base00 came out as the dark
    // region's lightness wearing the bright region's hue: a warm olive over an
    // image whose dominant area is a cool near-neutral.
    //
    // Hue is not lightness-independent to look at, either. Hue 92 at L 0.93 is
    // the cream glow actually in that photograph; hue 92 at L 0.25 is olive. The
    // old pairing produced a colour that appears nowhere in the wallpaper.
    //
    // One region, one colour. The dominant cluster answers all three questions
    // and they cannot disagree.
    let dominant = background_anchor(&clusters);
    let tint = dominant.map(|c| c.hue()).unwrap_or(250.0);
    let bg_chroma = dominant.map(|c| c.chroma()).unwrap_or(0.0);
    let anchor = dominant
        .map(|c| c.l)
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

    let chroma_ramp = neutral_chroma(&corpus.chroma, bg_chroma);
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
        // Same "is this a colour at all" test as everywhere else in this file:
        // the corpus's own accent floor. It used to be a separate hardcoded 0.02,
        // a second opinion on the same question with no reason to differ.
        let mut pool: Vec<&Cluster> = clusters.iter().filter(|c| c.chroma() >= lo).collect();
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

    let mut taken: Vec<((f64, f64, f64), (f64, f64, f64))> = Vec::with_capacity(SLOTS.len());

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
                        // Lean toward the image, bounded by the slot's own
                        // budget — never past the midpoint to its neighbour.
                        let budget = corpus.lean[idx];
                        let pull = hue_signed(target, h).clamp(-budget, budget);
                        (target + pull).rem_euclid(360.0)
                    }
                    None => target,
                }
            }
        };

        // Collision guard, now in BOTH modes. It used to be skipped under the
        // default, on the theory that a wallpaper with one strong hue should be
        // allowed to give eight shades of it, pywal-style.
        //
        // That theory does not survive meeting one. A dusk photograph has eight
        // clusters above the pool's chroma floor and every one of them sits
        // between hue 87 and 103, so all eight accents came out the same olive —
        // base08 through base0F within 16° of each other. A palette where the
        // error colour and the string colour are the same colour has not carried
        // the wallpaper's character, it has lost seven slots, and no terminal is
        // readable in it.
        //
        // A slot whose hue is already taken falls back to its own, which is what
        // the guard was always for. The image still sets every accent it has a
        // distinct colour for; it just cannot claim the same colour eight times.
        // base0F stays exempt — brown is a near-neighbour of red by convention
        // and separated by lightness instead.
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

        // The guard needs the finished colour, not just its hue — which is why
        // chroma and lightness are settled first.
        if slot != "base0F" {
            // The bar for each pair is what that PAIR would be if both slots took
            // their corpus hue — not one global distance.
            //
            // A global floor cannot work here, and the corpus's own p25 (0.0609)
            // proves it: base09 and base0A sit 28.3° apart in the corpus, which
            // at accent chroma is a distance of 0.0445, so orange and yellow
            // violate that floor while being exactly where they belong. A floor
            // no correct palette can satisfy fires on every scheme and forces
            // fallbacks that make things worse.
            //
            // Asking instead "are these two closer than they would be if the
            // image had not touched them" is self-consistent: it is satisfied by
            // construction the moment a slot retreats to its own hue, so the
            // retreat always terminates, and it still catches the real failure —
            // eight accents dragged onto one hue are far closer than the layout
            // would ever put them.
            let canonical = (l, chroma, target);
            let clashes = |cand: (f64, f64, f64)| {
                taken.iter().any(|(their, their_canon): &_| {
                    oklab_dist(*their, cand) < oklab_dist(*their_canon, canonical)
                })
            };
            if clashes((l, chroma, hue)) {
                // First retreat: this slot's hue, leaned toward the image, so a
                // recovered accent still belongs to this wallpaper.
                let budget = corpus.lean[idx];
                let pull = hue_signed(target, hue).clamp(-budget, budget);
                hue = (target + pull).rem_euclid(360.0);
                if clashes((l, chroma, hue)) {
                    hue = target;
                }
            }
            taken.push(((l, chroma, hue), canonical));
        }
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
