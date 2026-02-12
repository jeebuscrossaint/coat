#include "qt.h"
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

// Generate Qt5ct/Qt6ct color scheme
static void write_qt_colorscheme(FILE *f, const Base16Scheme *scheme) {
    // Qt color scheme format uses comma-separated colors in hex format
    // Each color group needs 18 color values
    
    fprintf(f, "[ColorScheme]\n");
    
    // Active colors (normal state)
    fprintf(f, "active_colors=#%s, ", strip_hash(scheme->base05));  // WindowText
    fprintf(f, "#%s, ", strip_hash(scheme->base01));  // Button
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Light
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Dark
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Mid
    fprintf(f, "#%s, ", strip_hash(scheme->base05));  // Text
    fprintf(f, "#%s, ", strip_hash(scheme->base07));  // BrightText
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Base
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Window
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // Shadow
    fprintf(f, "#%s, ", strip_hash(scheme->base0D));  // Highlight
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // HighlightedText
    fprintf(f, "#%s, ", strip_hash(scheme->base0D));  // Link
    fprintf(f, "#%s, ", strip_hash(scheme->base0E));  // LinkVisited
    fprintf(f, "#%s, ", strip_hash(scheme->base05));  // AlternateBase
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // ToolTipBase
    fprintf(f, "#%s, ", strip_hash(scheme->base05));  // ToolTipText
    fprintf(f, "#%s\n", strip_hash(scheme->base05));  // PlaceholderText
    
    // Disabled colors (grayed out state)
    fprintf(f, "disabled_colors=#%s, ", strip_hash(scheme->base03));  // WindowText
    fprintf(f, "#%s, ", strip_hash(scheme->base01));  // Button
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Light
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Dark
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Mid
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // Text
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // BrightText
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Base
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Window
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // Shadow
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Highlight
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // HighlightedText
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // Link
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // LinkVisited
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // AlternateBase
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // ToolTipBase
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // ToolTipText
    fprintf(f, "#%s\n", strip_hash(scheme->base03));  // PlaceholderText
    
    // Inactive colors (unfocused state)
    fprintf(f, "inactive_colors=#%s, ", strip_hash(scheme->base04));  // WindowText
    fprintf(f, "#%s, ", strip_hash(scheme->base01));  // Button
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Light
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Dark
    fprintf(f, "#%s, ", strip_hash(scheme->base02));  // Mid
    fprintf(f, "#%s, ", strip_hash(scheme->base04));  // Text
    fprintf(f, "#%s, ", strip_hash(scheme->base05));  // BrightText
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Base
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // Window
    fprintf(f, "#%s, ", strip_hash(scheme->base03));  // Shadow
    fprintf(f, "#%s, ", strip_hash(scheme->base0C));  // Highlight
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // HighlightedText
    fprintf(f, "#%s, ", strip_hash(scheme->base0D));  // Link
    fprintf(f, "#%s, ", strip_hash(scheme->base0E));  // LinkVisited
    fprintf(f, "#%s, ", strip_hash(scheme->base04));  // AlternateBase
    fprintf(f, "#%s, ", strip_hash(scheme->base00));  // ToolTipBase
    fprintf(f, "#%s, ", strip_hash(scheme->base04));  // ToolTipText
    fprintf(f, "#%s\n", strip_hash(scheme->base04));  // PlaceholderText
}

