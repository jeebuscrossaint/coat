//! CAM16-UCS, the colour space this generator measures and reasons in.
//!
//! Oklab was the previous choice and it is a good one — cheap, well behaved, far
//! better than Lab. Its known weakness is hue linearity in the blues: a ramp of
//! increasing chroma at a fixed blue hue bends visibly toward purple, so "the
//! same hue, more saturated" is not the same hue. That matters here more than it
//! would in most code, because this file's whole job is deciding that two colours
//! share a hue, holding a hue constant while moving lightness, and measuring how
//! far a corpus hue wanders.
//!
//! CAM16 is a colour appearance model rather than a colour space: it takes
//! viewing conditions as input and reports what a colour looks like under them.
//! CAM16-UCS is its uniform form, built so Euclidean distance tracks perceived
//! difference. It is the current CIE recommendation and it fixes the blue
//! problem.
//!
//! Implemented from the specification (Li, Luo et al., 2017), not from another
//! themer. The scales differ from Oklab and that is expected: J' runs 0..100 and
//! M' is an openended colourfulness rather than a 0..0.4 chroma. Nothing in coat
//! hardcodes those magnitudes any more — the corpus is measured in whatever space
//! is in use — so the swap does not strand a pile of constants.

/// Viewing conditions. These are the standard sRGB display assumptions: a D65
/// white, a mid-grey background, and an average surround.
///
/// They are inputs to the model, not taste — CAM16 cannot answer "what does this
/// look like" without being told where it is being looked at. Changing them
/// changes every appearance value consistently, which is the point.
pub struct Conditions {
    aw: f64,
    nbb: f64,
    ncb: f64,
    c: f64,
    nc: f64,
    fl: f64,
    fl_root: f64,
    z: f64,
    n: f64,
    d_rgb: [f64; 3],
}

/// CAT16 chromatic adaptation matrix, and its inverse.
const M16: [[f64; 3]; 3] = [
    [0.401288, 0.650173, -0.051461],
    [-0.250268, 1.204414, 0.045854],
    [-0.002079, 0.048952, 0.953127],
];
const M16_INV: [[f64; 3]; 3] = [
    [1.8620678, -1.0112547, 0.1491867],
    [0.3875265, 0.6214474, -0.0089739],
    [-0.0158415, -0.0341229, 1.0499644],
];

/// D65, scaled to Y = 100.
const WHITE: [f64; 3] = [95.047, 100.0, 108.883];

fn mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// The post-adaptation compressive nonlinearity, signed so it handles the
/// out-of-gamut negatives that show up mid-search during gamut mapping.
fn adapt(x: f64) -> f64 {
    let p = (x.abs() / 100.0).powf(0.42);
    x.signum() * 400.0 * p / (p + 27.13) + 0.1
}

fn unadapt(x: f64) -> f64 {
    let a = (x - 0.1).abs();
    x.signum() * 100.0 * (27.13 * a / (400.0 - a)).powf(1.0 / 0.42)
}

