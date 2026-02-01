//
// Created by amarnath on 1/18/26.
//

#ifndef COAT_YAML_H
#define COAT_YAML_H

#include <stdbool.h>

#define MAX_ENABLED_ITEMS 64
#define MAX_STRING_LEN 256

// Font sizes structure
typedef struct {
    int terminal;    // Size for terminals and text editors
    int desktop;     // Size for window titles, status bars
    int popups;      // Size for notifications, popups
} FontSizes;

// Opacity settings structure
typedef struct {
    float terminal;      // Opacity for terminal windows (0.0-1.0)
    float applications;  // Opacity for application windows (0.0-1.0)
    float desktop;       // Opacity for bars/widgets (0.0-1.0)
    float popups;        // Opacity for notifications/popups (0.0-1.0)
} OpacityConfig;

// Font configuration structure
typedef struct {
    char emoji[MAX_STRING_LEN];
    char monospace[MAX_STRING_LEN];
    char sansserif[MAX_STRING_LEN];
    char serif[MAX_STRING_LEN];
    FontSizes sizes;  // Font sizes
} FontConfig;

// Main configuration structure
typedef struct {
    char **enabled;           // Array of enabled items
    int enabled_count;        // Number of enabled items
    char scheme[MAX_STRING_LEN];  // Color scheme name
    FontConfig font;          // Font configuration
    OpacityConfig opacity;    // Opacity configuration
} CoatConfig;

// Function declarations
CoatConfig* coat_config_new(void);
void coat_config_free(CoatConfig *config);
int coat_config_load(CoatConfig *config, const char *filepath);
int coat_config_load_default(CoatConfig *config);

#endif //COAT_YAML_H