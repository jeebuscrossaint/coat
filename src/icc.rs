//! Minimal ICC profile reading, for the one question `coat match` needs answered:
//! what do this image's bytes actually mean?
//!
//! The pipeline used to assume every wallpaper was sRGB. Most are, but a photo
//! off a modern phone or camera is routinely Display P3 or Adobe RGB, and reading
//! those bytes as sRGB is not a rounding error — the primaries are in different
//! places, so a saturated red lands at the wrong hue AND the wrong chroma, and
//! every accent derived from it inherits both. The profile is sitting in the
//! file; there is no reason to guess.
//!
//! Only matrix/TRC RGB profiles are handled, which is what display profiles are.
//! Anything else (LUT-based, CMYK, absent, malformed) returns `None` and the
//! caller falls back to sRGB — the same assumption as before, but now it is a
//! fallback rather than a premise.

/// A profile reduced to what a decoder needs: a per-channel tone curve, as a
/// 256-entry table over byte values, and a matrix taking linear profile RGB to
/// XYZ under D65.
pub struct Profile {
    pub trc: [[f64; 256]; 3],
    pub to_xyz_d65: [[f64; 3]; 3],
}

fn be_u32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?))
}
fn be_u16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
/// s15Fixed16Number — the fixed-point format ICC uses for everything numeric.
fn s15(b: &[u8], o: usize) -> Option<f64> {
    Some(i32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?) as f64 / 65536.0)
}

/// Locate a tag's payload by its four-character signature.
fn tag<'a>(data: &'a [u8], sig: &[u8; 4]) -> Option<&'a [u8]> {
    let count = be_u32(data, 128)? as usize;
    for i in 0..count {
        let e = 132 + i * 12;
        if data.get(e..e + 4)? == sig {
            let off = be_u32(data, e + 4)? as usize;
            let len = be_u32(data, e + 8)? as usize;
            return data.get(off..off.checked_add(len)?);
        }
    }
    None
}

/// One column of the RGB→XYZ matrix, from an `XYZType` colorant tag.
fn xyz_tag(data: &[u8], sig: &[u8; 4]) -> Option<[f64; 3]> {
    let t = tag(data, sig)?;
    if t.get(0..4)? != b"XYZ " {
        return None;
    }
    Some([s15(t, 8)?, s15(t, 12)?, s15(t, 16)?])
}

