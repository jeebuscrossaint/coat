//
// Created by amarnath on 2/27/26.
//

#include "../../include/material_you.h"
#include "../../include/hct.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

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

// Utility: Convert RGB to hex string (always includes # prefix)
static void rgb_to_hex(int r, int g, int b, char *hex_out) {
    sprintf(hex_out, "#%02x%02x%02x", r, g, b);
}

// Material You color transformation for accent colors using HCT
// Matugen approach: Force specific tonal values for dark/light themes
// Dark theme: Tone 80, Light theme: Tone 40
static void transform_color_hct(const char *hex_in, char *hex_out, MaterialYouConfig *config, double seed_hue, int is_dark) {
    int r, g, b;
    hex_to_rgb(hex_in, &r, &g, &b);
    
    RGB rgb_in = {r, g, b};
    HCT hct = rgb_to_hct(rgb_in);
    
    // Material You (matugen) tonal values
    // Dark theme: Tone 80 (bright and vibrant)
    // Light theme: Tone 40 (darker but saturated)
    
    if (is_dark) {
        hct.t = 80.0;  // Tone 80 for dark themes
        // Maximum chroma for vivid colors (Material You uses 80-120)
        if (hct.c < 48.0) {
            hct.c = 80.0;  // Boost low chroma colors
        } else if (hct.c > 120.0) {
            hct.c = 120.0;  // Cap at max
        } else {
            hct.c = fmin(hct.c * 1.5, 120.0);  // Boost existing chroma
        }
    } else {
        hct.t = 40.0;  // Tone 40 for light themes
        // High chroma for light themes too
        if (hct.c < 48.0) {
            hct.c = 80.0;
        } else if (hct.c > 120.0) {
            hct.c = 120.0;
        } else {
            hct.c = fmin(hct.c * 1.5, 120.0);
        }
    }
    
    // Harmonization: Shift hue toward seed color (optional, matugen does this)
    if (config->harmonize) {
        double hue_diff = seed_hue - hct.h;
        if (hue_diff > 180) hue_diff -= 360;
        if (hue_diff < -180) hue_diff += 360;
        
        // 15% blend toward seed hue for color harmony
        hct.h += hue_diff * 0.15;
        if (hct.h < 0) hct.h += 360;
        if (hct.h >= 360) hct.h -= 360;
    }
    
    RGB rgb_out = hct_to_rgb(hct);
    rgb_to_hex(rgb_out.r, rgb_out.g, rgb_out.b, hex_out);
}

// Transform neutrals using HCT with Material You tonal palette
// Matugen uses chroma ~4human-6 for neutrals with subtle primary tint
static void transform_neutral_hct(const char *hex_in, char *hex_out, MaterialYouConfig *config, double tint_hue, double target_tone) {
    (void)hex_in;  // We'll create from scratch based on target tone
    
    HCT hct;
    hct.h = tint_hue;  // Use primary hue for subtle tinting
    hct.c = config->harmonize ? 6.0 : 2.0;  // Material You neutral chroma
    hct.t = target_tone;
    
    RGB rgb = hct_to_rgb(hct);
    rgb_to_hex(rgb.r, rgb.g, rgb.b, hex_out);
}

