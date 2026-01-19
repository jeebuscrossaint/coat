//
// Created by amarnath on 1/19/26.
//

#include "vscode.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <json-c/json.h>

// Helper to strip # from hex color if present
static const char* strip_hash(const char *color) {
    return (color[0] == '#') ? color + 1 : color;
}

// Helper to create a json color object
static json_object* color_obj(const char *color) {
    char buf[8];
    snprintf(buf, sizeof(buf), "#%s", strip_hash(color));
    return json_object_new_string(buf);
}

// Create directories recursively
static int mkdirp(const char *path) {
    char tmp[512];
    char *p = NULL;
    size_t len;
    
    snprintf(tmp, sizeof(tmp), "%s", path);
    len = strlen(tmp);
    if (tmp[len - 1] == '/') {
        tmp[len - 1] = 0;
    }
    
    for (p = tmp + 1; *p; p++) {
        if (*p == '/') {
            *p = 0;
            mkdir(tmp, 0755);
            *p = '/';
        }
    }
    return mkdir(tmp, 0755);
}

// Helper to create color customizations object
static json_object* create_workbench_colors(const Base16Scheme *scheme) {
    json_object *colors = json_object_new_object();
    
    // Activity bar
    json_object_object_add(colors, "activityBar.background", color_obj(scheme->base00));
    json_object_object_add(colors, "activityBar.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "activityBar.inactiveForeground", color_obj(scheme->base03));
    json_object_object_add(colors, "activityBarBadge.background", color_obj(scheme->base0D));
    json_object_object_add(colors, "activityBarBadge.foreground", color_obj(scheme->base00));
    
    // Editor
    json_object_object_add(colors, "editor.background", color_obj(scheme->base00));
    json_object_object_add(colors, "editor.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "editor.lineHighlightBackground", color_obj(scheme->base01));
    json_object_object_add(colors, "editor.selectionBackground", color_obj(scheme->base02));
    json_object_object_add(colors, "editor.findMatchBackground", color_obj(scheme->base0A));
    char transp_color[10];
    snprintf(transp_color, sizeof(transp_color), "#%s80", strip_hash(scheme->base0A));
    json_object_object_add(colors, "editor.findMatchHighlightBackground", json_object_new_string(transp_color));
    json_object_object_add(colors, "editorCursor.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "editorWhitespace.foreground", color_obj(scheme->base03));
    json_object_object_add(colors, "editorIndentGuide.background", color_obj(scheme->base02));
    json_object_object_add(colors, "editorIndentGuide.activeBackground", color_obj(scheme->base03));
    json_object_object_add(colors, "editorLineNumber.foreground", color_obj(scheme->base03));
    json_object_object_add(colors, "editorLineNumber.activeForeground", color_obj(scheme->base05));
    
    // Editor widget
    json_object_object_add(colors, "editorWidget.background", color_obj(scheme->base01));
    json_object_object_add(colors, "editorWidget.border", color_obj(scheme->base03));
    json_object_object_add(colors, "editorSuggestWidget.background", color_obj(scheme->base01));
    json_object_object_add(colors, "editorSuggestWidget.selectedBackground", color_obj(scheme->base02));
    
    // Sidebar
    json_object_object_add(colors, "sideBar.background", color_obj(scheme->base01));
    json_object_object_add(colors, "sideBar.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "sideBarTitle.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "sideBarSectionHeader.background", color_obj(scheme->base00));
    
    // Status bar
    json_object_object_add(colors, "statusBar.background", color_obj(scheme->base0D));
    json_object_object_add(colors, "statusBar.foreground", color_obj(scheme->base00));
    json_object_object_add(colors, "statusBar.noFolderBackground", color_obj(scheme->base0E));
    json_object_object_add(colors, "statusBar.debuggingBackground", color_obj(scheme->base08));
    
    // Title bar
    json_object_object_add(colors, "titleBar.activeBackground", color_obj(scheme->base00));
    json_object_object_add(colors, "titleBar.activeForeground", color_obj(scheme->base05));
    json_object_object_add(colors, "titleBar.inactiveBackground", color_obj(scheme->base01));
    json_object_object_add(colors, "titleBar.inactiveForeground", color_obj(scheme->base03));
    
    // Tabs
    json_object_object_add(colors, "tab.activeBackground", color_obj(scheme->base00));
    json_object_object_add(colors, "tab.activeForeground", color_obj(scheme->base05));
    json_object_object_add(colors, "tab.inactiveBackground", color_obj(scheme->base01));
    json_object_object_add(colors, "tab.inactiveForeground", color_obj(scheme->base03));
    json_object_object_add(colors, "tab.border", color_obj(scheme->base00));
    
    // Panel
    json_object_object_add(colors, "panel.background", color_obj(scheme->base00));
    json_object_object_add(colors, "panel.border", color_obj(scheme->base02));
    json_object_object_add(colors, "panelTitle.activeForeground", color_obj(scheme->base05));
    json_object_object_add(colors, "panelTitle.inactiveForeground", color_obj(scheme->base03));
    
    // Terminal
    json_object_object_add(colors, "terminal.background", color_obj(scheme->base00));
    json_object_object_add(colors, "terminal.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "terminal.ansiBlack", color_obj(scheme->base00));
    json_object_object_add(colors, "terminal.ansiRed", color_obj(scheme->base08));
    json_object_object_add(colors, "terminal.ansiGreen", color_obj(scheme->base0B));
    json_object_object_add(colors, "terminal.ansiYellow", color_obj(scheme->base0A));
    json_object_object_add(colors, "terminal.ansiBlue", color_obj(scheme->base0D));
    json_object_object_add(colors, "terminal.ansiMagenta", color_obj(scheme->base0E));
    json_object_object_add(colors, "terminal.ansiCyan", color_obj(scheme->base0C));
    json_object_object_add(colors, "terminal.ansiWhite", color_obj(scheme->base05));
    json_object_object_add(colors, "terminal.ansiBrightBlack", color_obj(scheme->base03));
    json_object_object_add(colors, "terminal.ansiBrightRed", color_obj(scheme->base08));
    json_object_object_add(colors, "terminal.ansiBrightGreen", color_obj(scheme->base0B));
    json_object_object_add(colors, "terminal.ansiBrightYellow", color_obj(scheme->base0A));
    json_object_object_add(colors, "terminal.ansiBrightBlue", color_obj(scheme->base0D));
    json_object_object_add(colors, "terminal.ansiBrightMagenta", color_obj(scheme->base0E));
    json_object_object_add(colors, "terminal.ansiBrightCyan", color_obj(scheme->base0C));
    json_object_object_add(colors, "terminal.ansiBrightWhite", color_obj(scheme->base07));
    
    // Input
    json_object_object_add(colors, "input.background", color_obj(scheme->base01));
    json_object_object_add(colors, "input.foreground", color_obj(scheme->base05));
    json_object_object_add(colors, "input.border", color_obj(scheme->base03));
    
    // Buttons
    json_object_object_add(colors, "button.background", color_obj(scheme->base0D));
    json_object_object_add(colors, "button.foreground", color_obj(scheme->base00));
    
    // Lists
    json_object_object_add(colors, "list.activeSelectionBackground", color_obj(scheme->base0D));
    json_object_object_add(colors, "list.activeSelectionForeground", color_obj(scheme->base00));
    json_object_object_add(colors, "list.inactiveSelectionBackground", color_obj(scheme->base02));
    json_object_object_add(colors, "list.hoverBackground", color_obj(scheme->base02));
    
    // Notifications
    json_object_object_add(colors, "notificationCenter.border", color_obj(scheme->base03));
    json_object_object_add(colors, "notifications.background", color_obj(scheme->base01));
    json_object_object_add(colors, "notifications.foreground", color_obj(scheme->base05));
    
    // Git
    json_object_object_add(colors, "gitDecoration.modifiedResourceForeground", color_obj(scheme->base0A));
    json_object_object_add(colors, "gitDecoration.deletedResourceForeground", color_obj(scheme->base08));
    json_object_object_add(colors, "gitDecoration.untrackedResourceForeground", color_obj(scheme->base0B));
    json_object_object_add(colors, "gitDecoration.ignoredResourceForeground", color_obj(scheme->base03));
    json_object_object_add(colors, "gitDecoration.conflictingResourceForeground", color_obj(scheme->base09));
    
    return colors;
}

// Helper to create token colors array
static json_object* create_token_colors(const Base16Scheme *scheme) {
    json_object *rules = json_object_new_array();
    
    // Comment
    json_object *rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("comment"));
    json_object *settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base03));
    json_object_object_add(settings, "fontStyle", json_object_new_string("italic"));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Constant
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("constant"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base09));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Entity name
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("entity.name"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base0A));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Function name
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("entity.name.function"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base0D));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Keyword
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("keyword"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Storage
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("storage"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // String
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("string"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base0B));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    // Variable
    rule = json_object_new_object();
    json_object_object_add(rule, "scope", json_object_new_string("variable"));
    settings = json_object_new_object();
    json_object_object_add(settings, "foreground", color_obj(scheme->base08));
    json_object_object_add(rule, "settings", settings);
    json_object_array_add(rules, rule);
    
    return rules;
}

