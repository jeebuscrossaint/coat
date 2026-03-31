#include "fuzzel.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
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

    char path[1024];
    snprintf(path, sizeof(path), "%s/fuzzel.ini", config_dir);

    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create fuzzel config: %s\n", path);
        return -1;
    }

    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    fprintf(f, "[main]\n");
    if (font && font->monospace[0])
        fprintf(f, "font=%s:size=%d\n", font->monospace, font->sizes.terminal);
    fprintf(f, "terminal=footclient\n");
    fprintf(f, "layer=overlay\n");
    fprintf(f, "width=35\n");
    fprintf(f, "lines=10\n");
    fprintf(f, "border-width=1\n");
    fprintf(f, "border-radius=0\n");
    fprintf(f, "prompt=> \n");
    fprintf(f, "icons-enabled=no\n");
    fprintf(f, "\n");

    // fuzzel colors use rrggbbaa format
    fprintf(f, "[colors]\n");
    fprintf(f, "background=%sff\n",       strip_hash(scheme->base00));
    fprintf(f, "text=%sff\n",             strip_hash(scheme->base05));
    fprintf(f, "match=%sff\n",            strip_hash(scheme->base0D));
    fprintf(f, "selection=%sff\n",        strip_hash(scheme->base02));
    fprintf(f, "selection-text=%sff\n",   strip_hash(scheme->base05));
    fprintf(f, "selection-match=%sff\n",  strip_hash(scheme->base0D));
    fprintf(f, "counter=%sff\n",          strip_hash(scheme->base03));
    fprintf(f, "border=%sff\n",           strip_hash(scheme->base02));

    fclose(f);
    printf("  ✓ %s\n", path);
    return 0;
}
