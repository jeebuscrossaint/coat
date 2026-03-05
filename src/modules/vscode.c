#define _POSIX_C_SOURCE 200809L
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

int vscode_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not get HOME directory\n");
        return -1;
    }
    
    // Ensure the settings directory exists
    char settings_dir[512];
    snprintf(settings_dir, sizeof(settings_dir), "%s/.config/Code/User", home);
    mkdirp(settings_dir);
    
    char settings_path[512];
    snprintf(settings_path, sizeof(settings_path), "%s/settings.json", settings_dir);
    
    // Read existing settings
    json_object *root = NULL;
    FILE *f = fopen(settings_path, "r");
    if (f) {
        fseek(f, 0, SEEK_END);
        long fsize = ftell(f);
        fseek(f, 0, SEEK_SET);
        
        char *content = malloc(fsize + 1);
        if (content) {
            size_t read = fread(content, 1, fsize, f);
            content[read] = 0;
            root = json_tokener_parse(content);
            free(content);
        }
        fclose(f);
    }
    
    if (!root) {
        root = json_object_new_object();
    }
    
    // Create color customizations object (UI colors)
    json_object *color_customizations = json_object_new_object();
    
    // Activity bar
    json_object_object_add(color_customizations, "activityBar.background", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "activityBar.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "activityBar.inactiveForeground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "activityBarBadge.background", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "activityBarBadge.foreground", color_obj(scheme->base00));
    
    // Editor
    json_object_object_add(color_customizations, "editor.background", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "editor.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "editor.lineHighlightBackground", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "editor.selectionBackground", color_obj(scheme->base02));
    json_object_object_add(color_customizations, "editor.findMatchBackground", color_obj(scheme->base0A));
    char transp_color[10];
    snprintf(transp_color, sizeof(transp_color), "#%s80", strip_hash(scheme->base0A));
    json_object_object_add(color_customizations, "editor.findMatchHighlightBackground", json_object_new_string(transp_color));
    json_object_object_add(color_customizations, "editorCursor.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "editorWhitespace.foreground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "editorIndentGuide.background", color_obj(scheme->base02));
    json_object_object_add(color_customizations, "editorIndentGuide.activeBackground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "editorLineNumber.foreground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "editorLineNumber.activeForeground", color_obj(scheme->base05));
    
    // Editor widget
    json_object_object_add(color_customizations, "editorWidget.background", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "editorWidget.border", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "editorSuggestWidget.background", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "editorSuggestWidget.selectedBackground", color_obj(scheme->base02));
    
    // Sidebar
    json_object_object_add(color_customizations, "sideBar.background", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "sideBar.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "sideBarTitle.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "sideBarSectionHeader.background", color_obj(scheme->base00));
    
    // Status bar
    json_object_object_add(color_customizations, "statusBar.background", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "statusBar.foreground", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "statusBar.noFolderBackground", color_obj(scheme->base0E));
    json_object_object_add(color_customizations, "statusBar.debuggingBackground", color_obj(scheme->base08));
    
    // Title bar
    json_object_object_add(color_customizations, "titleBar.activeBackground", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "titleBar.activeForeground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "titleBar.inactiveBackground", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "titleBar.inactiveForeground", color_obj(scheme->base03));
    
    // Tabs
    json_object_object_add(color_customizations, "tab.activeBackground", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "tab.activeForeground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "tab.inactiveBackground", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "tab.inactiveForeground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "tab.border", color_obj(scheme->base00));
    
    // Panel
    json_object_object_add(color_customizations, "panel.background", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "panel.border", color_obj(scheme->base02));
    json_object_object_add(color_customizations, "panelTitle.activeForeground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "panelTitle.inactiveForeground", color_obj(scheme->base03));
    
    // Terminal
    json_object_object_add(color_customizations, "terminal.background", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "terminal.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "terminal.ansiBlack", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "terminal.ansiRed", color_obj(scheme->base08));
    json_object_object_add(color_customizations, "terminal.ansiGreen", color_obj(scheme->base0B));
    json_object_object_add(color_customizations, "terminal.ansiYellow", color_obj(scheme->base0A));
    json_object_object_add(color_customizations, "terminal.ansiBlue", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "terminal.ansiMagenta", color_obj(scheme->base0E));
    json_object_object_add(color_customizations, "terminal.ansiCyan", color_obj(scheme->base0C));
    json_object_object_add(color_customizations, "terminal.ansiWhite", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "terminal.ansiBrightBlack", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "terminal.ansiBrightRed", color_obj(scheme->base08));
    json_object_object_add(color_customizations, "terminal.ansiBrightGreen", color_obj(scheme->base0B));
    json_object_object_add(color_customizations, "terminal.ansiBrightYellow", color_obj(scheme->base0A));
    json_object_object_add(color_customizations, "terminal.ansiBrightBlue", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "terminal.ansiBrightMagenta", color_obj(scheme->base0E));
    json_object_object_add(color_customizations, "terminal.ansiBrightCyan", color_obj(scheme->base0C));
    json_object_object_add(color_customizations, "terminal.ansiBrightWhite", color_obj(scheme->base07));
    
    // Input
    json_object_object_add(color_customizations, "input.background", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "input.foreground", color_obj(scheme->base05));
    json_object_object_add(color_customizations, "input.border", color_obj(scheme->base03));
    
    // Buttons
    json_object_object_add(color_customizations, "button.background", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "button.foreground", color_obj(scheme->base00));
    
    // Lists
    json_object_object_add(color_customizations, "list.activeSelectionBackground", color_obj(scheme->base0D));
    json_object_object_add(color_customizations, "list.activeSelectionForeground", color_obj(scheme->base00));
    json_object_object_add(color_customizations, "list.inactiveSelectionBackground", color_obj(scheme->base02));
    json_object_object_add(color_customizations, "list.hoverBackground", color_obj(scheme->base02));
    
    // Notifications
    json_object_object_add(color_customizations, "notificationCenter.border", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "notifications.background", color_obj(scheme->base01));
    json_object_object_add(color_customizations, "notifications.foreground", color_obj(scheme->base05));
    
    // Git decorations
    json_object_object_add(color_customizations, "gitDecoration.modifiedResourceForeground", color_obj(scheme->base0A));
    json_object_object_add(color_customizations, "gitDecoration.deletedResourceForeground", color_obj(scheme->base08));
    json_object_object_add(color_customizations, "gitDecoration.untrackedResourceForeground", color_obj(scheme->base0B));
    json_object_object_add(color_customizations, "gitDecoration.ignoredResourceForeground", color_obj(scheme->base03));
    json_object_object_add(color_customizations, "gitDecoration.conflictingResourceForeground", color_obj(scheme->base09));
    
    // Add to settings
    json_object_object_add(root, "workbench.colorCustomizations", color_customizations);
    
    // Create token color customizations (syntax highlighting)
    json_object *token_customizations = json_object_new_object();
    json_object *text_mate_rules = json_object_new_array();
    
    // Comments
    json_object *comment = json_object_new_object();
    json_object_object_add(comment, "scope", json_object_new_string("comment"));
    json_object *comment_settings = json_object_new_object();
    json_object_object_add(comment_settings, "foreground", color_obj(scheme->base03));
    json_object_object_add(comment_settings, "fontStyle", json_object_new_string("italic"));
    json_object_object_add(comment, "settings", comment_settings);
    json_object_array_add(text_mate_rules, comment);
    
    // Constants
    json_object *constant = json_object_new_object();
    json_object_object_add(constant, "scope", json_object_new_string("constant"));
    json_object *constant_settings = json_object_new_object();
    json_object_object_add(constant_settings, "foreground", color_obj(scheme->base09));
    json_object_object_add(constant, "settings", constant_settings);
    json_object_array_add(text_mate_rules, constant);
    
    // Entity names
    json_object *entity_name = json_object_new_object();
    json_object_object_add(entity_name, "scope", json_object_new_string("entity.name"));
    json_object *entity_name_settings = json_object_new_object();
    json_object_object_add(entity_name_settings, "foreground", color_obj(scheme->base0A));
    json_object_object_add(entity_name, "settings", entity_name_settings);
    json_object_array_add(text_mate_rules, entity_name);
    
    // Functions
    json_object *func = json_object_new_object();
    json_object_object_add(func, "scope", json_object_new_string("entity.name.function"));
    json_object *func_settings = json_object_new_object();
    json_object_object_add(func_settings, "foreground", color_obj(scheme->base0D));
    json_object_object_add(func, "settings", func_settings);
    json_object_array_add(text_mate_rules, func);
    
    // Keywords
    json_object *keyword = json_object_new_object();
    json_object_object_add(keyword, "scope", json_object_new_string("keyword"));
    json_object *keyword_settings = json_object_new_object();
    json_object_object_add(keyword_settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(keyword, "settings", keyword_settings);
    json_object_array_add(text_mate_rules, keyword);
    
    // Storage
    json_object *storage = json_object_new_object();
    json_object_object_add(storage, "scope", json_object_new_string("storage"));
    json_object *storage_settings = json_object_new_object();
    json_object_object_add(storage_settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(storage, "settings", storage_settings);
    json_object_array_add(text_mate_rules, storage);
    
    // Strings
    json_object *string = json_object_new_object();
    json_object_object_add(string, "scope", json_object_new_string("string"));
    json_object *string_settings = json_object_new_object();
    json_object_object_add(string_settings, "foreground", color_obj(scheme->base0B));
    json_object_object_add(string, "settings", string_settings);
    json_object_array_add(text_mate_rules, string);
    
    // Variables
    json_object *variable = json_object_new_object();
    json_object_object_add(variable, "scope", json_object_new_string("variable"));
    json_object *variable_settings = json_object_new_object();
    json_object_object_add(variable_settings, "foreground", color_obj(scheme->base08));
    json_object_object_add(variable, "settings", variable_settings);
    json_object_array_add(text_mate_rules, variable);
    
    json_object_object_add(token_customizations, "textMateRules", text_mate_rules);
    json_object_object_add(root, "editor.tokenColorCustomizations", token_customizations);
    
    // Add font if provided
    if (font && font->monospace[0]) {
        json_object_object_add(root, "editor.fontFamily", json_object_new_string(font->monospace));
    }
    
    // Write settings.json
    f = fopen(settings_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write settings.json\n");
        json_object_put(root);
        return -1;
    }
    fprintf(f, "%s\n", json_object_to_json_string_ext(root, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    json_object_put(root);
    
    printf("  ✓ %s\n", settings_path);
    
    return 0;
}
