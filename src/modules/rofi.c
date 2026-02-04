#include "rofi.h"
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

// Generate a rofi theme file from a Base16 scheme
int rofi_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create rofi theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "/**\n");
    fprintf(f, " * coat theme: %s\n", scheme->name);
    fprintf(f, " * %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, " * variant: %s\n", scheme->variant);
    }
    fprintf(f, " */\n\n");
    
    // Define color variables
    fprintf(f, "* {\n");
    fprintf(f, "    base00: #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "    base01: #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "    base02: #%s;\n", strip_hash(scheme->base02));
    fprintf(f, "    base03: #%s;\n", strip_hash(scheme->base03));
    fprintf(f, "    base04: #%s;\n", strip_hash(scheme->base04));
    fprintf(f, "    base05: #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "    base06: #%s;\n", strip_hash(scheme->base06));
    fprintf(f, "    base07: #%s;\n", strip_hash(scheme->base07));
    fprintf(f, "    base08: #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "    base09: #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "    base0A: #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "    base0B: #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "    base0C: #%s;\n", strip_hash(scheme->base0C));
    fprintf(f, "    base0D: #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "    base0E: #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "    base0F: #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "\n");
    
    // Simplified Base16 color scheme - cleaner approach
    fprintf(f, "    background:     @base00;\n");
    fprintf(f, "    background-alt: @base01;\n");
    fprintf(f, "    foreground:     @base05;\n");
    fprintf(f, "    selected:       @base0D;\n");
    fprintf(f, "    active:         @base0B;\n");
    fprintf(f, "    urgent:         @base08;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Configuration
    fprintf(f, "configuration {\n");
    fprintf(f, "    display-drun: \"Applications:\";\n");
    fprintf(f, "    display-window: \"Windows:\";\n");
    fprintf(f, "    drun-display-format: \"{name}\";\n");
    int font_size = font ? font->sizes.popups : 10;
    fprintf(f, "    font: \"%s %d\";\n", font && font->monospace[0] ? font->monospace : "monospace", font_size);
    fprintf(f, "    modi: \"window,run,drun\";\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Window styling
    fprintf(f, "window {\n");
    fprintf(f, "    transparency: \"real\";\n");
    fprintf(f, "    background-color: @background;\n");
    fprintf(f, "    text-color: @foreground;\n");
    fprintf(f, "    border: 2px;\n");
    fprintf(f, "    border-color: @selected;\n");
    fprintf(f, "    border-radius: 0px;\n");
    fprintf(f, "    width: 30%%;\n");
    fprintf(f, "    location: center;\n");
    fprintf(f, "    anchor: center;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Prompt
    fprintf(f, "prompt {\n");
    fprintf(f, "    enabled: true;\n");
    fprintf(f, "    padding: 8px;\n");
    fprintf(f, "    background-color: @background-alt;\n");
    fprintf(f, "    text-color: @foreground;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Text entry
    fprintf(f, "entry {\n");
    fprintf(f, "    background-color: @background-alt;\n");
    fprintf(f, "    text-color: @foreground;\n");
    fprintf(f, "    placeholder-color: @base03;\n");
    fprintf(f, "    expand: true;\n");
    fprintf(f, "    horizontal-align: 0;\n");
    fprintf(f, "    placeholder: \"Search...\";\n");
    fprintf(f, "    blink: true;\n");
    fprintf(f, "    padding: 8px;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Input bar
    fprintf(f, "inputbar {\n");
    fprintf(f, "    children: [ prompt, entry ];\n");
    fprintf(f, "    background-color: @background-alt;\n");
    fprintf(f, "    text-color: @foreground;\n");
    fprintf(f, "    expand: false;\n");
    fprintf(f, "    border: 0 0 1px 0;\n");
    fprintf(f, "    border-color: @selected;\n");
    fprintf(f, "    margin: 0;\n");
    fprintf(f, "    padding: 0;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // List view
    fprintf(f, "listview {\n");
    fprintf(f, "    background-color: @background;\n");
    fprintf(f, "    columns: 1;\n");
    fprintf(f, "    lines: 8;\n");
    fprintf(f, "    spacing: 4px;\n");
    fprintf(f, "    cycle: false;\n");
    fprintf(f, "    dynamic: true;\n");
    fprintf(f, "    layout: vertical;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    // Main List Elements
    fprintf(f, "element {\n");
    fprintf(f, "    background-color: @background;\n");
    fprintf(f, "    text-color: @foreground;\n");
    fprintf(f, "    orientation: horizontal;\n");
    fprintf(f, "    border-radius: 0;\n");
    fprintf(f, "    padding: 8px;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element-icon {\n");
    fprintf(f, "    background-color: inherit;\n");
    fprintf(f, "    text-color: inherit;\n");
    fprintf(f, "    size: 24px;\n");
    fprintf(f, "    border: 0;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element-text {\n");
    fprintf(f, "    background-color: inherit;\n");
    fprintf(f, "    text-color: inherit;\n");
    fprintf(f, "    expand: true;\n");
    fprintf(f, "    horizontal-align: 0;\n");
    fprintf(f, "    vertical-align: 0.5;\n");
    fprintf(f, "    margin: 0 0 0 4px;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element normal.urgent,\n");
    fprintf(f, "element alternate.urgent {\n");
    fprintf(f, "    background-color: @urgent;\n");
    fprintf(f, "    text-color: @background;\n");
    fprintf(f, "    border-radius: 0;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element normal.active,\n");
    fprintf(f, "element alternate.active {\n");
    fprintf(f, "    background-color: @active;\n");
    fprintf(f, "    text-color: @background;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element selected {\n");
    fprintf(f, "    background-color: @selected;\n");
    fprintf(f, "    text-color: @background;\n");
    fprintf(f, "    border: 0;\n");
    fprintf(f, "    border-radius: 0;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element selected.urgent {\n");
    fprintf(f, "    background-color: @urgent;\n");
    fprintf(f, "    text-color: @background;\n");
    fprintf(f, "}\n");
    fprintf(f, "\n");
    
    fprintf(f, "element selected.active {\n");
    fprintf(f, "    background-color: @active;\n");
    fprintf(f, "    text-color: @background;\n");
    fprintf(f, "}\n");
    
    fclose(f);
    return 0;
}

// Apply rofi theme to current rofi configuration
int rofi_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create rofi config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/rofi", home);
    mkdir(config_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat.rasi", config_dir);
    
    printf("Generating rofi theme: %s\n", theme_path);
    
    if (rofi_generate_theme(scheme, theme_path, font) != 0) {
        return -1;
    }
    
    printf("  ✓ %s\n", theme_path);
    
    return 0;
}
