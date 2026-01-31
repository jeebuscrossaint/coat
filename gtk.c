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

// Generate GTK color definitions to an already-open file
static void write_gtk_colors(FILE *f, const Base16Scheme *scheme) {
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
    fprintf(f, "@define-color headerbar_border_color #%s;\n", strip_hash(scheme->base01));
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
    
    // Override Adwaita's colors with our Base16 colors
    fprintf(f, "/* Color overrides */\n");
    fprintf(f, "window { background-color: @window_bg_color; color: @window_fg_color; }\n");
    fprintf(f, "headerbar { background-color: @headerbar_bg_color; color: @headerbar_fg_color; }\n");
    fprintf(f, "entry { background-color: @view_bg_color; color: @view_fg_color; caret-color: @accent_color; }\n");
    fprintf(f, "textview text { background-color: @view_bg_color; color: @view_fg_color; }\n");
    fprintf(f, "button { background: @accent_bg_color; color: @accent_fg_color; }\n");
    fprintf(f, "button:checked { background: @accent_bg_color; }\n");
    fprintf(f, ".view { background-color: @view_bg_color; color: @view_fg_color; }\n");
    fprintf(f, "list { background-color: @view_bg_color; color: @view_fg_color; }\n");
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
    
    // Create theme directory
    char theme_dir[1024];
    char gtk3_dir[1024];
    char gtk4_dir[1024];
    
    snprintf(theme_dir, sizeof(theme_dir), "%s/.themes/coat", home);
    snprintf(gtk3_dir, sizeof(gtk3_dir), "%s/gtk-3.0", theme_dir);
    snprintf(gtk4_dir, sizeof(gtk4_dir), "%s/gtk-4.0", theme_dir);
    
    // Create directories
    mkdir(theme_dir, 0755);
    mkdir(gtk3_dir, 0755);
    mkdir(gtk4_dir, 0755);
    
    // Generate GTK 3.0 CSS with Adwaita import
    char gtk3_css[1024];
    snprintf(gtk3_css, sizeof(gtk3_css), "%s/gtk.css", gtk3_dir);
    
    FILE *f = fopen(gtk3_css, "w");
    if (!f) {
        fprintf(stderr, "Failed to create GTK 3.0 CSS: %s\n", gtk3_css);
        return -1;
    }
    
    // Import base Adwaita dark theme, then override colors
    fprintf(f, "/* coat GTK theme based on Adwaita-dark */\n");
    fprintf(f, "@import url(\"resource:///org/gtk/libgtk/theme/Adwaita/gtk-contained-dark.css\");\n\n");
    
    // Write color overrides
    write_gtk_colors(f, scheme);
    fclose(f);
    
    // Generate GTK 4.0 CSS
    char gtk4_css[1024];
    snprintf(gtk4_css, sizeof(gtk4_css), "%s/gtk.css", gtk4_dir);
    
    f = fopen(gtk4_css, "w");
    if (!f) {
        fprintf(stderr, "Failed to create GTK 4.0 CSS: %s\n", gtk4_css);
        return -1;
    }
    
    fprintf(f, "/* coat GTK 4 theme based on Adwaita-dark */\n");
    fprintf(f, "@import url(\"resource:///org/gtk/libgtk/theme/Adwaita/gtk-contained-dark.css\");\n\n");
    write_gtk_colors(f, scheme);
    fclose(f);
    
    // Create index.theme
    char index_theme[1024];
    snprintf(index_theme, sizeof(index_theme), "%s/index.theme", theme_dir);
    
    f = fopen(index_theme, "w");
    if (!f) {
        fprintf(stderr, "Failed to create index.theme: %s\n", index_theme);
        return -1;
    }
    
    fprintf(f, "[Desktop Entry]\n");
    fprintf(f, "Type=X-GNOME-Metatheme\n");
    fprintf(f, "Name=coat\n");
    fprintf(f, "Comment=Base16 theme: %s (based on Adwaita-dark)\n", scheme->name);
    fprintf(f, "Encoding=UTF-8\n\n");
    fprintf(f, "[X-GNOME-Metatheme]\n");
    fprintf(f, "GtkTheme=coat\n");
    fprintf(f, "IconTheme=Adwaita\n");
    fprintf(f, "CursorTheme=Adwaita\n");
    fclose(f);
    
    // Set GTK font via gsettings
    char font_main[256] = "";
    char font_mono[256] = "";
    if (font) {
        char font_cmd[512];
        // Always use the same font and size for both font-name and monospace-font-name
        snprintf(font_main, sizeof(font_main), "%s%s", font->sansserif[0] ? font->sansserif : "Sans",
             strstr(font->sansserif, "Regular") ? "" : " Regular");
        snprintf(font_cmd, sizeof(font_cmd),
            "gsettings set org.gnome.desktop.interface font-name '%s %d' 2>/dev/null",
            font_main, font->sizes.terminal);
        system(font_cmd);

        snprintf(font_mono, sizeof(font_mono), "%s%s", font->monospace[0] ? font->monospace : "monospace",
             strstr(font->monospace, "Regular") ? "" : " Regular");
        snprintf(font_cmd, sizeof(font_cmd),
            "gsettings set org.gnome.desktop.interface monospace-font-name '%s %d' 2>/dev/null",
            font_mono, font->sizes.terminal);
        system(font_cmd);
    }
    
    printf("  Created GTK theme: %s\n", theme_dir);
    printf("  ✓ GTK 3.0: %s\n", gtk3_css);
    printf("  ✓ GTK 4.0: %s\n", gtk4_css);
    if (font) {
        printf("  ✓ Set font: %s %d, monospace: %s %d\n",
               font_main, font->sizes.terminal, font_mono, font->sizes.terminal);
    }
    printf("\n  To apply: Select 'coat' in nwg-look or run:\n");
    printf("    gsettings set org.gnome.desktop.interface gtk-theme 'coat'\n");
    
    return 0;
}
