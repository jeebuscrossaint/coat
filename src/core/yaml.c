#define _POSIX_C_SOURCE 200809L
#include "/usr/include/yaml.h"
#include  "yaml.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pwd.h>

// Create a new config structure with default values
CoatConfig* coat_config_new(void) {
    CoatConfig *config = malloc(sizeof(CoatConfig));
    if (!config) {
        return NULL;
    }

    config->enabled = malloc(sizeof(char*) * MAX_ENABLED_ITEMS);
    if (!config->enabled) {
        free(config);
        return NULL;
    }

    config->enabled_count = 0;
    config->scheme[0] = '\0';
    config->font.emoji[0] = '\0';
    config->font.monospace[0] = '\0';
    config->font.sansserif[0] = '\0';
    config->font.serif[0] = '\0';
    config->font.sizes.terminal = 12;
    config->font.sizes.desktop = 10;
    config->font.sizes.popups = 10;
    config->opacity.terminal = 1.0;
    config->opacity.applications = 1.0;
    config->opacity.desktop = 1.0;
    config->opacity.popups = 1.0;

    return config;
}

// Free config structure
void coat_config_free(CoatConfig *config) {
    if (!config) {
        return;
    }

    if (config->enabled) {
        for (int i = 0; i < config->enabled_count; i++) {
            free(config->enabled[i]);
        }
        free(config->enabled);
    }

    free(config);
}

