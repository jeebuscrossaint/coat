#include "lf.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>

static void hex_to_rgb(const char *hex, int *r, int *g, int *b) {
    const char *h = (hex[0] == '#') ? hex + 1 : hex;
    unsigned int rv = 0, gv = 0, bv = 0;
    sscanf(h, "%02x%02x%02x", &rv, &gv, &bv);
    *r = (int)rv; *g = (int)gv; *b = (int)bv;
}

/* Write a foreground truecolor ANSI code: 38;2;R;G;B */
static void write_fg(FILE *f, const char *hex) {
    int r, g, b;
    hex_to_rgb(hex, &r, &g, &b);
    fprintf(f, "38;2;%d;%d;%d", r, g, b);
}

int lf_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    (void)font;
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) return -1;

    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/lf", home);
    mkdir(config_dir, 0755);

    char path[1024];
    snprintf(path, sizeof(path), "%s/colors", config_dir);

    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create lf colors file: %s\n", path);
        return -1;
    }

    fprintf(f, "# coat lf colorscheme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);

    /* ── File types ─────────────────────────────────── */
    fprintf(f, "# File types\n");

    /* Directories — bold blue (base0D) */
    fprintf(f, "di=1;"); write_fg(f, scheme->base0D); fprintf(f, "\n");

    /* Symlinks — cyan (base0C) */
    fprintf(f, "ln="); write_fg(f, scheme->base0C); fprintf(f, "\n");

    /* Broken/missing symlinks — bold red (base08) */
    fprintf(f, "mi=1;"); write_fg(f, scheme->base08); fprintf(f, "\n");

    /* Executables — bold green (base0B) */
    fprintf(f, "ex=1;"); write_fg(f, scheme->base0B); fprintf(f, "\n");

    /* Regular files — default foreground (base05) */
    fprintf(f, "fi="); write_fg(f, scheme->base05); fprintf(f, "\n");

    /* Named pipes — yellow (base0A) */
    fprintf(f, "pi="); write_fg(f, scheme->base0A); fprintf(f, "\n");

    /* Sockets — magenta (base0E) */
    fprintf(f, "so="); write_fg(f, scheme->base0E); fprintf(f, "\n");

    /* Block/char devices — bold orange (base09) */
    fprintf(f, "bd=1;"); write_fg(f, scheme->base09); fprintf(f, "\n");
    fprintf(f, "cd="); write_fg(f, scheme->base09); fprintf(f, "\n");

    /* setuid / setgid */
    fprintf(f, "su=1;"); write_fg(f, scheme->base08); fprintf(f, "\n");
    fprintf(f, "sg=1;"); write_fg(f, scheme->base0A); fprintf(f, "\n");

    /* Sticky + other-writable dirs */
    fprintf(f, "tw=1;"); write_fg(f, scheme->base0B); fprintf(f, "\n");
    fprintf(f, "ow="); write_fg(f, scheme->base0B); fprintf(f, "\n");

    fprintf(f, "\n");

    /* ── Archives ───────────────────────────────────── */
    fprintf(f, "# Archives\n");
    const char *archives[] = {
        "*.tar","*.tgz","*.tbz2","*.txz","*.gz","*.bz2","*.xz","*.zst",
        "*.zip","*.rar","*.7z","*.deb","*.rpm","*.pkg","*.dmg",NULL
    };
    for (int i = 0; archives[i]; i++) {
        fprintf(f, "%s=", archives[i]); write_fg(f, scheme->base08); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Images ─────────────────────────────────────── */
    fprintf(f, "# Images\n");
    const char *images[] = {
        "*.png","*.jpg","*.jpeg","*.gif","*.bmp","*.webp","*.svg",
        "*.ico","*.tiff","*.psd","*.xcf","*.avif","*.heic",NULL
    };
    for (int i = 0; images[i]; i++) {
        fprintf(f, "%s=", images[i]); write_fg(f, scheme->base0C); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Video ──────────────────────────────────────── */
    fprintf(f, "# Video\n");
    const char *video[] = {
        "*.mp4","*.mkv","*.avi","*.mov","*.webm","*.flv","*.wmv",
        "*.m4v","*.mpg","*.mpeg","*.vob",NULL
    };
    for (int i = 0; video[i]; i++) {
        fprintf(f, "%s=", video[i]); write_fg(f, scheme->base0E); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Audio ──────────────────────────────────────── */
    fprintf(f, "# Audio\n");
    const char *audio[] = {
        "*.mp3","*.flac","*.ogg","*.wav","*.aac","*.m4a","*.opus",
        "*.wma","*.aiff",NULL
    };
    for (int i = 0; audio[i]; i++) {
        fprintf(f, "%s=", audio[i]); write_fg(f, scheme->base0D); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Documents ──────────────────────────────────── */
    fprintf(f, "# Documents\n");
    const char *docs[] = {
        "*.pdf","*.doc","*.docx","*.odt","*.xls","*.xlsx","*.ods",
        "*.ppt","*.pptx","*.odp","*.epub",NULL
    };
    for (int i = 0; docs[i]; i++) {
        fprintf(f, "%s=", docs[i]); write_fg(f, scheme->base0A); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Config / markup ────────────────────────────── */
    fprintf(f, "# Config & markup\n");
    const char *config[] = {
        "*.yaml","*.yml","*.toml","*.json","*.json5","*.jsonc",
        "*.ini","*.conf","*.cfg","*.env","*.xml","*.html","*.htm",
        "*.css","*.scss","*.sass",NULL
    };
    for (int i = 0; config[i]; i++) {
        fprintf(f, "%s=", config[i]); write_fg(f, scheme->base09); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Source code ────────────────────────────────── */
    fprintf(f, "# Source code\n");
    const char *code[] = {
        "*.c","*.h","*.cpp","*.cc","*.cxx","*.hpp","*.hxx",
        "*.rs","*.go","*.py","*.rb","*.java","*.kt","*.swift",
        "*.js","*.ts","*.jsx","*.tsx","*.vue","*.lua","*.sh",
        "*.bash","*.zsh","*.fish","*.pl","*.zig","*.nim",NULL
    };
    for (int i = 0; code[i]; i++) {
        fprintf(f, "%s=", code[i]); write_fg(f, scheme->base0B); fprintf(f, "\n");
    }
    fprintf(f, "\n");

    /* ── Markdown / text ────────────────────────────── */
    fprintf(f, "# Text\n");
    const char *text[] = {"*.md","*.rst","*.txt","*.log","*.csv",NULL};
    for (int i = 0; text[i]; i++) {
        fprintf(f, "%s=", text[i]); write_fg(f, scheme->base04); fprintf(f, "\n");
    }

    fclose(f);
    printf("  ✓ %s\n", path);
    printf("    Add to ~/.config/lf/lfrc: set color true\n");
    return 0;
}
