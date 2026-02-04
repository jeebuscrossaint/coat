#include "i3.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Generate an i3 window manager theme file from a Base16 scheme
int i3_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create i3 theme file: %s\n", output_path);
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
        fprintf(f, "font pango:%s %d\n", font->monospace, font->sizes.desktop);
        fprintf(f, "\n");
    }
    
    // Define color variables for easy reference
    fprintf(f, "# Base16 color definitions\n");
    fprintf(f, "set $base00 %s\n", scheme->base00);
    fprintf(f, "set $base01 %s\n", scheme->base01);
    fprintf(f, "set $base02 %s\n", scheme->base02);
    fprintf(f, "set $base03 %s\n", scheme->base03);
    fprintf(f, "set $base04 %s\n", scheme->base04);
    fprintf(f, "set $base05 %s\n", scheme->base05);
    fprintf(f, "set $base06 %s\n", scheme->base06);
    fprintf(f, "set $base07 %s\n", scheme->base07);
    fprintf(f, "set $base08 %s\n", scheme->base08);
    fprintf(f, "set $base09 %s\n", scheme->base09);
    fprintf(f, "set $base0A %s\n", scheme->base0A);
    fprintf(f, "set $base0B %s\n", scheme->base0B);
    fprintf(f, "set $base0C %s\n", scheme->base0C);
    fprintf(f, "set $base0D %s\n", scheme->base0D);
    fprintf(f, "set $base0E %s\n", scheme->base0E);
    fprintf(f, "set $base0F %s\n", scheme->base0F);
    fprintf(f, "\n");
    
    // Window colors
    fprintf(f, "# Window colors\n");
    fprintf(f, "# Property Name         Border  BG      Text    Indicator Child Border\n");
    fprintf(f, "client.focused          $base0D $base0D $base00 $base0D   $base0C\n");
    fprintf(f, "client.focused_inactive $base01 $base01 $base05 $base03   $base01\n");
    fprintf(f, "client.unfocused        $base01 $base00 $base05 $base01   $base01\n");
    fprintf(f, "client.urgent           $base08 $base08 $base00 $base08   $base08\n");
    fprintf(f, "client.placeholder      $base00 $base00 $base05 $base00   $base00\n");
    fprintf(f, "client.background       $base07\n");
    fprintf(f, "\n");
    
    // i3bar colors
    fprintf(f, "# i3bar colors\n");
    fprintf(f, "bar {\n");
    fprintf(f, "    status_command i3status\n");
    fprintf(f, "    colors {\n");
    fprintf(f, "        background $base00\n");
    fprintf(f, "        separator  $base01\n");
    fprintf(f, "        statusline $base04\n");
    fprintf(f, "\n");
    fprintf(f, "        # State             Border  BG      Text\n");
    fprintf(f, "        focused_workspace   $base0D $base0D $base00\n");
    fprintf(f, "        active_workspace    $base03 $base03 $base00\n");
    fprintf(f, "        inactive_workspace  $base01 $base01 $base05\n");
    fprintf(f, "        urgent_workspace    $base08 $base08 $base00\n");
    fprintf(f, "        binding_mode        $base0A $base0A $base00\n");
    fprintf(f, "    }\n");
    fprintf(f, "}\n");
    
    fclose(f);
    return 0;
}

// Apply i3 theme to current i3 configuration
int i3_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create i3 config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/i3", home);
    mkdir(config_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.conf", config_dir);
    
    printf("Generating i3 theme: %s\n", theme_path);
    
    if (i3_generate_theme(scheme, theme_path, font) != 0) {
        return -1;
    }
    
    printf("  ✓ %s\n", theme_path);
    
    // Try to reload i3 config
    system("i3-msg reload 2>/dev/null");
    
    return 0;
}
