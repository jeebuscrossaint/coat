//
// Created by amarnath on 1/19/26.
//

#include "gtk.h"
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

// Generate GTK 3.0 CSS file from a Base16 scheme
static int gtk_generate_gtk3_css(const Base16Scheme *scheme, const char *output_path) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create GTK 3.0 CSS file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "/*\n");
    fprintf(f, " * coat GTK theme: %s\n", scheme->name);
    fprintf(f, " * %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, " * variant: %s\n", scheme->variant);
    }
    fprintf(f, " */\n\n");
    
    // Libadwaita / GNOME color definitions
    // Based on Stylix's GTK implementation
    fprintf(f, "@define-color accent_color #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color accent_bg_color #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color accent_fg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color destructive_color #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color destructive_bg_color #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color destructive_fg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color success_color #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color success_bg_color #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color success_fg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color warning_color #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color warning_bg_color #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color warning_fg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color error_color #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color error_bg_color #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color error_fg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color window_bg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color window_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color view_bg_color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color view_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color headerbar_bg_color #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color headerbar_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color headerbar_border_color rgba(%s, 0.7);\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color headerbar_backdrop_color @window_bg_color;\n");
    fprintf(f, "@define-color headerbar_shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color headerbar_darker_shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color sidebar_bg_color #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color sidebar_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color sidebar_backdrop_color @window_bg_color;\n");
    fprintf(f, "@define-color sidebar_shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color secondary_sidebar_bg_color @sidebar_bg_color;\n");
    fprintf(f, "@define-color secondary_sidebar_fg_color @sidebar_fg_color;\n");
    fprintf(f, "@define-color secondary_sidebar_backdrop_color @sidebar_backdrop_color;\n");
    fprintf(f, "@define-color secondary_sidebar_shade_color @sidebar_shade_color;\n");
    fprintf(f, "@define-color card_bg_color #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color card_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color card_shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color dialog_bg_color #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color dialog_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color popover_bg_color #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color popover_fg_color #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color popover_shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color shade_color rgba(0, 0, 0, 0.07);\n");
    fprintf(f, "@define-color scrollbar_outline_color #%s;\n", strip_hash(scheme->base02));
    fprintf(f, "\n");
    
    // Extended color palette for widgets
    fprintf(f, "/* Extended palette */\n");
    fprintf(f, "@define-color blue_1 #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color blue_2 #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color blue_3 #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color blue_4 #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color blue_5 #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color green_1 #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color green_2 #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color green_3 #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color green_4 #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color green_5 #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color yellow_1 #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color yellow_2 #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color yellow_3 #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color yellow_4 #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color yellow_5 #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color orange_1 #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color orange_2 #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color orange_3 #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color orange_4 #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color orange_5 #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color red_1 #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color red_2 #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color red_3 #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color red_4 #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color red_5 #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color purple_1 #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color purple_2 #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color purple_3 #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color purple_4 #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color purple_5 #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color brown_1 #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "@define-color brown_2 #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "@define-color brown_3 #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "@define-color brown_4 #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "@define-color brown_5 #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "@define-color light_1 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color light_2 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color light_3 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color light_4 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color light_5 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color dark_1 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color dark_2 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color dark_3 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color dark_4 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color dark_5 #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "\n");
    
    fclose(f);
    return 0;
}

// Apply GTK theme
int gtk_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    // Get home directory
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create GTK config directories
    char gtk3_dir[1024];
    char gtk4_dir[1024];
    snprintf(gtk3_dir, sizeof(gtk3_dir), "%s/.config/gtk-3.0", home);
    snprintf(gtk4_dir, sizeof(gtk4_dir), "%s/.config/gtk-4.0", home);
    
    // Create directories if they don't exist
    mkdir(gtk3_dir, 0755);
    mkdir(gtk4_dir, 0755);
    
    // Generate GTK 3.0 CSS
    char gtk3_css_path[1024];
    snprintf(gtk3_css_path, sizeof(gtk3_css_path), "%s/gtk.css", gtk3_dir);
    
    if (gtk_generate_gtk3_css(scheme, gtk3_css_path) != 0) {
        return -1;
    }
    
    printf("  Created GTK 3.0 theme: %s\n", gtk3_css_path);
    
    // Generate GTK 4.0 CSS (same content as GTK 3.0)
    char gtk4_css_path[1024];
    snprintf(gtk4_css_path, sizeof(gtk4_css_path), "%s/gtk.css", gtk4_dir);
    
    if (gtk_generate_gtk3_css(scheme, gtk4_css_path) != 0) {
        return -1;
    }
    
    printf("  Created GTK 4.0 theme: %s\n", gtk4_css_path);
    
    // Create GTK settings file for GTK 3.0
    char gtk3_settings_path[1024];
    snprintf(gtk3_settings_path, sizeof(gtk3_settings_path), "%s/settings.ini", gtk3_dir);
    
    FILE *f = fopen(gtk3_settings_path, "w");
    if (f) {
        fprintf(f, "[Settings]\n");
        
        // Set theme preference based on variant
        if (scheme->variant[0] && strcmp(scheme->variant, "dark") == 0) {
            fprintf(f, "gtk-application-prefer-dark-theme=1\n");
        } else {
            fprintf(f, "gtk-application-prefer-dark-theme=0\n");
        }
        
        // Set font if provided
        if (font && font->sansserif[0]) {
            fprintf(f, "gtk-font-name=%s 10\n", font->sansserif);
        }
        
        fclose(f);
        printf("  Created GTK 3.0 settings: %s\n", gtk3_settings_path);
    }
    
    // Create GTK settings file for GTK 4.0
    char gtk4_settings_path[1024];
    snprintf(gtk4_settings_path, sizeof(gtk4_settings_path), "%s/settings.ini", gtk4_dir);
    
    f = fopen(gtk4_settings_path, "w");
    if (f) {
        fprintf(f, "[Settings]\n");
        
        // Set theme preference based on variant
        if (scheme->variant[0] && strcmp(scheme->variant, "dark") == 0) {
            fprintf(f, "gtk-application-prefer-dark-theme=1\n");
        } else {
            fprintf(f, "gtk-application-prefer-dark-theme=0\n");
        }
        
        // Set font if provided
        if (font && font->sansserif[0]) {
            fprintf(f, "gtk-font-name=%s 10\n", font->sansserif);
        }
        
        fclose(f);
        printf("  Created GTK 4.0 settings: %s\n", gtk4_settings_path);
    }
    
    printf("  GTK theme applied successfully!\n");
    printf("  Note: You may need to restart GTK applications for changes to take effect.\n");
    
    return 0;
}
