#define _DEFAULT_SOURCE
#include "anyrun.h"
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

static int anyrun_write_css(const Base16Scheme *scheme, const char *path, const FontConfig *font) {
    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create anyrun CSS: %s\n", path);
        return -1;
    }

    fprintf(f, "/* coat anyrun theme: %s */\n", scheme->name);
    fprintf(f, "/* %s */\n\n", scheme->author);

    /* Colors only — let anyrun's defaults handle layout/sizing */

    fprintf(f, "box#main {\n");
    fprintf(f, "    background: #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "    border-color: #%s;\n", strip_hash(scheme->base02));
    fprintf(f, "}\n\n");

    fprintf(f, "entry {\n");
    if (font && font->monospace[0]) {
        fprintf(f, "    font-family: \"%s\";\n", font->monospace);
    }
    fprintf(f, "    color: #%s;\n", strip_hash(scheme->base07));
    fprintf(f, "    background: #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "    caret-color: #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "}\n\n");

    fprintf(f, "row {\n");
    fprintf(f, "    color: #%s;\n", strip_hash(scheme->base07));
    fprintf(f, "}\n\n");

    fprintf(f, "row:selected {\n");
    fprintf(f, "    background: #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "    color: #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "}\n\n");

    fprintf(f, "row:hover:not(:selected) {\n");
    fprintf(f, "    background: #%s;\n", strip_hash(scheme->base01));
    fprintf(f, "}\n\n");

    fprintf(f, "label#plugin {\n");
    fprintf(f, "    color: #%s;\n", strip_hash(scheme->base03));
    fprintf(f, "}\n\n");

    fprintf(f, "label#match {\n");
    fprintf(f, "    color: #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "}\n");

    fclose(f);
    return 0;
}

int anyrun_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
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
    snprintf(config_dir, sizeof(config_dir), "%s/.config/anyrun", home);
    mkdir(config_dir, 0755);

    char css_path[1024];
    snprintf(css_path, sizeof(css_path), "%s/style.css", config_dir);
    if (anyrun_write_css(scheme, css_path, font) != 0) return -1;
    printf("  ✓ %s\n", css_path);

    return 0;
}
