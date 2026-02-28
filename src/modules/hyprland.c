//
// Created by amarnath on 2/27/26.
//

#include "hyprland.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <pwd.h>
#include <ctype.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Apply Base16 colors to Hyprland config
int hyprland_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    // Get home directory
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
    
    // Hyprland config path
    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/.config/hypr/hyprland.conf", home);
    
    char backup_path[1024];
    snprintf(backup_path, sizeof(backup_path), "%s.coat.backup", config_path);
    
    char temp_path[1024];
    snprintf(temp_path, sizeof(temp_path), "%s.coat.tmp", config_path);
    
    // Check if config exists
    FILE *config = fopen(config_path, "r");
    if (!config) {
        fprintf(stderr, "Hyprland config not found at %s - creating new color section\n", config_path);
        
        // Create directory if needed
        char config_dir[1024];
        snprintf(config_dir, sizeof(config_dir), "%s/.config/hypr", home);
        mkdir(config_dir, 0755);
        
        // Create new config with just colors
        config = fopen(config_path, "w");
        if (!config) {
            fprintf(stderr, "Failed to create Hyprland config\n");
            return -1;
        }
        
        fprintf(config, "# coat - Hyprland colors\n");
        fprintf(config, "# Scheme: %s by %s\n\n", scheme->name, scheme->author);
        
        // Write color variables
        fprintf(config, "# Base16 color variables\n");
        fprintf(config, "$base00 = rgb(%s)\n", strip_hash(scheme->base00));
        fprintf(config, "$base01 = rgb(%s)\n", strip_hash(scheme->base01));
        fprintf(config, "$base02 = rgb(%s)\n", strip_hash(scheme->base02));
        fprintf(config, "$base03 = rgb(%s)\n", strip_hash(scheme->base03));
        fprintf(config, "$base04 = rgb(%s)\n", strip_hash(scheme->base04));
        fprintf(config, "$base05 = rgb(%s)\n", strip_hash(scheme->base05));
        fprintf(config, "$base06 = rgb(%s)\n", strip_hash(scheme->base06));
        fprintf(config, "$base07 = rgb(%s)\n", strip_hash(scheme->base07));
        fprintf(config, "$base08 = rgb(%s)\n", strip_hash(scheme->base08));
        fprintf(config, "$base09 = rgb(%s)\n", strip_hash(scheme->base09));
        fprintf(config, "$base0A = rgb(%s)\n", strip_hash(scheme->base0A));
        fprintf(config, "$base0B = rgb(%s)\n", strip_hash(scheme->base0B));
        fprintf(config, "$base0C = rgb(%s)\n", strip_hash(scheme->base0C));
        fprintf(config, "$base0D = rgb(%s)\n", strip_hash(scheme->base0D));
        fprintf(config, "$base0E = rgb(%s)\n", strip_hash(scheme->base0E));
        fprintf(config, "$base0F = rgb(%s)\n", strip_hash(scheme->base0F));
        fprintf(config, "\n");
        
        // Write general colors section
        fprintf(config, "general {\n");
        fprintf(config, "    col.active_border = $base0D $base0C 45deg\n");
        fprintf(config, "    col.inactive_border = $base01\n");
        fprintf(config, "}\n\n");
        
        // Decoration colors
        fprintf(config, "decoration {\n");
        fprintf(config, "    col.shadow = rgba(%saa)\n", strip_hash(scheme->base00));
        fprintf(config, "}\n\n");
        
        // Group colors
        fprintf(config, "group {\n");
        fprintf(config, "    col.border_active = $base0D\n");
        fprintf(config, "    col.border_inactive = $base01\n");
        fprintf(config, "    groupbar {\n");
        fprintf(config, "        col.active = $base0D\n");
        fprintf(config, "        col.inactive = $base03\n");
        fprintf(config, "    }\n");
        fprintf(config, "}\n");
        
        fclose(config);
        printf("  ✓ Created new Hyprland config with colors\n");
        return 0;
    }
    
    // Read existing config
    fseek(config, 0, SEEK_END);
    long config_size = ftell(config);
    fseek(config, 0, SEEK_SET);
    
    char *config_content = malloc(config_size + 1);
    if (!config_content) {
        fclose(config);
        return -1;
    }
    
    fread(config_content, 1, config_size, config);
    config_content[config_size] = '\0';
    fclose(config);
    
    // Create backup
    FILE *backup = fopen(backup_path, "w");
    if (backup) {
        fputs(config_content, backup);
        fclose(backup);
    }
    
    // Open temp file for writing
    FILE *temp = fopen(temp_path, "w");
    if (!temp) {
        free(config_content);
        fprintf(stderr, "Failed to create temporary file\n");
        return -1;
    }
    
    // Write header
    fprintf(temp, "# coat - Hyprland colors\n");
    fprintf(temp, "# Scheme: %s by %s\n", scheme->name, scheme->author);
    fprintf(temp, "# Original config backed up to: %s\n\n", backup_path);
    
    // Write color variables
    fprintf(temp, "# Base16 color variables (managed by coat)\n");
    fprintf(temp, "$base00 = rgb(%s)\n", strip_hash(scheme->base00));
    fprintf(temp, "$base01 = rgb(%s)\n", strip_hash(scheme->base01));
    fprintf(temp, "$base02 = rgb(%s)\n", strip_hash(scheme->base02));
    fprintf(temp, "$base03 = rgb(%s)\n", strip_hash(scheme->base03));
    fprintf(temp, "$base04 = rgb(%s)\n", strip_hash(scheme->base04));
    fprintf(temp, "$base05 = rgb(%s)\n", strip_hash(scheme->base05));
    fprintf(temp, "$base06 = rgb(%s)\n", strip_hash(scheme->base06));
    fprintf(temp, "$base07 = rgb(%s)\n", strip_hash(scheme->base07));
    fprintf(temp, "$base08 = rgb(%s)\n", strip_hash(scheme->base08));
    fprintf(temp, "$base09 = rgb(%s)\n", strip_hash(scheme->base09));
    fprintf(temp, "$base0A = rgb(%s)\n", strip_hash(scheme->base0A));
    fprintf(temp, "$base0B = rgb(%s)\n", strip_hash(scheme->base0B));
    fprintf(temp, "$base0C = rgb(%s)\n", strip_hash(scheme->base0C));
    fprintf(temp, "$base0D = rgb(%s)\n", strip_hash(scheme->base0D));
    fprintf(temp, "$base0E = rgb(%s)\n", strip_hash(scheme->base0E));
    fprintf(temp, "$base0F = rgb(%s)\n", strip_hash(scheme->base0F));
    fprintf(temp, "\n");
    
    // Parse and copy original config, skipping old coat color sections
    char *line = strtok(config_content, "\n");
    int skip_coat_section = 0;
    int skip_base_vars = 0;
    
    while (line != NULL) {
        // Skip coat header comments
        if (strstr(line, "# coat -") || strstr(line, "# Scheme:") || 
            strstr(line, "# Original config backed up")) {
            line = strtok(NULL, "\n");
            continue;
        }
        
        // Detect start of coat color section
        if (strstr(line, "# Base16 color variables (managed by coat)") ||
            strstr(line, "# Base16 color variables")) {
            skip_base_vars = 1;
            line = strtok(NULL, "\n");
            continue;
        }
        
        // Skip base variable definitions
        if (skip_base_vars) {
            // Check if line is a base variable
            char *trimmed = line;
            while (*trimmed && isspace(*trimmed)) trimmed++;
            
            if (trimmed[0] == '$' && strncmp(trimmed + 1, "base", 4) == 0) {
                line = strtok(NULL, "\n");
                continue;
            } else {
                skip_base_vars = 0;
                // Fall through to write this line
            }
        }
        
        // Write line to temp file
        fprintf(temp, "%s\n", line);
        line = strtok(NULL, "\n");
    }
    
    free(config_content);
    fclose(temp);
    
    // Replace original with temp
    if (rename(temp_path, config_path) != 0) {
        fprintf(stderr, "Failed to update Hyprland config\n");
        unlink(temp_path);
        return -1;
    }
    
    printf("  ✓ %s\n", config_path);
    printf("  ✓ Backup saved: %s\n", backup_path);
    printf("\n");
    printf("  To use the colors in your Hyprland config, reference:\n");
    printf("    $base00-%s (background/foreground)\n", "07");
    printf("    $base08 (red), $base09 (orange), $base0A (yellow)\n");
    printf("    $base0B (green), $base0C (cyan), $base0D (blue)\n");
    printf("    $base0E (magenta), $base0F (brown)\n");
    printf("\n");
    printf("  Example usage:\n");
    printf("    general {\n");
    printf("        col.active_border = $base0D $base0C 45deg\n");
    printf("        col.inactive_border = $base01\n");
    printf("    }\n");
    
    return 0;
}
