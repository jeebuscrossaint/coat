#include "hct.h"
#include <math.h>

// sRGB to linear RGB (gamma 2.4)
static double srgb_to_linear(double val) {
    if (val <= 0.04045) {
        return val / 12.92;
    }
    return pow((val + 0.055) / 1.055, 2.4);
}

// Linear RGB to sRGB (inverse gamma)
static double linear_to_srgb(double val) {
    if (val <= 0.0031308) {
        return val * 12.92;
    }
    return 1.055 * pow(val, 1.0 / 2.4) - 0.055;
}

// Convert Y (from XYZ, 0-100 range) to L* (LAB lightness, 0-100)
// This is the "tone" in Material You HCT
static double y_to_lstar(double y) {
    y = y / 100.0;  // Normalize
    if (y <= 216.0 / 24389.0) {  // 0.008856...
        return y * 24389.0 / 27.0;
    }
    return 116.0 * pow(y, 1.0 / 3.0) - 16.0;
}

// Convert L* back to Y
static double lstar_to_y(double lstar) {
    if (lstar <= 8.0) {
        return lstar * 27.0 / 24389.0 * 100.0;
    }
    return pow((lstar + 16.0) / 116.0, 3.0) * 100.0;
}

// LAB helper: f(t) function for XYZ→LAB conversion
static double lab_f(double t) {
    const double delta = 6.0 / 29.0;  // 0.206896...
    const double delta_cubed = delta * delta * delta;
    
    if (t > delta_cubed) {
        return pow(t, 1.0 / 3.0);
    }
    return t / (3.0 * delta * delta) + 4.0 / 29.0;
}

// LAB helper: inverse f(t) function for LAB→XYZ conversion
static double lab_f_inv(double t) {
    const double delta = 6.0 / 29.0;
    
    if (t > delta) {
        return pow(t, 3.0);
    }
    return 3.0 * delta * delta * (t - 4.0 / 29.0);
}

// XYZ to LAB (D65 white point)
LAB xyz_to_lab(XYZ xyz) {
    // D65 white point
    const double Xn = 95.047;
    const double Yn = 100.0;
    const double Zn = 108.883;
    
    double fx = lab_f(xyz.x / Xn);
    double fy = lab_f(xyz.y / Yn);
    double fz = lab_f(xyz.z / Zn);
    
    LAB lab;
    lab.l = 116.0 * fy - 16.0;
    lab.a = 500.0 * (fx - fy);
    lab.b = 200.0 * (fy - fz);
    
    return lab;
}

// LAB to XYZ (D65 white point)
XYZ lab_to_xyz(LAB lab) {
    // D65 white point
    const double Xn = 95.047;
    const double Yn = 100.0;
    const double Zn = 108.883;
    
    double fy = (lab.l + 16.0) / 116.0;
    double fx = lab.a / 500.0 + fy;
    double fz = fy - lab.b / 200.0;
    
    XYZ xyz;
    xyz.x = Xn * lab_f_inv(fx);
    xyz.y = Yn * lab_f_inv(fy);
    xyz.z = Zn * lab_f_inv(fz);
    
    return xyz;
}

// RGB to LAB (convenience function)
LAB rgb_to_lab(RGB rgb) {
    XYZ xyz = rgb_to_xyz(rgb);
    return xyz_to_lab(xyz);
}

// LAB to RGB (convenience function)
RGB lab_to_rgb(LAB lab) {
    XYZ xyz = lab_to_xyz(lab);
    return xyz_to_rgb(xyz);
}

// RGB (0-255) to XYZ (D65, 0-100 range)
XYZ rgb_to_xyz(RGB rgb) {
    // Linearize
    double r = srgb_to_linear(rgb.r / 255.0);
    double g = srgb_to_linear(rgb.g / 255.0);
    double b = srgb_to_linear(rgb.b / 255.0);
    
    // sRGB to XYZ matrix (D65 white point)
    XYZ xyz;
    xyz.x = (r * 0.4124564 + g * 0.3575761 + b * 0.1804375) * 100.0;
    xyz.y = (r * 0.2126729 + g * 0.7151522 + b * 0.0721750) * 100.0;
    xyz.z = (r * 0.0193339 + g * 0.1191920 + b * 0.9503041) * 100.0;
    
    return xyz;
}

