#define _DEFAULT_SOURCE
#include "conky.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <pwd.h>

static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

static int conky_write_theme(const Base16Scheme *scheme, const char *path, const FontConfig *font) {
    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create conky theme: %s\n", path);
        return -1;
    }

    const char *mono = (font && font->monospace[0]) ? font->monospace : "monospace";
    int size = (font && font->sizes.desktop) ? font->sizes.desktop * 2 : 22;

    fprintf(f, "-- coat conky theme: %s\n", scheme->name);
    fprintf(f, "-- %s\n\n", scheme->author);

    fprintf(f, "-- Apply via: dofile(os.getenv('HOME') .. '/.config/conky/coat-theme.lua')\n");
    fprintf(f, "-- Then reference coat_font, coat_fg, coat_accent, coat_dim, coat_green, coat_red\n\n");

    fprintf(f, "coat_font    = '%s:pixelsize=%d'\n", mono, size);
    fprintf(f, "coat_fg      = '%s'\n", strip_hash(scheme->base05));
    fprintf(f, "coat_accent  = '%s'\n", strip_hash(scheme->base0D));
    fprintf(f, "coat_dim     = '%s'\n", strip_hash(scheme->base03));
    fprintf(f, "coat_green   = '%s'\n", strip_hash(scheme->base0B));
    fprintf(f, "coat_red     = '%s'\n", strip_hash(scheme->base08));

    fclose(f);
    return 0;
}

int conky_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) {
        struct passwd *pw = getpwuid(getuid());
        if (pw) home = pw->pw_dir;
        else {
            fprintf(stderr, "Could not determine home directory\n");
            return -1;
        }
    }

    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/conky", home);
    mkdir(config_dir, 0755);

    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/coat-theme.lua", config_dir);
    if (conky_write_theme(scheme, config_path, font) != 0) return -1;
    printf("  ✓ %s\n", config_path);

    /* Reload conky if running */
    system("pkill -SIGUSR1 conky 2>/dev/null; true");

    return 0;
}
