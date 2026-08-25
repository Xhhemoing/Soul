//! Colour primitives: 8-bit sRGB, CIE L\*a\*b\* under D65, and CIEDE2000.
//!
//! Two conversions live here and they are kept apart on purpose. The CIEDE2000
//! implementation follows Sharma, Wu and Dalal (2005), whose published test
//! table is stated in Lab, so [`ciede2000`] takes [`Lab`] and never sees an
//! [`Rgb`]. The sRGB pipeline is a separate, separately tested step: pinning
//! the paper's table through a colour-space conversion would fold two error
//! budgets into one assertion and hide whichever of them broke.

use std::fmt;

/// An 8-bit sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse `#RRGGBB` or `RRGGBB`, case-insensitive.
    pub fn from_hex(hex: &str) -> Result<Self, HexError> {
        let body = hex.strip_prefix('#').unwrap_or(hex);
        if body.len() != 6 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(HexError {
                input: hex.to_owned(),
            });
        }
        let byte = |from: usize| u8::from_str_radix(&body[from..from + 2], 16).unwrap_or(0);
        Ok(Self::new(byte(0), byte(2), byte(4)))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Convert to CIE L\*a\*b\* with the D65 2° observer.
    pub fn to_lab(self) -> Lab {
        srgb_to_lab(self)
    }
}

/// `#RRGGBB` was expected and something else arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexError {
    pub input: String,
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a #RRGGBB colour", self.input)
    }
}

impl std::error::Error for HexError {}

/// A colour in CIE L\*a\*b\*.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Lab {
    pub const fn new(l: f64, a: f64, b: f64) -> Self {
        Self { l, a, b }
    }

    /// CIEDE2000 distance to another Lab colour, with all weighting factors 1.
    pub fn delta_e(self, other: Lab) -> f64 {
        ciede2000(self, other)
    }
}

/// CIE XYZ, normalised so that the D65 white point has `y == 1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// D65, 2° observer, the white point sRGB is defined against.
pub const WHITE_D65: Xyz = Xyz {
    x: 0.950_47,
    y: 1.0,
    z: 1.088_83,
};

/// Undo the sRGB transfer function for one channel given as 0.0..=1.0.
fn srgb_expand(channel: f64) -> f64 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB (IEC 61966-2-1) to CIE XYZ under D65.
pub fn srgb_to_xyz(rgb: Rgb) -> Xyz {
    let r = srgb_expand(f64::from(rgb.r) / 255.0);
    let g = srgb_expand(f64::from(rgb.g) / 255.0);
    let b = srgb_expand(f64::from(rgb.b) / 255.0);
    Xyz {
        x: 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b,
        y: 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b,
        z: 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b,
    }
}

