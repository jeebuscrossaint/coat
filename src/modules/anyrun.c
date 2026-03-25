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
    (void)font; /* anyrun handles font via its own config */

    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create anyrun CSS: %s\n", path);
        return -1;
    }

    fprintf(f, "/* coat anyrun theme: %s */\n", scheme->name);
    fprintf(f, "/* %s */\n\n", scheme->author);

    /* Color variables — used throughout the rules below */
    fprintf(f, "@define-color accent #%s;\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color bg-color #%s;\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color fg-color #%s;\n", strip_hash(scheme->base07));
    fprintf(f, "@define-color desc-color #%s;\n\n", strip_hash(scheme->base04));

    /* Full default ruleset — user CSS replaces the default entirely, so we must
     * include all rules here (sourced from /etc/xdg/anyrun/style.css). */
    fprintf(f,
        "window {\n"
        "  background: transparent;\n"
        "}\n\n"
        "box.main {\n"
        "  padding: 5px;\n"
        "  margin: 10px;\n"
        "  border-radius: 10px;\n"
        "  border: 2px solid @accent;\n"
        "  background-color: @bg-color;\n"
        "  box-shadow: 0 0 5px black;\n"
        "}\n\n"
        "text {\n"
        "  min-height: 30px;\n"
        "  padding: 5px;\n"
        "  border-radius: 5px;\n"
        "  color: @fg-color;\n"
        "}\n\n"
        ".matches {\n"
        "  background-color: rgba(0, 0, 0, 0);\n"
        "  border-radius: 10px;\n"
        "}\n\n"
        "box.plugin:first-child {\n"
        "  margin-top: 5px;\n"
        "}\n\n"
        "box.plugin.info {\n"
        "  min-width: 200px;\n"
        "}\n\n"
        "list.plugin {\n"
        "  background-color: rgba(0, 0, 0, 0);\n"
        "}\n\n"
        "label.match {\n"
        "  color: @fg-color;\n"
        "}\n\n"
        "label.match.description {\n"
        "  font-size: 10px;\n"
        "  color: @desc-color;\n"
        "}\n\n"
        "label.plugin.info {\n"
        "  font-size: 14px;\n"
        "  color: @fg-color;\n"
        "}\n\n"
        ".match {\n"
        "  background: transparent;\n"
        "}\n\n"
        ".match:selected {\n"
        "  border-left: 4px solid @accent;\n"
        "  background: transparent;\n"
        "  animation: fade 0.1s linear;\n"
        "}\n\n"
        "@keyframes fade {\n"
        "  0%% { opacity: 0; }\n"
        "  100%% { opacity: 1; }\n"
        "}\n"
    );

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
