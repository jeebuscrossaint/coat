//
// Created for coat bemenu theming module
//

#include "bemenu.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

// Apply theme to bemenu dynamic menu
int bemenu_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create coat config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/coat", home);
    mkdir(config_dir, 0755);
    
    // Generate bemenu script path
    char script_path[1024];
    snprintf(script_path, sizeof(script_path), "%s/bemenu-theme.sh", config_dir);
    
    printf("Generating bemenu theme script: %s\n", script_path);
    
    FILE *f = fopen(script_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create bemenu theme script: %s\n", script_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "#!/bin/sh\n");
    fprintf(f, "# bemenu theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    fprintf(f, "# Export bemenu color variables\n");
    fprintf(f, "# Source this file or use the bemenu-run wrapper\n");
    fprintf(f, "\n");
    
    // Export color environment variables
    // Title bar
    fprintf(f, "export BEMENU_TF='%s'\n", scheme->base05);  // Title foreground
    fprintf(f, "export BEMENU_TB='%s'\n", scheme->base00);  // Title background
    
    // Filter/prompt
    fprintf(f, "export BEMENU_FF='%s'\n", scheme->base0D);  // Filter foreground (blue)
    fprintf(f, "export BEMENU_FB='%s'\n", scheme->base00);  // Filter background
    
    // Normal items
    fprintf(f, "export BEMENU_NF='%s'\n", scheme->base05);  // Normal foreground
    fprintf(f, "export BEMENU_NB='%s'\n", scheme->base00);  // Normal background
    
    // Highlighted items (on hover)
    fprintf(f, "export BEMENU_HF='%s'\n", scheme->base00);  // Highlighted foreground
    fprintf(f, "export BEMENU_HB='%s'\n", scheme->base0D);  // Highlighted background (blue)
    
    // Selected items
    fprintf(f, "export BEMENU_SF='%s'\n", scheme->base00);  // Selected foreground
    fprintf(f, "export BEMENU_SB='%s'\n", scheme->base0B);  // Selected background (green)
    
    // Alternating items
    fprintf(f, "export BEMENU_AF='%s'\n", scheme->base05);  // Alternating foreground
    fprintf(f, "export BEMENU_AB='%s'\n", scheme->base01);  // Alternating background
    
    // Border
    fprintf(f, "export BEMENU_BDR='%s'\n", scheme->base03); // Border color
    
    fprintf(f, "\n");
    fprintf(f, "# Run bemenu with theme\n");
    fprintf(f, "bemenu \"$@\" \\\n");
    fprintf(f, "  --tf \"$BEMENU_TF\" \\\n");
    fprintf(f, "  --tb \"$BEMENU_TB\" \\\n");
    fprintf(f, "  --ff \"$BEMENU_FF\" \\\n");
    fprintf(f, "  --fb \"$BEMENU_FB\" \\\n");
    fprintf(f, "  --nf \"$BEMENU_NF\" \\\n");
    fprintf(f, "  --nb \"$BEMENU_NB\" \\\n");
    fprintf(f, "  --hf \"$BEMENU_HF\" \\\n");
    fprintf(f, "  --hb \"$BEMENU_HB\" \\\n");
    fprintf(f, "  --sf \"$BEMENU_SF\" \\\n");
    fprintf(f, "  --sb \"$BEMENU_SB\" \\\n");
    fprintf(f, "  --af \"$BEMENU_AF\" \\\n");
    fprintf(f, "  --ab \"$BEMENU_AB\" \\\n");
    fprintf(f, "  --bdr \"$BEMENU_BDR\"\n");
    
    fclose(f);
    
    // Make the script executable
    chmod(script_path, 0755);
    
    // Also create a bemenu-run wrapper
    char wrapper_path[1024];
    snprintf(wrapper_path, sizeof(wrapper_path), "%s/bemenu-run", config_dir);
    
    f = fopen(wrapper_path, "w");
    if (f) {
        fprintf(f, "#!/bin/sh\n");
        fprintf(f, "# bemenu-run wrapper with coat theme\n");
        fprintf(f, "\n");
        fprintf(f, ". %s\n", script_path);
        fclose(f);
        chmod(wrapper_path, 0755);
    }
    
    printf("Bemenu theme generated successfully!\n");
    printf("\nTo use the themed bemenu:\n");
    printf("\n1. Use the wrapper script:\n");
    printf("   %s\n", wrapper_path);
    printf("\n2. Or source the theme in your scripts:\n");
    printf("   . %s\n", script_path);
    printf("   echo \"item1\\nitem2\\nitem3\" | bemenu\n");
    printf("\n3. Or add to your Sway config:\n");
    printf("   bindsym $mod+d exec %s\n", wrapper_path);
    printf("\n4. Replace bemenu-run in your PATH:\n");
    printf("   ln -sf %s ~/bin/bemenu-run\n", wrapper_path);
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
