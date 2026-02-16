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
    for (int i = 0; i < 16; ++i) {
        fprintf(f, "  --base%02x: #%s;\n", i, strip_hash(base_colors[i]));
    }
    fprintf(f, "}\n\n");
    
    // Main browser background
    fprintf(f, "/* Main browser chrome */\n");
    fprintf(f, "#navigator-toolbox { background-color: var(--base00) !important; }\n");
    fprintf(f, "#TabsToolbar, #nav-bar, toolbar { background-color: var(--base00) !important; }\n");
    fprintf(f, "window, #main-window, #browser { background-color: var(--base00) !important; }\n\n");
    
    // Tabs
    fprintf(f, "/* Tabs */\n");
    fprintf(f, ".tabbrowser-tab { background-color: var(--base01) !important; color: var(--base04) !important; }\n");
    fprintf(f, ".tabbrowser-tab[selected] { background-color: var(--base02) !important; color: var(--base05) !important; }\n");
    fprintf(f, ".tabbrowser-tab:hover { background-color: var(--base02) !important; }\n\n");
    
    // URL bar
    fprintf(f, "/* URL bar and search */\n");
    fprintf(f, "#urlbar, #searchbar { background-color: var(--base01) !important; color: var(--base05) !important; border: 1px solid var(--base03) !important; }\n");
    fprintf(f, "#urlbar:focus-within, #searchbar:focus-within { border-color: var(--base0d) !important; }\n\n");
    
    // Sidebar
    fprintf(f, "/* Sidebar */\n");
    fprintf(f, "#sidebar-box { background-color: var(--base00) !important; }\n");
    fprintf(f, "#sidebar-header { background-color: var(--base01) !important; color: var(--base05) !important; }\n\n");
    
    // Context menus
    fprintf(f, "/* Menus and popups */\n");
    fprintf(f, "menupopup, popup, panel { background-color: var(--base01) !important; color: var(--base05) !important; border: 1px solid var(--base03) !important; }\n");
    fprintf(f, "menuitem:hover, menu:hover { background-color: var(--base02) !important; }\n\n");
    
    // General text colors
    fprintf(f, "/* General text */\n");
    fprintf(f, "toolbar, toolbarbutton, tab, menuitem { color: var(--base05) !important; }\n\n");
    
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