/// Decode a `curveType` or `parametricCurveType` into a 256-entry table.
///
/// The parametric forms are the ICC spec's own piecewise definitions; sRGB's own
/// curve is type 3, so a profile that merely restates sRGB round-trips exactly
/// rather than approximately.
fn trc_tag(data: &[u8], sig: &[u8; 4]) -> Option<[f64; 256]> {
    let t = tag(data, sig)?;
    let mut out = [0.0f64; 256];
    match t.get(0..4)? {
        b"curv" => {
            let n = be_u32(t, 8)? as usize;
            if n == 0 {
                // Identity: the channel is already linear.
                for (i, v) in out.iter_mut().enumerate() {
                    *v = i as f64 / 255.0;
                }
            } else if n == 1 {
                // u8Fixed8Number gamma.
                let g = be_u16(t, 12)? as f64 / 256.0;
                for (i, v) in out.iter_mut().enumerate() {
                    *v = (i as f64 / 255.0).powf(g);
                }
            } else {
                // Sampled curve: interpolate the table at each byte value.
                for (i, v) in out.iter_mut().enumerate() {
                    let x = (i as f64 / 255.0) * (n - 1) as f64;
                    let lo = x.floor() as usize;
                    let hi = (lo + 1).min(n - 1);
                    let f = x - lo as f64;
                    let a = be_u16(t, 12 + lo * 2)? as f64 / 65535.0;
                    let b = be_u16(t, 12 + hi * 2)? as f64 / 65535.0;
                    *v = a + (b - a) * f;
                }
            }
        }
        b"para" => {
            let kind = be_u16(t, 8)?;
            let p = |i: usize| s15(t, 12 + i * 4);
            let g = p(0)?;
            for (i, v) in out.iter_mut().enumerate() {
                let x = i as f64 / 255.0;
                *v = match kind {
                    0 => x.powf(g),
                    1 => {
                        let (a, b) = (p(1)?, p(2)?);
                        if x >= -b / a { (a * x + b).powf(g) } else { 0.0 }
                    }
                    2 => {
                        let (a, b, c) = (p(1)?, p(2)?, p(3)?);
                        if x >= -b / a { (a * x + b).powf(g) + c } else { c }
                    }
                    3 => {
                        let (a, b, c, d) = (p(1)?, p(2)?, p(3)?, p(4)?);
                        if x >= d { (a * x + b).powf(g) } else { c * x }
                    }
                    4 => {
                        let (a, b, c, d, e, f) = (p(1)?, p(2)?, p(3)?, p(4)?, p(5)?, p(6)?);
                        if x >= d { (a * x + b).powf(g) + e } else { c * x + f }
                    }
                    _ => return None,
                };
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Bradford adaptation, D50 → D65.
///
/// Needed because ICC's connection space is D50 by definition, so the colorant
/// tags describe the primaries under D50, while Oklab is defined from XYZ under
/// D65. Skipping this is a real error — it tints everything slightly warm.
const BRADFORD_D50_TO_D65: [[f64; 3]; 3] = [
    [0.9555766, -0.0230393, 0.0631636],
    [-0.0282895, 1.0099416, 0.0210077],
    [0.0122982, -0.0204830, 1.3299098],
];

impl Profile {
    /// Parse an embedded profile, or `None` if it is not a matrix/TRC RGB one.
    pub fn parse(data: &[u8]) -> Option<Profile> {
        // Header: data colour space must be RGB for the colorant tags to mean
        // what we are about to assume they mean.
        if data.len() < 132 || data.get(16..20)? != b"RGB " {
            return None;
        }
        let (r, g, b) = (
            xyz_tag(data, b"rXYZ")?,
            xyz_tag(data, b"gXYZ")?,
            xyz_tag(data, b"bXYZ")?,
        );
        let trc = [
            trc_tag(data, b"rTRC")?,
            trc_tag(data, b"gTRC")?,
            trc_tag(data, b"bTRC")?,
        ];
        // Columns are the primaries; rows are X, Y, Z.
        let d50 = [
            [r[0], g[0], b[0]],
            [r[1], g[1], b[1]],
            [r[2], g[2], b[2]],
        ];
        let mut to_xyz_d65 = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                to_xyz_d65[i][j] = (0..3).map(|k| BRADFORD_D50_TO_D65[i][k] * d50[k][j]).sum();
            }
        }
        // A profile whose matrix is degenerate describes nothing usable.
        let det = to_xyz_d65[0][0]
            * (to_xyz_d65[1][1] * to_xyz_d65[2][2] - to_xyz_d65[1][2] * to_xyz_d65[2][1])
            - to_xyz_d65[0][1]
                * (to_xyz_d65[1][0] * to_xyz_d65[2][2] - to_xyz_d65[1][2] * to_xyz_d65[2][0])
            + to_xyz_d65[0][2]
                * (to_xyz_d65[1][0] * to_xyz_d65[2][1] - to_xyz_d65[1][1] * to_xyz_d65[2][0]);
        if !det.is_finite() || det.abs() < 1e-9 {
            return None;
        }
        Some(Profile { trc, to_xyz_d65 })
    }

    /// sRGB, for images that carry no profile — the old premise, kept as the
    /// explicit fallback it should always have been.
    pub fn srgb() -> Profile {
        let mut t = [0.0f64; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let c = i as f64 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        Profile {
            trc: [t, t, t],
            to_xyz_d65: [
                [0.4123907993, 0.3575843394, 0.1804807884],
                [0.2126390059, 0.7151686788, 0.0721923154],
                [0.0193308187, 0.1191947798, 0.9505321522],
            ],
        }
    }
}
