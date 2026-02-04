#include "fish.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char* strip_hash(const char *color) {
    return (color && color[0] == '#') ? color + 1 : color;
}

// Generate a fish shell theme file from a Base16 scheme
int fish_generate_theme(const Base16Scheme *scheme, const char *output_path) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create fish theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Base16 to fish color mappings based on the styling guide
    fprintf(f, "# Syntax Highlighting Colors\n");
    fprintf(f, "fish_color_normal %s\n", strip_hash(scheme->base05));           // Default foreground
    fprintf(f, "fish_color_command %s\n", strip_hash(scheme->base0D));          // Functions (blue)
    fprintf(f, "fish_color_keyword %s\n", strip_hash(scheme->base0E));          // Keywords (magenta)
    fprintf(f, "fish_color_quote %s\n", strip_hash(scheme->base0B));            // Strings (green)
    fprintf(f, "fish_color_redirection %s\n", strip_hash(scheme->base0C));      // Redirection operators (cyan)
    fprintf(f, "fish_color_end %s\n", strip_hash(scheme->base0C));              // End command (cyan)
    fprintf(f, "fish_color_error %s\n", strip_hash(scheme->base08));            // Errors (red)
    fprintf(f, "fish_color_param %s\n", strip_hash(scheme->base05));            // Parameters (default foreground)
    fprintf(f, "fish_color_comment %s\n", strip_hash(scheme->base03));          // Comments (gray)
    fprintf(f, "fish_color_selection --background=%s\n", strip_hash(scheme->base02)); // Selection background
    fprintf(f, "fish_color_search_match --background=%s\n", strip_hash(scheme->base0A)); // Search match (yellow bg)
    fprintf(f, "fish_color_operator %s\n", strip_hash(scheme->base0C));         // Operators (cyan)
    fprintf(f, "fish_color_escape %s\n", strip_hash(scheme->base0C));           // Escape characters (cyan)
    fprintf(f, "fish_color_autosuggestion %s\n", strip_hash(scheme->base03));   // Autosuggestions (gray)
    fprintf(f, "fish_color_cwd %s\n", strip_hash(scheme->base0D));              // Current directory (blue)
    fprintf(f, "fish_color_cwd_root %s\n", strip_hash(scheme->base08));         // Root directory (red)
    fprintf(f, "fish_color_user %s\n", strip_hash(scheme->base0B));             // Username (green)
    fprintf(f, "fish_color_host %s\n", strip_hash(scheme->base0D));             // Hostname (blue)
    fprintf(f, "fish_color_host_remote %s\n", strip_hash(scheme->base09));      // Remote host (orange)
    fprintf(f, "fish_color_cancel %s\n", strip_hash(scheme->base08));           // Cancel (red)
    fprintf(f, "\n");
    
    fprintf(f, "# Completion Pager Colors\n");
    fprintf(f, "fish_pager_color_progress %s\n", strip_hash(scheme->base03));   // Progress indicator
    fprintf(f, "fish_pager_color_background\n");                                 // Use default
    fprintf(f, "fish_pager_color_prefix %s\n", strip_hash(scheme->base0D));     // Prefix (blue)
    fprintf(f, "fish_pager_color_completion %s\n", strip_hash(scheme->base05)); // Completion text
    fprintf(f, "fish_pager_color_description %s\n", strip_hash(scheme->base03));// Description (gray)
    fprintf(f, "fish_pager_color_selected_background --background=%s\n", strip_hash(scheme->base02)); // Selected bg
    fprintf(f, "fish_pager_color_selected_prefix %s\n", strip_hash(scheme->base0A)); // Selected prefix (yellow)
    fprintf(f, "fish_pager_color_selected_completion %s\n", strip_hash(scheme->base05)); // Selected completion
    fprintf(f, "fish_pager_color_selected_description %s\n", strip_hash(scheme->base04)); // Selected description
    fprintf(f, "fish_pager_color_secondary_background\n");                       // Use default
    fprintf(f, "fish_pager_color_secondary_prefix %s\n", strip_hash(scheme->base0D)); // Secondary prefix
    fprintf(f, "fish_pager_color_secondary_completion %s\n", strip_hash(scheme->base04)); // Secondary completion
    fprintf(f, "fish_pager_color_secondary_description %s\n", strip_hash(scheme->base03)); // Secondary description
    
    fclose(f);
    return 0;
}

// Apply fish theme to current fish configuration
int fish_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create fish config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/fish", home);
    mkdir(config_dir, 0755);
    
    // Create themes directory if it doesn't exist
    char themes_dir[1024];
    snprintf(themes_dir, sizeof(themes_dir), "%s/themes", config_dir);
    mkdir(themes_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat.theme", themes_dir);
    
    if (fish_generate_theme(scheme, theme_path) != 0) {
        fprintf(stderr, "Failed to generate fish theme\n");
        return -1;
    }
    
    printf("  ✓ %s\n", theme_path);
    return 0;
}