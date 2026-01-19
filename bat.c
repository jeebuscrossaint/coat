//
// Created by amarnath on 1/19/26.
//

#include "bat.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

// Helper to strip '#' from hex colors
static const char* strip_hash(const char *color) {
    return (color && color[0] == '#') ? color + 1 : color;
}

// Generate bat theme in tmTheme (Sublime Text) format
static int bat_generate_theme(const Base16Scheme *scheme, const char *theme_path) {
    FILE *f = fopen(theme_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create bat theme: %s\n", theme_path);
        return -1;
    }
    
    // Write tmTheme XML header
    fprintf(f, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    fprintf(f, "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
    fprintf(f, "<plist version=\"1.0\">\n");
    fprintf(f, "<dict>\n");
    fprintf(f, "\t<key>name</key>\n");
    fprintf(f, "\t<string>%s</string>\n", scheme->name);
    fprintf(f, "\t<key>author</key>\n");
    fprintf(f, "\t<string>%s</string>\n", scheme->author);
    fprintf(f, "\t<key>settings</key>\n");
    fprintf(f, "\t<array>\n");
    
    // Global settings
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>background</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base00));
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base05));
    fprintf(f, "\t\t\t\t<key>caret</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base05));
    fprintf(f, "\t\t\t\t<key>lineHighlight</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base01));
    fprintf(f, "\t\t\t\t<key>selection</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base02));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Comments
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Comment</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>comment</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base03));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Strings
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>String</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>string</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0B));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Numbers
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Number</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>constant.numeric</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base09));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Keywords
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Keyword</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>keyword, storage</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0E));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Functions
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Function</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>entity.name.function, support.function</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0D));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Classes/Types
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Class</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>entity.name.class, entity.name.type, support.type, support.class</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0A));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Variables
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Variable</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>variable</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base08));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Constants
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Constant</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>constant</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base09));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Operators
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Operator</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>keyword.operator</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0C));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Tags (HTML/XML)
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Tag</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>entity.name.tag</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base08));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Attributes
    fprintf(f, "\t\t<dict>\n");
    fprintf(f, "\t\t\t<key>name</key>\n");
    fprintf(f, "\t\t\t<string>Attribute</string>\n");
    fprintf(f, "\t\t\t<key>scope</key>\n");
    fprintf(f, "\t\t\t<string>entity.other.attribute-name</string>\n");
    fprintf(f, "\t\t\t<key>settings</key>\n");
    fprintf(f, "\t\t\t<dict>\n");
    fprintf(f, "\t\t\t\t<key>foreground</key>\n");
    fprintf(f, "\t\t\t\t<string>#%s</string>\n", strip_hash(scheme->base0A));
    fprintf(f, "\t\t\t</dict>\n");
    fprintf(f, "\t\t</dict>\n");
    
    // Close XML tags
    fprintf(f, "\t</array>\n");
    fprintf(f, "</dict>\n");
    fprintf(f, "</plist>\n");
    
    fclose(f);
    return 0;
}

// Apply bat theme
int bat_apply_theme(const Base16Scheme *scheme) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // Create bat themes directory
    char themes_dir[1024];
    snprintf(themes_dir, sizeof(themes_dir), "%s/.config/bat/themes", home);
    
    // Create directories recursively
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/bat", home);
    mkdir(config_dir, 0755);
    mkdir(themes_dir, 0755);
    
    // Generate theme file
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat.tmTheme", themes_dir);
    
    printf("Generating bat theme: %s\n", theme_path);
    
    if (bat_generate_theme(scheme, theme_path) != 0) {
        return -1;
    }
    
    printf("Bat theme generated successfully!\n");
    printf("Building bat theme cache...\n");
    
    // Run bat cache --build to rebuild the theme cache
    int cache_result = system("bat cache --build > /dev/null 2>&1");
    if (cache_result == 0) {
        printf("✓ Bat cache rebuilt!\n");
    } else {
        printf("Note: Could not rebuild bat cache automatically.\n");
        printf("Run manually: bat cache --build\n");
    }
    
    printf("\nTo activate, add to ~/.config/bat/config:\n");
    printf("  --theme=\"coat\"\n");
    printf("\nOr use temporarily with:\n");
    printf("  bat --theme=coat <file>\n");
    printf("\nSee USAGE.md for more details.\n");
    
    return 0;
}
