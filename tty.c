//
// Created for coat TTY theming module
//

#include "tty.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Helper to convert hex to RGB components
static void hex_to_rgb(const char *hex, int *r, int *g, int *b) {
    unsigned int color;
    sscanf(strip_hash(hex), "%x", &color);
    *r = (color >> 16) & 0xFF;
    *g = (color >> 8) & 0xFF;
    *b = color & 0xFF;
}

// Generate a TTY color script from a Base16 scheme
int tty_generate_script(const Base16Scheme *scheme, const char *output_path) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create TTY theme script: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "#!/bin/sh\n");
    fprintf(f, "# coat TTY theme: %s\n", scheme->name);
    fprintf(f, "# %s\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "# variant: %s\n", scheme->variant);
    }
    fprintf(f, "\n");
    fprintf(f, "# Apply Base16 colors to Linux TTY/console\n");
    fprintf(f, "# This script should be run with root privileges or from /etc/profile.d/\n");
    fprintf(f, "\n");
    fprintf(f, "if [ \"$TERM\" = \"linux\" ]; then\n");
    
    // Map Base16 colors to the 16 ANSI colors
    // The standard mapping follows the Base16 styling guidelines:
    // 0-7: base00-base07 (backgrounds and foregrounds)
    // 8-15: base08-base0F (accent colors)
    
    const char *colors[16] = {
        scheme->base00,  // 0: Black (background)
        scheme->base08,  // 1: Red
        scheme->base0B,  // 2: Green
        scheme->base0A,  // 3: Yellow
        scheme->base0D,  // 4: Blue
        scheme->base0E,  // 5: Magenta
        scheme->base0C,  // 6: Cyan
        scheme->base05,  // 7: White (foreground)
        scheme->base03,  // 8: Bright Black (gray)
        scheme->base08,  // 9: Bright Red (reuse)
        scheme->base0B,  // 10: Bright Green (reuse)
        scheme->base0A,  // 11: Bright Yellow (reuse)
        scheme->base0D,  // 12: Bright Blue (reuse)
        scheme->base0E,  // 13: Bright Magenta (reuse)
        scheme->base0C,  // 14: Bright Cyan (reuse)
        scheme->base07   // 15: Bright White
    };
    
    // Generate escape sequences for each color
    for (int i = 0; i < 16; i++) {
        int r, g, b;
        hex_to_rgb(colors[i], &r, &g, &b);
        fprintf(f, "    printf '\\033]P%X%02X%02X%02X'\n", i, r, g, b);
    }
    
    fprintf(f, "    clear\n");
    fprintf(f, "fi\n");
    
    fclose(f);
    
    // Make the script executable
    chmod(output_path, 0755);
    
    return 0;
}

// Apply TTY theme by executing the generated script
int tty_apply_theme(const Base16Scheme *scheme) {
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
    
    // Generate theme script
    char script_path[1024];
    snprintf(script_path, sizeof(script_path), "%s/tty-theme.sh", config_dir);
    
    printf("Generating TTY theme script: %s\n", script_path);
    
    if (tty_generate_script(scheme, script_path) != 0) {
        return -1;
    }
    
    printf("TTY theme script generated successfully!\n");
    
    // Try to apply immediately if we're in a TTY
    const char *term = getenv("TERM");
    if (term && strcmp(term, "linux") == 0) {
        printf("Applying theme to current TTY...\n");
        char cmd[1100];
        snprintf(cmd, sizeof(cmd), "%s", script_path);
        int result = system(cmd);
        if (result == 0) {
            printf("✓ TTY theme applied!\n");
        } else {
            printf("Note: Could not apply automatically. Run the script manually.\n");
        }
    } else {
        printf("Not running in a TTY. Theme will apply on next TTY login.\n");
    }
    
    printf("\nTo make permanent, add one of the following:\n");
    printf("\n1. For systemd systems, create /etc/systemd/system/tty-theme.service:\n");
    printf("   [Unit]\n");
    printf("   Description=Set TTY color scheme\n");
    printf("   DefaultDependencies=no\n");
    printf("   After=local-fs.target\n");
    printf("\n");
    printf("   [Service]\n");
    printf("   Type=oneshot\n");
    printf("   ExecStart=%s\n", script_path);
    printf("   StandardInput=tty\n");
    printf("   StandardOutput=tty\n");
    printf("\n");
    printf("   [Install]\n");
    printf("   WantedBy=sysinit.target\n");
    printf("\n   Then run: sudo systemctl enable --now tty-theme.service\n");
    printf("\n2. Or add to ~/.bashrc or ~/.zshrc:\n");
    printf("   %s\n", script_path);
    printf("\n3. Or copy to /etc/profile.d/ (requires root):\n");
    printf("   sudo cp %s /etc/profile.d/tty-theme.sh\n", script_path);
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
