#include "avizo.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Helper to convert hex to RGB components
static void hex_to_rgb(const char *hex, int *r, int *g, int *b) {
    unsigned int color;
    sscanf(strip_hash(hex), "%x", &color);
    *r = (color >> 16) & 0xFF;
    *g = (color >> 8) & 0xFF;
    *b = color & 0xFF;
}

// Apply theme to Avizo notification daemon
int avizo_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create avizo config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/avizo", home);
    mkdir(config_dir, 0755);
    
    // Generate config path
    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/config.ini", config_dir);
    
    printf("Generating Avizo config: %s\n", config_path);
    
    FILE *f = fopen(config_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create Avizo config: %s\n", config_path);
        return -1;
    }
    
    // Write header comment
    fprintf(f, "# Avizo notification daemon configuration\n");
    fprintf(f, "# Themed by coat with: %s\n", scheme->name);
    fprintf(f, "# Author: %s\n", scheme->author);
    fprintf(f, "\n");
    
    fprintf(f, "[default]\n");
    
    // Convert Base16 colors to RGBA format
    int r, g, b;
    
    // Background: base00 with 90% opacity
    hex_to_rgb(scheme->base00, &r, &g, &b);
    fprintf(f, "background = rgba(%d, %d, %d, 0.9)\n", r, g, b);
    
    // Border: base03 (selection/comment color) with 80% opacity
    hex_to_rgb(scheme->base03, &r, &g, &b);
    fprintf(f, "border-color = rgba(%d, %d, %d, 0.8)\n", r, g, b);
    
    // Bar foreground (filled): base0D (blue accent) with 90% opacity
    hex_to_rgb(scheme->base0D, &r, &g, &b);
    fprintf(f, "bar-fg-color = rgba(%d, %d, %d, 0.9)\n", r, g, b);
    
    // Bar background (unfilled): base01 (lighter background) with 60% opacity
    hex_to_rgb(scheme->base01, &r, &g, &b);
    fprintf(f, "bar-bg-color = rgba(%d, %d, %d, 0.6)\n", r, g, b);
    
    // Add some reasonable defaults if not already configured
    fprintf(f, "\n# Visual settings\n");
    fprintf(f, "width = 248\n");
    fprintf(f, "height = 232\n");
    fprintf(f, "padding = 24\n");
    fprintf(f, "y-offset = 0.75\n");
    fprintf(f, "x-offset = 0.5\n");
    fprintf(f, "border-radius = 16\n");
    fprintf(f, "border-width = 1\n");
    fprintf(f, "block-height = 10\n");
    fprintf(f, "block-spacing = 2\n");
    fprintf(f, "block-count = 20\n");
    fprintf(f, "fade-in = 0.2\n");
    fprintf(f, "fade-out = 0.5\n");
    
    fclose(f);
    
    printf("Avizo config generated successfully!\n");
    printf("\nAvizo will automatically use the new theme on next notification.\n");
    printf("\nMake sure avizo-service is running:\n");
    printf("  avizo-service &\n");
    printf("\nOr add to your Sway config:\n");
    printf("  exec \"avizo-service\"\n");
    printf("\nTest with:\n");
    printf("  volumectl -u up\n");
    printf("  lightctl up\n");
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
