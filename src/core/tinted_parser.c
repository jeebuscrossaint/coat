#define _POSIX_C_SOURCE 200809L
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include "/usr/include/yaml.h"

// Helper function to slugify a string (convert to lowercase, replace spaces with dashes)
static void slugify(const char *input, char *output, size_t output_size) {
    size_t j = 0;
    for (size_t i = 0; input[i] && j < output_size - 1; i++) {
        if (isspace(input[i]) || input[i] == '_') {
            output[j++] = '-';
        } else if (isalnum(input[i]) || input[i] == '-') {
            output[j++] = tolower(input[i]);
        }
    }
    output[j] = '\0';
    
    // Remove consecutive dashes
    char temp[MAX_SCHEME_NAME];
    j = 0;
    int last_was_dash = 0;
    for (size_t i = 0; output[i]; i++) {
        if (output[i] == '-') {
            if (!last_was_dash) {
                temp[j++] = '-';
                last_was_dash = 1;
            }
        } else {
            temp[j++] = output[i];
            last_was_dash = 0;
        }
    }
    temp[j] = '\0';
    
    // Remove leading/trailing dashes
    size_t start = 0;
    while (temp[start] == '-') start++;
    size_t end = strlen(temp);
    while (end > start && temp[end - 1] == '-') end--;
    
    strncpy(output, temp + start, end - start);
    output[end - start] = '\0';
}

// Create a new Base16 scheme structure
Base16Scheme* base16_scheme_new(void) {
    Base16Scheme *scheme = calloc(1, sizeof(Base16Scheme));
    return scheme;
}

// Free a Base16 scheme structure
void base16_scheme_free(Base16Scheme *scheme) {
    if (scheme) {
        free(scheme);
    }
}

