#include "hyprland.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <pwd.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Generate Hyprland theme file from a Base16 scheme
static int hyprland_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create Hyprland theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Base16 color variables
    fprintf(f, "# Base16 color variables\n");
    fprintf(f, "$base00 = rgb(%s)  # Default Background\n", strip_hash(scheme->base00));
    fprintf(f, "$base01 = rgb(%s)  # Lighter Background\n", strip_hash(scheme->base01));
    fprintf(f, "$base02 = rgb(%s)  # Selection Background\n", strip_hash(scheme->base02));
    fprintf(f, "$base03 = rgb(%s)  # Comments, Invisibles\n", strip_hash(scheme->base03));
    fprintf(f, "$base04 = rgb(%s)  # Dark Foreground\n", strip_hash(scheme->base04));
    fprintf(f, "$base05 = rgb(%s)  # Default Foreground\n", strip_hash(scheme->base05));
    fprintf(f, "$base06 = rgb(%s)  # Light Foreground\n", strip_hash(scheme->base06));
    fprintf(f, "$base07 = rgb(%s)  # Light Background\n", strip_hash(scheme->base07));
    fprintf(f, "$base08 = rgb(%s)  # Red\n", strip_hash(scheme->base08));
    fprintf(f, "$base09 = rgb(%s)  # Orange\n", strip_hash(scheme->base09));
    fprintf(f, "$base0A = rgb(%s)  # Yellow\n", strip_hash(scheme->base0A));
    fprintf(f, "$base0B = rgb(%s)  # Green\n", strip_hash(scheme->base0B));
    fprintf(f, "$base0C = rgb(%s)  # Cyan\n", strip_hash(scheme->base0C));
    fprintf(f, "$base0D = rgb(%s)  # Blue\n", strip_hash(scheme->base0D));
    fprintf(f, "$base0E = rgb(%s)  # Magenta\n", strip_hash(scheme->base0E));
    fprintf(f, "$base0F = rgb(%s)  # Brown\n", strip_hash(scheme->base0F));
    fprintf(f, "\n");
    
    // Example usage section
    fprintf(f, "# Example theming - uncomment and customize:\n");
    fprintf(f, "#\n");
    fprintf(f, "# general {\n");
    fprintf(f, "#     col.active_border = $base0D $base0C 45deg  # Blue to Cyan gradient\n");
    fprintf(f, "#     col.inactive_border = $base01  # Dim border\n");
    fprintf(f, "# }\n");
    fprintf(f, "#\n");
    fprintf(f, "# decoration {\n");
    fprintf(f, "#     col.shadow = rgba(%saa)  # Shadow with alpha\n", strip_hash(scheme->base00));
    fprintf(f, "# }\n");
    fprintf(f, "#\n");
    fprintf(f, "# group {\n");
    fprintf(f, "#     col.border_active = $base0D\n");
    fprintf(f, "#     col.border_inactive = $base01\n");
    fprintf(f, "#     col.border_locked_active = $base09\n");
    fprintf(f, "#     col.border_locked_inactive = $base03\n");
    fprintf(f, "#\n");
    fprintf(f, "#     groupbar {\n");
    fprintf(f, "#         col.active = $base0D\n");
    fprintf(f, "#         col.inactive = $base03\n");
    fprintf(f, "#         text_color = $base07\n");
    fprintf(f, "#     }\n");
    fprintf(f, "# }\n");
    fprintf(f, "#\n");
    fprintf(f, "# misc {\n");
    fprintf(f, "#     background_color = $base00\n");
    fprintf(f, "#     col.splash = $base0E\n");
    fprintf(f, "# }\n");
    
    fclose(f);
    return 0;
}

// Apply Hyprland theme
int hyprland_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        struct passwd *pw = getpwuid(getuid());
        if (pw) {
            home = pw->pw_dir;
        } else {
            fprintf(stderr, "Could not determine home directory\n");
            return -1;
        }
    }
    
    // Create hypr config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/hypr", home);
    mkdir(config_dir, 0755);
    
    // Generate theme path
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.conf", config_dir);
    
    if (hyprland_generate_theme(scheme, theme_path, font) != 0) {
        return -1;
    }
    
    printf("  ✓ %s\n", theme_path);
    
    return 0;
}
