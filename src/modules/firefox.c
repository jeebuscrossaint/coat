#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#include "firefox.h"
#include "tinted_parser.h"

// Helper: Find default Firefox profile directory
static int find_firefox_profile(char *profile_path, size_t size) {
    const char *home = getenv("HOME");
    if (!home) return -1;
    char profiles_ini[1024];
    snprintf(profiles_ini, sizeof(profiles_ini), "%s/.mozilla/firefox/profiles.ini", home);
    FILE *f = fopen(profiles_ini, "r");
    if (!f) return -1;
    char line[512];
    char path[256] = "";
    int is_default = 0;
    while (fgets(line, sizeof(line), f)) {
        if (strstr(line, "[Profile") != NULL) {
            is_default = 0;
        }
        if (strncmp(line, "Default=1", 9) == 0) {
            is_default = 1;
        }
        if (strncmp(line, "Path=", 5) == 0) {
            strncpy(path, line + 5, sizeof(path) - 1);
            path[strcspn(path, "\r\n")] = 0;
            if (is_default) break;
        }
    }
    fclose(f);
    if (!path[0]) return -1;
    snprintf(profile_path, size, "%s/.mozilla/firefox/%s", home, path);
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
    fprintf(f, ":root {\n");
    const char *base_colors[16] = {
        scheme->base00, scheme->base01, scheme->base02, scheme->base03,
        scheme->base04, scheme->base05, scheme->base06, scheme->base07,
        scheme->base08, scheme->base09, scheme->base0A, scheme->base0B,
        scheme->base0C, scheme->base0D, scheme->base0E, scheme->base0F
    };
    for (int i = 0; i < 16; ++i) {
        fprintf(f, "  --base%02d: #%s;\n", i, base_colors[i]);
    }
    fprintf(f, "}\n");
    // Example: tab bar, toolbar, etc.
    fprintf(f, "#TabsToolbar, #nav-bar { background-color: var(--base00) !important; color: var(--base05) !important; }\n");
    fprintf(f, "#navigator-toolbox { background-color: var(--base00) !important; }\n");
    if (font) {
        fprintf(f, "* { font-family: '%s', monospace !important; font-size: %dpt !important; }\n",
            font->monospace[0] ? font->monospace : "monospace", font->sizes.terminal);
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
            fprintf(f, "  --base%02d: #%s;\n", i, base_colors[i]);
        }
        fprintf(f, "}\n");
        fclose(f);
    }

    printf("  ✓ %s\n", chrome_dir);
    return 0;
}
