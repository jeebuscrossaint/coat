//
// Created by amarnath on 2/27/26.
//

#include "../../include/material_you.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// HSL color structure for intermediate calculations
typedef struct {
    float h;  // Hue: 0-360
    float s;  // Saturation: 0-100
    float l;  // Lightness: 0-100
} HSL;

// Utility: Clamp a value between min and max
static float clamp(float value, float min, float max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

// Utility: Convert hex string to RGB values
static void hex_to_rgb(const char *hex, int *r, int *g, int *b) {
    const char *hex_str = hex;
    if (hex[0] == '#') hex_str++;
    
    unsigned int rgb;
    sscanf(hex_str, "%x", &rgb);
    *r = (rgb >> 16) & 0xFF;
    *g = (rgb >> 8) & 0xFF;
    *b = rgb & 0xFF;
}

// Utility: Convert RGB to HSL
static HSL rgb_to_hsl(int r, int g, int b) {
    HSL hsl;
    float r_norm = r / 255.0f;
    float g_norm = g / 255.0f;
    float b_norm = b / 255.0f;
    
    float max = fmaxf(r_norm, fmaxf(g_norm, b_norm));
    float min = fminf(r_norm, fminf(g_norm, b_norm));
    float delta = max - min;
    
    // Lightness
    hsl.l = (max + min) / 2.0f * 100.0f;
    
    if (delta == 0) {
        // Achromatic (gray)
        hsl.h = 0;
        hsl.s = 0;
    } else {
        // Saturation
        if (hsl.l < 50) {
            hsl.s = delta / (max + min) * 100.0f;
        } else {
            hsl.s = delta / (2.0f - max - min) * 100.0f;
        }
        
        // Hue
        if (max == r_norm) {
            hsl.h = 60.0f * fmodf((g_norm - b_norm) / delta, 6.0f);
        } else if (max == g_norm) {
            hsl.h = 60.0f * ((b_norm - r_norm) / delta + 2.0f);
        } else {
            hsl.h = 60.0f * ((r_norm - g_norm) / delta + 4.0f);
        }
        
        if (hsl.h < 0) hsl.h += 360.0f;
    }
    
    return hsl;
}

// Utility: HSL to RGB conversion helper
static float hsl_helper(float p, float q, float t) {
    if (t < 0) t += 1.0f;
    if (t > 1.0f) t -= 1.0f;
    if (t < 1.0f/6.0f) return p + (q - p) * 6.0f * t;
    if (t < 1.0f/2.0f) return q;
    if (t < 2.0f/3.0f) return p + (q - p) * (2.0f/3.0f - t) * 6.0f;
    return p;
}

// Utility: Convert HSL to RGB
static void hsl_to_rgb(HSL hsl, int *r, int *g, int *b) {
    float h = hsl.h / 360.0f;
    float s = hsl.s / 100.0f;
    float l = hsl.l / 100.0f;
    
    if (hsl.s == 0) {
        // Achromatic
        *r = *g = *b = (int)(l * 255.0f);
    } else {
        float q = l < 0.5f ? l * (1.0f + s) : l + s - l * s;
        float p = 2.0f * l - q;
        
        *r = (int)(hsl_helper(p, q, h + 1.0f/3.0f) * 255.0f);
        *g = (int)(hsl_helper(p, q, h) * 255.0f);
        *b = (int)(hsl_helper(p, q, h - 1.0f/3.0f) * 255.0f);
    }
    
    *r = clamp(*r, 0, 255);
    *g = clamp(*g, 0, 255);
    *b = clamp(*b, 0, 255);
}

// Utility: Convert RGB to hex string (always includes # prefix)
static void rgb_to_hex(int r, int g, int b, char *hex_out) {
    sprintf(hex_out, "#%02x%02x%02x", r, g, b);
}

// Material You color transformation for a single color
// Based on Material Design 3 HCT color system principles
static void transform_color(const char *hex_in, char *hex_out, MaterialYouConfig *config, float seed_hue) {
    int r, g, b;
    hex_to_rgb(hex_in, &r, &g, &b);
    
    HSL hsl = rgb_to_hsl(r, g, b);
    
    // Material You transformations:
    
    // 1. Saturation boost for more vibrant colors (Material You chroma ~36-120)
    if (hsl.s > 10) {  // Only boost colorful elements
        hsl.s *= config->saturation_boost;
        // Material You typically caps chroma, which translates to ~70-80% saturation in HSL
        hsl.s = clamp(hsl.s, 10, 75);
    }
    
    // 2. Lightness adjustment for proper tone
    hsl.l += config->lightness_adjust;
    hsl.l = clamp(hsl.l, 10, 90);  // Keep within readable range
    
    // 3. Contrast enhancement (ensure vibrant accents stand out)
    if (config->contrast_enhance > 1.0f && hsl.s > 10) {
        float l_normalized = (hsl.l - 50.0f) / 50.0f;  // -1 to 1
        l_normalized *= config->contrast_enhance;
        hsl.l = clamp(l_normalized * 50.0f + 50.0f, 10, 90);
    }
    
    // 4. Harmonization (shift hue toward primary color)
    // Material You uses HCT blend, we approximate with HSL hue shift
    if (config->harmonize && hsl.s > 10) {
        float hue_diff = seed_hue - hsl.h;
        if (hue_diff > 180) hue_diff -= 360;
        if (hue_diff < -180) hue_diff += 360;
        
        // Shift 15% toward seed hue for harmonization
        hsl.h += hue_diff * 0.15f;
        if (hsl.h < 0) hsl.h += 360;
        if (hsl.h >= 360) hsl.h -= 360;
    }
    
    hsl_to_rgb(hsl, &r, &g, &b);
    rgb_to_hex(r, g, b, hex_out);
}

// Transform neutrals (grays) with Material You neutral palette approach
// Material You neutrals have chroma ~6, which is approximately 5-8% saturation
static void transform_neutral(const char *hex_in, char *hex_out, MaterialYouConfig *config, float tint_hue) {
    int r, g, b;
    hex_to_rgb(hex_in, &r, &g, &b);
    
    HSL hsl = rgb_to_hsl(r, g, b);
    
    // Material You neutrals: very low chroma (~6 in HCT ≈ 5-8% saturation in HSL)
    // Apply subtle tint from primary color for cohesion
    if (config->harmonize && hsl.s > 0) {
        // Reduce saturation to Material You neutral range (5-8%)
        hsl.s = clamp(hsl.s * 0.3f, 3, 8);
        
        // Slight hue shift toward primary for tinted neutrals
        float hue_diff = tint_hue - hsl.h;
        if (hue_diff > 180) hue_diff -= 360;
        if (hue_diff < -180) hue_diff += 360;
        
        // Very subtle hue shift (5%) for neutral tinting
        hsl.h += hue_diff * 0.05f;
        if (hsl.h < 0) hsl.h += 360;
        if (hsl.h >= 360) hsl.h -= 360;
    } else {
        // Pure neutral - desaturate significantly
        hsl.s = clamp(hsl.s * 0.2f, 0, 5);
    }
    
    // Lightness adjustment
    hsl.l += config->lightness_adjust;
    hsl.l = clamp(hsl.l, 5, 98);
    
    // Gentle contrast enhancement for neutrals
    // Material You ensures minimum tone difference of 50 for contrast
    if (config->contrast_enhance > 1.0f) {
        float l_normalized = (hsl.l - 50.0f) / 50.0f;
        float gentle_enhance = 1.0f + (config->contrast_enhance - 1.0f) * 0.4f;
        l_normalized *= gentle_enhance;
        hsl.l = clamp(l_normalized * 50.0f + 50.0f, 5, 98);
    }
    
    hsl_to_rgb(hsl, &r, &g, &b);
    rgb_to_hex(r, g, b, hex_out);
}

void material_you_transform(Base16Scheme *scheme, MaterialYouConfig *config) {
    if (!scheme || !config) return;
    
    // Find the most vibrant color (highest saturation) to use as primary/seed
    // This mimics Material You's color extraction from wallpaper
    float max_saturation = 0;
    const char *primary_color = scheme->base0D;  // Default to blue
    float primary_hue = 0;
    
    // Check all accent colors for the most saturated one
    const char *accent_colors[] = {
        scheme->base08, scheme->base09, scheme->base0A,
        scheme->base0B, scheme->base0C, scheme->base0D,
        scheme->base0E, scheme->base0F
    };
    
    for (int i = 0; i < 8; i++) {
        int r, g, b;
        hex_to_rgb(accent_colors[i], &r, &g, &b);
        HSL hsl = rgb_to_hsl(r, g, b);
        
        if (hsl.s > max_saturation) {
            max_saturation = hsl.s;
            primary_color = accent_colors[i];
            primary_hue = hsl.h;
        }
    }
    
    // If no highly saturated color found, use base0D (blue) as fallback
    if (max_saturation < 20) {
        int r, g, b;
        hex_to_rgb(scheme->base0D, &r, &g, &b);
        HSL hsl = rgb_to_hsl(r, g, b);
        primary_hue = hsl.h;
    }
    
    char transformed[MAX_COLOR_VALUE];
    
    // Transform background/foreground neutrals (base00-base07)
    // Material You approach: very low chroma with subtle primary tint
    transform_neutral(scheme->base00, transformed, config, primary_hue);
    strncpy(scheme->base00, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base01, transformed, config, primary_hue);
    strncpy(scheme->base01, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base02, transformed, config, primary_hue);
    strncpy(scheme->base02, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base03, transformed, config, primary_hue);
    strncpy(scheme->base03, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base04, transformed, config, primary_hue);
    strncpy(scheme->base04, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base05, transformed, config, primary_hue);
    strncpy(scheme->base05, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base06, transformed, config, primary_hue);
    strncpy(scheme->base06, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base07, transformed, config, primary_hue);
    strncpy(scheme->base07, transformed, MAX_COLOR_VALUE - 1);
    
    // Transform accent colors (base08-base0F)
    // Material You approach: boost chroma, harmonize hues
    transform_color(scheme->base08, transformed, config, primary_hue);
    strncpy(scheme->base08, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base09, transformed, config, primary_hue);
    strncpy(scheme->base09, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0A, transformed, config, primary_hue);
    strncpy(scheme->base0A, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0B, transformed, config, primary_hue);
    strncpy(scheme->base0B, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0C, transformed, config, primary_hue);
    strncpy(scheme->base0C, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0D, transformed, config, primary_hue);
    strncpy(scheme->base0D, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0E, transformed, config, primary_hue);
    strncpy(scheme->base0E, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0F, transformed, config, primary_hue);
    strncpy(scheme->base0F, transformed, MAX_COLOR_VALUE - 1);
    
    // If Base24, transform extended colors too
    if (scheme->is_base24) {
        transform_color(scheme->base10, transformed, config, primary_hue);
        strncpy(scheme->base10, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base11, transformed, config, primary_hue);
        strncpy(scheme->base11, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base12, transformed, config, primary_hue);
        strncpy(scheme->base12, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base13, transformed, config, primary_hue);
        strncpy(scheme->base13, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base14, transformed, config, primary_hue);
        strncpy(scheme->base14, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base15, transformed, config, primary_hue);
        strncpy(scheme->base15, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base16, transformed, config, primary_hue);
        strncpy(scheme->base16, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base17, transformed, config, primary_hue);
        strncpy(scheme->base17, transformed, MAX_COLOR_VALUE - 1);
    }
}

void material_you_transform_default(Base16Scheme *scheme) {
    // Material You-inspired default configuration
    // Based on Material Design 3 color system principles:
    // - Neutrals with chroma ~6 (≈5-8% saturation in HSL)
    // - Vibrant accents with chroma ~36-120 (≈25-75% saturation)  
    // - Minimum tone difference of 50 for contrast (WCAG AA)
    // - Harmonized hues for color cohesion
    MaterialYouConfig config = {
        .saturation_boost = 1.25f,    // 25% saturation boost for vibrant accents
        .lightness_adjust = 0.0f,     // No lightness change (preserve tones)
        .contrast_enhance = 1.12f,    // 12% contrast enhancement
        .harmonize = 1                // Enable hue harmonization
    };
    
    material_you_transform(scheme, &config);
}
