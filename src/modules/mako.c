#define _DEFAULT_SOURCE
#include "mako.h"
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

static int mako_write_config(const Base16Scheme *scheme, const char *path, const FontConfig *font) {
    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create mako config: %s\n", path);
        return -1;
    }

    fprintf(f, "# coat mako theme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    /* Global settings */
    fprintf(f, "background-color=#%s\n", strip_hash(scheme->base00));
    fprintf(f, "text-color=#%s\n", strip_hash(scheme->base07));
    fprintf(f, "border-color=#%s\n", strip_hash(scheme->base02));
    fprintf(f, "progress-color=over #%s\n", strip_hash(scheme->base0D));

    if (font && font->monospace[0]) {
        fprintf(f, "font=%s %d\n", font->monospace, font->sizes.popups ? font->sizes.popups : 11);
    } else {
        fprintf(f, "font=monospace 11\n");
    }

    fprintf(f, "\n");
    fprintf(f, "width=400\n");
    fprintf(f, "height=200\n");
    fprintf(f, "margin=16\n");
    fprintf(f, "padding=12,14\n");
    fprintf(f, "border-size=1\n");
    fprintf(f, "border-radius=10\n");
    fprintf(f, "icons=1\n");
    fprintf(f, "icon-path=/usr/share/icons/Papirus-Dark\n");
    fprintf(f, "max-icon-size=48\n");
    fprintf(f, "markup=1\n");
    fprintf(f, "actions=1\n");
    fprintf(f, "format=<b>%%s</b>\\n%%b\n");
    fprintf(f, "anchor=top-right\n");
    fprintf(f, "max-visible=5\n");
    fprintf(f, "layer=overlay\n");
    fprintf(f, "sort=-time\n");
    fprintf(f, "\n");
    fprintf(f, "on-button-left=dismiss\n");
    fprintf(f, "on-button-right=dismiss-all\n");
    fprintf(f, "on-button-middle=invoke-default-action\n");
    fprintf(f, "on-touch=dismiss\n");

    /* Urgency overrides */
    fprintf(f, "\n[urgency=low]\n");
    fprintf(f, "border-color=#%s\n", strip_hash(scheme->base03));
    fprintf(f, "text-color=#%s\n", strip_hash(scheme->base04));
    fprintf(f, "default-timeout=4000\n");

    fprintf(f, "\n[urgency=normal]\n");
    fprintf(f, "border-color=#%s\n", strip_hash(scheme->base02));
    fprintf(f, "default-timeout=6000\n");

    fprintf(f, "\n[urgency=high]\n");
    fprintf(f, "border-color=#%s\n", strip_hash(scheme->base08));
    fprintf(f, "text-color=#%s\n", strip_hash(scheme->base08));
    fprintf(f, "default-timeout=0\n");

    fclose(f);
    return 0;
}

int mako_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
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
    snprintf(config_dir, sizeof(config_dir), "%s/.config/mako", home);
    mkdir(config_dir, 0755);

    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/config", config_dir);
    if (mako_write_config(scheme, config_path, font) != 0) return -1;
    printf("  ✓ %s\n", config_path);

    /* Reload mako if running */
    system("makoctl reload 2>/dev/null");

    return 0;
}
