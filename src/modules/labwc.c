#define _DEFAULT_SOURCE
#include "labwc.h"
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

static int labwc_write_themerc(const Base16Scheme *scheme, const char *path, const FontConfig *font) {
    (void)font;

    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create labwc themerc: %s\n", path);
        return -1;
    }

    fprintf(f, "# coat labwc theme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    fprintf(f, "border.width: 2\n\n");

    /* Active window — base01 bg, base07 text, base0D accent border */
    fprintf(f, "# Active window\n");
    fprintf(f, "window.active.title.bg.color: #%s\n",   strip_hash(scheme->base01));
    fprintf(f, "window.active.label.text.color: #%s\n", strip_hash(scheme->base07));
    fprintf(f, "window.active.button.unpressed.image.color: #%s\n", strip_hash(scheme->base07));
    fprintf(f, "window.active.button.hover.image.color: #%s\n",     strip_hash(scheme->base0D));
    fprintf(f, "window.active.border.color: #%s\n\n",               strip_hash(scheme->base0D));

    /* Inactive window — base00 bg, base03 text, base02 border */
    fprintf(f, "# Inactive window\n");
    fprintf(f, "window.inactive.title.bg.color: #%s\n",   strip_hash(scheme->base00));
    fprintf(f, "window.inactive.label.text.color: #%s\n", strip_hash(scheme->base03));
    fprintf(f, "window.inactive.button.unpressed.image.color: #%s\n", strip_hash(scheme->base03));
    fprintf(f, "window.inactive.button.hover.image.color: #%s\n",     strip_hash(scheme->base03));
    fprintf(f, "window.inactive.border.color: #%s\n\n",               strip_hash(scheme->base02));

    fprintf(f, "# Padding\n");
    fprintf(f, "padding.height: 2\n");
    fprintf(f, "padding.width: 4\n\n");

    /* Menu — base01 bg, base05 text, base0D active */
    fprintf(f, "# Menu\n");
    fprintf(f, "menu.items.bg.color: #%s\n",        strip_hash(scheme->base01));
    fprintf(f, "menu.items.text.color: #%s\n",      strip_hash(scheme->base05));
    fprintf(f, "menu.items.active.bg.color: #%s\n", strip_hash(scheme->base0D));
    fprintf(f, "menu.items.active.text.color: #%s\n", strip_hash(scheme->base00));
    fprintf(f, "menu.border.width: 1\n");
    fprintf(f, "menu.border.color: #%s\n\n",        strip_hash(scheme->base02));

    /* OSD */
    fprintf(f, "# OSD\n");
    fprintf(f, "osd.bg.color: #%s\n",          strip_hash(scheme->base01));
    fprintf(f, "osd.border.color: #%s\n",      strip_hash(scheme->base0D));
    fprintf(f, "osd.border.width: 1\n");
    fprintf(f, "osd.label.text.color: #%s\n",  strip_hash(scheme->base05));

    fclose(f);
    return 0;
}

int labwc_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
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
    snprintf(config_dir, sizeof(config_dir), "%s/.config/labwc", home);
    mkdir(config_dir, 0755);

    char themerc_path[1024];
    snprintf(themerc_path, sizeof(themerc_path), "%s/themerc", config_dir);
    if (labwc_write_themerc(scheme, themerc_path, font) != 0) return -1;
    printf("  ✓ %s\n", themerc_path);

    /* Reload labwc if running */
    system("labwc --reconfigure 2>/dev/null; true");

    return 0;
}
