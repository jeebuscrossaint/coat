#include "mangowc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

void mangowc_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    (void)font; // Unused - mangowc doesn't support font config
    
    if (!scheme) return;
    
    char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return;
    }
    
    // Create mango config directory if it doesn't exist
    char config_dir[512];
    snprintf(config_dir, sizeof(config_dir), "%s/.config", home);
    mkdir(config_dir, 0755);
    snprintf(config_dir, sizeof(config_dir), "%s/.config/mango", home);
    mkdir(config_dir, 0755);
    
    // Generate colors file (not config.conf to avoid overwriting user config)
    char colors_path[512];
    snprintf(colors_path, sizeof(colors_path), "%s/coat-colors.conf", config_dir);
    
    printf("Generating mangowc colors: %s\n", colors_path);
    
    FILE *f = fopen(colors_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create mangowc colors file: %s\n", colors_path);
        return;
    }
    
    // Write header
    fprintf(f, "# coat theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Write color configuration in mangowc format (0xRRGGBBAA)
    fprintf(f, "# Window border colors\n");
    fprintf(f, "bordercolor=0x%sff\n", scheme->base03 + 1); // Skip the '#'
    fprintf(f, "focuscolor=0x%sff\n", scheme->base0D + 1);
    fprintf(f, "urgentcolor=0x%sff\n", scheme->base08 + 1);
    fprintf(f, "maximizescreencolor=0x%sff\n", scheme->base0B + 1);
    fprintf(f, "scratchpadcolor=0x%sff\n", scheme->base0C + 1);
    fprintf(f, "globalcolor=0x%sff\n", scheme->base0E + 1);
    fprintf(f, "overlaycolor=0x%sff\n", scheme->base0A + 1);
    fprintf(f, "rootcolor=0x%sff\n", scheme->base00 + 1);
    
    // Shadow color with transparency
    fprintf(f, "shadowscolor=0x%s80\n", scheme->base00 + 1); // 50% alpha
    fprintf(f, "\n");
    
    // Border width
    fprintf(f, "# Border configuration\n");
    fprintf(f, "borderpx=2\n");
    
    fclose(f);
    
    printf("  ✓ %s\n", colors_path);
}
