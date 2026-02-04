#include "vesktop.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

static void write_css(FILE *f, const Base16Scheme *scheme) {
    fprintf(f, "/**\n");
    fprintf(f, " * @name Coat Theme\n");
    fprintf(f, " * @author coat\n");
    fprintf(f, " * @version 1.0.0\n");
    fprintf(f, " * @description Theme configured via coat - Scheme: %s by %s\n", scheme->name, scheme->author);
    fprintf(f, " */\n\n");
    
    // First define base16 color variables in :root (Stylix approach)
    fprintf(f, ":root {\n");
    fprintf(f, "  --base00: #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "  --base01: #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "  --base02: #%s;\n", strip_hash(scheme->base02));
    fprintf(f, "  --base03: #%s;\n", strip_hash(scheme->base03));
    fprintf(f, "  --base04: #%s;\n", strip_hash(scheme->base04));
    fprintf(f, "  --base05: #%s;\n", strip_hash(scheme->base05));
    fprintf(f, "  --base06: #%s;\n", strip_hash(scheme->base06));
    fprintf(f, "  --base07: #%s;\n", strip_hash(scheme->base07));
    fprintf(f, "  --base08: #%s;\n", strip_hash(scheme->base08));
    fprintf(f, "  --base09: #%s;\n", strip_hash(scheme->base09));
    fprintf(f, "  --base0A: #%s;\n", strip_hash(scheme->base0A));
    fprintf(f, "  --base0B: #%s;\n", strip_hash(scheme->base0B));
    fprintf(f, "  --base0C: #%s;\n", strip_hash(scheme->base0C));
    fprintf(f, "  --base0D: #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "  --base0E: #%s;\n", strip_hash(scheme->base0E));
    fprintf(f, "  --base0F: #%s;\n", strip_hash(scheme->base0F));
    fprintf(f, "}\n\n");
    
    // Target all Discord themes (Stylix approach - comprehensive variable mapping)
    fprintf(f, ".theme-light,\n");
    fprintf(f, ".theme-dark,\n");
    fprintf(f, ".theme-darker,\n");
    fprintf(f, ".theme-midnight,\n");
    fprintf(f, ".visual-refresh {\n");
    
    // Background colors
    fprintf(f, "  --activity-card-background: var(--base01) !important;\n");
    fprintf(f, "  --background-accent: var(--base03) !important;\n");
    fprintf(f, "  --background-floating: var(--base02) !important;\n");
    fprintf(f, "  --background-mentioned-hover: var(--base02) !important;\n");
    fprintf(f, "  --background-mentioned: var(--base01) !important;\n");
    fprintf(f, "  --background-message-highlight: var(--base01) !important;\n");
    fprintf(f, "  --background-message-hover: var(--base00) !important;\n");
    fprintf(f, "  --background-modifier-accent: var(--base02) !important;\n");
    fprintf(f, "  --background-modifier-active: var(--base02) !important;\n");
    fprintf(f, "  --background-modifier-hover: var(--base00) !important;\n");
    fprintf(f, "  --background-modifier-selected: var(--base01) !important;\n");
    fprintf(f, "  --background-primary: var(--base00) !important;\n");
    fprintf(f, "  --background-secondary-alt: var(--base01) !important;\n");
    fprintf(f, "  --background-secondary: var(--base01) !important;\n");
    fprintf(f, "  --background-surface-highest: var(--base02) !important;\n");
    fprintf(f, "  --background-surface-higher: var(--base02) !important;\n");
    fprintf(f, "  --background-surface-high: var(--base02) !important;\n");
    fprintf(f, "  --background-tertiary: var(--base00) !important;\n");
    fprintf(f, "  --background-base-low: var(--base01) !important;\n");
    fprintf(f, "  --background-base-lower: var(--base00) !important;\n");
    fprintf(f, "  --background-base-lowest: var(--base00) !important;\n");
    fprintf(f, "  --background-base-tertiary: var(--base00) !important;\n");
    fprintf(f, "  --background-code: var(--base02) !important;\n");
    fprintf(f, "  --background-mod-subtle: var(--base02) !important;\n");
    fprintf(f, "  --bg-base-secondary: var(--base01) !important;\n");
    fprintf(f, "  --bg-base-tertiary: var(--base00) !important;\n");
    fprintf(f, "  --bg-brand: var(--base03) !important;\n");
    fprintf(f, "  --bg-mod-faint: var(--base01) !important;\n");
    fprintf(f, "  --bg-overlay-2: var(--base01) !important;\n");
    fprintf(f, "  --bg-overlay-3: var(--base01) !important;\n");
    fprintf(f, "  --bg-overlay-color-inverse: var(--base03) !important;\n");
    fprintf(f, "  --bg-surface-raised: var(--base02) !important;\n");
    fprintf(f, "  --bg-surface-overlay: var(--base00) !important;\n");
    fprintf(f, "  --black: var(--base00) !important;\n");
    
    // Brand colors
    fprintf(f, "  --brand-05a: var(--base01) !important;\n");
    fprintf(f, "  --brand-10a: var(--base01) !important;\n");
    fprintf(f, "  --brand-15a: var(--base01) !important;\n");
    fprintf(f, "  --brand-260: var(--base0D) !important;\n");
    fprintf(f, "  --brand-360: var(--base0D) !important;\n");
    fprintf(f, "  --brand-500: var(--base0F) !important;\n");
    fprintf(f, "  --brand-560: var(--base01) !important;\n");
    fprintf(f, "  --brand-experiment: var(--base0D) !important;\n");
    fprintf(f, "  --brand-experiment-hover: var(--base0C) !important;\n");
    fprintf(f, "  --brand-experiment-560: var(--base0D) !important;\n");
    
    // Button colors
    fprintf(f, "  --button-danger-background: var(--base08) !important;\n");
    fprintf(f, "  --button-filled-brand-background: var(--base0D) !important;\n");
    fprintf(f, "  --button-filled-brand-background-hover: var(--base03) !important;\n");
    fprintf(f, "  --button-filled-brand-text: var(--base00) !important;\n");
    fprintf(f, "  --button-filled-brand-text-hover: var(--base05) !important;\n");
    fprintf(f, "  --button-outline-positive-border: var(--base0B) !important;\n");
    fprintf(f, "  --button-outline-danger-background-hover: var(--base08) !important;\n");
    fprintf(f, "  --button-outline-danger-border-hover: var(--base08) !important;\n");
    fprintf(f, "  --button-positive-background: var(--base0B) !important;\n");
    fprintf(f, "  --button-positive-background-hover: var(--base03) !important;\n");
    fprintf(f, "  --button-secondary-background: var(--base02) !important;\n");
    fprintf(f, "  --button-secondary-background-hover: var(--base03) !important;\n");
    
    // Card and channel colors
    fprintf(f, "  --card-primary-bg: var(--base02) !important;\n");
    fprintf(f, "  --channel-icon: var(--base04) !important;\n");
    fprintf(f, "  --channels-default: var(--base04) !important;\n");
    fprintf(f, "  --channel-text-area-placeholder: var(--base03) !important;\n");
    fprintf(f, "  --channeltextarea-background: var(--base01) !important;\n");
    fprintf(f, "  --chat-background-default: var(--base02) !important;\n");
    
    // Checkbox colors  \n");
    fprintf(f, "  --checkbox-background-checked: var(--base0D) !important;\n");
    fprintf(f, "  --checkbox-border-checked: var(--base0D) !important;\n");
    fprintf(f, "  --checkbox-background-default: var(--base02) !important;\n");
    fprintf(f, "  --checkbox-border-default: var(--base03) !important;\n");
    
    // Control colors
    fprintf(f, "  --control-brand-foreground-new: var(--base0D) !important;\n");
    fprintf(f, "  --control-brand-foreground: var(--base04) !important;\n");
    
    // Header colors
    fprintf(f, "  --header-primary: var(--base04) !important;\n");
    fprintf(f, "  --header-secondary: var(--base04) !important;\n");
    fprintf(f, "  --home-background: var(--base00) !important;\n");
    
    // Input colors
    fprintf(f, "  --input-background: var(--base02) !important;\n");
    
    // Interactive colors
    fprintf(f, "  --interactive-active: var(--base05) !important;\n");
    fprintf(f, "  --interactive-hover: var(--base05) !important;\n");
    fprintf(f, "  --interactive-muted: var(--base03) !important;\n");
    fprintf(f, "  --interactive-normal: var(--base05) !important;\n");
    
    // Mention colors
    fprintf(f, "  --mention-background: var(--base03) !important;\n");
    fprintf(f, "  --mention-foreground: var(--base05) !important;\n");
    
    // Menu colors
    fprintf(f, "  --menu-item-danger-active-bg: var(--base08) !important;\n");
    fprintf(f, "  --menu-item-danger-hover-bg: var(--base08) !important;\n");
    fprintf(f, "  --menu-item-default-hover-bg: var(--base03) !important;\n");
    
    // Message colors
    fprintf(f, "  --message-reacted-background: var(--base02) !important;\n");
    fprintf(f, "  --message-reacted-text: var(--base05) !important;\n");
    
    // Modal colors
    fprintf(f, "  --modal-background: var(--base01) !important;\n");
    fprintf(f, "  --modal-footer-background: var(--base00) !important;\n");
    
    // Primary colors (critical for main UI surfaces)
    fprintf(f, "  --primary-130: var(--base05) !important;\n");
    fprintf(f, "  --primary-300: var(--base05) !important;\n");
    fprintf(f, "  --primary-500: var(--base02) !important;\n");
    fprintf(f, "  --primary-600: var(--base00) !important;\n");
    fprintf(f, "  --primary-630: var(--base01) !important;\n");
    fprintf(f, "  --primary-660: var(--base00) !important;\n");
    fprintf(f, "  --primary-800: var(--base00) !important;\n");
    
    // Scrollbar colors
    fprintf(f, "  --scrollbar-auto-thumb: var(--base00) !important;\n");
    fprintf(f, "  --scrollbar-auto-track: transparent;\n");
    fprintf(f, "  --scrollbar-thin-thumb: var(--base00) !important;\n");
    fprintf(f, "  --scrollbar-thin-track: transparent;\n");
    
    // Status colors
    fprintf(f, "  --status-danger-background: var(--base08) !important;\n");
    fprintf(f, "  --status-danger: var(--base08) !important;\n");
    fprintf(f, "  --status-negative: var(--base08) !important;\n");
    fprintf(f, "  --status-positive-background: var(--base0B) !important;\n");
    fprintf(f, "  --status-positive-text: var(--base0B) !important;\n");
    fprintf(f, "  --status-positive: var(--base0B) !important;\n");
    fprintf(f, "  --status-success: var(--base0B) !important;\n");
    fprintf(f, "  --status-warning-background: var(--base03) !important;\n");
    fprintf(f, "  --status-warning: var(--base09) !important;\n");
    
    // Text colors
    fprintf(f, "  --text-brand: var(--base07) !important;\n");
    fprintf(f, "  --text-feedback-positive: var(--base0B) !important;\n");
    fprintf(f, "  --text-feedback-negative: var(--base08) !important;\n");
    fprintf(f, "  --text-feedback-warning: var(--base09) !important;\n");
    fprintf(f, "  --text-feedback-success: var(--base0B) !important;\n");
    fprintf(f, "  --text-link: var(--base04) !important;\n");
    fprintf(f, "  --text-muted: var(--base05) !important;\n");
    fprintf(f, "  --text-negative: var(--base08) !important;\n");
    fprintf(f, "  --text-normal: var(--base05) !important;\n");
    fprintf(f, "  --text-positive: var(--base0B) !important;\n");
    fprintf(f, "  --text-primary: var(--base05) !important;\n");
    fprintf(f, "  --text-secondary: var(--base04) !important;\n");
    fprintf(f, "  --text-tertiary: var(--base03) !important;\n");
    fprintf(f, "  --text-warning: var(--base09) !important;\n");
    
    // Miscellaneous colors
    fprintf(f, "  --border-faint: var(--base02) !important;\n");
    fprintf(f, "  --info-warning-foreground: var(--base0A) !important;\n");
    fprintf(f, "  --textbox-markdown-syntax: var(--base05) !important;\n");
    fprintf(f, "  --theme-base-color: var(--base00) !important;\n");
    fprintf(f, "  --white-100: var(--base05) !important;\n");
    fprintf(f, "  --white-200: var(--base05) !important;\n");
    fprintf(f, "  --white-500: var(--base05) !important;\n");
    fprintf(f, "  --white: var(--base05) !important;\n");
    
    fprintf(f, "}\n");
}

// Apply theme to Vesktop/Vencord
int vesktop_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Theme paths for Vesktop and Vencord
    const char *theme_paths[] = {
        "/.config/vesktop/themes/coat.theme.css",
        "/.config/Vencord/themes/coat.theme.css"
    };
    
    int success_count = 0;
    
    for (size_t i = 0; i < sizeof(theme_paths) / sizeof(theme_paths[0]); i++) {
        char theme_path[1024];
        snprintf(theme_path, sizeof(theme_path), "%s%s", home, theme_paths[i]);
        
        // Create themes directory if it doesn't exist
        char themes_dir[1024];
        snprintf(themes_dir, sizeof(themes_dir), "%s", theme_path);
        char *last_slash = strrchr(themes_dir, '/');
        if (last_slash) {
            *last_slash = '\0';
            
            // Create parent directories recursively
            char *slash = themes_dir;
            while ((slash = strchr(slash + 1, '/')) != NULL) {
                *slash = '\0';
                mkdir(themes_dir, 0755);
                *slash = '/';
            }
            mkdir(themes_dir, 0755);
        }
        
        // Check if base config directory exists (to determine if this Discord client is installed)
        char base_dir[1024];
        if (i == 0) {
            snprintf(base_dir, sizeof(base_dir), "%s/.config/vesktop", home);
        } else {
            snprintf(base_dir, sizeof(base_dir), "%s/.config/Vencord", home);
        }
        
        struct stat st;
        if (stat(base_dir, &st) != 0) {
            continue; // Skip if directory doesn't exist
        }
        
        FILE *f = fopen(theme_path, "w");
        if (!f) {
            continue; // Skip if can't write
        }
        
        write_css(f, scheme);
        fclose(f);
        
        success_count++;
        printf("  ✓ %s\n", theme_path);
    }
    
    if (success_count == 0) {
        fprintf(stderr, "  ✗ Neither Vesktop nor Vencord installation found\n");
        return -1;
    }
    
    return 0;
}