// Load a Base16 scheme from a file
int base16_scheme_load(Base16Scheme *scheme, const char *filepath) {
    if (!scheme || !filepath) {
        return -1;
    }

    FILE *file = fopen(filepath, "rb");
    if (!file) {
        // Don't print error - caller will handle it
        return -1;
    }

    yaml_parser_t parser;
    yaml_event_t event;
    
    if (!yaml_parser_initialize(&parser)) {
        fprintf(stderr, "Failed to initialize YAML parser\n");
        fclose(file);
        return -1;
    }
    
    yaml_parser_set_input_file(&parser, file);
    
    char current_key[256] = {0};
    int in_palette = 0;
    int done = 0;
    
    while (!done) {
        if (!yaml_parser_parse(&parser, &event)) {
            fprintf(stderr, "Parser error %d\n", parser.error);
            yaml_parser_delete(&parser);
            fclose(file);
            return -1;
        }
        
        switch (event.type) {
            case YAML_SCALAR_EVENT: {
                const char *value = (const char *)event.data.scalar.value;
                
                if (current_key[0] == '\0') {
                    // This is a key
                    strncpy(current_key, value, sizeof(current_key) - 1);
                    
                    if (strcmp(current_key, "palette") == 0) {
                        in_palette = 1;
                        current_key[0] = '\0';
                    }
                } else {
                    // This is a value
                    if (strcmp(current_key, "system") == 0) {
                        strncpy(scheme->system, value, sizeof(scheme->system) - 1);
                    } else if (strcmp(current_key, "name") == 0) {
                        strncpy(scheme->name, value, sizeof(scheme->name) - 1);
                    } else if (strcmp(current_key, "slug") == 0) {
                        strncpy(scheme->slug, value, sizeof(scheme->slug) - 1);
                    } else if (strcmp(current_key, "author") == 0) {
                        strncpy(scheme->author, value, sizeof(scheme->author) - 1);
                    } else if (strcmp(current_key, "description") == 0) {
                        strncpy(scheme->description, value, sizeof(scheme->description) - 1);
                    } else if (strcmp(current_key, "variant") == 0) {
                        strncpy(scheme->variant, value, sizeof(scheme->variant) - 1);
                    } else if (in_palette) {
                        // Handle palette colors
                        if (strcmp(current_key, "base00") == 0) {
                            strncpy(scheme->base00, value, sizeof(scheme->base00) - 1);
                        } else if (strcmp(current_key, "base01") == 0) {
                            strncpy(scheme->base01, value, sizeof(scheme->base01) - 1);
                        } else if (strcmp(current_key, "base02") == 0) {
                            strncpy(scheme->base02, value, sizeof(scheme->base02) - 1);
                        } else if (strcmp(current_key, "base03") == 0) {
                            strncpy(scheme->base03, value, sizeof(scheme->base03) - 1);
                        } else if (strcmp(current_key, "base04") == 0) {
                            strncpy(scheme->base04, value, sizeof(scheme->base04) - 1);
                        } else if (strcmp(current_key, "base05") == 0) {
                            strncpy(scheme->base05, value, sizeof(scheme->base05) - 1);
                        } else if (strcmp(current_key, "base06") == 0) {
                            strncpy(scheme->base06, value, sizeof(scheme->base06) - 1);
                        } else if (strcmp(current_key, "base07") == 0) {
                            strncpy(scheme->base07, value, sizeof(scheme->base07) - 1);
                        } else if (strcmp(current_key, "base08") == 0) {
                            strncpy(scheme->base08, value, sizeof(scheme->base08) - 1);
                        } else if (strcmp(current_key, "base09") == 0) {
                            strncpy(scheme->base09, value, sizeof(scheme->base09) - 1);
                        } else if (strcmp(current_key, "base0A") == 0) {
                            strncpy(scheme->base0A, value, sizeof(scheme->base0A) - 1);
                        } else if (strcmp(current_key, "base0B") == 0) {
                            strncpy(scheme->base0B, value, sizeof(scheme->base0B) - 1);
                        } else if (strcmp(current_key, "base0C") == 0) {
                            strncpy(scheme->base0C, value, sizeof(scheme->base0C) - 1);
                        } else if (strcmp(current_key, "base0D") == 0) {
                            strncpy(scheme->base0D, value, sizeof(scheme->base0D) - 1);
                        } else if (strcmp(current_key, "base0E") == 0) {
                            strncpy(scheme->base0E, value, sizeof(scheme->base0E) - 1);
                        } else if (strcmp(current_key, "base0F") == 0) {
                            strncpy(scheme->base0F, value, sizeof(scheme->base0F) - 1);
                        } else if (strcmp(current_key, "base10") == 0) {
                            strncpy(scheme->base10, value, sizeof(scheme->base10) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base11") == 0) {
                            strncpy(scheme->base11, value, sizeof(scheme->base11) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base12") == 0) {
                            strncpy(scheme->base12, value, sizeof(scheme->base12) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base13") == 0) {
                            strncpy(scheme->base13, value, sizeof(scheme->base13) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base14") == 0) {
                            strncpy(scheme->base14, value, sizeof(scheme->base14) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base15") == 0) {
                            strncpy(scheme->base15, value, sizeof(scheme->base15) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base16") == 0) {
                            strncpy(scheme->base16, value, sizeof(scheme->base16) - 1);
                            scheme->is_base24 = 1;
                        } else if (strcmp(current_key, "base17") == 0) {
                            strncpy(scheme->base17, value, sizeof(scheme->base17) - 1);
                            scheme->is_base24 = 1;
                        }
                    }
                    current_key[0] = '\0';
                }
                break;
            }
            
            case YAML_MAPPING_END_EVENT:
                if (in_palette) {
                    in_palette = 0;
                }
                break;
                
            case YAML_STREAM_END_EVENT:
                done = 1;
                break;
                
            default:
                break;
        }
        
        yaml_event_delete(&event);
    }
    
    yaml_parser_delete(&parser);
    fclose(file);
    
    // If slug is not provided, generate it from name
    if (scheme->slug[0] == '\0' && scheme->name[0] != '\0') {
        slugify(scheme->name, scheme->slug, sizeof(scheme->slug));
    }
    
    return 0;
}

// Find and load a scheme by name from the schemes directory
int base16_scheme_load_by_name(Base16Scheme *scheme, const char *scheme_name, const char *schemes_dir, int prefer_base24) {
    if (!scheme || !scheme_name || !schemes_dir) {
        return -1;
    }
    
    // Extract parent directory (remove /base16 suffix if present)
    char parent_dir[2048];
    strncpy(parent_dir, schemes_dir, sizeof(parent_dir) - 1);
    parent_dir[sizeof(parent_dir) - 1] = '\0';
    
    char *base16_suffix = strstr(parent_dir, "/base16");
    if (base16_suffix && base16_suffix[7] == '\0') {
        *base16_suffix = '\0';
    }
    
    char filepath[2048];
    
    // If prefer_base24 is set, try base24 first
    if (prefer_base24) {
        snprintf(filepath, sizeof(filepath), "%s/base24/%s.yaml", parent_dir, scheme_name);
        if (base16_scheme_load(scheme, filepath) == 0) {
            return 0;
        }
        // Fall back to base16
        snprintf(filepath, sizeof(filepath), "%s/base16/%s.yaml", parent_dir, scheme_name);
        int result = base16_scheme_load(scheme, filepath);
        if (result != 0) {
            fprintf(stderr, "Failed to load scheme '%s' (tried base24 and base16 directories)\n", scheme_name);
        }
        return result;
    }
    
    // Default: Try base16 directory first
    snprintf(filepath, sizeof(filepath), "%s/base16/%s.yaml", parent_dir, scheme_name);
    if (base16_scheme_load(scheme, filepath) == 0) {
        return 0;
    }
    
    // If not found, try base24 directory
    snprintf(filepath, sizeof(filepath), "%s/base24/%s.yaml", parent_dir, scheme_name);
    int result = base16_scheme_load(scheme, filepath);
    
    // Only print error if scheme wasn't found in either directory
    if (result != 0) {
        fprintf(stderr, "Failed to load scheme '%s' (tried base16 and base24 directories)\n", scheme_name);
    }
    
    return result;
}