void material_you_transform(Base16Scheme *scheme, MaterialYouConfig *config) {
    if (!scheme || !config) return;
    
    // Find the most vibrant color (highest chroma in HCT) to use as primary/seed
    // This mimics Material You's color extraction from wallpaper
    double max_chroma = 0;
    const char *primary_color = scheme->base0D;  // Default to blue
    double primary_hue = 0;
    
    // Check all accent colors for the most saturated one
    const char *accent_colors[] = {
        scheme->base08, scheme->base09, scheme->base0A,
        scheme->base0B, scheme->base0C, scheme->base0D,
        scheme->base0E, scheme->base0F
    };
    
    for (int i = 0; i < 8; i++) {
        int r, g, b;
        hex_to_rgb(accent_colors[i], &r, &g, &b);
        RGB rgb = {r, g, b};
        HCT hct = rgb_to_hct(rgb);
        
        if (hct.c > max_chroma) {
            max_chroma = hct.c;
            primary_color = accent_colors[i];
            primary_hue = hct.h;
        }
    }
    
    // If no highly saturated color found, use base0D (blue) as fallback
    if (max_chroma < 15) {
        int r, g, b;
        hex_to_rgb(scheme->base0D, &r, &g, &b);
        RGB rgb = {r, g, b};
        HCT hct = rgb_to_hct(rgb);
        primary_hue = hct.h;
    }
    
    // Detect if this is a dark or light theme
    int r0, g0, b0, r7, g7, b7;
    hex_to_rgb(scheme->base00, &r0, &g0, &b0);
    hex_to_rgb(scheme->base07, &r7, &g7, &b7);
    RGB rgb0 = {r0, g0, b0};
    RGB rgb7 = {r7, g7, b7};
    HCT base00_hct = rgb_to_hct(rgb0);
    HCT base07_hct = rgb_to_hct(rgb7);
    
    int is_dark = base00_hct.t < base07_hct.t;
    
    char transformed[MAX_COLOR_VALUE];
    
    // Transform background/foreground neutrals (base00-base07)
    // Material You (matugen) approach: Use specific tonal values from HCT
    // Dark theme tones: 10, 17, 25, 35, 60, 75, 82, 90
    // Light theme tones: 99, 95, 88, 75, 50, 30, 20, 10
    
    if (is_dark) {
        // Dark theme: backgrounds dark, foregrounds light
        transform_neutral_hct(scheme->base00, transformed, config, primary_hue, 10.0);
        strncpy(scheme->base00, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base01, transformed, config, primary_hue, 17.0);
        strncpy(scheme->base01, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base02, transformed, config, primary_hue, 25.0);
        strncpy(scheme->base02, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base03, transformed, config, primary_hue, 35.0);
        strncpy(scheme->base03, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base04, transformed, config, primary_hue, 60.0);
        strncpy(scheme->base04, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base05, transformed, config, primary_hue, 75.0);
        strncpy(scheme->base05, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base06, transformed, config, primary_hue, 82.0);
        strncpy(scheme->base06, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base07, transformed, config, primary_hue, 90.0);
        strncpy(scheme->base07, transformed, MAX_COLOR_VALUE - 1);
    } else {
        // Light theme: backgrounds light, foregrounds dark
        transform_neutral_hct(scheme->base00, transformed, config, primary_hue, 99.0);
        strncpy(scheme->base00, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base01, transformed, config, primary_hue, 95.0);
        strncpy(scheme->base01, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base02, transformed, config, primary_hue, 88.0);
        strncpy(scheme->base02, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base03, transformed, config, primary_hue, 75.0);
        strncpy(scheme->base03, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base04, transformed, config, primary_hue, 50.0);
        strncpy(scheme->base04, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base05, transformed, config, primary_hue, 30.0);
        strncpy(scheme->base05, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base06, transformed, config, primary_hue, 20.0);
        strncpy(scheme->base06, transformed, MAX_COLOR_VALUE - 1);
        
        transform_neutral_hct(scheme->base07, transformed, config, primary_hue, 10.0);
        strncpy(scheme->base07, transformed, MAX_COLOR_VALUE - 1);
    }
    
    // Transform accent colors (base08-base0F)
    // Material You (matugen) approach: Force to proper tonal values using HCT
    transform_color_hct(scheme->base08, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base08, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base09, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base09, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0A, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0A, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0B, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0B, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0C, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0C, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0D, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0D, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0E, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0E, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color_hct(scheme->base0F, transformed, config, primary_hue, is_dark);
    strncpy(scheme->base0F, transformed, MAX_COLOR_VALUE - 1);
    
    // If Base24, transform extended colors too
    if (scheme->is_base24) {
        transform_color_hct(scheme->base10, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base10, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base11, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base11, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base12, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base12, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base13, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base13, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base14, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base14, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base15, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base15, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base16, transformed, config, primary_hue, is_dark);
        strncpy(scheme->base16, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color_hct(scheme->base17, transformed, config, primary_hue, is_dark);
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
