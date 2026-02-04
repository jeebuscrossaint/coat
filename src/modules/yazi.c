#include "yazi.h"
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

// Generate a yazi theme file from a Base16 scheme
int yazi_generate_theme(const Base16Scheme *scheme, const char *output_path) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create yazi theme file: %s\n", output_path);
        return -1;
    }
    
    // Write TOML header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Manager section - file listing colors
    fprintf(f, "[manager]\n");
    fprintf(f, "cwd = { fg = \"#%s\" }\n\n", strip_hash(scheme->base0D));
    
    fprintf(f, "# Hovered\n");
    fprintf(f, "hovered = { reversed = true }\n");
    fprintf(f, "preview_hovered = { underline = true }\n\n");
    
    fprintf(f, "# Find\n");
    fprintf(f, "find_keyword = { fg = \"#%s\", italic = true, underline = true }\n", strip_hash(scheme->base0A));
    fprintf(f, "find_position = { fg = \"#%s\", bg = \"reset\", bold = true, italic = true }\n\n", strip_hash(scheme->base0E));
    
    fprintf(f, "# Marker\n");
    fprintf(f, "marker_copied = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base00), strip_hash(scheme->base0B));
    fprintf(f, "marker_cut = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base00), strip_hash(scheme->base08));
    fprintf(f, "marker_marked = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base00), strip_hash(scheme->base0C));
    fprintf(f, "marker_selected = { fg = \"#%s\", bg = \"#%s\" }\n\n", strip_hash(scheme->base00), strip_hash(scheme->base09));
    
    fprintf(f, "# Tab\n");
    fprintf(f, "tab_active = { reversed = true }\n");
    fprintf(f, "tab_inactive = {}\n");
    fprintf(f, "tab_width = 1\n\n");
    
    fprintf(f, "# Count\n");
    fprintf(f, "count_copied = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base00), strip_hash(scheme->base0B));
    fprintf(f, "count_cut = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base00), strip_hash(scheme->base08));
    fprintf(f, "count_selected = { fg = \"#%s\", bg = \"#%s\" }\n\n", strip_hash(scheme->base00), strip_hash(scheme->base09));
    
    fprintf(f, "# Border\n");
    fprintf(f, "border_symbol = \"│\"\n");
    fprintf(f, "border_style = { fg = \"#%s\" }\n\n", strip_hash(scheme->base03));
    
    // Status line section
    fprintf(f, "[status]\n");
    fprintf(f, "separator_open = \"\"\n");
    fprintf(f, "separator_close = \"\"\n");
    fprintf(f, "separator_style = { fg = \"#%s\", bg = \"#%s\" }\n\n", strip_hash(scheme->base02), strip_hash(scheme->base00));
    
    fprintf(f, "# Mode\n");
    fprintf(f, "mode_normal = { bg = \"#%s\", fg = \"#%s\", bold = true }\n", strip_hash(scheme->base0D), strip_hash(scheme->base00));
    fprintf(f, "mode_select = { bg = \"#%s\", fg = \"#%s\", bold = true }\n", strip_hash(scheme->base0B), strip_hash(scheme->base00));
    fprintf(f, "mode_unset = { bg = \"#%s\", fg = \"#%s\", bold = true }\n\n", strip_hash(scheme->base08), strip_hash(scheme->base00));
    
    fprintf(f, "# Progress\n");
    fprintf(f, "progress_label = { fg = \"#%s\", bold = true }\n", strip_hash(scheme->base05));
    fprintf(f, "progress_normal = { fg = \"#%s\", bg = \"#%s\" }\n", strip_hash(scheme->base0D), strip_hash(scheme->base02));
    fprintf(f, "progress_error = { fg = \"#%s\", bg = \"#%s\" }\n\n", strip_hash(scheme->base08), strip_hash(scheme->base02));
    
    fprintf(f, "# Permissions\n");
    fprintf(f, "permissions_t = { fg = \"#%s\" }\n", strip_hash(scheme->base0B));
    fprintf(f, "permissions_r = { fg = \"#%s\" }\n", strip_hash(scheme->base0A));
    fprintf(f, "permissions_w = { fg = \"#%s\" }\n", strip_hash(scheme->base08));
    fprintf(f, "permissions_x = { fg = \"#%s\" }\n", strip_hash(scheme->base0B));
    fprintf(f, "permissions_s = { fg = \"#%s\" }\n\n", strip_hash(scheme->base03));
    
    // Input section
    fprintf(f, "[input]\n");
    fprintf(f, "border = { fg = \"#%s\" }\n", strip_hash(scheme->base0D));
    fprintf(f, "title = {}\n");
    fprintf(f, "value = { fg = \"#%s\" }\n", strip_hash(scheme->base0A));
    fprintf(f, "selected = { reversed = true }\n\n");
    
    // Select section
    fprintf(f, "[select]\n");
    fprintf(f, "border = { fg = \"#%s\" }\n", strip_hash(scheme->base0D));
    fprintf(f, "active = { fg = \"#%s\", bold = true }\n", strip_hash(scheme->base0E));
    fprintf(f, "inactive = {}\n\n");
    
    // Tasks section
    fprintf(f, "[tasks]\n");
    fprintf(f, "border = { fg = \"#%s\" }\n", strip_hash(scheme->base0D));
    fprintf(f, "title = {}\n");
    fprintf(f, "hovered = { fg = \"#%s\", underline = true }\n\n", strip_hash(scheme->base0E));
    
    // Which section (keybindings help)
    fprintf(f, "[which]\n");
    fprintf(f, "cols = 3\n");
    fprintf(f, "mask = { bg = \"#%s\" }\n", strip_hash(scheme->base00));
    fprintf(f, "cand = { fg = \"#%s\" }\n", strip_hash(scheme->base0C));
    fprintf(f, "rest = { fg = \"#%s\" }\n", strip_hash(scheme->base03));
    fprintf(f, "desc = { fg = \"#%s\" }\n", strip_hash(scheme->base05));
    fprintf(f, "separator = \"  \"\n");
    fprintf(f, "separator_style = { fg = \"#%s\" }\n\n", strip_hash(scheme->base03));
    
    // Help section
    fprintf(f, "[help]\n");
    fprintf(f, "on = { fg = \"#%s\" }\n", strip_hash(scheme->base0B));
    fprintf(f, "run = { fg = \"#%s\" }\n", strip_hash(scheme->base0E));
    fprintf(f, "hovered = { reversed = true, bold = true }\n");
    fprintf(f, "footer = { fg = \"#%s\", bg = \"#%s\" }\n\n", strip_hash(scheme->base05), strip_hash(scheme->base00));
    
    // Filetype colors
    fprintf(f, "[filetype]\n");
    fprintf(f, "rules = [\n");
    fprintf(f, "  { mime = \"image/*\", fg = \"#%s\" },\n", strip_hash(scheme->base0C));
    fprintf(f, "  { mime = \"video/*\", fg = \"#%s\" },\n", strip_hash(scheme->base0A));
    fprintf(f, "  { mime = \"audio/*\", fg = \"#%s\" },\n", strip_hash(scheme->base0A));
    fprintf(f, "  { mime = \"application/zip\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/gzip\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/x-tar\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/x-bzip*\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/x-7z-compressed\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/x-rar\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { mime = \"application/xz\", fg = \"#%s\" },\n", strip_hash(scheme->base08));
    fprintf(f, "  { name = \"*/\", fg = \"#%s\" },\n", strip_hash(scheme->base0D));
    fprintf(f, "  { name = \"*\", fg = \"#%s\" },\n", strip_hash(scheme->base05));
    fprintf(f, "]\n\n");
    
    // Icon section
    fprintf(f, "[icon]\n");
    fprintf(f, "prepend_rules = [\n");
    fprintf(f, "  { name = \"*/\", text = \"\" },\n");
    fprintf(f, "]\n\n");
    
    fprintf(f, "append_rules = [\n");
    fprintf(f, "  { name = \"*\", text = \"\" },\n");
    fprintf(f, "]\n");
    
    fclose(f);
    return 0;
}

// Apply yazi theme to current yazi configuration
int yazi_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create yazi config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/yazi", home);
    
    struct stat st = {0};
    if (stat(config_dir, &st) == -1) {
        if (mkdir(config_dir, 0755) != 0) {
            fprintf(stderr, "Failed to create yazi config directory\n");
            return -1;
        }
    }
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/theme.toml", config_dir);
    
    if (yazi_generate_theme(scheme, theme_path) != 0) {
        fprintf(stderr, "Failed to generate yazi theme\n");
        return -1;
    }
    
    printf("Theme applied to: %s\n", theme_path);
    printf("Restart yazi to see the changes.\n");
    
    return 0;
}