// Generate a VSCode theme from a Base16 scheme
int vscode_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    (void)output_path;  // Not used in JSON approach
    (void)font;  // Font will be handled in apply_theme
    (void)scheme;  // Suppress unused warnings
    return 0;
}

// Apply VSCode theme to current VSCode configuration
int vscode_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    if (!scheme) {
        return -1;
    }
    
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not determine home directory\n");
        return -1;
    }
    
    // VSCode config directory
    char config_dir[1024];
    snprintf(config_dir, sizeof(config_dir), "%s/.config/Code/User", home);
    
    struct stat st = {0};
    if (stat(config_dir, &st) == -1) {
        if (mkdir(config_dir, 0755) != 0) {
            fprintf(stderr, "Failed to create VSCode config directory\n");
            return -1;
        }
    }
    
    // Settings path
    char settings_path[1024];
    snprintf(settings_path, sizeof(settings_path), "%s/settings.json", config_dir);
    
    // Load existing settings or create new
    json_object *root = NULL;
    FILE *f = fopen(settings_path, "r");
    if (f) {
        fseek(f, 0, SEEK_END);
        long fsize = ftell(f);
        fseek(f, 0, SEEK_SET);
        
        char *content = malloc(fsize + 1);
        if (content) {
            fread(content, 1, fsize, f);
            content[fsize] = 0;
            root = json_tokener_parse(content);
            free(content);
        }
        fclose(f);
    }
    
    // Create new object if parse failed or file didn't exist
    if (!root) {
        root = json_object_new_object();
    }
    
    // Add/update font if provided
    if (font && font->monospace[0]) {
        json_object_object_add(root, "editor.fontFamily", json_object_new_string(font->monospace));
    }
    
    // Apply workbench colors directly (not scoped to a theme)
    json_object *workbench_colors = create_workbench_colors(scheme);
    json_object_object_add(root, "workbench.colorCustomizations", workbench_colors);
    
    // Apply token colors directly (not scoped to a theme)
    json_object *token_custom = json_object_new_object();
    json_object_object_add(token_custom, "textMateRules", create_token_colors(scheme));
    json_object_object_add(root, "editor.tokenColorCustomizations", token_custom);
    
    // Write back to file with pretty printing
    f = fopen(settings_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write settings.json\n");
        json_object_put(root);
        return -1;
    }
    
    fprintf(f, "%s\n", json_object_to_json_string_ext(root, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    
    json_object_put(root);
    
    printf("VSCode settings updated: %s\n", settings_path);
    printf("Theme 'Coat' applied. VSCode will auto-reload the settings.\n");
    
    return 0;
}