impl Conditions {
    /// Standard sRGB viewing conditions.
    ///
    /// Adapting luminance 11.726 cd/m² and a background of L* 50 are the values
    /// the CAM16 literature and every implementation that ships use for a display
    /// viewed normally; "average" surround (c = 0.69) is the unconstrained case.
    pub fn srgb() -> Conditions {
        let la: f64 = 11.725676537;
        let yb: f64 = 18.418651851;
        let (f, c, nc): (f64, f64, f64) = (1.0, 0.69, 1.0);

        let k = 1.0 / (5.0 * la + 1.0);
        let k4 = k * k * k * k;
        let fl = k4 * la + 0.1 * (1.0 - k4) * (1.0 - k4) * (5.0 * la).cbrt();
        let n = yb / WHITE[1];
        let z = 1.48 + n.sqrt();
        let nbb = 0.725 * n.powf(-0.2);

        // Full adaptation (the "discounting the illuminant" case), deliberately.
        //
        // CAM16's degree-of-adaptation formula would give 0.845 here, modelling a
        // viewer only partly adapted to the white they are looking at. That is
        // the right model when a colour is specified under one white and viewed
        // under another. It is the wrong model here: the wallpaper, the scheme
        // and the display are all D65, so there is no white-point difference to
        // adapt across, and applying a partial transform anyway manufactures a
        // tint rather than describing one.
        //
        // It is not academic. At D = 0.845 a neutral grey comes back with
        // colourfulness 0.66 and a definite hue — small, but this generator's
        // entire failure mode is a small phantom hue getting amplified into a
        // whole desktop. At D = 1 a grey is exactly neutral, which is what it is.
        let _ = f;
        let d = 1.0;
        let rgb_w = mul(&M16, WHITE);
        let mut d_rgb = [0.0; 3];
        for i in 0..3 {
            d_rgb[i] = d * WHITE[1] / rgb_w[i] + 1.0 - d;
        }
        let rgb_aw = [
            adapt(d_rgb[0] * rgb_w[0] * fl / 100.0 * 100.0),
            adapt(d_rgb[1] * rgb_w[1] * fl / 100.0 * 100.0),
            adapt(d_rgb[2] * rgb_w[2] * fl / 100.0 * 100.0),
        ];
        let aw = (2.0 * rgb_aw[0] + rgb_aw[1] + 0.05 * rgb_aw[2] - 0.305) * nbb;

        Conditions {
            aw,
            nbb,
            ncb: nbb,
            c,
            nc,
            fl,
            fl_root: fl.powf(0.25),
            z,
            n,
            d_rgb,
        }
    }
}

/// A colour as CAM16-UCS lightness, colourfulness and hue.
///
/// Deliberately the same shape as the Oklch it replaces — `j` where `l` was,
/// `m` where `c` was, `h` in degrees either way — so the code that reasons about
/// ramps and hues did not have to change to follow it.
#[derive(Clone, Copy, Debug)]
pub struct Jmh {
    pub j: f64,
    pub m: f64,
    pub h: f64,
}

/// XYZ (D65, Y in 0..100) to CAM16-UCS.
pub fn xyz_to_ucs(x: f64, y: f64, z: f64, vc: &Conditions) -> Jmh {
    let rgb = mul(&M16, [x, y, z]);
    let mut ra = [0.0; 3];
    for i in 0..3 {
        ra[i] = adapt(vc.d_rgb[i] * rgb[i] * vc.fl / 100.0 * 100.0);
    }

    let a = ra[0] - 12.0 * ra[1] / 11.0 + ra[2] / 11.0;
    let b = (ra[0] + ra[1] - 2.0 * ra[2]) / 9.0;
    let h_rad = b.atan2(a);
    let h = h_rad.to_degrees().rem_euclid(360.0);

    let e_t = 0.25 * ((h_rad + 2.0).cos() + 3.8);
    let big_a = (2.0 * ra[0] + ra[1] + 0.05 * ra[2] - 0.305) * vc.nbb;
    if big_a <= 0.0 {
        return Jmh { j: 0.0, m: 0.0, h };
    }
    let j = 100.0 * (big_a / vc.aw).powf(vc.c * vc.z);

    let t = (50000.0 / 13.0 * vc.nc * vc.ncb * e_t * (a * a + b * b).sqrt())
        / (ra[0] + ra[1] + 21.0 * ra[2] / 20.0 + 0.305);
    let alpha = t.powf(0.9) * (1.64 - 0.29f64.powf(vc.n)).powf(0.73);
    let big_c = alpha * (j / 100.0).sqrt();
    let big_m = big_c * vc.fl_root;

    // UCS compression: J' and M'.
    Jmh {
        j: 1.7 * j / (1.0 + 0.007 * j),
        m: (1.0 + 0.0228 * big_m).ln() / 0.0228,
        h,
    }
}

