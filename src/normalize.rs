//! Material-You-style perceptual regularization of a Base16/Base24 palette.
//!
//! The idea, borrowed from Material You's HCT tonal palettes: a colour's *hue*
//! carries the scheme's identity, while its *lightness* is what decides whether
//! text is readable and whether two accents look distinct. So we keep the hue
//! the scheme author chose and snap lightness (and chroma) onto fixed per-slot
//! targets. Two accents at the same lightness can then only differ by hue,
//! which is exactly what makes a palette read as "standardized".
//!
//! Work happens in Oklch — a perceptually uniform space, so a fixed step in L
//! looks like the same step everywhere on the wheel (unlike HSL, where yellow
//! at 50% "lightness" is far brighter than blue at 50%).
//!
//! Everything is gated behind `normalize.enabled` in coat.yaml and scaled by
//! `strength`, so 0.0 is the untouched scheme and 1.0 is fully standardized.
//! Mid-range values keep a scheme recognizable while pulling in its outliers.

use crate::config::NormalizeConfig;
use crate::scheme::Scheme;
use std::sync::OnceLock;

// ── Global config ───────────────────────────────────────────────────────────
// Normalization is a property of how coat renders colour *everywhere* — applying
// it in some code paths but not others would make `coat list` previews lie about
// what `coat apply` will write. So it is set once from main() and consulted from
// the single funnel every scheme load passes through (`Scheme::load_file`).
// If it is never set (config failed to parse), every call below is a no-op.

static CFG: OnceLock<NormalizeConfig> = OnceLock::new();

pub fn init(cfg: NormalizeConfig) {
    let _ = CFG.set(cfg);
}

// ── sRGB ↔ Oklab/Oklch ──────────────────────────────────────────────────────
// Oklab matrices from Björn Ottosson's reference implementation.

#[derive(Debug, Clone, Copy)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    /// Hue in degrees, 0..360.
    pub h: f64,
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Parse "RRGGBB" (with or without '#') into 0..1 sRGB components.
fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some((
        ((v >> 16) & 0xFF) as f64 / 255.0,
        ((v >> 8) & 0xFF) as f64 / 255.0,
        (v & 0xFF) as f64 / 255.0,
    ))
}

fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
    let q = |x: f64| -> u8 { (x.clamp(0.0, 1.0) * 255.0).round() as u8 };
    format!("{:02X}{:02X}{:02X}", q(r), q(g), q(b))
}

fn rgb_to_oklch(r: f64, g: f64, b: f64) -> Oklch {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let (l_, m_, s_) = (l.cbrt(), m.cbrt(), s.cbrt());

    let ok_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let ok_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let ok_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    let c = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let mut h = ok_b.atan2(ok_a).to_degrees();
    if h < 0.0 {
        h += 360.0;
    }
    Oklch { l: ok_l, c, h }
}

/// Oklch → sRGB. May return components outside 0..1 (out of gamut) — callers
/// that care should go through `oklch_to_hex`, which maps back into gamut.
fn oklch_to_rgb_raw(col: Oklch) -> (f64, f64, f64) {
    let hr = col.h.to_radians();
    let ok_a = col.c * hr.cos();
    let ok_b = col.c * hr.sin();

    let l_ = col.l + 0.3963377774 * ok_a + 0.2158037573 * ok_b;
    let m_ = col.l - 0.1055613458 * ok_a - 0.0638541728 * ok_b;
    let s_ = col.l - 0.0894841775 * ok_a - 1.2914855480 * ok_b;

    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
}

fn in_gamut(rgb: (f64, f64, f64)) -> bool {
    const EPS: f64 = 1e-4;
    let (r, g, b) = rgb;
    (-EPS..=1.0 + EPS).contains(&r)
        && (-EPS..=1.0 + EPS).contains(&g)
        && (-EPS..=1.0 + EPS).contains(&b)
}