// XYZ to RGB
RGB xyz_to_rgb(XYZ xyz) {
    // Scale from 0-100 to 0-1
    double x = xyz.x / 100.0;
    double y = xyz.y / 100.0;
    double z = xyz.z / 100.0;
    
    // XYZ to linear RGB (inverse matrix)
    double r = x *  3.2404542 + y * -1.5371385 + z * -0.4985314;
    double g = x * -0.9692660 + y *  1.8760108 + z *  0.0415560;
    double b = x *  0.0556434 + y * -0.2040259 + z *  1.0572252;
    
    // Apply gamma
    r = linear_to_srgb(r);
    g = linear_to_srgb(g);
    b = linear_to_srgb(b);
    
    // Convert to 0-255 and clamp
    RGB rgb;
    rgb.r = (int)fmax(0, fmin(255, round(r * 255.0)));
    rgb.g = (int)fmax(0, fmin(255, round(g * 255.0)));
    rgb.b = (int)fmax(0, fmin(255, round(b * 255.0)));
    
    return rgb;
}

// RGB to HCT
HCT rgb_to_hct(RGB rgb) {
    // Get XYZ for  proper tone (L*)
    XYZ xyz = rgb_to_xyz(rgb);
    
    // Convert to HSL for hue/chroma
    double r = rgb.r / 255.0;
    double g = rgb.g / 255.0;
    double b = rgb.b / 255.0;
    
    double max = fmax(r, fmax(g, b));
    double min = fmin(r, fmin(g, b));
    double delta = max - min;
    
    HCT hct;
    
    // Tone is L* from LAB (perceptual lightness)
    hct.t = y_to_lstar(xyz.y);
    
    // Hue (0-360)
    hct.h = 0;
    if (delta != 0) {
        if (max == r) {
            hct.h = 60.0 * fmod((g - b) / delta, 6.0);
        } else if (max == g) {
            hct.h = 60.0 * ((b - r) / delta + 2.0);
        } else {
            hct.h = 60.0 * ((r - g) / delta + 4.0);
        }
    }
    if (hct.h < 0) hct.h += 360.0;
    
    // Chroma approximation from saturation
    double lightness = (max + min) / 2.0;
    double saturation = 0;
    if (delta != 0) {
        saturation = delta / (1.0 - fabs(2.0 * lightness - 1.0));
    }
    hct.c = saturation * 120.0;
    
    return hct;
}

// HCT to RGB
RGB hct_to_rgb(HCT hct) {
    // Use L* for lightness in HSL approximation
    double lightness = hct.t / 100.0;
    
    // Map chroma to saturation
    double saturation = fmin(1.0, hct.c / 120.0);
    
    // HSL to RGB
    double c = (1.0 - fabs(2.0 * lightness - 1.0)) * saturation;
    double x = c * (1.0 - fabs(fmod(hct.h / 60.0, 2.0) - 1.0));
    double m = lightness - c / 2.0;
    
    double r_prime, g_prime, b_prime;
    
    if (hct.h >= 0 && hct.h < 60) {
        r_prime = c; g_prime = x; b_prime = 0;
    } else if (hct.h >= 60 && hct.h < 120) {
        r_prime = x; g_prime = c; b_prime = 0;
    } else if (hct.h >= 120 && hct.h < 180) {
        r_prime = 0; g_prime = c; b_prime = x;
    } else if (hct.h >= 180 && hct.h < 240) {
        r_prime = 0; g_prime = x; b_prime = c;
    } else if (hct.h >= 240 && hct.h < 300) {
        r_prime = x; g_prime = 0; b_prime = c;
    } else {
        r_prime = c; g_prime = 0; b_prime = x;
    }
    
    RGB rgb;
    rgb.r = (int)round((r_prime + m) * 255.0);
    rgb.g = (int)round((g_prime + m) * 255.0);
    rgb.b = (int)round((b_prime + m) * 255.0);
    
    // Clamp
    rgb.r = fmax(0, fmin(255, rgb.r));
    rgb.g = fmax(0, fmin(255, rgb.g));
    rgb.b = fmax(0, fmin(255, rgb.b));
    
    return rgb;
}

// Dummy implementations for unused CAM16 functions (kept for API compatibility)
CAM16 xyz_to_cam16(XYZ xyz) { (void)xyz; CAM16 cam = {0}; return cam; }
XYZ cam16_to_xyz(CAM16 cam) { (void)cam; XYZ xyz = {0}; return xyz; }
HCT cam16_to_hct(CAM16 cam) { (void)cam; HCT hct = {0}; return hct; }
CAM16 hct_to_cam16(HCT hct) { (void)hct; CAM16 cam = {0}; return cam; }
