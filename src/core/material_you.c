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
static void transform_color(const char *hex_in, char *hex_out, MaterialYouConfig *config, float seed_hue) {
    int r, g, b;
    hex_to_rgb(hex_in, &r, &g, &b);
    
    HSL hsl = rgb_to_hsl(r, g, b);
    
    // Material You transformations:
    
    // 1. Saturation boost for more vibrant colors
    if (hsl.s > 5) {  // Only boost non-grays
        hsl.s *= config->saturation_boost;
        hsl.s = clamp(hsl.s, 0, 85);  // Cap saturation to avoid oversaturation
    }
    
    // 2. Lightness adjustment
    hsl.l += config->lightness_adjust;
    hsl.l = clamp(hsl.l, 0, 100);
    
    // 3. Contrast enhancement (push away from middle gray)
    if (config->contrast_enhance > 1.0f) {
        float l_normalized = (hsl.l - 50.0f) / 50.0f;  // -1 to 1
        l_normalized *= config->contrast_enhance;
        hsl.l = clamp(l_normalized * 50.0f + 50.0f, 0, 100);
    }
    
    // 4. Harmonization (subtle hue shift toward seed color)
    if (config->harmonize && hsl.s > 5) {
        float hue_diff = seed_hue - hsl.h;
        if (hue_diff > 180) hue_diff -= 360;
        if (hue_diff < -180) hue_diff += 360;
        
        // Shift 10% toward seed hue for harmonization (reduced from 15%)
        hsl.h += hue_diff * 0.10f;
        if (hsl.h < 0) hsl.h += 360;
        if (hsl.h >= 360) hsl.h -= 360;
    }
    
    hsl_to_rgb(hsl, &r, &g, &b);
    rgb_to_hex(r, g, b, hex_out);
}

// Transform neutrals (grays) for Material You aesthetic
static void transform_neutral(const char *hex_in, char *hex_out, MaterialYouConfig *config) {
    int r, g, b;
    hex_to_rgb(hex_in, &r, &g, &b);
    
    HSL hsl = rgb_to_hsl(r, g, b);
    
    // For neutrals, only apply lightness and contrast adjustments
    hsl.l += config->lightness_adjust;
    hsl.l = clamp(hsl.l, 0, 100);
    
    // Gentler contrast enhancement for neutrals to maintain readability
    if (config->contrast_enhance > 1.0f) {
        float l_normalized = (hsl.l - 50.0f) / 50.0f;
        // Use sqrt to reduce the enhancement effect (e.g., 1.10 becomes ~1.05)
        float gentle_enhance = 1.0f + (config->contrast_enhance - 1.0f) * 0.5f;
        l_normalized *= gentle_enhance;
        hsl.l = clamp(l_normalized * 50.0f + 50.0f, 5, 95);  // Keep within safe bounds
    }
    
    hsl_to_rgb(hsl, &r, &g, &b);
    rgb_to_hex(r, g, b, hex_out);
}

void material_you_transform(Base16Scheme *scheme, MaterialYouConfig *config) {
    if (!scheme || !config) return;
    
    // Use base0D (blue - functions) as the seed color for harmonization
    int r, g, b;
    hex_to_rgb(scheme->base0D, &r, &g, &b);
    HSL seed_hsl = rgb_to_hsl(r, g, b);
    float seed_hue = seed_hsl.h;
    
    char transformed[MAX_COLOR_VALUE];
    
    // Transform background/foreground neutrals (base00-base07)
    transform_neutral(scheme->base00, transformed, config);
    strncpy(scheme->base00, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base01, transformed, config);
    strncpy(scheme->base01, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base02, transformed, config);
    strncpy(scheme->base02, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base03, transformed, config);
    strncpy(scheme->base03, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base04, transformed, config);
    strncpy(scheme->base04, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base05, transformed, config);
    strncpy(scheme->base05, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base06, transformed, config);
    strncpy(scheme->base06, transformed, MAX_COLOR_VALUE - 1);
    
    transform_neutral(scheme->base07, transformed, config);
    strncpy(scheme->base07, transformed, MAX_COLOR_VALUE - 1);
    
    // Transform accent colors (base08-base0F)
    transform_color(scheme->base08, transformed, config, seed_hue);
    strncpy(scheme->base08, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base09, transformed, config, seed_hue);
    strncpy(scheme->base09, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0A, transformed, config, seed_hue);
    strncpy(scheme->base0A, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0B, transformed, config, seed_hue);
    strncpy(scheme->base0B, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0C, transformed, config, seed_hue);
    strncpy(scheme->base0C, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0D, transformed, config, seed_hue);
    strncpy(scheme->base0D, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0E, transformed, config, seed_hue);
    strncpy(scheme->base0E, transformed, MAX_COLOR_VALUE - 1);
    
    transform_color(scheme->base0F, transformed, config, seed_hue);
    strncpy(scheme->base0F, transformed, MAX_COLOR_VALUE - 1);
    
    // If Base24, transform extended colors too
    if (scheme->is_base24) {
        transform_color(scheme->base10, transformed, config, seed_hue);
        strncpy(scheme->base10, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base11, transformed, config, seed_hue);
        strncpy(scheme->base11, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base12, transformed, config, seed_hue);
        strncpy(scheme->base12, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base13, transformed, config, seed_hue);
        strncpy(scheme->base13, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base14, transformed, config, seed_hue);
        strncpy(scheme->base14, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base15, transformed, config, seed_hue);
        strncpy(scheme->base15, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base16, transformed, config, seed_hue);
        strncpy(scheme->base16, transformed, MAX_COLOR_VALUE - 1);
        
        transform_color(scheme->base17, transformed, config, seed_hue);
        strncpy(scheme->base17, transformed, MAX_COLOR_VALUE - 1);
    }
}

void material_you_transform_default(Base16Scheme *scheme) {
    MaterialYouConfig config = {
        .saturation_boost = 1.20f,    // 20% more saturation (reduced from 25%)
        .lightness_adjust = 0.0f,     // No lightness change
        .contrast_enhance = 1.10f,    // 10% more contrast (reduced from 15%)
        .harmonize = 1                // Enable harmonization
    };
    
    material_you_transform(scheme, &config);
}
