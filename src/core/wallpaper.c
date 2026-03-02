#define _POSIX_C_SOURCE 200809L
#include "wallpaper.h"
#include "material_you.h"
#include "hct.h"
#include "quantize_celebi.h"
#include "score.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"

// Color structure for extraction
typedef struct {
    unsigned char r, g, b;
    int count;  // Frequency in image
    float score;  // Computed score for theming
} Color;

// HSL for color analysis
typedef struct {
    float h, s, l;
} HSL;

// Convert RGB to HSL
static HSL rgb_to_hsl_internal(unsigned char r, unsigned char g, unsigned char b) {
    HSL hsl;
    float r_norm = r / 255.0f;
    float g_norm = g / 255.0f;
    float b_norm = b / 255.0f;
    
    float max = fmaxf(r_norm, fmaxf(g_norm, b_norm));
    float min = fminf(r_norm, fminf(g_norm, b_norm));
    float delta = max - min;
    
    hsl.l = (max + min) / 2.0f;
    
    if (delta == 0) {
        hsl.h = 0;
        hsl.s = 0;
    } else {
        hsl.s = delta / (hsl.l < 0.5f ? (max + min) : (2.0f - max - min));
        
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

// Score a color for Material You theming (higher is better)
// Based on Material Color Utilities scoring:
// - Prioritizes chroma (colorfulness) - colors with 48+ chroma preferred
// - Valid tone range - colors should be usable (tone 30-90, ideally 40-80)
// - Population/frequency - more common colors score higher
static float score_color(Color *color) {
    RGB rgb = {color->r, color->g, color->b};
    HCT hct = rgb_to_hct(rgb);
    
    // Chroma score: Material You requires minimum chroma 48 for vibrant accents
    // Reject colors with chroma < 15 (too gray)
    if (hct.c < 15.0) {
        return 0.1f;  // Nearly zero score for gray colors
    }
    
    // Scale chroma: 15-48 moderate, 48+ excellent
    float chroma_score = 0.0f;
    if (hct.c >= 48.0) {
        chroma_score = 1.0f + (hct.c - 48.0) / 72.0;  // 1.0 to 2.0 for chroma 48-120
    } else {
        chroma_score = (hct.c - 15.0) / 33.0;  // 0.0 to 1.0 for chroma 15-48
    }
    
    // Tone score: Prefer usable tones (30-90), heavily penalize extremes
    // Material You accents are typically tone 40 (light mode) or tone 80 (dark mode)
    float tone_score = 0.0f;
    if (hct.t < 15.0 || hct.t > 95.0) {
        return 0.1f;  // Nearly zero score for extreme tones (too dark/light)
    } else if (hct.t >= 30.0 && hct.t <= 90.0) {
        // Ideal range: tone 30-90
        tone_score = 1.0f;
        // Bonus for mid-tones (40-80) which work well in both light/dark themes
        if (hct.t >= 40.0 && hct.t <= 80.0) {
            tone_score = 1.2f;
        }
    } else {
        // Marginal range: 15-30 or 90-95
        tone_score = 0.4f;
    }
    
    // Population score: More common colors are better (logarithmic to prevent dominance)
    float pop_score = logf(1.0f + color->count) / 12.0f;
    if (pop_score > 1.0f) pop_score = 1.0f;
    
    // Combined score:
    // - Chroma is most important (60 points)
    // - Tone validity is critical (30 points)
    // - Population helps (10 points)
    return chroma_score * 60.0f + tone_score * 30.0f + pop_score * 10.0f;
}

// Compare function for sorting colors by score
static int compare_colors(const void *a, const void *b) {
    Color *ca = (Color*)a;
    Color *cb = (Color*)b;
    return (cb->score > ca->score) - (cb->score < ca->score);
}

// Simple k-means color quantization
// Reduces image to k dominant colors
static Color* quantize_colors(unsigned char *pixels, int width, int height, int k, int *out_count) {
    int total_pixels = width * height;
    
    // Downsample for performance (every 4th pixel)
    int sample_size = total_pixels / 16;
    if (sample_size > 10000) sample_size = 10000;  // Cap at 10k samples
    
    Color *colors = calloc(k, sizeof(Color));
    if (!colors) return NULL;
    
    // Initialize k-means centroids with evenly spaced pixels for deterministic results
    for (int i = 0; i < k; i++) {
        // Evenly distribute initial centroids across the image
        int pixel_idx = (i * total_pixels / k);
        int idx = pixel_idx * 3;
        
        // Ensure within bounds
        if (idx + 2 >= total_pixels * 3) {
            idx = (total_pixels - 1) * 3;
        }
        
        colors[i].r = pixels[idx];
        colors[i].g = pixels[idx + 1];
        colors[i].b = pixels[idx + 2];
        colors[i].count = 0;
    }
    
    // K-means iterations
    for (int iter = 0; iter < 10; iter++) {
        // Reset counts
        for (int i = 0; i < k; i++) {
            colors[i].count = 0;
        }
        
        int *cluster_r = calloc(k, sizeof(int));
        int *cluster_g = calloc(k, sizeof(int));
        int *cluster_b = calloc(k, sizeof(int));
        
        // Assign pixels to nearest centroid (sample every 4th pixel)
        for (int p = 0; p < total_pixels; p += 4) {
            int idx = p * 3;
            unsigned char r = pixels[idx];
            unsigned char g = pixels[idx + 1];
            unsigned char b = pixels[idx + 2];
            
            // Find nearest centroid
            int nearest = 0;
            int min_dist = INT_MAX;
            for (int c = 0; c < k; c++) {
                int dr = r - colors[c].r;
                int dg = g - colors[c].g;
                int db = b - colors[c].b;
                int dist = dr*dr + dg*dg + db*db;
                
                if (dist < min_dist) {
                    min_dist = dist;
                    nearest = c;
                }
            }
            
            cluster_r[nearest] += r;
            cluster_g[nearest] += g;
            cluster_b[nearest] += b;
            colors[nearest].count++;
        }
        
        // Update centroids
        for (int i = 0; i < k; i++) {
            if (colors[i].count > 0) {
                colors[i].r = cluster_r[i] / colors[i].count;
                colors[i].g = cluster_g[i] / colors[i].count;
                colors[i].b = cluster_b[i] / colors[i].count;
            }
        }
        
        free(cluster_r);
        free(cluster_g);
        free(cluster_b);
    }
    
    // Score and sort colors
    for (int i = 0; i < k; i++) {
        colors[i].score = score_color(&colors[i]);
    }
    
    qsort(colors, k, sizeof(Color), compare_colors);
    
    // Check if we have any viable colors (score > 1.0)
    // If not, this is a low-chroma image - use relaxed scoring
    if (colors[0].score < 1.0f) {
        for (int i = 0; i < k; i++) {
            RGB rgb = {colors[i].r, colors[i].g, colors[i].b};
            HCT hct = rgb_to_hct(rgb);
            
            // Relaxed scoring for low-chroma images
            // Still prefer some chroma, but accept lower values
            float chroma_weight = hct.c / 20.0f;  // Normalize to ~20 max
            if (chroma_weight > 2.0f) chroma_weight = 2.0f;
            
            // Tone weight: Strongly prefer usable tones (20-80)
            // HEAVILY penalize extreme darks/lights
            float tone_weight = 0.0f;
            if (hct.t < 10.0) {
                tone_weight = 0.1f;  // Nearly unusable (too dark)
            } else if (hct.t >= 10.0 && hct.t < 20.0) {
                tone_weight = 0.5f;  // Marginal
            } else if (hct.t >= 20.0 && hct.t <= 80.0) {
                tone_weight = 2.0f;  // Good range
                // Bonus for ideal mid-range (40-70)
                if (hct.t >= 40.0 && hct.t <= 70.0) {
                    tone_weight = 2.5f;
                }
            } else if (hct.t > 80.0 && hct.t <= 90.0) {
                tone_weight = 1.2f;  // Acceptable (light)
            } else {
                tone_weight = 0.3f;  // Too light (> 90)
            }
            
            float pop_weight = logf(1.0f + colors[i].count) / 12.0f;
            if (pop_weight > 1.0f) pop_weight = 1.0f;
            
            // For low-chroma images, tone validity is MORE important than chroma
            colors[i].score = chroma_weight * 30.0f + tone_weight * 60.0f + pop_weight * 10.0f;
        }
        qsort(colors, k, sizeof(Color), compare_colors);
    }
    
    *out_count = k;
    return colors;
}

// Get wallpaper path from swww
char* wallpaper_get_path(void) {
    FILE *fp = popen("swww query 2>/dev/null", "r");
    if (!fp) {
        fprintf(stderr, "Failed to run 'swww query'. Is swww running?\n");
        return NULL;
    }
    
    char line[2048];
    char *path = NULL;
    
    while (fgets(line, sizeof(line), fp)) {
        // Parse line like: "eDP-1: ... currently displaying: image: /path/to/wallpaper.png"
        char *image_marker = strstr(line, "image: ");
        if (image_marker) {
            image_marker += 7;  // Skip "image: "
            
            // Trim newline
            char *newline = strchr(image_marker, '\n');
            if (newline) *newline = '\0';
            
            // Trim whitespace
            while (*image_marker == ' ') image_marker++;
            
            path = strdup(image_marker);
            break;
        }
    }
    
    pclose(fp);
    return path;
}

// Extract colors from wallpaper and generate Base16 scheme
int wallpaper_extract_scheme(Base16Scheme *scheme, int apply_material_you) {
    if (!scheme) return -1;
    
    // Get wallpaper path from swww
    char *wallpaper_path = wallpaper_get_path();
    if (!wallpaper_path) {
        fprintf(stderr, "Could not find wallpaper. Make sure swww is running.\n");
        return -1;
    }
    
    printf("Extracting colors from: %s\n", wallpaper_path);
    
    // Load image
    int width, height, channels;
    unsigned char *pixels = stbi_load(wallpaper_path, &width, &height, &channels, 3);
    
    if (!pixels) {
        fprintf(stderr, "Failed to load image: %s\n", wallpaper_path);
        free(wallpaper_path);
        return -1;
    }
    
    printf("Image loaded: %dx%d\n", width, height);
    
    // Extract dominant colors (k-means + Material You scoring)
    // Note: matugen uses Wu quantization + Wsmeans, but k-means gives similar results
    int color_count;
    Color *colors = quantize_colors(pixels, width, height, 32, &color_count);
    
    stbi_image_free(pixels);
    free(wallpaper_path);
    
    if (!colors) {
        fprintf(stderr, "Color quantization failed\n");
        return -1;
    }
    
    // Convert to RGB array and populations for scoring
    RGB *rgb_colors = malloc(color_count * sizeof(RGB));
    int *populations = malloc(color_count * sizeof(int));
    
    for (int i = 0; i < color_count; i++) {
        rgb_colors[i].r = colors[i].r;
        rgb_colors[i].g = colors[i].g;
        rgb_colors[i].b = colors[i].b;
        populations[i] = colors[i].count;
    }
    
    free(colors);
    
    // Create quantize result for scoring
    QuantizeResult quant_result;
    quant_result.colors = rgb_colors;
    quant_result.populations = populations;
    quant_result.count = color_count;
    
    printf("Extracted %d colors, scoring...\n", quant_result.count);
    
    // Score colors using Material You algorithm
    int scored_count;
    ScoredColor *scored = score_colors(&quant_result, 16, &scored_count);
    
    // Free quantize result
    free(rgb_colors);
    free(populations);
    
    if (!scored || scored_count == 0) {
        fprintf(stderr, "Color scoring failed\n");
        return -1;
    }
    
    printf("✓ Selected %d theme colors\n", scored_count);
    
    // Generate Base16 scheme
    // Use the highest-scored color as primary
    RGB primary = scored[0].color;
    HCT primary_hct = rgb_to_hct(primary);
    
    // Strategy: Don't use extracted colors directly for neutrals
    // Instead, generate neutrals from primary color (like Material You does)
    // This ensures consistency and proper contrast
    
    // For accents, use the scored colors
    char *accent_targets[] = {
        scheme->base08, scheme->base09, scheme->base0A, scheme->base0B,
        scheme->base0C, scheme->base0D, scheme->base0E, scheme->base0F
    };
    
    for (int i = 0; i < 8 && i < scored_count; i++) {
        RGB color = scored[i].color;
        sprintf(accent_targets[i], "#%02x%02x%02x", color.r, color.g, color.b);
    }
    
    // Fill remaining accents if needed
    for (int i = scored_count; i < 8; i++) {
        sprintf(accent_targets[i], "%s", accent_targets[i % scored_count]);
    }
    
    // Generate neutrals: Create from primary hue with low chroma
    // Dark theme: tones 10, 17, 25, 35, 60, 75, 82, 90
    double neutral_tones[] = {10, 17, 25, 35, 60, 75, 82, 90};
    char *neutral_targets[] = {
        scheme->base00, scheme->base01, scheme->base02, scheme->base03,
        scheme->base04, scheme->base05, scheme->base06, scheme->base07
    };
    
    for (int i = 0; i < 8; i++) {
        HCT neutral_hct;
        neutral_hct.h = primary_hct.h;
        neutral_hct.c = 6.0;  // Low chroma for neutrals
        neutral_hct.t = neutral_tones[i];
        
        RGB neutral = hct_to_rgb(neutral_hct);
        sprintf(neutral_targets[i], "#%02x%02x%02x", neutral.r, neutral.g, neutral.b);
    }
    
    score_colors_free(scored);
    
    // Set scheme metadata
    strncpy(scheme->name, "Wallpaper", sizeof(scheme->name) - 1);
    strncpy(scheme->author, "coat (Material You)", sizeof(scheme->author) - 1);
    strncpy(scheme->variant, "dark", sizeof(scheme->variant) - 1);
    scheme->is_base24 = 0;
    
    printf("✓ Generated Base16 scheme with Material You colors\n");
    
    // Apply Material You transformation if requested
    if (apply_material_you) {
        printf("Applying Material You transformation...\n");
        material_you_transform_default(scheme);
    }
    
    return 0;
}
