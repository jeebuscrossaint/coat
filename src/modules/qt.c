#define _DEFAULT_SOURCE
#include "qt.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char *strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

/*
 * Qt5ct / Qt6ct color scheme format (.conf):
 *
 *   [ColorScheme]
 *   active_colors=   <18 comma-separated #rrggbb values>
 *   disabled_colors= <18 comma-separated #rrggbb values>
 *   inactive_colors= <18 comma-separated #rrggbb values>
 *
 * QPalette role order (18 values):
 *   0  WindowText       text on windows/widgets
 *   1  Button           button background
 *   2  Light            lighter than Button
 *   3  Midlight         between Button and Light
 *   4  Dark             darker than Button
 *   5  Mid              between Button and Dark
 *   6  Text             text in views/line edits
 *   7  BrightText       high-contrast text (e.g. on dark buttons)
 *   8  ButtonText       text on buttons
 *   9  Base             background of text inputs / list views
 *   10 Window           general window/dialog background
 *   11 Shadow           shadow color
 *   12 Highlight        selection background
 *   13 HighlightedText  text ON a selection — must contrast with Highlight
 *   14 Link             hyperlink color
 *   15 LinkVisited      visited hyperlink color
 *   16 AlternateBase    alternating row background
 *   17 ToolTipBase      tooltip background
 *   18 ToolTipText      tooltip text
 *   (PlaceholderText in newer Qt — appended as 19th value)
 */
static void write_colorscheme(FILE *f, const Base16Scheme *scheme) {
    /* Convenience macro: #color */
#define C(x) strip_hash(scheme->x)

    fprintf(f, "[ColorScheme]\n");

    /* ── Active (normal focused state) ───────────────────────── */
    fprintf(f, "active_colors=");
    fprintf(f, "#%s, ", C(base05));   /*  0 WindowText        */
    fprintf(f, "#%s, ", C(base02));   /*  1 Button            */
    fprintf(f, "#%s, ", C(base03));   /*  2 Light             */
    fprintf(f, "#%s, ", C(base02));   /*  3 Midlight          */
    fprintf(f, "#%s, ", C(base01));   /*  4 Dark              */
    fprintf(f, "#%s, ", C(base02));   /*  5 Mid               */
    fprintf(f, "#%s, ", C(base05));   /*  6 Text              */
    fprintf(f, "#%s, ", C(base07));   /*  7 BrightText        */
    fprintf(f, "#%s, ", C(base05));   /*  8 ButtonText        */
    fprintf(f, "#%s, ", C(base00));   /*  9 Base              */
    fprintf(f, "#%s, ", C(base01));   /* 10 Window            */
    fprintf(f, "#%s, ", C(base00));   /* 11 Shadow            */
    fprintf(f, "#%s, ", C(base0D));   /* 12 Highlight         */
    fprintf(f, "#%s, ", C(base07));   /* 13 HighlightedText   — light text on accent bg */
    fprintf(f, "#%s, ", C(base0D));   /* 14 Link              */
    fprintf(f, "#%s, ", C(base0E));   /* 15 LinkVisited       */
    fprintf(f, "#%s, ", C(base01));   /* 16 AlternateBase     */
    fprintf(f, "#%s, ", C(base01));   /* 17 ToolTipBase       */
    fprintf(f, "#%s, ", C(base05));   /* 18 ToolTipText       */
    fprintf(f, "#%s\n", C(base04));   /* 19 PlaceholderText   */

    /* ── Disabled (grayed-out state) ────────────────────────── */
    fprintf(f, "disabled_colors=");
    fprintf(f, "#%s, ", C(base03));   /*  0 WindowText        */
    fprintf(f, "#%s, ", C(base01));   /*  1 Button            */
    fprintf(f, "#%s, ", C(base02));   /*  2 Light             */
    fprintf(f, "#%s, ", C(base02));   /*  3 Midlight          */
    fprintf(f, "#%s, ", C(base01));   /*  4 Dark              */
    fprintf(f, "#%s, ", C(base01));   /*  5 Mid               */
    fprintf(f, "#%s, ", C(base03));   /*  6 Text              */
    fprintf(f, "#%s, ", C(base04));   /*  7 BrightText        */
    fprintf(f, "#%s, ", C(base03));   /*  8 ButtonText        */
    fprintf(f, "#%s, ", C(base00));   /*  9 Base              */
    fprintf(f, "#%s, ", C(base00));   /* 10 Window            */
    fprintf(f, "#%s, ", C(base00));   /* 11 Shadow            */
    fprintf(f, "#%s, ", C(base02));   /* 12 Highlight         */
    fprintf(f, "#%s, ", C(base03));   /* 13 HighlightedText   */
    fprintf(f, "#%s, ", C(base03));   /* 14 Link              */
    fprintf(f, "#%s, ", C(base03));   /* 15 LinkVisited       */
    fprintf(f, "#%s, ", C(base00));   /* 16 AlternateBase     */
    fprintf(f, "#%s, ", C(base00));   /* 17 ToolTipBase       */
    fprintf(f, "#%s, ", C(base03));   /* 18 ToolTipText       */
    fprintf(f, "#%s\n", C(base03));   /* 19 PlaceholderText   */

    /* ── Inactive (unfocused windows) ───────────────────────── */
    fprintf(f, "inactive_colors=");
    fprintf(f, "#%s, ", C(base04));   /*  0 WindowText        */
    fprintf(f, "#%s, ", C(base01));   /*  1 Button            */
    fprintf(f, "#%s, ", C(base02));   /*  2 Light             */
    fprintf(f, "#%s, ", C(base02));   /*  3 Midlight          */
    fprintf(f, "#%s, ", C(base01));   /*  4 Dark              */
    fprintf(f, "#%s, ", C(base01));   /*  5 Mid               */
    fprintf(f, "#%s, ", C(base04));   /*  6 Text              */
    fprintf(f, "#%s, ", C(base05));   /*  7 BrightText        */
    fprintf(f, "#%s, ", C(base04));   /*  8 ButtonText        */
    fprintf(f, "#%s, ", C(base00));   /*  9 Base              */
    fprintf(f, "#%s, ", C(base01));   /* 10 Window            */
    fprintf(f, "#%s, ", C(base00));   /* 11 Shadow            */
    fprintf(f, "#%s, ", C(base03));   /* 12 Highlight (dim)   */
    fprintf(f, "#%s, ", C(base06));   /* 13 HighlightedText   */
    fprintf(f, "#%s, ", C(base0D));   /* 14 Link              */
    fprintf(f, "#%s, ", C(base0E));   /* 15 LinkVisited       */
    fprintf(f, "#%s, ", C(base00));   /* 16 AlternateBase     */
    fprintf(f, "#%s, ", C(base01));   /* 17 ToolTipBase       */
    fprintf(f, "#%s, ", C(base04));   /* 18 ToolTipText       */
    fprintf(f, "#%s\n", C(base03));   /* 19 PlaceholderText   */

#undef C
}

