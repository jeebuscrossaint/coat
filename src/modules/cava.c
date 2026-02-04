#include "cava.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Apply theme to CAVA audio visualizer
int cava_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create cava config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/cava", home);
    mkdir(config_dir, 0755);
    
    // Generate config path
    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/config", config_dir);
    
    printf("Generating CAVA config: %s\n", config_path);
    
    FILE *f = fopen(config_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create CAVA config: %s\n", config_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "# CAVA config - themed by coat\n");
    fprintf(f, "# Scheme: %s\n", scheme->name);
    fprintf(f, "# Author: %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# Variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // General settings
    fprintf(f, "[general]\n");
    fprintf(f, "framerate = 60\n");
    fprintf(f, "bars = 0\n");
    fprintf(f, "bar_width = 2\n");
    fprintf(f, "bar_spacing = 1\n");
    fprintf(f, "\n");
    
    // Input settings
    fprintf(f, "[input]\n");
    fprintf(f, "method = pulse\n");
    fprintf(f, "\n");
    
    // Output settings
    fprintf(f, "[output]\n");
    fprintf(f, "method = ncurses\n");
    fprintf(f, "\n");
    
    // Color settings - this is where the magic happens
    fprintf(f, "[color]\n");
    
    // Gradient mode with 8 colors
    fprintf(f, "gradient = 1\n");
    fprintf(f, "gradient_count = 8\n");
    
    // Create a gradient from green -> cyan -> blue -> magenta -> red
    // CAVA expects hex colors in the format '#xxxxxx'
    
    // Color 1: green (base0B)
    fprintf(f, "gradient_color_1 = '%s'\n", scheme->base0B);
    
    // Color 2: cyan (base0C)
    fprintf(f, "gradient_color_2 = '%s'\n", scheme->base0C);
    
    // Color 3: blue (base0D)
    fprintf(f, "gradient_color_3 = '%s'\n", scheme->base0D);
    
    // Color 4: bright blue (lighter variant)
    fprintf(f, "gradient_color_4 = '%s'\n", scheme->base0D);
    
    // Color 5: magenta (base0E)
    fprintf(f, "gradient_color_5 = '%s'\n", scheme->base0E);
    
    // Color 6: orange (base09)
    fprintf(f, "gradient_color_6 = '%s'\n", scheme->base09);
    
    // Color 7: red (base08)
    fprintf(f, "gradient_color_7 = '%s'\n", scheme->base08);
    
    // Color 8: bright red (lighter variant)
    fprintf(f, "gradient_color_8 = '%s'\n", scheme->base08);
    
    fprintf(f, "\n");
    
    // Smoothing
    fprintf(f, "[smoothing]\n");
    fprintf(f, "integral = 77\n");
    fprintf(f, "monstercat = 0\n");
    fprintf(f, "waves = 0\n");
    fprintf(f, "gravity = 100\n");
    fprintf(f, "ignore = 0\n");
    fprintf(f, "\n");
    
    // EQ settings
    fprintf(f, "[eq]\n");
    fprintf(f, "1 = 1\n");
    fprintf(f, "2 = 1\n");
    fprintf(f, "3 = 1\n");
    fprintf(f, "4 = 1\n");
    fprintf(f, "5 = 1\n");
    
    fclose(f);
    
    printf("CAVA config generated successfully!\n");
    printf("\nCAVA will automatically use the new colors on next launch.\n");
    printf("\nTo start CAVA:\n");
    printf("  cava\n");
    printf("\nOr in a terminal split/tmux pane for a persistent visualizer.\n");
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
