#include "fuzzel.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

int fuzzel_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) return -1;

    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/fuzzel", home);
    mkdir(config_dir, 0755);

    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.ini", config_dir);

    FILE *f = fopen(theme_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create fuzzel theme file: %s\n", theme_path);
        return -1;
    }

    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    // fuzzel colors use rrggbbaa format
    fprintf(f, "[colors]\n");
    fprintf(f, "background=%sff\n",        strip_hash(scheme->base00));
    fprintf(f, "text=%sff\n",              strip_hash(scheme->base05));
    fprintf(f, "match=%sff\n",             strip_hash(scheme->base0D));
    fprintf(f, "selection=%sff\n",         strip_hash(scheme->base02));
    fprintf(f, "selection-text=%sff\n",    strip_hash(scheme->base05));
    fprintf(f, "selection-match=%sff\n",   strip_hash(scheme->base0D));
    fprintf(f, "counter=%sff\n",           strip_hash(scheme->base03));
    fprintf(f, "border=%sff\n",            strip_hash(scheme->base02));

    fclose(f);
    printf("  ✓ %s\n", theme_path);
    return 0;
}