// Parse YAML file
int coat_config_load(CoatConfig *config, const char *filepath) {
    FILE *file = fopen(filepath, "r");
    if (!file) {
        fprintf(stderr, "Failed to open config file: %s\n", filepath);
        return -1;
    }

    yaml_parser_t parser;
    yaml_event_t event;

    if (!yaml_parser_initialize(&parser)) {
        fclose(file);
        fprintf(stderr, "Failed to initialize YAML parser\n");
        return -1;
    }

    yaml_parser_set_input_file(&parser, file);

    // Parser state tracking
    enum {
        STATE_NONE,
        STATE_ENABLED,
        STATE_SCHEME,
        STATE_FONT,
        STATE_FONT_EMOJI,
        STATE_FONT_MONOSPACE,
        STATE_FONT_SANSSERIF,
        STATE_FONT_SERIF,
        STATE_FONT_SIZES,
        STATE_FONT_SIZE_TERMINAL,
        STATE_FONT_SIZE_DESKTOP,
        STATE_FONT_SIZE_POPUPS,
        STATE_OPACITY,
        STATE_OPACITY_TERMINAL,
        STATE_OPACITY_APPLICATIONS,
        STATE_OPACITY_DESKTOP,
        STATE_OPACITY_POPUPS
    } state = STATE_NONE;

    char current_key[MAX_STRING_LEN] = {0};
    int in_font_section = 0;
    int in_opacity_section = 0;
    int done = 0;

    while (!done) {
        if (!yaml_parser_parse(&parser, &event)) {
            fprintf(stderr, "Parser error %d\n", parser.error);
            yaml_parser_delete(&parser);
            fclose(file);
            return -1;
        }

        switch (event.type) {
            case YAML_STREAM_END_EVENT:
                done = 1;
                break;

            case YAML_MAPPING_START_EVENT:
                if (strcmp(current_key, "font") == 0) {
                    in_font_section = 1;
                } else if (strcmp(current_key, "opacity") == 0) {
                    in_opacity_section = 1;
                    state = STATE_OPACITY;
                } else if (strcmp(current_key, "sizes") == 0 && in_font_section) {
                    state = STATE_FONT_SIZES;
                }
                break;

            case YAML_MAPPING_END_EVENT:
                if (state == STATE_FONT_SIZES) {
                    state = STATE_NONE;
                } else if (state == STATE_OPACITY) {
                    state = STATE_NONE;
                    in_opacity_section = 0;
                } else if (in_font_section) {
                    in_font_section = 0;
                }
                break;

            case YAML_SEQUENCE_START_EVENT:
                if (strcmp(current_key, "enabled") == 0) {
                    state = STATE_ENABLED;
                }
                break;

            case YAML_SEQUENCE_END_EVENT:
                state = STATE_NONE;
                break;

            case YAML_SCALAR_EVENT: {
                char *value = (char *)event.data.scalar.value;

                if (state == STATE_ENABLED) {
                    // Add to enabled array
                    if (config->enabled_count < MAX_ENABLED_ITEMS) {
                        config->enabled[config->enabled_count] = strdup(value);
                        config->enabled_count++;
                    }
                } else if (state == STATE_SCHEME) {
                    strncpy(config->scheme, value, MAX_STRING_LEN - 1);
                    state = STATE_NONE;
                } else if (state == STATE_FONT_SIZES) {
                    // Handle font.sizes subsection
                    if (state == STATE_FONT_SIZE_TERMINAL) {
                        config->font.sizes.terminal = atoi(value);
                        state = STATE_FONT_SIZES;
                    } else if (state == STATE_FONT_SIZE_DESKTOP) {
                        config->font.sizes.desktop = atoi(value);
                        state = STATE_FONT_SIZES;
                    } else if (state == STATE_FONT_SIZE_POPUPS) {
                        config->font.sizes.popups = atoi(value);
                        state = STATE_FONT_SIZES;
                    } else {
                        // This is a key in font.sizes
                        if (strcmp(value, "terminal") == 0) {
                            state = STATE_FONT_SIZE_TERMINAL;
                        } else if (strcmp(value, "desktop") == 0) {
                            state = STATE_FONT_SIZE_DESKTOP;
                        } else if (strcmp(value, "popups") == 0) {
                            state = STATE_FONT_SIZE_POPUPS;
                        }
                    }
                } else if (in_font_section) {
                    // Handle font subsections
                    if (state == STATE_FONT_EMOJI) {
                        strncpy(config->font.emoji, value, MAX_STRING_LEN - 1);
                        state = STATE_NONE;
                    } else if (state == STATE_FONT_MONOSPACE) {
                        strncpy(config->font.monospace, value, MAX_STRING_LEN - 1);
                        state = STATE_NONE;
                    } else if (state == STATE_FONT_SANSSERIF) {
                        strncpy(config->font.sansserif, value, MAX_STRING_LEN - 1);
                        state = STATE_NONE;
                    } else if (state == STATE_FONT_SERIF) {
                        strncpy(config->font.serif, value, MAX_STRING_LEN - 1);
                        state = STATE_NONE;
                    } else {
                        // This is a key in the font section
                        if (strcmp(value, "emoji") == 0) {
                            state = STATE_FONT_EMOJI;
                        } else if (strcmp(value, "monospace") == 0) {
                            state = STATE_FONT_MONOSPACE;
                        } else if (strcmp(value, "sansserif") == 0) {
                            state = STATE_FONT_SANSSERIF;
                        } else if (strcmp(value, "serif") == 0) {
                            state = STATE_FONT_SERIF;
                        }
                    }
                } else if (in_opacity_section) {
                    // Handle opacity subsections
                    if (state == STATE_OPACITY_TERMINAL) {
                        config->opacity.terminal = atof(value);
                        state = STATE_OPACITY;
                    } else if (state == STATE_OPACITY_APPLICATIONS) {
                        config->opacity.applications = atof(value);
                        state = STATE_OPACITY;
                    } else if (state == STATE_OPACITY_DESKTOP) {
                        config->opacity.desktop = atof(value);
                        state = STATE_OPACITY;
                    } else if (state == STATE_OPACITY_POPUPS) {
                        config->opacity.popups = atof(value);
                        state = STATE_OPACITY;
                    } else {
                        // This is a key in the opacity section
                        if (strcmp(value, "terminal") == 0) {
                            state = STATE_OPACITY_TERMINAL;
                        } else if (strcmp(value, "applications") == 0) {
                            state = STATE_OPACITY_APPLICATIONS;
                        } else if (strcmp(value, "desktop") == 0) {
                            state = STATE_OPACITY_DESKTOP;
                        } else if (strcmp(value, "popups") == 0) {
                            state = STATE_OPACITY_POPUPS;
                        }
                    }
                } else {
                    // Top-level key
                    strncpy(current_key, value, MAX_STRING_LEN - 1);
                    if (strcmp(value, "scheme") == 0) {
                        state = STATE_SCHEME;
                    }
                }
                break;
            }

            default:
                break;
        }

        yaml_event_delete(&event);
    }

    yaml_parser_delete(&parser);
    fclose(file);

    return 0;
}

// Load config from default location (~/.config/coat/coat.yaml)
int coat_config_load_default(CoatConfig *config) {
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

    char config_path[1024];
    snprintf(config_path, sizeof(config_path), "%s/.config/coat/coat.yaml", home);

    return coat_config_load(config, config_path);
}