/*
 * Write a minimal but correct qt5ct.conf / qt6ct.conf.
 * We only set [Appearance] and [Fonts] so we don't clobber
 * any user settings in other sections.
 *
 * Key fixes over the old implementation:
 *   - custom_palette=true  (without this qt5ct ignores the color file)
 *   - style=Fusion         (only Fusion reliably honours custom palettes)
 *   - Font format: "Family,pointSize,-1,5,50,0,0,0,0,0" — plain ini string,
 *     not the broken @Variant binary encoding
 */
static void write_qt_conf(FILE *f,
                          const char *colors_path,
                          const FontConfig *font) {
    fprintf(f, "[Appearance]\n");
    fprintf(f, "color_scheme_path=%s\n", colors_path);
    fprintf(f, "custom_palette=true\n");
    fprintf(f, "standard_dialogs=default\n");
    fprintf(f, "style=Fusion\n");
    fprintf(f, "\n");

    if (font && (font->monospace[0] || font->sansserif[0])) {
        fprintf(f, "[Fonts]\n");
        if (font->monospace[0]) {
            fprintf(f, "fixed=\"%s,%d,-1,5,50,0,0,0,1,0\"\n",
                    font->monospace, font->sizes.terminal ? font->sizes.terminal : 12);
        }
        if (font->sansserif[0]) {
            fprintf(f, "general=\"%s,%d,-1,5,50,0,0,0,0,0\"\n",
                    font->sansserif, font->sizes.desktop ? font->sizes.desktop : 10);
        }
        fprintf(f, "\n");
    }
}

static int apply_for_tool(const char *tool_dir,
                          const char *conf_name,
                          const Base16Scheme *scheme,
                          const FontConfig *font) {
    /* Only proceed if the tool's config directory already exists —
       no point creating qt5ct config if qt5ct isn't installed. */
    struct stat st;
    if (stat(tool_dir, &st) != 0 || !S_ISDIR(st.st_mode)) return 1; /* skip */

    /* Create colors/ subdir */
    char colors_dir[1024];
    snprintf(colors_dir, sizeof(colors_dir), "%s/colors", tool_dir);
    mkdir(colors_dir, 0755);

    /* Write color scheme */
    char scheme_path[1024];
    snprintf(scheme_path, sizeof(scheme_path), "%s/coat.conf", colors_dir);
    FILE *f = fopen(scheme_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write %s\n", scheme_path);
        return -1;
    }
    fprintf(f, "# coat Qt color scheme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);
    write_colorscheme(f, scheme);
    fclose(f);

    /* Write main conf */
    char conf_path[1024];
    snprintf(conf_path, sizeof(conf_path), "%s/%s", tool_dir, conf_name);
    f = fopen(conf_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write %s\n", conf_path);
        return -1;
    }
    write_qt_conf(f, scheme_path, font);
    fclose(f);

    printf("  ✓ %s\n", scheme_path);
    printf("  ✓ %s\n", conf_path);
    return 0;
}

int qt_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }

    char qt5ct_dir[1024], qt6ct_dir[1024];
    snprintf(qt5ct_dir, sizeof(qt5ct_dir), "%s/.config/qt5ct", home);
    snprintf(qt6ct_dir, sizeof(qt6ct_dir), "%s/.config/qt6ct", home);

    int ok5 = apply_for_tool(qt5ct_dir, "qt5ct.conf", scheme, font);
    int ok6 = apply_for_tool(qt6ct_dir, "qt6ct.conf", scheme, font);

    if (ok5 == 1 && ok6 == 1) {
        fprintf(stderr, "  ✗ Neither qt5ct nor qt6ct config directory found.\n");
        fprintf(stderr, "    Install qt5ct or qt6ct and run it once to initialise.\n");
        return -1;
    }

    if (ok5 == 0 || ok6 == 0) {
        printf("    Set QT_QPA_PLATFORMTHEME=qt5ct (or qt6ct) in your environment.\n");
    }

    return 0;
}
