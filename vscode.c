//
// Created by amarnath on 1/19/26.
//

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

// Generate the VSCode theme JSON
static json_object* generate_theme_json(const Base16Scheme *scheme) {
    json_object *theme = json_object_new_object();
    
    // Determine if this is a light or dark theme
    const char *theme_type = "dark";
    if (scheme->variant[0] && strcmp(scheme->variant, "light") == 0) {
        theme_type = "light";
    }
    
    // Theme metadata
    json_object_object_add(theme, "name", json_object_new_string("Coat"));
    json_object_object_add(theme, "type", json_object_new_string(theme_type));
    
    // Colors object (UI theming)
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
    
    // Git decorations
    json_object_object_add(colors, "gitDecoration.modifiedResourceForeground", color_obj(scheme->base0A));
    json_object_object_add(colors, "gitDecoration.deletedResourceForeground", color_obj(scheme->base08));
    json_object_object_add(colors, "gitDecoration.untrackedResourceForeground", color_obj(scheme->base0B));
    json_object_object_add(colors, "gitDecoration.ignoredResourceForeground", color_obj(scheme->base03));
    json_object_object_add(colors, "gitDecoration.conflictingResourceForeground", color_obj(scheme->base09));
    
    json_object_object_add(theme, "colors", colors);
    
    // Token colors (syntax highlighting)
    json_object *token_colors = json_object_new_array();
    
    // Comments
    json_object *comment = json_object_new_object();
    json_object_object_add(comment, "scope", json_object_new_string("comment"));
    json_object *comment_settings = json_object_new_object();
    json_object_object_add(comment_settings, "foreground", color_obj(scheme->base03));
    json_object_object_add(comment_settings, "fontStyle", json_object_new_string("italic"));
    json_object_object_add(comment, "settings", comment_settings);
    json_object_array_add(token_colors, comment);
    
    // Constants
    json_object *constant = json_object_new_object();
    json_object_object_add(constant, "scope", json_object_new_string("constant"));
    json_object *constant_settings = json_object_new_object();
    json_object_object_add(constant_settings, "foreground", color_obj(scheme->base09));
    json_object_object_add(constant, "settings", constant_settings);
    json_object_array_add(token_colors, constant);
    
    // Entity names
    json_object *entity_name = json_object_new_object();
    json_object_object_add(entity_name, "scope", json_object_new_string("entity.name"));
    json_object *entity_name_settings = json_object_new_object();
    json_object_object_add(entity_name_settings, "foreground", color_obj(scheme->base0A));
    json_object_object_add(entity_name, "settings", entity_name_settings);
    json_object_array_add(token_colors, entity_name);
    
    // Functions
    json_object *func = json_object_new_object();
    json_object_object_add(func, "scope", json_object_new_string("entity.name.function"));
    json_object *func_settings = json_object_new_object();
    json_object_object_add(func_settings, "foreground", color_obj(scheme->base0D));
    json_object_object_add(func, "settings", func_settings);
    json_object_array_add(token_colors, func);
    
    // Keywords
    json_object *keyword = json_object_new_object();
    json_object_object_add(keyword, "scope", json_object_new_string("keyword"));
    json_object *keyword_settings = json_object_new_object();
    json_object_object_add(keyword_settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(keyword, "settings", keyword_settings);
    json_object_array_add(token_colors, keyword);
    
    // Storage
    json_object *storage = json_object_new_object();
    json_object_object_add(storage, "scope", json_object_new_string("storage"));
    json_object *storage_settings = json_object_new_object();
    json_object_object_add(storage_settings, "foreground", color_obj(scheme->base0E));
    json_object_object_add(storage, "settings", storage_settings);
    json_object_array_add(token_colors, storage);
    
    // Strings
    json_object *string = json_object_new_object();
    json_object_object_add(string, "scope", json_object_new_string("string"));
    json_object *string_settings = json_object_new_object();
    json_object_object_add(string_settings, "foreground", color_obj(scheme->base0B));
    json_object_object_add(string, "settings", string_settings);
    json_object_array_add(token_colors, string);
    
    // Variables
    json_object *variable = json_object_new_object();
    json_object_object_add(variable, "scope", json_object_new_string("variable"));
    json_object *variable_settings = json_object_new_object();
    json_object_object_add(variable_settings, "foreground", color_obj(scheme->base08));
    json_object_object_add(variable, "settings", variable_settings);
    json_object_array_add(token_colors, variable);
    
    json_object_object_add(theme, "tokenColors", token_colors);
    
    return theme;
}

// Generate package.json for the extension
static json_object* generate_package_json(const Base16Scheme *scheme) {
    json_object *package = json_object_new_object();
    
    // Determine base UI theme
    const char *ui_theme = "vs-dark";
    if (scheme->variant[0] && strcmp(scheme->variant, "light") == 0) {
        ui_theme = "vs";
    }
    
    json_object_object_add(package, "name", json_object_new_string("coat-theme"));
    json_object_object_add(package, "displayName", json_object_new_string("Coat Theme"));
    json_object_object_add(package, "description", json_object_new_string("Base16 theme generated by Coat"));
    json_object_object_add(package, "version", json_object_new_string("1.0.0"));
    json_object_object_add(package, "publisher", json_object_new_string("coat"));
    
    json_object *engines = json_object_new_object();
    json_object_object_add(engines, "vscode", json_object_new_string("^1.50.0"));
    json_object_object_add(package, "engines", engines);
    
    json_object *categories = json_object_new_array();
    json_object_array_add(categories, json_object_new_string("Themes"));
    json_object_object_add(package, "categories", categories);
    
    json_object *contributes = json_object_new_object();
    json_object *themes = json_object_new_array();
    json_object *theme_contrib = json_object_new_object();
    json_object_object_add(theme_contrib, "label", json_object_new_string("Coat"));
    json_object_object_add(theme_contrib, "uiTheme", json_object_new_string(ui_theme));
    json_object_object_add(theme_contrib, "path", json_object_new_string("./themes/coat-color-theme.json"));
    json_object_array_add(themes, theme_contrib);
    json_object_object_add(contributes, "themes", themes);
    json_object_object_add(package, "contributes", contributes);
    
    return package;
}

int vscode_apply_theme(const Base16Scheme *scheme, const FontConfig *font) {
    const char *home = getenv("HOME");
    if (!home) {
        fprintf(stderr, "Could not get HOME directory\n");
        return -1;
    }
    
    // Create extension directory
    char ext_dir[512];
    snprintf(ext_dir, sizeof(ext_dir), "%s/.vscode/extensions/coat-theme", home);
    mkdirp(ext_dir);
    
    // Create themes subdirectory
    char themes_dir[512];
    snprintf(themes_dir, sizeof(themes_dir), "%s/themes", ext_dir);
    mkdir(themes_dir, 0755);
    
    // Write package.json
    char package_path[512];
    snprintf(package_path, sizeof(package_path), "%s/package.json", ext_dir);
    FILE *f = fopen(package_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write package.json\n");
        return -1;
    }
    json_object *package = generate_package_json(scheme);
    fprintf(f, "%s\n", json_object_to_json_string_ext(package, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    json_object_put(package);
    
    // Write theme file
    char theme_path[512];
    snprintf(theme_path, sizeof(theme_path), "%s/themes/coat-color-theme.json", ext_dir);
    f = fopen(theme_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write theme file\n");
        return -1;
    }
    json_object *theme = generate_theme_json(scheme);
    fprintf(f, "%s\n", json_object_to_json_string_ext(theme, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    json_object_put(theme);
    
    // Update settings.json to use the theme
    // Ensure the settings directory exists
    char settings_dir[512];
    snprintf(settings_dir, sizeof(settings_dir), "%s/.config/Code/User", home);
    mkdirp(settings_dir);
    
    char settings_path[512];
    snprintf(settings_path, sizeof(settings_path), "%s/settings.json", settings_dir);
    
    json_object *root = NULL;
    f = fopen(settings_path, "r");
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
    
    // Force theme reload by removing the old setting first
    json_object_object_del(root, "workbench.colorTheme");
    
    // Set the theme (this forces VSCode to reload it)
    json_object_object_add(root, "workbench.colorTheme", json_object_new_string("Coat"));
    
    // Add font if provided
    if (font && font->monospace[0]) {
        json_object_object_add(root, "editor.fontFamily", json_object_new_string(font->monospace));
    }
    
    // Write back
    f = fopen(settings_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write settings.json\n");
        json_object_put(root);
        return -1;
    }
    fprintf(f, "%s\n", json_object_to_json_string_ext(root, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    json_object_put(root);
    
    printf("  VSCode theme extension created at %s\n", ext_dir);
    printf("  Theme set to 'Coat' in settings.json\n");
    printf("  VSCode should detect the change and reload automatically.\n");
    
    return 0;
}

int vscode_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font) {
    (void)font;  // Unused for theme generation
    json_object *theme = generate_theme_json(scheme);
    
    FILE *f = fopen(output_path, "w");
    if (!f) {
        fprintf(stderr, "Failed to write theme to %s\n", output_path);
        json_object_put(theme);
        return -1;
    }
    
    fprintf(f, "%s\n", json_object_to_json_string_ext(theme, JSON_C_TO_STRING_PRETTY));
    fclose(f);
    json_object_put(theme);
    return 0;
}