/// CAM16-UCS back to XYZ (D65, Y in 0..100).
pub fn ucs_to_xyz(col: Jmh, vc: &Conditions) -> [f64; 3] {
    let j = col.j / (1.7 - 0.007 * col.j);
    let big_m = (col.m * 0.0228).exp_m1() / 0.0228;
    let big_c = big_m / vc.fl_root;

    if j <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let alpha = big_c / (j / 100.0).sqrt();
    let t = (alpha / (1.64 - 0.29f64.powf(vc.n)).powf(0.73)).powf(1.0 / 0.9);

    let h_rad = col.h.to_radians();
    let e_t = 0.25 * ((h_rad + 2.0).cos() + 3.8);
    let big_a = vc.aw * (j / 100.0).powf(1.0 / (vc.c * vc.z));

    let p1 = e_t * (50000.0 / 13.0) * vc.nc * vc.ncb;
    let p2 = big_a / vc.nbb + 0.305;

    let (sin_h, cos_h) = (h_rad.sin(), h_rad.cos());
    let (a, b) = if t == 0.0 {
        (0.0, 0.0)
    } else {
        let gamma = 23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * cos_h + 108.0 * t * sin_h);
        (gamma * cos_h, gamma * sin_h)
    };

    let ra = [
        (460.0 * p2 + 451.0 * a + 288.0 * b) / 1403.0,
        (460.0 * p2 - 891.0 * a - 261.0 * b) / 1403.0,
        (460.0 * p2 - 220.0 * a - 6300.0 * b) / 1403.0,
    ];
    let mut rgb_c = [0.0; 3];
    for i in 0..3 {
        rgb_c[i] = unadapt(ra[i]) / vc.fl * 100.0 / 100.0;
    }
    let mut rgb = [0.0; 3];
    for i in 0..3 {
        rgb[i] = rgb_c[i] / vc.d_rgb[i];
    }
    mul(&M16_INV, rgb)
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

/// Linear-light sRGB (0..1) to XYZ with Y in 0..100.
pub fn linear_rgb_to_xyz(r: f64, g: f64, b: f64) -> [f64; 3] {
    [
        100.0 * (0.4123907993 * r + 0.3575843394 * g + 0.1804807884 * b),
        100.0 * (0.2126390059 * r + 0.7151686788 * g + 0.0721923154 * b),
        100.0 * (0.0193308187 * r + 0.1191947798 * g + 0.9505321522 * b),
    ]
}

fn xyz_to_linear_rgb(xyz: [f64; 3]) -> (f64, f64, f64) {
    let (x, y, z) = (xyz[0] / 100.0, xyz[1] / 100.0, xyz[2] / 100.0);
    (
        3.2409699419 * x - 1.5373831776 * y - 0.4986107603 * z,
        -0.9692436363 * x + 1.8759675015 * y + 0.0415550574 * z,
        0.0556300797 * x - 0.2039769589 * y + 1.0569715142 * z,
    )
}

/// Parse "RRGGBB" (with or without `#`) into CAM16-UCS.
pub fn hex_to_ucs(hex: &str, vc: &Conditions) -> Option<Jmh> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    let f = |shift: u32| srgb_to_linear(((v >> shift) & 0xFF) as f64 / 255.0);
    let xyz = linear_rgb_to_xyz(f(16), f(8), f(0));
    Some(xyz_to_ucs(xyz[0], xyz[1], xyz[2], vc))
}

fn in_gamut((r, g, b): (f64, f64, f64)) -> bool {
    let ok = |c: f64| c >= -0.0001 && c <= 1.0001;
    ok(r) && ok(g) && ok(b)
}

