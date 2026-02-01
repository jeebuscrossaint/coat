//
// Created by amarnath on 1/19/26.
//

#include "kitty.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Generate a kitty terminal theme file from a Base16 scheme
int kitty_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font, const OpacityConfig *opacity) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create kitty theme file: %s\n", output_path);
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
        fprintf(f, "font_family %s\n", font->monospace);
        fprintf(f, "font_size %d\n", font->sizes.terminal);
        fprintf(f, "\n");
    }
    
    // Opacity configuration
    if (opacity && opacity->terminal < 1.0) {
        fprintf(f, "# Opacity\n");
        fprintf(f, "background_opacity %.2f\n", opacity->terminal);
        fprintf(f, "\n");
    }
    
    // Basic colors
    fprintf(f, "# Basic colors\n");
    fprintf(f, "foreground %s\n", scheme->base05);
    fprintf(f, "background %s\n", scheme->base00);
    fprintf(f, "selection_foreground %s\n", scheme->base00);
    fprintf(f, "selection_background %s\n", scheme->base05);
    fprintf(f, "\n");
    
    // Cursor colors
    fprintf(f, "# Cursor colors\n");
    fprintf(f, "cursor %s\n", scheme->base05);
    fprintf(f, "cursor_text_color %s\n", scheme->base00);
    fprintf(f, "\n");
    
    // URL underline color
    fprintf(f, "# URL underline color when hovering with mouse\n");
    fprintf(f, "url_color %s\n", scheme->base0D);
    fprintf(f, "\n");
    
    // Border colors
    fprintf(f, "# Border colors\n");
    fprintf(f, "active_border_color %s\n", scheme->base0D);
    fprintf(f, "inactive_border_color %s\n", scheme->base03);
    fprintf(f, "bell_border_color %s\n", scheme->base09);
    fprintf(f, "\n");
    
    // Tab bar colors
    fprintf(f, "# Tab bar colors\n");
    fprintf(f, "active_tab_foreground %s\n", scheme->base00);
    fprintf(f, "active_tab_background %s\n", scheme->base0D);
    fprintf(f, "inactive_tab_foreground %s\n", scheme->base04);
    fprintf(f, "inactive_tab_background %s\n", scheme->base01);
    fprintf(f, "tab_bar_background %s\n", scheme->base00);
    fprintf(f, "\n");
    
    // Mark colors
    fprintf(f, "# Marks\n");
    fprintf(f, "mark1_foreground %s\n", scheme->base00);
    fprintf(f, "mark1_background %s\n", scheme->base0D);
    fprintf(f, "mark2_foreground %s\n", scheme->base00);
    fprintf(f, "mark2_background %s\n", scheme->base0E);
    fprintf(f, "mark3_foreground %s\n", scheme->base00);
    fprintf(f, "mark3_background %s\n", scheme->base0C);
    fprintf(f, "\n");
    
    // ANSI color palette (0-15)
    // Base16 to ANSI color mapping
    fprintf(f, "# The 16 terminal colors\n");
    fprintf(f, "\n");
    fprintf(f, "# Black\n");
    fprintf(f, "color0 %s\n", scheme->base00);
    fprintf(f, "color8 %s\n", scheme->base03);
    fprintf(f, "\n");
    
    fprintf(f, "# Red\n");
    fprintf(f, "color1 %s\n", scheme->base08);
    fprintf(f, "color9 %s\n", scheme->base08);
    fprintf(f, "\n");
    
    fprintf(f, "# Green\n");
    fprintf(f, "color2 %s\n", scheme->base0B);
    fprintf(f, "color10 %s\n", scheme->base0B);
    fprintf(f, "\n");
    
    fprintf(f, "# Yellow\n");
    fprintf(f, "color3 %s\n", scheme->base0A);
    fprintf(f, "color11 %s\n", scheme->base0A);
    fprintf(f, "\n");
    
    fprintf(f, "# Blue\n");
    fprintf(f, "color4 %s\n", scheme->base0D);
    fprintf(f, "color12 %s\n", scheme->base0D);
    fprintf(f, "\n");
    
    fprintf(f, "# Magenta\n");
    fprintf(f, "color5 %s\n", scheme->base0E);
    fprintf(f, "color13 %s\n", scheme->base0E);
    fprintf(f, "\n");
    
    fprintf(f, "# Cyan\n");
    fprintf(f, "color6 %s\n", scheme->base0C);
    fprintf(f, "color14 %s\n", scheme->base0C);
    fprintf(f, "\n");
    
    fprintf(f, "# White\n");
    fprintf(f, "color7 %s\n", scheme->base05);
    fprintf(f, "color15 %s\n", scheme->base07);
    
    fclose(f);
    return 0;
}

// Apply kitty theme to current kitty configuration
int kitty_apply_theme(const Base16Scheme *scheme, const FontConfig *font, const OpacityConfig *opacity) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create kitty config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/kitty", home);
    mkdir(config_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.conf", config_dir);
    
    printf("Generating kitty theme: %s\n", theme_path);
    
    if (kitty_generate_theme(scheme, theme_path, font, opacity) != 0) {
        return -1;
    }
    
    printf("Kitty theme generated successfully!\n");
    
    // Auto-apply the theme using kitty remote control
    printf("Activating theme...\n");
    
    // First try to apply colors to all running kitty instances
    char apply_cmd[2048];
    snprintf(apply_cmd, sizeof(apply_cmd), 
             "kitty @ --to unix:/tmp/kitty set-colors --all --configured %s 2>/dev/null", 
             theme_path);
    
    int result = system(apply_cmd);
    
    if (result == 0) {
        printf("✓ Kitty theme activated in running instances!\n");
    } else {
        // Try alternative method - send SIGUSR1 to reload config
        printf("Attempting to reload kitty config...\n");
        result = system("pkill -SIGUSR1 kitty 2>/dev/null");
        
        if (result == 0) {
            printf("✓ Sent reload signal to kitty!\n");
        }
    }
    
    printf("\nTo make permanent, add to ~/.config/kitty/kitty.conf:\n");
    printf("  include coat-theme.conf\n");
    printf("\nOr reload manually: Ctrl+Shift+F5\n");
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
