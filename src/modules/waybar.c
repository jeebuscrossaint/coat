#include "waybar.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <pwd.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Generate waybar CSS theme from a Base16 scheme
int waybar_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    if (!scheme || !output_path) {
        return -1;
    }
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to create waybar theme file: %s\n", output_path);
        return -1;
    }
    
    // Write header
    fprintf(f, "/* coat waybar theme: %s */\n", scheme->name);
    fprintf(f, "/* %s */\n", scheme->author);
    if (scheme->variant[0]) {
        fprintf(f, "/* variant: %s */\n", scheme->variant);
    }
    fprintf(f, "\n");
    
    // Define CSS variables for easy customization
    fprintf(f, "/* Base16 color variables */\n");
    fprintf(f, "@define-color base00 #%s; /* Default Background */\n", strip_hash(scheme->base00));
    fprintf(f, "@define-color base01 #%s; /* Lighter Background */\n", strip_hash(scheme->base01));
    fprintf(f, "@define-color base02 #%s; /* Selection Background */\n", strip_hash(scheme->base02));
    fprintf(f, "@define-color base03 #%s; /* Comments, Invisibles */\n", strip_hash(scheme->base03));
    fprintf(f, "@define-color base04 #%s; /* Dark Foreground */\n", strip_hash(scheme->base04));
    fprintf(f, "@define-color base05 #%s; /* Default Foreground */\n", strip_hash(scheme->base05));
    fprintf(f, "@define-color base06 #%s; /* Light Foreground */\n", strip_hash(scheme->base06));
    fprintf(f, "@define-color base07 #%s; /* Light Background */\n", strip_hash(scheme->base07));
    fprintf(f, "@define-color base08 #%s; /* Red */\n", strip_hash(scheme->base08));
    fprintf(f, "@define-color base09 #%s; /* Orange */\n", strip_hash(scheme->base09));
    fprintf(f, "@define-color base0A #%s; /* Yellow */\n", strip_hash(scheme->base0A));
    fprintf(f, "@define-color base0B #%s; /* Green */\n", strip_hash(scheme->base0B));
    fprintf(f, "@define-color base0C #%s; /* Cyan */\n", strip_hash(scheme->base0C));
    fprintf(f, "@define-color base0D #%s; /* Blue */\n", strip_hash(scheme->base0D));
    fprintf(f, "@define-color base0E #%s; /* Magenta */\n", strip_hash(scheme->base0E));
    fprintf(f, "@define-color base0F #%s; /* Brown */\n", strip_hash(scheme->base0F));
    fprintf(f, "\n");
    
    // Global window styling
    fprintf(f, "/* Global window styling */\n");
    fprintf(f, "* {\n");
    fprintf(f, "    border: none;\n");
    fprintf(f, "    border-radius: 0;\n");
    if (font && font->monospace[0]) {
        fprintf(f, "    font-family: \"%s\";\n", font->monospace);
        fprintf(f, "    font-size: %dpx;\n", font->sizes.desktop);
    } else {
        fprintf(f, "    font-family: monospace;\n");
        fprintf(f, "    font-size: 13px;\n");
    }
    fprintf(f, "    min-height: 0;\n");
    fprintf(f, "}\n\n");
    
    // Main waybar window
    fprintf(f, "/* Main waybar window */\n");
    fprintf(f, "window#waybar {\n");
    fprintf(f, "    background: @base00;\n");
    fprintf(f, "    color: @base05;\n");
    fprintf(f, "}\n\n");
    
    // Workspace buttons
    fprintf(f, "/* Workspaces */\n");
    fprintf(f, "#workspaces button {\n");
    fprintf(f, "    padding: 0 8px;\n");
    fprintf(f, "    background: transparent;\n");
    fprintf(f, "    color: @base04;\n");
    fprintf(f, "    border-bottom: 2px solid transparent;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#workspaces button.focused,\n");
    fprintf(f, "#workspaces button.active {\n");
    fprintf(f, "    background: @base02;\n");
    fprintf(f, "    color: @base0D;\n");
    fprintf(f, "    border-bottom: 2px solid @base0D;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#workspaces button:hover {\n");
    fprintf(f, "    background: @base01;\n");
    fprintf(f, "    color: @base05;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#workspaces button.urgent {\n");
    fprintf(f, "    background: @base08;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "}\n\n");
    
    // Mode indicator
    fprintf(f, "/* Mode indicator */\n");
    fprintf(f, "#mode {\n");
    fprintf(f, "    background: @base0A;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "    padding: 0 10px;\n");
    fprintf(f, "    font-weight: bold;\n");
    fprintf(f, "}\n\n");
    
    // Module styling
    fprintf(f, "/* Individual modules */\n");
    fprintf(f, "#clock,\n");
    fprintf(f, "#battery,\n");
    fprintf(f, "#cpu,\n");
    fprintf(f, "#memory,\n");
    fprintf(f, "#disk,\n");
    fprintf(f, "#temperature,\n");
    fprintf(f, "#backlight,\n");
    fprintf(f, "#network,\n");
    fprintf(f, "#pulseaudio,\n");
    fprintf(f, "#wireplumber,\n");
    fprintf(f, "#custom-media,\n");
    fprintf(f, "#tray,\n");
    fprintf(f, "#idle_inhibitor,\n");
    fprintf(f, "#mpd,\n");
    fprintf(f, "#language,\n");
    fprintf(f, "#keyboard-state {\n");
    fprintf(f, "    padding: 0 10px;\n");
    fprintf(f, "    margin: 0 2px;\n");
    fprintf(f, "    color: @base05;\n");
    fprintf(f, "    background: @base01;\n");
    fprintf(f, "}\n\n");
    
    // Clock - accent color
    fprintf(f, "#clock {\n");
    fprintf(f, "    background: @base0D;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "    font-weight: bold;\n");
    fprintf(f, "}\n\n");
    
    // Battery states
    fprintf(f, "/* Battery states */\n");
    fprintf(f, "#battery {\n");
    fprintf(f, "    color: @base0B;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#battery.charging {\n");
    fprintf(f, "    color: @base0B;\n");
    fprintf(f, "    background: @base01;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#battery.warning:not(.charging) {\n");
    fprintf(f, "    background: @base09;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#battery.critical:not(.charging) {\n");
    fprintf(f, "    background: @base08;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "    animation: blink 0.5s linear infinite alternate;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "@keyframes blink {\n");
    fprintf(f, "    to {\n");
    fprintf(f, "        background: @base00;\n");
    fprintf(f, "        color: @base08;\n");
    fprintf(f, "    }\n");
    fprintf(f, "}\n\n");
    
    // Network states
    fprintf(f, "/* Network states */\n");
    fprintf(f, "#network.disconnected {\n");
    fprintf(f, "    color: @base08;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#network.connected {\n");
    fprintf(f, "    color: @base0B;\n");
    fprintf(f, "}\n\n");
    
    // Audio states
    fprintf(f, "/* Audio states */\n");
    fprintf(f, "#pulseaudio.muted,\n");
    fprintf(f, "#wireplumber.muted {\n");
    fprintf(f, "    color: @base08;\n");
    fprintf(f, "}\n\n");
    
    // CPU/Memory warnings
    fprintf(f, "/* System resource warnings */\n");
    fprintf(f, "#cpu.warning,\n");
    fprintf(f, "#memory.warning,\n");
    fprintf(f, "#temperature.warning {\n");
    fprintf(f, "    background: @base09;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#cpu.critical,\n");
    fprintf(f, "#memory.critical,\n");
    fprintf(f, "#temperature.critical {\n");
    fprintf(f, "    background: @base08;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "}\n\n");
    
    // Tray
    fprintf(f, "/* Tray */\n");
    fprintf(f, "#tray {\n");
    fprintf(f, "    background: transparent;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#tray > .passive {\n");
    fprintf(f, "    -gtk-icon-effect: dim;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#tray > .needs-attention {\n");
    fprintf(f, "    -gtk-icon-effect: highlight;\n");
    fprintf(f, "    background: @base08;\n");
    fprintf(f, "}\n\n");
    
    // Idle inhibitor
    fprintf(f, "/* Idle inhibitor */\n");
    fprintf(f, "#idle_inhibitor.activated {\n");
    fprintf(f, "    background: @base0E;\n");
    fprintf(f, "    color: @base00;\n");
    fprintf(f, "}\n\n");
    
    // MPD
    fprintf(f, "/* MPD */\n");
    fprintf(f, "#mpd.playing {\n");
    fprintf(f, "    color: @base0B;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#mpd.paused {\n");
    fprintf(f, "    color: @base03;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "#mpd.stopped {\n");
    fprintf(f, "    color: @base08;\n");
    fprintf(f, "}\n\n");
    
    // Tooltip
    fprintf(f, "/* Tooltips */\n");
    fprintf(f, "tooltip {\n");
    fprintf(f, "    background: @base01;\n");
    fprintf(f, "    border: 1px solid @base03;\n");
    fprintf(f, "}\n\n");
    
    fprintf(f, "tooltip label {\n");
    fprintf(f, "    color: @base05;\n");
    fprintf(f, "}\n\n");
    
    fclose(f);
    return 0;
}

// Apply waybar theme
int waybar_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        struct passwd *pw = getpwuid(getuid());
        if (pw) {
            home = pw->pw_dir;
        } else {
            fprintf(stderr, "Could not determine home directory\n");
            return -1;
        }
    }
    
    // Create waybar config directory if it doesn't exist
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/waybar", home);
    mkdir(config_dir, 0755);
    
    // Generate theme path
    char theme_path[1024];
    snprintf(theme_path, sizeof(theme_path), "%s/coat-theme.css", config_dir);
    
    if (waybar_generate_theme(scheme, theme_path, font) != 0) {
        return -1;
    }
    
    printf("  ✓ %s\n", theme_path);
    
    // Try to reload waybar
    system("pkill -SIGUSR2 waybar 2>/dev/null");
    
    return 0;
}
