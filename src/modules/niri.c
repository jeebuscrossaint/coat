#include "niri.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Generate a niri compositor theme file from a Base16 scheme
int niri_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create niri theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "// coat theme: %s\n", scheme->name);
    fprintf(f, "// %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "// variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Start layout section
    fprintf(f, "layout {\n");
    
    // Set background color
    fprintf(f, "    // Workspace background color\n");
    fprintf(f, "    background-color \"%s\"\n", scheme->base00);
    fprintf(f, "\n");
    
    // Focus ring configuration
    fprintf(f, "    // Focus ring around active window\n");
    fprintf(f, "    focus-ring {\n");
    fprintf(f, "        on\n");
    fprintf(f, "        width 4\n");
    fprintf(f, "        active-color \"%s\"\n", scheme->base0D);
    fprintf(f, "        inactive-color \"%s\"\n", scheme->base03);
    fprintf(f, "        urgent-color \"%s\"\n", scheme->base08);
    fprintf(f, "    }\n");
    fprintf(f, "\n");
    
    // Border configuration
    fprintf(f, "    // Window borders\n");
    fprintf(f, "    border {\n");
    fprintf(f, "        off\n");
    fprintf(f, "        // Uncomment to enable borders instead of focus rings\n");
    fprintf(f, "        // on\n");
    fprintf(f, "        // width 4\n");
    fprintf(f, "        // active-color \"%s\"\n", scheme->base0A);
    fprintf(f, "        // inactive-color \"%s\"\n", scheme->base03);
    fprintf(f, "        // urgent-color \"%s\"\n", scheme->base08);
    fprintf(f, "    }\n");
    fprintf(f, "\n");
    
    // Shadow configuration
    fprintf(f, "    // Window shadows\n");
    fprintf(f, "    shadow {\n");
    fprintf(f, "        off\n");
    fprintf(f, "        // Uncomment to enable shadows\n");
    fprintf(f, "        // on\n");
    fprintf(f, "        // softness 30\n");
    fprintf(f, "        // spread 5\n");
    fprintf(f, "        // offset x=0 y=5\n");
    fprintf(f, "        // color \"%s70\"\n", scheme->base00);
    fprintf(f, "    }\n");
    fprintf(f, "\n");
    
    // Tab indicator configuration
    fprintf(f, "    // Tab indicator for tabbed columns\n");
    fprintf(f, "    tab-indicator {\n");
    fprintf(f, "        on\n");
    fprintf(f, "        hide-when-single-tab\n");
    fprintf(f, "        active-color \"%s\"\n", scheme->base08);
    fprintf(f, "        inactive-color \"%s\"\n", scheme->base03);
    fprintf(f, "        urgent-color \"%s\"\n", scheme->base09);
    fprintf(f, "    }\n");
    fprintf(f, "\n");
    
    // Insert hint configuration
    fprintf(f, "    // Insert hint when moving windows\n");
    fprintf(f, "    insert-hint {\n");
    fprintf(f, "        on\n");
    fprintf(f, "        color \"%s80\"\n", scheme->base0A);
    fprintf(f, "    }\n");
    
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Add font configuration as a comment if available
    if (font && (font->monospace[0] || font->sansserif[0])) {
        fprintf(f, "// Font configuration (example)\n");
        fprintf(f, "// You can set fonts using environment variables:\n");
        if (font->monospace[0]) {
            fprintf(f, "// export XCURSOR_THEME=\"%s\"\n", font->monospace);
        }
        if (font->sansserif[0]) {
            fprintf(f, "// Or configure your bar/terminal to use: %s\n", font->sansserif);
        }
        fprintf(f, "\n");
    }
    
    // Add color scheme reference as comments
    fprintf(f, "// Base16 Color Reference:\n");
    fprintf(f, "// base00 (background): %s\n", scheme->base00);
    fprintf(f, "// base01 (lighter bg): %s\n", scheme->base01);
    fprintf(f, "// base02 (selection):  %s\n", scheme->base02);
    fprintf(f, "// base03 (comments):   %s\n", scheme->base03);
    fprintf(f, "// base04 (dark fg):    %s\n", scheme->base04);
    fprintf(f, "// base05 (foreground): %s\n", scheme->base05);
    fprintf(f, "// base06 (light fg):   %s\n", scheme->base06);
    fprintf(f, "// base07 (bright):     %s\n", scheme->base07);
    fprintf(f, "// base08 (red):        %s\n", scheme->base08);
    fprintf(f, "// base09 (orange):     %s\n", scheme->base09);
    fprintf(f, "// base0A (yellow):     %s\n", scheme->base0A);
    fprintf(f, "// base0B (green):      %s\n", scheme->base0B);
    fprintf(f, "// base0C (cyan):       %s\n", scheme->base0C);
    fprintf(f, "// base0D (blue):       %s\n", scheme->base0D);
    fprintf(f, "// base0E (magenta):    %s\n", scheme->base0E);
    fprintf(f, "// base0F (brown):      %s\n", scheme->base0F);
    
    fclose(f);
    return 0;
}

// Apply niri theme to current niri configuration
int niri_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Construct niri config directory path
    char config_dir[1024];
    const char *xdg_config = getenv("XDG_CONFIG_HOME");
    if (xdg_config && xdg_config[0]) {
        snprintf(config_dir, sizeof(config_dir), "%s/niri", xdg_config);
    } else {
        snprintf(config_dir, sizeof(config_dir), "%s/.config/niri", home);
    }
    
    // Create config directory if it doesn't exist
    struct stat st = {0};
    if (stat(config_dir, &st) == -1) {
        if (mkdir(config_dir, 0755) == -1) {
            fprintf(stderr, "Failed to create niri config directory: %s\n", config_dir);
            return -1;
        }
    }
    
    // Generate theme file path
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.kdl", config_dir);
    
    // Generate the theme file
    if (niri_generate_theme(scheme, theme_path, font) != 0) {
        return -1;
    }
    
    printf("Generated niri theme: %s\n", theme_path);
    printf("To use this theme, add the following line to your niri config.kdl:\n");
    printf("  include \"%s\"\n", theme_path);
    printf("\nNote: niri will automatically reload the config when you save it.\n");
    
    return 0;
}
