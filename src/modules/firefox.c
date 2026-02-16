#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#include "firefox.h"
#include "tinted_parser.h"

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Helper: Find default Firefox profile directory
static int find_firefox_profile(char *profile_path, size_t size) {
    const char *home = getenv("HOME");
    if (!home) return -1;
    
    // Try multiple possible Firefox profile locations
    const char *firefox_dirs[] = {
        "%s/.config/mozilla/firefox",
        "%s/.mozilla/firefox",
        "%s/snap/firefox/common/.mozilla/firefox",
        "%s/.var/app/org.mozilla.firefox/.mozilla/firefox"
    };
    
    char profiles_ini[1024];
    char firefox_base[1024];
    FILE *f = NULL;
    
    // Find which location exists
    for (size_t i = 0; i < sizeof(firefox_dirs) / sizeof(firefox_dirs[0]); i++) {
        snprintf(firefox_base, sizeof(firefox_base), firefox_dirs[i], home);
        snprintf(profiles_ini, sizeof(profiles_ini), "%s/profiles.ini", firefox_base);
        f = fopen(profiles_ini, "r");
        if (f) break;
    }
    
    if (!f) return -1;
    
    char line[512];
    char path[256] = "";
    int is_default = 0;
    
    while (fgets(line, sizeof(line), f)) {
        // New profile section resets default flag
        if (strstr(line, "[Profile") != NULL || strstr(line, "[Install") != NULL) {
            is_default = 0;
        }
        // Check if this profile is marked as default
        if (strncmp(line, "Default=1", 9) == 0) {
            is_default = 1;
        }
        // Extract path
        if (strncmp(line, "Path=", 5) == 0) {
            strncpy(path, line + 5, sizeof(path) - 1);
            path[strcspn(path, "\r\n")] = 0;
            if (is_default) break;
        }
    }
    fclose(f);
    
    if (!path[0]) return -1;
    snprintf(profile_path, size, "%s/%s", firefox_base, path);
    return 0;
}