/// The CIE lightness companding function, with its linear segment near zero.
fn lab_f(t: f64) -> f64 {
    const EPSILON: f64 = 216.0 / 24_389.0; // (6/29)^3
    const KAPPA: f64 = 24_389.0 / 27.0; // (29/3)^3
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

/// CIE XYZ (D65-relative) to CIE L\*a\*b\*.
pub fn xyz_to_lab(xyz: Xyz) -> Lab {
    let fx = lab_f(xyz.x / WHITE_D65.x);
    let fy = lab_f(xyz.y / WHITE_D65.y);
    let fz = lab_f(xyz.z / WHITE_D65.z);
    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

/// sRGB to CIE L\*a\*b\* under D65, 2°.
pub fn srgb_to_lab(rgb: Rgb) -> Lab {
    xyz_to_lab(srgb_to_xyz(rgb))
}

/// CIEDE2000 colour difference, `kL = kC = kH = 1`.
///
/// Straight from Sharma, Wu and Dalal, *The CIEDE2000 Color-Difference
/// Formula: Implementation Notes, Supplementary Test Data, and Mathematical
/// Observations* (Color Research and Application, 2005), including the three
/// discontinuity cases their notes call out: a hue angle of zero when the
/// chroma is zero, the hue difference that must wrap through ±180°, and the
/// mean hue that must not be averaged across the 0°/360° seam.
pub fn ciede2000(reference: Lab, sample: Lab) -> f64 {
    const POW25_7: f64 = 6_103_515_625.0; // 25^7

    let (l1, a1, b1) = (reference.l, reference.a, reference.b);
    let (l2, a2, b2) = (sample.l, sample.a, sample.b);

    let c1 = a1.hypot(b1);
    let c2 = a2.hypot(b2);
    let c_bar = (c1 + c2) / 2.0;
    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + POW25_7)).sqrt());

    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = a1p.hypot(b1);
    let c2p = a2p.hypot(b2);

    let h1p = hue_angle(a1p, b1);
    let h2p = hue_angle(a2p, b2);

    let delta_lp = l2 - l1;
    let delta_cp = c2p - c1p;

    // With either chroma at zero the hue difference is undefined; the notes fix
    // it at zero so that the term drops out instead of contributing noise.
    let delta_hp_deg = if c1p * c2p == 0.0 {
        0.0
    } else {
        let raw = h2p - h1p;
        if raw > 180.0 {
            raw - 360.0
        } else if raw < -180.0 {
            raw + 360.0
        } else {
            raw
        }
    };
    let delta_big_hp = 2.0 * (c1p * c2p).sqrt() * (delta_hp_deg.to_radians() / 2.0).sin();

    let l_bar_p = (l1 + l2) / 2.0;
    let c_bar_p = (c1p + c2p) / 2.0;

    let h_bar_p = if c1p * c2p == 0.0 {
        h1p + h2p
    } else {
        let sum = h1p + h2p;
        if (h1p - h2p).abs() <= 180.0 {
            sum / 2.0
        } else if sum < 360.0 {
            (sum + 360.0) / 2.0
        } else {
            (sum - 360.0) / 2.0
        }
    };

    let t = 1.0 - 0.17 * (h_bar_p - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_p).to_radians().cos()
        + 0.32 * (3.0 * h_bar_p + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_p - 63.0).to_radians().cos();

    let delta_theta = 30.0 * (-((h_bar_p - 275.0) / 25.0).powi(2)).exp();
    let c_bar_p7 = c_bar_p.powi(7);
    let r_c = 2.0 * (c_bar_p7 / (c_bar_p7 + POW25_7)).sqrt();
    let r_t = -(2.0 * delta_theta).to_radians().sin() * r_c;

    let l_offset = (l_bar_p - 50.0).powi(2);
    let s_l = 1.0 + (0.015 * l_offset) / (20.0 + l_offset).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_p;
    let s_h = 1.0 + 0.015 * c_bar_p * t;

    let term_l = delta_lp / s_l;
    let term_c = delta_cp / s_c;
    let term_h = delta_big_hp / s_h;

    (term_l * term_l + term_c * term_c + term_h * term_h + r_t * term_c * term_h).sqrt()
}

/// Hue angle in degrees over `[0, 360)`, defined as zero when both components
/// are zero rather than left to `atan2`'s sign conventions.
fn hue_angle(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        return 0.0;
    }
    let degrees = b.atan2(a).to_degrees();
    if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let colour = Rgb::from_hex("#1a2B3c").expect("valid hex");
        assert_eq!(colour, Rgb::new(0x1A, 0x2B, 0x3C));
        assert_eq!(colour.to_hex(), "#1A2B3C");
    }

    #[test]
    fn hex_rejects_junk() {
        assert!(Rgb::from_hex("#12345").is_err());
        assert!(Rgb::from_hex("#12345g").is_err());
        assert!(Rgb::from_hex("").is_err());
    }

    #[test]
    fn hue_angle_is_zero_at_the_origin() {
        assert_eq!(hue_angle(0.0, 0.0), 0.0);
        assert!((hue_angle(0.0, 1.0) - 90.0).abs() < 1e-12);
        assert!((hue_angle(-1.0, 0.0) - 180.0).abs() < 1e-12);
        assert!((hue_angle(0.0, -1.0) - 270.0).abs() < 1e-12);
    }
}