/// Convert to hex, gamut-mapping by *reducing chroma* rather than clipping RGB.
/// Clipping each channel independently shifts hue (a too-saturated blue clips to
/// a purple); walking chroma down holds hue and lightness steady, which is the
/// whole point of doing this in a perceptual space.
fn oklch_to_hex(col: Oklch) -> String {
    let direct = oklch_to_rgb_raw(col);
    if in_gamut(direct) {
        let (r, g, b) = direct;
        return rgb_to_hex(r, g, b);
    }
    // Binary search the largest in-gamut chroma at this L and H.
    let (mut lo, mut hi) = (0.0_f64, col.c);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if in_gamut(oklch_to_rgb_raw(Oklch { c: mid, ..col })) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (r, g, b) = oklch_to_rgb_raw(Oklch { c: lo, ..col });
    rgb_to_hex(r, g, b)
}

// ── Contrast ────────────────────────────────────────────────────────────────

fn relative_luminance(r: f64, g: f64, b: f64) -> f64 {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG 2.x contrast ratio, 1.0..21.0.
fn contrast_ratio(a: &str, b: &str) -> f64 {
    let (Some(x), Some(y)) = (hex_to_rgb(a), hex_to_rgb(b)) else {
        return 21.0; // unparseable: don't let the floor logic fight a bad value
    };
    let la = relative_luminance(x.0, x.1, x.2);
    let lb = relative_luminance(y.0, y.1, y.2);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

// ── Tonal targets ───────────────────────────────────────────────────────────

/// Target lightness for base00..base07, darkest background → lightest foreground.
const RAMP_DARK: [f64; 8] = [0.17, 0.235, 0.30, 0.42, 0.57, 0.76, 0.87, 0.95];
/// Light schemes invert the ramp: base00 is the *lightest* slot there.
const RAMP_LIGHT: [f64; 8] = [0.97, 0.93, 0.87, 0.74, 0.58, 0.36, 0.24, 0.15];

/// Accent lightness. Accents must sit clear of the background, so a dark scheme
/// pushes them bright and a light scheme pushes them deep.
const ACCENT_L_DARK: f64 = 0.72;
const ACCENT_L_LIGHT: f64 = 0.52;
/// Shared chroma ceiling — stops one accent from shouting over the others.
const ACCENT_C: f64 = 0.13;

/// base0F is the odd one out (brown / "deprecated" slot): muted on purpose.
const BASE0F_C: f64 = 0.075;

/// Smallest lightness gap kept between adjacent ramp slots. Without this, a high
/// `ramp_contrast` shoves the top of the ramp into the 1.0 ceiling and base06 /
/// base07 collapse into the same white.
const RAMP_MIN_STEP: f64 = 0.022;

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Stretch the ramp away from mid-grey: blacks blacker, whites whiter.
///
/// `k = 1.0` is the tuned baseline. Above that, every slot moves away from 0.5
/// proportionally, so the two ends hit 0.0/1.0 first — which is the point. The
/// clamp would flatten the ends into each other, so a second pass re-imposes a
/// minimum step, walking inward from whichever end got pinned.
fn expand_ramp(base: [f64; 8], k: f64) -> [f64; 8] {
    let mut t = base;
    for v in t.iter_mut() {
        *v = (0.5 + (*v - 0.5) * k).clamp(0.0, 1.0);
    }

    // `base` is monotonic; keep whichever direction it runs (dark ramps ascend,
    // light ramps descend) and re-separate anything the clamp squashed.
    let ascending = base[7] > base[0];
    if ascending {
        for i in 1..8 {
            if t[i] < t[i - 1] + RAMP_MIN_STEP {
                t[i] = t[i - 1] + RAMP_MIN_STEP;
            }
        }
        // Overflowed the top: push the whole tail back down from 1.0.
        if t[7] > 1.0 {
            t[7] = 1.0;
            for i in (0..7).rev() {
                if t[i] > t[i + 1] - RAMP_MIN_STEP {
                    t[i] = t[i + 1] - RAMP_MIN_STEP;
                }
            }
        }
    } else {
        for i in 1..8 {
            if t[i] > t[i - 1] - RAMP_MIN_STEP {
                t[i] = t[i - 1] - RAMP_MIN_STEP;
            }
        }
        if t[7] < 0.0 {
            t[7] = 0.0;
            for i in (0..7).rev() {
                if t[i] < t[i + 1] + RAMP_MIN_STEP {
                    t[i] = t[i + 1] + RAMP_MIN_STEP;
                }
            }
        }
    }
    for v in t.iter_mut() {
        *v = v.clamp(0.0, 1.0);
    }
    t
}

/// Shortest-path interpolation around the hue circle.
fn lerp_hue(a: f64, b: f64, t: f64) -> f64 {
    let mut d = b - a;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    (a + d * t).rem_euclid(360.0)
}

/// Push hues apart until neighbours are at least `min_sep` degrees apart.
///
/// This is the "every colour is decently distinct" guarantee. Relaxation on a
/// circle: repeatedly find neighbouring pairs that are too close and nudge both
/// outward. Damped and iteration-capped so it always terminates, and it simply
/// gives up (leaving hues as-is) if the colours cannot all fit — with 7 accents
/// that needs min_sep > 51°, which no sane config sets.
fn repel_hues(hues: &mut [f64], min_sep: f64) {
    let n = hues.len();
    if n < 2 || min_sep <= 0.0 || min_sep * n as f64 >= 360.0 {
        return;
    }
    for _ in 0..64 {
        // Sort indices by hue so "neighbour" means neighbour on the circle.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| hues[a].partial_cmp(&hues[b]).unwrap());

        let mut moved = false;
        for k in 0..n {
            let i = order[k];
            let j = order[(k + 1) % n];
            let mut gap = hues[j] - hues[i];
            if gap < 0.0 {
                gap += 360.0;
            }
            if gap < min_sep {
                let push = (min_sep - gap) / 2.0 * 0.5; // damped
                hues[i] = (hues[i] - push).rem_euclid(360.0);
                hues[j] = (hues[j] + push).rem_euclid(360.0);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

// ── The pass ────────────────────────────────────────────────────────────────

/// Move one colour toward (target_l, target_c), keeping its hue, blended by
/// `strength`. Empty slots (base16 schemes leave base10..base17 blank) pass
/// through untouched.
/// Lightness and chroma blend independently. Tone is what makes a palette
/// legible and uniform; chroma is a big part of what makes a scheme recognizable
/// as *itself*. Splitting the two lets a config standardize brightness fully
/// while leaving saturation close to whatever the scheme author chose.
fn retone(hex: &str, target_l: f64, target_c: f64, l_strength: f64, c_strength: f64) -> String {
    let Some((r, g, b)) = hex_to_rgb(hex) else {
        return hex.to_string();
    };
    let src = rgb_to_oklch(r, g, b);
    // A near-greyscale source has a meaningless hue; don't invent chroma for it.
    let tc = if src.c < 0.012 { src.c } else { target_c };
    oklch_to_hex(Oklch {
        l: lerp(src.l, target_l, l_strength),
        c: lerp(src.c, tc, c_strength),
        h: src.h,
    })
}

/// Lift `fg` away from `bg` until they clear `floor` (WCAG ratio), or until we
/// run out of room. Runs *after* the strength blend, so it is a hard guarantee
/// rather than something a low strength can water down.
fn enforce_contrast(fg: &str, bg: &str, floor: f64) -> String {
    if floor <= 1.0 || contrast_ratio(fg, bg) >= floor {
        return fg.to_string();
    }
    let (Some((r, g, b)), Some(bgc)) = (hex_to_rgb(fg), hex_to_rgb(bg)) else {
        return fg.to_string();
    };
    let bg_l = rgb_to_oklch(bgc.0, bgc.1, bgc.2).l;
    let src = rgb_to_oklch(r, g, b);
    // Move away from the background: brighten on dark, darken on light.
    let dir = if src.l >= bg_l { 1.0 } else { -1.0 };

    let mut best = fg.to_string();
    for step in 1..=60 {
        let l = (src.l + dir * 0.01 * step as f64).clamp(0.0, 1.0);
        let cand = oklch_to_hex(Oklch { l, ..src });
        best = cand;
        if contrast_ratio(&best, bg) >= floor {
            break;
        }
        if l <= 0.0 || l >= 1.0 {
            break;
        }
    }
    best
}

/// Apply the configured normalization to a freshly loaded scheme, in place.
/// No-op unless `normalize.enabled` is set.
pub fn apply(s: &mut Scheme) {
    let Some(cfg) = CFG.get() else { return };
    if !cfg.enabled {
        return;
    }
    // `strength` drives hue/chroma (the scheme's identity); `lightness_strength`
    // drives tone (legibility) and defaults to following it. Only bail when both
    // are zero — a config may standardize tone alone.
    let strength = cfg.strength.clamp(0.0, 1.0);
    let l_strength = cfg.lightness_strength.unwrap_or(strength).clamp(0.0, 1.0);
    if strength <= 0.0 && l_strength <= 0.0 {
        return;
    }

    let dark = s.is_dark();
    let base_ramp = if dark { RAMP_DARK } else { RAMP_LIGHT };
    let ramp = expand_ramp(base_ramp, cfg.ramp_contrast.max(0.0));
    let accent_l = cfg
        .accent_lightness
        .unwrap_or(if dark { ACCENT_L_DARK } else { ACCENT_L_LIGHT })
        .clamp(0.0, 1.0);
    let accent_c = cfg.accent_chroma.max(0.0);

    // ── 1. The greyscale ramp (base00..base07) ──────────────────────────────
    if cfg.ramp {
        let slots: [&mut String; 8] = [
            &mut s.base00,
            &mut s.base01,
            &mut s.base02,
            &mut s.base03,
            &mut s.base04,
            &mut s.base05,
            &mut s.base06,
            &mut s.base07,
        ];
        for (i, slot) in slots.into_iter().enumerate() {
            // Ramp slots keep whatever faint tint the scheme gave them.
            let c = hex_to_rgb(slot)
                .map(|(r, g, b)| rgb_to_oklch(r, g, b).c)
                .unwrap_or(0.0);
            *slot = retone(slot, ramp[i], c, l_strength, strength);
        }

        // base24's extra backgrounds sit just past base00, same direction as the
        // ramp travels away from the foreground.
        let step = if dark { -0.045 } else { 0.030 };
        if !s.base10.is_empty() {
            s.base10 = retone(&s.base10, (ramp[0] + step).clamp(0.0, 1.0), 0.0, l_strength, strength);
        }
        if !s.base11.is_empty() {
            s.base11 = retone(
                &s.base11,
                (ramp[0] + step * 2.0).clamp(0.0, 1.0),
                0.0,
                l_strength,
                strength,
            );
        }
    }

    // ── 2. Accents (base08..base0E) to a common tone + chroma ───────────────
    {
        let accents: [&mut String; 7] = [
            &mut s.base08,
            &mut s.base09,
            &mut s.base0a,
            &mut s.base0b,
            &mut s.base0c,
            &mut s.base0d,
            &mut s.base0e,
        ];
        for slot in accents {
            *slot = retone(slot, accent_l, accent_c, l_strength, strength);
        }
    }
    // base0F (brown) stays deliberately duller and a touch darker — scaled off
    // the configured chroma so raising `accent_chroma` lifts it proportionally.
    let base0f_c = BASE0F_C * (accent_c / ACCENT_C);
    s.base0f = retone(&s.base0f, accent_l * 0.86, base0f_c, l_strength, strength);

    // ── 3. Hue repulsion — the "distinct" guarantee ─────────────────────────
    if cfg.min_hue_sep > 0.0 {
        let names: [&str; 7] = ["08", "09", "0a", "0b", "0c", "0d", "0e"];
        let current: Vec<String> = vec![
            s.base08.clone(),
            s.base09.clone(),
            s.base0a.clone(),
            s.base0b.clone(),
            s.base0c.clone(),
            s.base0d.clone(),
            s.base0e.clone(),
        ];
        let parsed: Vec<Option<Oklch>> = current
            .iter()
            .map(|h| hex_to_rgb(h).map(|(r, g, b)| rgb_to_oklch(r, g, b)))
            .collect();

        let mut hues: Vec<f64> = parsed.iter().filter_map(|p| p.map(|c| c.h)).collect();
        if hues.len() == 7 {
            let before = hues.clone();
            repel_hues(&mut hues, cfg.min_hue_sep);
            for (i, name) in names.iter().enumerate() {
                let Some(col) = parsed[i] else { continue };
                // Respect `strength` here too: partial normalization should mean
                // partial separation, not a hard snap.
                let h = lerp_hue(before[i], hues[i], strength);
                let hex = oklch_to_hex(Oklch { h, ..col });
                match *name {
                    "08" => s.base08 = hex,
                    "09" => s.base09 = hex,
                    "0a" => s.base0a = hex,
                    "0b" => s.base0b = hex,
                    "0c" => s.base0c = hex,
                    "0d" => s.base0d = hex,
                    _ => s.base0e = hex,
                }
            }
        }
    }

    // ── 4. base24 bright accents track their base counterparts ──────────────
    // Spec order: base12 red, base13 yellow, base14 green, base15 cyan,
    // base16 blue, base17 purple.
    let bright_l = if dark {
        (accent_l + 0.10).min(0.97)
    } else {
        (accent_l - 0.10).max(0.10)
    };
    for slot in [
        &mut s.base12,
        &mut s.base13,
        &mut s.base14,
        &mut s.base15,
        &mut s.base16_color,
        &mut s.base17,
    ] {
        if !slot.is_empty() {
            *slot = retone(slot, bright_l, accent_c, l_strength, strength);
        }
    }

    // ── 5. Hard contrast floor for body text ────────────────────────────────
    if cfg.contrast_floor > 1.0 {
        let bg = s.base00.clone();
        s.base05 = enforce_contrast(&s.base05, &bg, cfg.contrast_floor);
        // base04 is comment/secondary text — hold it to a lower bar so it stays
        // visibly dimmer than base05 rather than being dragged up to match.
        let secondary = (cfg.contrast_floor * 0.6).max(3.0);
        s.base04 = enforce_contrast(&s.base04, &bg, secondary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(hex: &str) -> String {
        let (r, g, b) = hex_to_rgb(hex).unwrap();
        oklch_to_hex(rgb_to_oklch(r, g, b))
    }

    #[test]
    fn oklch_roundtrip_is_lossless_enough() {
        for hex in ["000000", "FFFFFF", "212121", "A6A6A6", "D35A5F", "91BEB6"] {
            assert_eq!(roundtrip(hex), hex, "roundtrip drifted for {hex}");
        }
    }

    #[test]
    fn out_of_gamut_chroma_is_mapped_not_clipped() {
        // Absurd chroma at a mid lightness must still produce a valid sRGB hex
        // whose hue is close to what was asked for.
        let want = Oklch { l: 0.6, c: 0.9, h: 250.0 };
        let hex = oklch_to_hex(want);
        let (r, g, b) = hex_to_rgb(&hex).unwrap();
        let got = rgb_to_oklch(r, g, b);
        assert!((got.h - 250.0).abs() < 3.0, "hue drifted to {}", got.h);
        assert!(got.c < 0.9, "chroma was not reduced");
    }

    #[test]
    fn contrast_floor_lifts_low_contrast_foreground() {
        // The exact case that motivated this: "Later This Evening" base05/base00.
        let lifted = enforce_contrast("A6A6A6", "212121", 7.0);
        assert!(
            contrast_ratio(&lifted, "212121") >= 7.0,
            "got {lifted} at ratio {}",
            contrast_ratio(&lifted, "212121")
        );
    }

    #[test]
    fn contrast_floor_leaves_compliant_pairs_alone() {
        assert_eq!(enforce_contrast("FFFFFF", "000000", 7.0), "FFFFFF");
    }

    #[test]
    fn hue_repulsion_separates_crowded_accents() {
        let mut hues = vec![10.0, 12.0, 14.0, 200.0, 202.0, 300.0, 302.0];
        repel_hues(&mut hues, 20.0);
        let mut sorted = hues.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for w in sorted.windows(2) {
            assert!(
                w[1] - w[0] >= 19.0,
                "hues {:?} still crowded in {:?}",
                w,
                sorted
            );
        }
    }

    #[test]
    fn lightness_and_chroma_blend_independently() {
        // Full tonal snap, zero chroma snap: lightness must reach the target
        // while chroma stays where the source had it.
        let src = "6A8F5C";
        let out = retone(src, 0.80, 0.30, 1.0, 0.0);
        let (r, g, b) = hex_to_rgb(&out).unwrap();
        let got = rgb_to_oklch(r, g, b);
        let (sr, sg, sb) = hex_to_rgb(src).unwrap();
        let orig = rgb_to_oklch(sr, sg, sb);
        assert!((got.l - 0.80).abs() < 0.02, "lightness missed: {}", got.l);
        assert!(
            (got.c - orig.c).abs() < 0.02,
            "chroma moved: {} -> {}",
            orig.c,
            got.c
        );
    }

    #[test]
    fn zero_lightness_strength_preserves_tone() {
        let src = "6A8F5C";
        let (sr, sg, sb) = hex_to_rgb(src).unwrap();
        let orig = rgb_to_oklch(sr, sg, sb);
        let out = retone(src, 0.95, 0.13, 0.0, 1.0);
        let (r, g, b) = hex_to_rgb(&out).unwrap();
        assert!((rgb_to_oklch(r, g, b).l - orig.l).abs() < 0.02);
    }

    #[test]
    fn ramp_expansion_darkens_blacks_and_brightens_whites() {
        let out = expand_ramp(RAMP_DARK, 1.4);
        assert!(out[0] < RAMP_DARK[0], "base00 did not get darker");
        assert!(out[7] > RAMP_DARK[7], "base07 did not get brighter");
    }

    #[test]
    fn ramp_expansion_never_collapses_adjacent_slots() {
        // Absurd contrast pins both ends; every slot must still be distinct.
        for k in [1.0, 1.5, 2.0, 4.0, 10.0] {
            for ramp in [RAMP_DARK, RAMP_LIGHT] {
                let out = expand_ramp(ramp, k);
                for i in 1..8 {
                    let gap = (out[i] - out[i - 1]).abs();
                    assert!(
                        gap >= RAMP_MIN_STEP - 1e-9,
                        "k={k} collapsed slots {} and {i}: {:?}",
                        i - 1,
                        out
                    );
                }
                assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "out of range: {out:?}");
            }
        }
    }

    #[test]
    fn ramp_expansion_preserves_direction() {
        assert!(expand_ramp(RAMP_DARK, 1.6)[7] > expand_ramp(RAMP_DARK, 1.6)[0]);
        assert!(expand_ramp(RAMP_LIGHT, 1.6)[7] < expand_ramp(RAMP_LIGHT, 1.6)[0]);
    }

    #[test]
    fn ramp_expansion_is_identity_at_one() {
        for ramp in [RAMP_DARK, RAMP_LIGHT] {
            for (got, want) in expand_ramp(ramp, 1.0).iter().zip(ramp.iter()) {
                assert!((got - want).abs() < 1e-9, "{got} != {want}");
            }
        }
    }

    #[test]
    fn repulsion_gives_up_rather_than_spinning_when_infeasible() {
        let mut hues = vec![0.0, 1.0, 2.0];
        repel_hues(&mut hues, 200.0); // 3 * 200 > 360 — impossible
        assert_eq!(hues, vec![0.0, 1.0, 2.0]);
    }
}
