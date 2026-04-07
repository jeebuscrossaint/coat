#include "ranger.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

int ranger_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    (void)font;
    if (!scheme) return -1;

    const char *home = getenv("HOME");
    if (!home) return -1;

    char scheme_dir[1024];
    snprintf(scheme_dir, sizeof(scheme_dir), "%s/.config/ranger/colorschemes", home);

    // Create dirs
    char ranger_dir[1024];
    snprintf(ranger_dir, sizeof(ranger_dir), "%s/.config/ranger", home);
    mkdir(ranger_dir, 0755);
    mkdir(scheme_dir, 0755);

    char path[1024];
    snprintf(path, sizeof(path), "%s/coat.py", scheme_dir);

    FILE *f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create ranger colorscheme: %s\n", path);
        return -1;
    }

    // Ranger colorschemes use terminal color indices (0-15)
    // These map to whatever Base16 colors the terminal has loaded
    // base00=0 base08=1 base0B=2 base0A=3 base0D=4 base0E=5 base0C=6 base05=7
    // base03=8 base08=9 base0B=10 base0A=11 base0D=12 base0E=13 base0C=14 base07=15
    fprintf(f, "# coat colorscheme: %s\n", scheme->name);
    fprintf(f, "# %s\n\n", scheme->author);
    fprintf(f, "from ranger.gui.colorscheme import ColorScheme\n");
    fprintf(f, "from ranger.gui.color import *\n\n");
    fprintf(f, "class Coat(ColorScheme):\n");
    fprintf(f, "    progress_bar_color = 4  # base0D\n\n");
    fprintf(f, "    def use(self, context):\n");
    fprintf(f, "        fg, bg, attr = default_colors\n\n");
    fprintf(f, "        if context.reset:\n");
    fprintf(f, "            return default_colors\n\n");
    fprintf(f, "        elif context.in_browser:\n");
    fprintf(f, "            if context.selected:\n");
    fprintf(f, "                attr = reverse\n");
    fprintf(f, "            if context.empty or context.error:\n");
    fprintf(f, "                fg = 1   # base08 red\n");
    fprintf(f, "            if context.directory:\n");
    fprintf(f, "                fg = 4; attr = bold  # base0D blue\n");
    fprintf(f, "            elif context.executable and not any((\n");
    fprintf(f, "                    context.media, context.container,\n");
    fprintf(f, "                    context.fifo, context.socket)):\n");
    fprintf(f, "                fg = 2; attr = bold  # base0B green\n");
    fprintf(f, "            if context.socket:\n");
    fprintf(f, "                fg = 5; attr = bold  # base0E magenta\n");
    fprintf(f, "            if context.fifo or context.device:\n");
    fprintf(f, "                fg = 3; attr = bold  # base0A yellow\n");
    fprintf(f, "            if context.link:\n");
    fprintf(f, "                fg = 6 if context.good else 1  # base0C cyan / base08 red\n");
    fprintf(f, "            if context.media:\n");
    fprintf(f, "                fg = 3  # base0A yellow\n");
    fprintf(f, "            if context.container:\n");
    fprintf(f, "                fg = 1; attr = bold  # base08 red\n");
    fprintf(f, "            if context.tag_marker and not context.selected:\n");
    fprintf(f, "                attr = bold\n");
    fprintf(f, "                fg = 1 if not context.multiple_selection else 3\n");
    fprintf(f, "            if not context.selected and (context.cut or context.copied):\n");
    fprintf(f, "                fg = 8; attr = bold  # base03 bright black\n");
    fprintf(f, "            if context.main_column:\n");
    fprintf(f, "                if context.selected: attr = bold\n");
    fprintf(f, "                if context.marked: attr = bold | reverse\n");
    fprintf(f, "            if context.badinfo:\n");
    fprintf(f, "                fg = 5 if attr & reverse else 5\n\n");
    fprintf(f, "        elif context.in_titlebar:\n");
    fprintf(f, "            if context.hostname:\n");
    fprintf(f, "                fg = 1 if context.bad else 2  # red / green\n");
    fprintf(f, "            elif context.directory:\n");
    fprintf(f, "                fg = 4  # base0D blue\n");
    fprintf(f, "            elif context.tab:\n");
    fprintf(f, "                if context.good: bg = 2\n");
    fprintf(f, "            elif context.link:\n");
    fprintf(f, "                fg = 6  # base0C cyan\n\n");
    fprintf(f, "        elif context.in_statusbar:\n");
    fprintf(f, "            if context.permissions:\n");
    fprintf(f, "                fg = 6 if context.good else 5  # cyan / magenta\n");
    fprintf(f, "            if context.marked:\n");
    fprintf(f, "                attr = bold | reverse; fg = 3\n");
    fprintf(f, "            if context.message:\n");
    fprintf(f, "                if context.bad: attr = bold; fg = 1\n");
    fprintf(f, "            if context.loaded:\n");
    fprintf(f, "                bg = self.progress_bar_color\n");
    fprintf(f, "            if context.vcsinfo: fg = 4; attr = normal\n");
    fprintf(f, "            if context.vcscommit: fg = 3; attr = normal\n");
    fprintf(f, "            if context.vcsdate: fg = 6; attr = normal\n\n");
    fprintf(f, "        if context.text and context.highlight:\n");
    fprintf(f, "            attr = reverse\n\n");
    fprintf(f, "        if context.in_taskview:\n");
    fprintf(f, "            if context.title: fg = 4\n");
    fprintf(f, "            if context.selected: attr = reverse\n");
    fprintf(f, "            if context.loaded:\n");
    fprintf(f, "                fg = self.progress_bar_color if context.selected \\\n");
    fprintf(f, "                    else default\n");
    fprintf(f, "                bg = self.progress_bar_color if not context.selected \\\n");
    fprintf(f, "                    else default\n\n");
    fprintf(f, "        if context.vcsfile and not context.selected:\n");
    fprintf(f, "            attr = normal\n");
    fprintf(f, "            if context.vcsconflict: fg = 5; attr = bold\n");
    fprintf(f, "            elif context.vcschanged: fg = 1\n");
    fprintf(f, "            elif context.vcsunknown: fg = 1\n");
    fprintf(f, "            elif context.vcsstaged: fg = 2\n");
    fprintf(f, "            elif context.vcssync: fg = 2\n");
    fprintf(f, "            elif context.vcsignored: fg = default\n\n");
    fprintf(f, "        elif context.vcsremote and not context.selected:\n");
    fprintf(f, "            attr = normal\n");
    fprintf(f, "            if context.vcssync or context.vcsnone: fg = 2\n");
    fprintf(f, "            elif context.vcsbehind: fg = 1\n");
    fprintf(f, "            elif context.vcsahead: fg = 4\n");
    fprintf(f, "            elif context.vcsdiverged: fg = 5\n");
    fprintf(f, "            elif context.vcsunknown: fg = 1\n\n");
    fprintf(f, "        return fg, bg, attr\n");

    fclose(f);
    printf("  ✓ %s\n", path);
    printf("    Add to ~/.config/ranger/rc.conf: set colorscheme coat\n");
    return 0;
}
