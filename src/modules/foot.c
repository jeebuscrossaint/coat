#include "foot.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

int foot_apply_theme(const Base16Scheme *scheme, const FontConfig *font, const OpacityConfig *opacity) {
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) return -1;

    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/foot", home);
    mkdir(config_dir, 0755);

    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.ini", config_dir);

    printf("Generating foot theme: %s\n", theme_path);

    FILE *f = fopen(theme_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create foot theme file: %s\n", theme_path);
        return -1;
    }

    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    // Font
    if (font && font->monospace[0]) {
        fprintf(f, "[main]\n");
        fprintf(f, "font=%s:size=%d\n\n", font->monospace, font->sizes.terminal);
    }

    // Colors — foot uses bare hex, no # prefix
    // Use [colors-dark] as [colors] is deprecated in foot 1.19+
    fprintf(f, "[colors-dark]\n");

    if (opacity) {
        fprintf(f, "alpha=%.2f\n", opacity->terminal);
    }

    // cursor requires 8-char rrggbbaa format
    fprintf(f, "foreground=%s\n",           strip_hash(scheme->base05));
    fprintf(f, "background=%s\n",           strip_hash(scheme->base00));
    fprintf(f, "cursor=%s %s\n",            strip_hash(scheme->base00), strip_hash(scheme->base05));
    fprintf(f, "selection-foreground=%s\n", strip_hash(scheme->base00));
    fprintf(f, "selection-background=%s\n", strip_hash(scheme->base05));
    fprintf(f, "\n");

    fprintf(f, "regular0=%s\n", strip_hash(scheme->base00));
    fprintf(f, "regular1=%s\n", strip_hash(scheme->base08));
    fprintf(f, "regular2=%s\n", strip_hash(scheme->base0B));
    fprintf(f, "regular3=%s\n", strip_hash(scheme->base0A));
    fprintf(f, "regular4=%s\n", strip_hash(scheme->base0D));
    fprintf(f, "regular5=%s\n", strip_hash(scheme->base0E));
    fprintf(f, "regular6=%s\n", strip_hash(scheme->base0C));
    fprintf(f, "regular7=%s\n", strip_hash(scheme->base05));
    fprintf(f, "\n");

    fprintf(f, "bright0=%s\n", strip_hash(scheme->base03));
    fprintf(f, "bright1=%s\n", strip_hash(scheme->base08));
    fprintf(f, "bright2=%s\n", strip_hash(scheme->base0B));
    fprintf(f, "bright3=%s\n", strip_hash(scheme->base0A));
    fprintf(f, "bright4=%s\n", strip_hash(scheme->base0D));
    fprintf(f, "bright5=%s\n", strip_hash(scheme->base0E));
    fprintf(f, "bright6=%s\n", strip_hash(scheme->base0C));
    fprintf(f, "bright7=%s\n", strip_hash(scheme->base07));

    fclose(f);
    printf("  ✓ %s\n", theme_path);
    return 0;
}