// Apply Qt theme
int qt_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    // Get home directory
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create Qt5ct and Qt6ct directories
    char qt5ct_dir[1024];
    char qt6ct_dir[1024];
    char qt5ct_colors_dir[1024];
    char qt6ct_colors_dir[1024];
    
    snprintf(qt5ct_dir, sizeof(qt5ct_dir), "%s/.config/qt5ct", home);
    snprintf(qt6ct_dir, sizeof(qt6ct_dir), "%s/.config/qt6ct", home);
    snprintf(qt5ct_colors_dir, sizeof(qt5ct_colors_dir), "%s/colors", qt5ct_dir);
    snprintf(qt6ct_colors_dir, sizeof(qt6ct_colors_dir), "%s/colors", qt6ct_dir);
    
    // Create directories
    mkdir(qt5ct_dir, 0755);
    mkdir(qt6ct_dir, 0755);
    mkdir(qt5ct_colors_dir, 0755);
    mkdir(qt6ct_colors_dir, 0755);
    
    // Generate Qt5ct color scheme
    char qt5ct_conf[1024];
    snprintf(qt5ct_conf, sizeof(qt5ct_conf), "%s/coat.conf", qt5ct_colors_dir);
    
    FILE *f = fopen(qt5ct_conf, "w");
    if (!f) {
        fprintf(stderr, "Failed to create Qt5ct color scheme: %s\n", qt5ct_conf);
        return -1;
    }
    
    write_qt_colorscheme(f, scheme);
    fclose(f);
    
    // Generate Qt6ct color scheme
    char qt6ct_conf[1024];
    snprintf(qt6ct_conf, sizeof(qt6ct_conf), "%s/coat.conf", qt6ct_colors_dir);
    
    f = fopen(qt6ct_conf, "w");
    if (!f) {
        fprintf(stderr, "Failed to create Qt6ct color scheme: %s\n", qt6ct_conf);
        return -1;
    }
    
    write_qt_colorscheme(f, scheme);
    fclose(f);
    
    // Update qt5ct.conf to use the coat color scheme
    char qt5ct_main_conf[1024];
    snprintf(qt5ct_main_conf, sizeof(qt5ct_main_conf), "%s/qt5ct.conf", qt5ct_dir);
    
    f = fopen(qt5ct_main_conf, "r");
    char *conf_content = NULL;
    size_t conf_size = 0;
    
    if (f) {
        // Read existing config
        fseek(f, 0, SEEK_END);
        conf_size = ftell(f);
        fseek(f, 0, SEEK_SET);
        conf_content = malloc(conf_size + 1);
        if (conf_content) {
            fread(conf_content, 1, conf_size, f);
            conf_content[conf_size] = '\0';
        }
        fclose(f);
    }
    
    f = fopen(qt5ct_main_conf, "w");
    if (f) {
        int appearance_written = 0;
        
        if (conf_content) {
            // Parse and update existing config
            char *line = strtok(conf_content, "\n");
            while (line) {
                if (strncmp(line, "[Appearance]", 12) == 0) {
                    fprintf(f, "[Appearance]\n");
                    fprintf(f, "color_scheme_path=%s/coat.conf\n", qt5ct_colors_dir);
                    if (font && font->sansserif[0]) {
                        fprintf(f, "standard_dialogs=default\n");
                        fprintf(f, "fixed=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                                font->monospace, font->sizes.terminal);
                        fprintf(f, "general=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                                font->sansserif, font->sizes.terminal);
                    }
                    appearance_written = 1;
                    // Skip lines until next section
                    line = strtok(NULL, "\n");
                    while (line && line[0] != '[') {
                        line = strtok(NULL, "\n");
                    }
                    continue;
                }
                fprintf(f, "%s\n", line);
                line = strtok(NULL, "\n");
            }
        }
        
        if (!appearance_written) {
            fprintf(f, "[Appearance]\n");
            fprintf(f, "color_scheme_path=%s/coat.conf\n", qt5ct_colors_dir);
            if (font && font->sansserif[0]) {
                fprintf(f, "standard_dialogs=default\n");
                fprintf(f, "fixed=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                        font->monospace, font->sizes.terminal);
                fprintf(f, "general=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                        font->sansserif, font->sizes.terminal);
            }
        }
        
        fclose(f);
    }
    
    free(conf_content);
    
    // Update qt6ct.conf similarly
    char qt6ct_main_conf[1024];
    snprintf(qt6ct_main_conf, sizeof(qt6ct_main_conf), "%s/qt6ct.conf", qt6ct_dir);
    
    f = fopen(qt6ct_main_conf, "r");
    conf_content = NULL;
    conf_size = 0;
    
    if (f) {
        fseek(f, 0, SEEK_END);
        conf_size = ftell(f);
        fseek(f, 0, SEEK_SET);
        conf_content = malloc(conf_size + 1);
        if (conf_content) {
            fread(conf_content, 1, conf_size, f);
            conf_content[conf_size] = '\0';
        }
        fclose(f);
    }
    
    f = fopen(qt6ct_main_conf, "w");
    if (f) {
        int appearance_written = 0;
        
        if (conf_content) {
            char *line = strtok(conf_content, "\n");
            while (line) {
                if (strncmp(line, "[Appearance]", 12) == 0) {
                    fprintf(f, "[Appearance]\n");
                    fprintf(f, "color_scheme_path=%s/coat.conf\n", qt6ct_colors_dir);
                    if (font && font->sansserif[0]) {
                        fprintf(f, "standard_dialogs=default\n");
                        fprintf(f, "fixed=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                                font->monospace, font->sizes.terminal);
                        fprintf(f, "general=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                                font->sansserif, font->sizes.terminal);
                    }
                    appearance_written = 1;
                    line = strtok(NULL, "\n");
                    while (line && line[0] != '[') {
                        line = strtok(NULL, "\n");
                    }
                    continue;
                }
                fprintf(f, "%s\n", line);
                line = strtok(NULL, "\n");
            }
        }
        
        if (!appearance_written) {
            fprintf(f, "[Appearance]\n");
            fprintf(f, "color_scheme_path=%s/coat.conf\n", qt6ct_colors_dir);
            if (font && font->sansserif[0]) {
                fprintf(f, "standard_dialogs=default\n");
                fprintf(f, "fixed=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                        font->monospace, font->sizes.terminal);
                fprintf(f, "general=@Variant(\\0\\0\\0@\\0\\0\\0\\x1c\\0%s\\0, %d,-1,5,50,0,0,0,0,0)\n", 
                        font->sansserif, font->sizes.terminal);
            }
        }
        
        fclose(f);
    }
    
    free(conf_content);
    
    printf("  ✓ %s\n", qt5ct_conf);
    printf("  ✓ %s\n", qt6ct_conf);
    
    return 0;
}