// Write userChrome.css and userContent.css for Firefox theming
int firefox_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    char profile_dir[1024];
    if (find_firefox_profile(profile_dir, sizeof(profile_dir)) != 0) {
        fprintf(stderr, "Could not find Firefox profile directory.\n");
        return -1;
    }
    char chrome_dir[1060];
    snprintf(chrome_dir, sizeof(chrome_dir), "%s/chrome", profile_dir);
    mkdir(chrome_dir, 0700);

    // userChrome.css: UI theming
    char userchrome[1100];
    snprintf(userchrome, sizeof(userchrome), "%s/userChrome.css", chrome_dir);
    FILE *f = fopen(userchrome, "w");
    if (!f) {
        fprintf(stderr, "Failed to write %s\n", userchrome);
        return -1;
    }
    fprintf(f, "/* coat Firefox UI theme */\n");
    fprintf(f, "/* To enable: Set toolkit.legacyUserProfileCustomizations.stylesheets = true in about:config */\n\n");
    fprintf(f, ":root {\n");
    const char *base_colors[16] = {
        scheme->base00, scheme->base01, scheme->base02, scheme->base03,
        scheme->base04, scheme->base05, scheme->base06, scheme->base07,
        scheme->base08, scheme->base09, scheme->base0A, scheme->base0B,
        scheme->base0C, scheme->base0D, scheme->base0E, scheme->base0F
    };
    
    // Write Base16 colors for reference
    for (int i = 0; i < 16; ++i) {
        fprintf(f, "  --base%02x: #%s;\n", i, strip_hash(base_colors[i]));
    }
    fprintf(f, "\n");
    
    // Override Firefox's built-in color variables
    fprintf(f, "  /* Toolbar colors */\n");
    fprintf(f, "  --toolbar-bgcolor: var(--base00) !important;\n");
    fprintf(f, "  --toolbar-color: var(--base05) !important;\n");
    fprintf(f, "  --toolbar-field-background-color: var(--base01) !important;\n");
    fprintf(f, "  --toolbar-field-color: var(--base05) !important;\n");
    fprintf(f, "  --toolbar-field-border-color: var(--base03) !important;\n");
    fprintf(f, "  --toolbar-field-focus-background-color: var(--base01) !important;\n");
    fprintf(f, "  --toolbar-field-focus-color: var(--base05) !important;\n");
    fprintf(f, "  --toolbar-field-focus-border-color: var(--base0d) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Tab colors */\n");
    fprintf(f, "  --tab-selected-bgcolor: var(--base02) !important;\n");
    fprintf(f, "  --tab-selected-textcolor: var(--base05) !important;\n");
    fprintf(f, "  --tab-hover-bgcolor: var(--base02) !important;\n");
    fprintf(f, "  --tab-hover-textcolor: var(--base05) !important;\n");
    fprintf(f, "  --lwt-tab-text: var(--base04) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Sidebar colors */\n");
    fprintf(f, "  --sidebar-background-color: var(--base00) !important;\n");
    fprintf(f, "  --sidebar-text-color: var(--base05) !important;\n");
    fprintf(f, "  --sidebar-border-color: var(--base02) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Popup/Arrowpanel colors */\n");
    fprintf(f, "  --arrowpanel-background: var(--base01) !important;\n");
    fprintf(f, "  --arrowpanel-color: var(--base05) !important;\n");
    fprintf(f, "  --arrowpanel-border-color: var(--base03) !important;\n");
    fprintf(f, "  --panel-background: var(--base01) !important;\n");
    fprintf(f, "  --panel-color: var(--base05) !important;\n");
    fprintf(f, "  --panel-border-color: var(--base03) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Button colors */\n");
    fprintf(f, "  --button-bgcolor: var(--base02) !important;\n");
    fprintf(f, "  --button-hover-bgcolor: var(--base03) !important;\n");
    fprintf(f, "  --button-active-bgcolor: var(--base03) !important;\n");
    fprintf(f, "  --button-color: var(--base05) !important;\n");
    fprintf(f, "  --toolbarbutton-hover-background: var(--base02) !important;\n");
    fprintf(f, "  --toolbarbutton-active-background: var(--base03) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Chrome background colors */\n");
    fprintf(f, "  --chrome-content-separator-color: var(--base02) !important;\n");
    fprintf(f, "  --lwt-accent-color: var(--base00) !important;\n");
    fprintf(f, "  --lwt-text-color: var(--base05) !important;\n");
    fprintf(f, "\n");
    
    fprintf(f, "  /* Autocomplete/URL dropdown */\n");
    fprintf(f, "  --autocomplete-popup-background: var(--base01) !important;\n");
    fprintf(f, "  --autocomplete-popup-color: var(--base05) !important;\n");
    fprintf(f, "  --autocomplete-popup-highlight-background: var(--base0d) !important;\n");
    fprintf(f, "  --autocomplete-popup-highlight-color: var(--base00) !important;\n");
    fprintf(f, "}\n\n");
    
    // Font configuration
    if (font && font->monospace[0]) {
        fprintf(f, "/* Font */\n");
        fprintf(f, "* { font-family: '%s', monospace !important; font-size: %dpt !important; }\n",
            font->monospace, font->sizes.terminal);
    }
    fclose(f);

    // userContent.css: web content theming (optional, minimal)
    char usercontent[1100];
    snprintf(usercontent, sizeof(usercontent), "%s/userContent.css", chrome_dir);
    f = fopen(usercontent, "w");
    if (f) {
        fprintf(f, "/* coat Firefox web content theme (minimal) */\n");
        fprintf(f, ":root {\n");
        for (int i = 0; i < 16; ++i) {
            fprintf(f, "  --base%02x: #%s;\n", i, strip_hash(base_colors[i]));
        }
        fprintf(f, "}\n");
        fclose(f);
    }

    printf("  ✓ %s\n", chrome_dir);
    printf("  ⓘ Enable userChrome.css in Firefox: Set toolkit.legacyUserProfileCustomizations.stylesheets = true in about:config\n");
    return 0;
}