/// CAM16-UCS to "RRGGBB", gamut-mapped by reducing COLOURFULNESS rather than
/// clipping channels.
///
/// Clipping each channel independently shifts hue — an oversaturated blue clips
/// to purple — which would throw away the reason for being in an appearance
/// model at all. Walking M' down holds hue and lightness fixed.
pub fn ucs_to_hex(col: Jmh, vc: &Conditions) -> String {
    let to_rgb = |c: Jmh| xyz_to_linear_rgb(ucs_to_xyz(c, vc));
    let direct = to_rgb(col);
    let (r, g, b) = if in_gamut(direct) {
        direct
    } else {
        let (mut lo, mut hi) = (0.0_f64, col.m);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(to_rgb(Jmh { m: mid, ..col })) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        to_rgb(Jmh { m: lo, ..col })
    };
    let q = |x: f64| -> u8 { (linear_to_srgb(x).clamp(0.0, 1.0) * 255.0).round() as u8 };
    format!("{:02X}{:02X}{:02X}", q(r), q(g), q(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_to_xyz(r: u8, g: u8, b: u8) -> [f64; 3] {
        let f = |c: u8| {
            let c = c as f64 / 255.0;
            100.0 * if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        let (r, g, b) = (f(r), f(g), f(b));
        [
            0.4123907993 * r + 0.3575843394 * g + 0.1804807884 * b,
            0.2126390059 * r + 0.7151686788 * g + 0.0721923154 * b,
            0.0193308187 * r + 0.1191947798 * g + 0.9505321522 * b,
        ]
    }

    /// The inverse has to actually invert, across the whole cube and not just
    /// the easy middle of it.
    #[test]
    fn ucs_round_trips_through_xyz() {
        let vc = Conditions::srgb();
        let mut worst = 0.0f64;
        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(15) {
                for b in (0..=255).step_by(15) {
                    let xyz = srgb_to_xyz(r, g, b);
                    let ucs = xyz_to_ucs(xyz[0], xyz[1], xyz[2], &vc);
                    let back = ucs_to_xyz(ucs, &vc);
                    for i in 0..3 {
                        worst = worst.max((xyz[i] - back[i]).abs());
                    }
                }
            }
        }
        assert!(worst < 0.01, "worst XYZ round-trip error {worst}");
    }

    /// Neutrals must come back with no colourfulness, or every grey wallpaper
    /// picks up a phantom hue.
    #[test]
    fn greys_have_no_colourfulness() {
        let vc = Conditions::srgb();
        for v in [0u8, 32, 64, 128, 192, 255] {
            let xyz = srgb_to_xyz(v, v, v);
            let ucs = xyz_to_ucs(xyz[0], xyz[1], xyz[2], &vc);
            assert!(ucs.m < 0.05, "grey {v} reported colourfulness {}", ucs.m);
        }
    }

    /// Hex in, hex out, unchanged — the path every generated colour takes.
    #[test]
    fn hex_round_trips() {
        let vc = Conditions::srgb();
        let mut worst = 0i32;
        for hex in [
            "000000", "FFFFFF", "1E1E2E", "CDD6F4", "F38BA8", "A6E3A1", "89B4FA",
            "FAB387", "7F849C", "313244", "94E2D5", "CBA6F7",
        ] {
            let c = hex_to_ucs(hex, &vc).unwrap();
            let back = ucs_to_hex(c, &vc);
            let v = |s: &str| i32::from_str_radix(s, 16).unwrap();
            for i in 0..3 {
                let d = (v(&hex[i * 2..i * 2 + 2]) - v(&back[i * 2..i * 2 + 2])).abs();
                worst = worst.max(d);
            }
        }
        assert!(worst <= 1, "worst channel drift {worst}");
    }

    /// White should land at the top of the lightness scale.
    #[test]
    fn white_is_full_lightness() {
        let vc = Conditions::srgb();
        let xyz = srgb_to_xyz(255, 255, 255);
        let ucs = xyz_to_ucs(xyz[0], xyz[1], xyz[2], &vc);
        assert!((ucs.j - 100.0).abs() < 1.0, "white J' = {}", ucs.j);
    }
}
