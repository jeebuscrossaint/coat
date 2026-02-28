#define _POSIX_C_SOURCE 200809L
#include "wallpaper.h"
#include "material_you.h"
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

// Score a color for theming (higher is better)
// Prioritizes: colorfulness, moderate lightness, uniqueness
static float score_color(Color *color) {
    HSL hsl = rgb_to_hsl_internal(color->r, color->g, color->b);
    
    // Saturation score (prefer colorful, 0.3-1.0 saturation)
    float sat_score = hsl.s;
    if (hsl.s < 0.3f) sat_score *= 0.5f;  // Penalize dull colors
    
    // Lightness score (prefer mid-tones, 0.2-0.8 lightness)
    float light_score = 1.0f;
    if (hsl.l < 0.2f || hsl.l > 0.8f) {
        light_score = 0.3f;  // Penalize very dark/light
    }
    
    // Frequency score (prefer common but not overwhelming)
    float freq_score = sqrtf(color->count / 1000.0f);
    if (freq_score > 1.0f) freq_score = 1.0f;
    
    return sat_score * light_score * freq_score * 100.0f;
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
    
    // Initialize k-means centroids with random pixels
    srand(42);  // Deterministic seed
    for (int i = 0; i < k; i++) {
        int idx = (rand() % sample_size) * 16 * 3;  // Random sample
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
    
    // Extract 16 dominant colors using k-means
    int color_count;
    Color *colors = quantize_colors(pixels, width, height, 16, &color_count);
    
    stbi_image_free(pixels);
    free(wallpaper_path);
    
    if (!colors) {
        fprintf(stderr, "Color quantization failed\n");
        return -1;
    }
    
    // Generate Base16 scheme from extracted colors
    // Strategy:
    // - base00-07: Generate neutral tones from darkest/lightest colors
    // - base08-0F: Use most vibrant colors for accents
    
    // Find darkest and lightest colors for neutrals
    Color *darkest = &colors[0];
    Color *lightest = &colors[0];
    
    for (int i = 0; i < color_count; i++) {
        HSL hsl = rgb_to_hsl_internal(colors[i].r, colors[i].g, colors[i].b);
        HSL dark_hsl = rgb_to_hsl_internal(darkest->r, darkest->g, darkest->b);
        HSL light_hsl = rgb_to_hsl_internal(lightest->r, lightest->g, lightest->b);
        
        if (hsl.l < dark_hsl.l) darkest = &colors[i];
        if (hsl.l > light_hsl.l) lightest = &colors[i];
    }
    
    // Generate neutrals (base00-07) as a grayscale ramp from darkest to lightest
    sprintf(scheme->base00, "#%02x%02x%02x", darkest->r, darkest->g, darkest->b);
    
    // Interpolate between darkest and lightest for neutral tones
    for (int i = 1; i < 8; i++) {
        float t = i / 7.0f;
        int r = darkest->r + (lightest->r - darkest->r) * t;
        int g = darkest->g + (lightest->g - darkest->g) * t;
        int b = darkest->b + (lightest->b - darkest->b) * t;
        
        char *target = NULL;
        switch(i) {
            case 1: target = scheme->base01; break;
            case 2: target = scheme->base02; break;
            case 3: target = scheme->base03; break;
            case 4: target = scheme->base04; break;
            case 5: target = scheme->base05; break;
            case 6: target = scheme->base06; break;
            case 7: target = scheme->base07; break;
        }
        if (target) sprintf(target, "#%02x%02x%02x", r, g, b);
    }
    
    // Assign top 8 vibrant colors to accents (base08-0F)
    // Skip colors that are too similar to neutrals
    int accent_idx = 0;
    char *accent_targets[] = {
        scheme->base08, scheme->base09, scheme->base0A, scheme->base0B,
        scheme->base0C, scheme->base0D, scheme->base0E, scheme->base0F
    };
    
    for (int i = 0; i < color_count && accent_idx < 8; i++) {
        HSL hsl = rgb_to_hsl_internal(colors[i].r, colors[i].g, colors[i].b);
        
        // Only use colorful colors for accents (saturation > 0.2)
        if (hsl.s > 0.2f) {
            sprintf(accent_targets[accent_idx], "#%02x%02x%02x", 
                    colors[i].r, colors[i].g, colors[i].b);
            accent_idx++;
        }
    }
    
    // Fill remaining accents with variations if needed
    while (accent_idx < 8) {
        sprintf(accent_targets[accent_idx], "%s", accent_targets[accent_idx % accent_idx]);
        accent_idx++;
    }
    
    // Set scheme metadata
    strncpy(scheme->name, "Wallpaper", sizeof(scheme->name) - 1);
    strncpy(scheme->author, "coat (extracted)", sizeof(scheme->author) - 1);
    strncpy(scheme->variant, "dark", sizeof(scheme->variant) - 1);
    scheme->is_base24 = 0;
    
    free(colors);
    
    printf("✓ Extracted %d colors and generated Base16 scheme\n", color_count);
    
    // Apply Material You transformation if requested
    if (apply_material_you) {
        printf("Applying Material You transformation...\n");
        material_you_transform_default(scheme);
    }
    
    return 0;
}
