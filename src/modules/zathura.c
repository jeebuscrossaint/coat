#include "zathura.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Generate a zathura PDF viewer theme file from a Base16 scheme
int zathura_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create zathura theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Font configuration
    if (font && font->monospace[0]) {
        fprintf(f, "# Font configuration\n");
        fprintf(f, "set font \"%s 10\"\n", font->monospace);
        fprintf(f, "\n");
    }
    
    // Zathura Base16 color scheme
    fprintf(f, "# Base16 colors\n");
    fprintf(f, "set notification-error-bg       \"#%s\" # base08 - red\n", strip_hash(scheme->base08));
    fprintf(f, "set notification-error-fg       \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set notification-warning-bg     \"#%s\" # base09 - orange\n", strip_hash(scheme->base09));
    fprintf(f, "set notification-warning-fg     \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set notification-bg             \"#%s\" # base0B - green\n", strip_hash(scheme->base0B));
    fprintf(f, "set notification-fg             \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "\n");
    
    fprintf(f, "set completion-group-bg         \"#%s\" # base01\n", strip_hash(scheme->base01));
    fprintf(f, "set completion-group-fg         \"#%s\" # base0D - blue\n", strip_hash(scheme->base0D));
    fprintf(f, "set completion-bg               \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set completion-fg               \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "set completion-highlight-bg     \"#%s\" # base0D - blue\n", strip_hash(scheme->base0D));
    fprintf(f, "set completion-highlight-fg     \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "\n");
    
    fprintf(f, "set inputbar-bg                 \"#%s\" # base01\n", strip_hash(scheme->base01));
    fprintf(f, "set inputbar-fg                 \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "\n");
    
    fprintf(f, "set statusbar-bg                \"#%s\" # base01\n", strip_hash(scheme->base01));
    fprintf(f, "set statusbar-fg                \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "\n");
    
    fprintf(f, "set highlight-color             \"#%s\" # base0A - yellow\n", strip_hash(scheme->base0A));
    fprintf(f, "set highlight-active-color      \"#%s\" # base0D - blue\n", strip_hash(scheme->base0D));
    fprintf(f, "\n");
    
    fprintf(f, "set default-bg                  \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set default-fg                  \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "set render-loading              true\n");
    fprintf(f, "set render-loading-bg           \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set render-loading-fg           \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "\n");
    
    fprintf(f, "# Recoloring mode settings\n");
    fprintf(f, "# <C-r> to switch modes\n");
    fprintf(f, "set recolor-lightcolor          \"#%s\" # base00\n", strip_hash(scheme->base00));
    fprintf(f, "set recolor-darkcolor           \"#%s\" # base05\n", strip_hash(scheme->base05));
    fprintf(f, "\n");
    
    fclose(f);
    return 0;
}

// Apply zathura theme to current zathura configuration
int zathura_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    // Get home directory
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/zathura", home);
    
    struct stat st = {0};
    if (stat(config_dir, &st) == -1) {
        if (mkdir(config_dir, 0755) != 0) {
            fprintf(stderr, "Failed to create zathura config directory\n");
            return -1;
        }
    }
    
    // Generate theme file path
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/zathurarc", config_dir);
    
    // Generate the theme
    if (zathura_generate_theme(scheme, theme_path, font) != 0) {
        fprintf(stderr, "Failed to generate zathura theme\n");
        return -1;
    }
    
    printf("Theme applied to: %s\n", theme_path);
    printf("Restart zathura or open a new PDF to see the changes.\n");
    
    return 0;
}
