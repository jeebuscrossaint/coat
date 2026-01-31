//
// Created by amarnath on 1/19/26.
//

#include "helix.h"
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

// Generate a helix editor theme file from a Base16 scheme
int helix_generate_theme(const Base16Scheme *scheme, const char *output_path) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create helix theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // UI elements (Stylix/base16-helix exact mapping, quoted keys)
    // IMPORTANT: All quoted key-value pairs MUST come before [palette] and [ansi] sections
    fprintf(f, "# UI\n");
    fprintf(f, "\"ui.background\" = { bg = 'base00' }\n");
    fprintf(f, "\"ui.text\" = 'base05'\n");
    fprintf(f, "\"ui.text.focus\" = { fg = 'base05', modifiers = ['bold'] }\n");
    fprintf(f, "\"ui.selection\" = { bg = 'base02' }\n");
    fprintf(f, "\"ui.selection.primary\" = { bg = 'base03' }\n");
    fprintf(f, "\"ui.linenr\" = 'base03'\n");
    fprintf(f, "\"ui.linenr.selected\" = { fg = 'base04', modifiers = ['bold'] }\n");
    fprintf(f, "\"ui.cursor\" = { fg = 'base00', bg = 'base05' }\n");
    fprintf(f, "\"ui.cursor.primary\" = { fg = 'base00', bg = 'base0D' }\n");
    fprintf(f, "\"ui.cursor.match\" = { bg = 'base03', modifiers = ['underlined'] }\n");
    fprintf(f, "\"ui.cursorline\" = { bg = 'base01' }\n");
    fprintf(f, "\"ui.statusline\" = { fg = 'base05', bg = 'base02' }\n");
    fprintf(f, "\"ui.statusline.inactive\" = { fg = 'base03', bg = 'base01' }\n");
    fprintf(f, "\"ui.popup\" = { bg = 'base01' }\n");
    fprintf(f, "\"ui.window\" = { bg = 'base01' }\n");
    fprintf(f, "\"ui.help\" = { fg = 'base05', bg = 'base01' }\n");
    fprintf(f, "\"ui.menu\" = { bg = 'base01' }\n");
    fprintf(f, "\"ui.menu.selected\" = { bg = 'base02' }\n");
    fprintf(f, "\"ui.virtual.whitespace\" = 'base03'\n");
    fprintf(f, "\"ui.virtual.ruler\" = { bg = 'base01' }\n");
    fprintf(f, "\"ui.virtual.indent-guide\" = 'base03'\n");
    fprintf(f, "\n");
    
    // Syntax highlighting
    fprintf(f, "# Syntax\n");
    fprintf(f, "\"comment\" = \"base03\"\n");
    fprintf(f, "\"constant\" = \"base09\"\n");
    fprintf(f, "\"constant.numeric\" = \"base09\"\n");
    fprintf(f, "\"constant.character.escape\" = \"base0C\"\n");
    fprintf(f, "\"string\" = \"base0B\"\n");
    fprintf(f, "\"variable\" = \"base08\"\n");
    fprintf(f, "\"variable.parameter\" = \"base08\"\n");
    fprintf(f, "\"variable.builtin\" = \"base09\"\n");
    fprintf(f, "\"variable.other.member\" = \"base08\"\n");
    fprintf(f, "\"type\" = \"base0A\"\n");
    fprintf(f, "\"type.builtin\" = \"base0A\"\n");
    fprintf(f, "\"constructor\" = \"base0D\"\n");
    fprintf(f, "\"function\" = \"base0D\"\n");
    fprintf(f, "\"function.builtin\" = \"base0D\"\n");
    fprintf(f, "\"function.macro\" = \"base08\"\n");
    fprintf(f, "\"keyword\" = \"base0E\"\n");
    fprintf(f, "\"keyword.control\" = \"base0E\"\n");
    fprintf(f, "\"keyword.directive\" = \"base0E\"\n");
    fprintf(f, "\"label\" = \"base0A\"\n");
    fprintf(f, "\"operator\" = \"base05\"\n");
    fprintf(f, "\"punctuation\" = \"base05\"\n");
    fprintf(f, "\"punctuation.delimiter\" = \"base05\"\n");
    fprintf(f, "\"punctuation.bracket\" = \"base05\"\n");
    fprintf(f, "\"special\" = \"base0C\"\n");
    fprintf(f, "\"namespace\" = \"base0A\"\n");
    fprintf(f, "\"tag\" = \"base08\"\n");
    fprintf(f, "\n");
    
    // Markup
    fprintf(f, "# Markup\n");
    fprintf(f, "\"markup.heading\" = { fg = \"base0D\", modifiers = [\"bold\"] }\n");
    fprintf(f, "\"markup.list\" = \"base08\"\n");
    fprintf(f, "\"markup.bold\" = { modifiers = [\"bold\"] }\n");
    fprintf(f, "\"markup.italic\" = { modifiers = [\"italic\"] }\n");
    fprintf(f, "\"markup.link.url\" = { fg = \"base09\", modifiers = [\"underlined\"] }\n");
    fprintf(f, "\"markup.link.text\" = \"base08\"\n");
    fprintf(f, "\"markup.quote\" = \"base0C\"\n");
    fprintf(f, "\"markup.raw\" = \"base0B\"\n");
    fprintf(f, "\n");
    
    // Diagnostics
    fprintf(f, "# Diagnostics\n");
    fprintf(f, "\"error\" = \"base08\"\n");
    fprintf(f, "\"warning\" = \"base0A\"\n");
    fprintf(f, "\"info\" = \"base0D\"\n");
    fprintf(f, "\"hint\" = \"base0C\"\n");
    fprintf(f, "\"diagnostic\" = { modifiers = [\"underlined\"] }\n");
    fprintf(f, "\"diagnostic.error\" = { underline = { style = \"curl\", color = \"base08\" } }\n");
    fprintf(f, "\"diagnostic.warning\" = { underline = { style = \"curl\", color = \"base0A\" } }\n");
    fprintf(f, "\"diagnostic.info\" = { underline = { style = \"curl\", color = \"base0D\" } }\n");
    fprintf(f, "\"diagnostic.hint\" = { underline = { style = \"curl\", color = \"base0C\" } }\n");
    fprintf(f, "\n");
    
    // Diff
    fprintf(f, "# Diff\n");
    fprintf(f, "\"diff.plus\" = \"base0B\"\n");
    fprintf(f, "\"diff.minus\" = \"base08\"\n");
    fprintf(f, "\"diff.delta\" = \"base0A\"\n");
    fprintf(f, "\n");
    
    // Palette section (all required baseXX colors)
    // IMPORTANT: This MUST come after all quoted key-value pairs
    fprintf(f, "[palette]\n");
    fprintf(f, "base00 = \"#%s\"\n", strip_hash(scheme->base00));
    fprintf(f, "base01 = \"#%s\"\n", strip_hash(scheme->base01));
    fprintf(f, "base02 = \"#%s\"\n", strip_hash(scheme->base02));
    fprintf(f, "base03 = \"#%s\"\n", strip_hash(scheme->base03));
    fprintf(f, "base04 = \"#%s\"\n", strip_hash(scheme->base04));
    fprintf(f, "base05 = \"#%s\"\n", strip_hash(scheme->base05));
    fprintf(f, "base06 = \"#%s\"\n", strip_hash(scheme->base06));
    fprintf(f, "base07 = \"#%s\"\n", strip_hash(scheme->base07));
    fprintf(f, "base08 = \"#%s\"\n", strip_hash(scheme->base08));
    fprintf(f, "base09 = \"#%s\"\n", strip_hash(scheme->base09));
    fprintf(f, "base0A = \"#%s\"\n", strip_hash(scheme->base0A));
    fprintf(f, "base0B = \"#%s\"\n", strip_hash(scheme->base0B));
    fprintf(f, "base0C = \"#%s\"\n", strip_hash(scheme->base0C));
    fprintf(f, "base0D = \"#%s\"\n", strip_hash(scheme->base0D));
    fprintf(f, "base0E = \"#%s\"\n", strip_hash(scheme->base0E));
    fprintf(f, "base0F = \"#%s\"\n", strip_hash(scheme->base0F));
    fprintf(f, "\n");

    // ANSI section for Helix (Stylix/base16-helix exact mapping)
    // IMPORTANT: This MUST come after [palette]
    fprintf(f, "[ansi]\n");
    fprintf(f, "black = 'base00'\n");
    fprintf(f, "red = 'base08'\n");
    fprintf(f, "green = 'base0B'\n");
    fprintf(f, "yellow = 'base0A'\n");
    fprintf(f, "blue = 'base0D'\n");
    fprintf(f, "magenta = 'base0E'\n");
    fprintf(f, "cyan = 'base0C'\n");
    fprintf(f, "white = 'base05'\n");
    fprintf(f, "bright-black = 'base03'\n");
    fprintf(f, "bright-red = 'base09'\n");
    fprintf(f, "bright-green = 'base0B'\n");
    fprintf(f, "bright-yellow = 'base0A'\n");
    fprintf(f, "bright-blue = 'base0D'\n");
    fprintf(f, "bright-magenta = 'base0E'\n");
    fprintf(f, "bright-cyan = 'base0C'\n");
    fprintf(f, "bright-white = 'base07'\n");
    
    fclose(f);
    return 0;
}

// Apply helix theme to current helix configuration
int helix_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create helix config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/helix", home);
    mkdir(config_dir, 0755);
    
    // Create themes directory if it doesn't exist
    char themes_dir[1024];
    snprintf(themes_dir, sizeof(themes_dir), "%s/themes", config_dir);
    mkdir(themes_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat.toml", themes_dir);
    
    printf("Generating helix theme: %s\n", theme_path);
    
    if (helix_generate_theme(scheme, theme_path) != 0) {
        return -1;
    }
    
    printf("Helix theme generated successfully!\n");
    printf("\nTo activate, add to ~/.config/helix/config.toml:\n");
    printf("  theme = \"coat\"\n");
    printf("\nThen restart helix or reload config with :config-reload\n");
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}