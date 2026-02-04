#define _POSIX_C_SOURCE 200809L
#include "schemes_list.h"
#include "tinted_parser.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <dirent.h>
#include <ctype.h>

// ANSI color codes
#define ANSI_RESET "\033[0m"
#define ANSI_BOLD "\033[1m"

// Convert hex color to RGB values
static void hex_to_rgb(const char *hex, int *r, int *g, int *b) {
    // Remove # if present
    const char *color = (hex[0] == '#') ? hex + 1 : hex;
    
    unsigned int value;
    sscanf(color, "%x", &value);
    
    *r = (value >> 16) & 0xFF;
    *g = (value >> 8) & 0xFF;
    *b = value & 0xFF;
}

// Print text with RGB background color using ANSI escape codes
static void print_color_block(const char *hex) {
    int r, g, b;
    hex_to_rgb(hex, &r, &g, &b);
    
    // Use 24-bit color (true color) ANSI escape code
    printf("\033[48;2;%d;%d;%dm  \033[0m %-7s ", r, g, b, hex);
}

// Show a visual preview of the scheme colors
void schemes_show_preview(const Base16Scheme *scheme) {
    printf("\n");
    printf(ANSI_BOLD "Color Preview:\n" ANSI_RESET);
    printf("─────────────────────────────────────────────────────\n");
    
    // Background/Foreground colors
    printf("Backgrounds & Foregrounds:\n");
    print_color_block(scheme->base00);
    print_color_block(scheme->base01);
    print_color_block(scheme->base02);
    print_color_block(scheme->base03);
    printf("\n");
    print_color_block(scheme->base04);
    print_color_block(scheme->base05);
    print_color_block(scheme->base06);
    print_color_block(scheme->base07);
    printf("\n\n");
    
    // Accent colors
    printf("Accent Colors:\n");
    print_color_block(scheme->base08);
    print_color_block(scheme->base09);
    print_color_block(scheme->base0A);
    print_color_block(scheme->base0B);
    printf("\n");
    print_color_block(scheme->base0C);
    print_color_block(scheme->base0D);
    print_color_block(scheme->base0E);
    print_color_block(scheme->base0F);
    printf("\n");
}

// Case-insensitive substring search
static int contains_ignore_case(const char *haystack, const char *needle) {
    if (!needle || !needle[0]) return 1;
    
    char *hay_lower = strdup(haystack);
    char *needle_lower = strdup(needle);
    
    for (int i = 0; hay_lower[i]; i++) {
        hay_lower[i] = tolower(hay_lower[i]);
    }
    for (int i = 0; needle_lower[i]; i++) {
        needle_lower[i] = tolower(needle_lower[i]);
    }
    
    int result = strstr(hay_lower, needle_lower) != NULL;
    
    free(hay_lower);
    free(needle_lower);
    
    return result;
}

// Comparison function for qsort
static int compare_schemes(const void *a, const void *b) {
    const Base16Scheme *scheme_a = (const Base16Scheme *)a;
    const Base16Scheme *scheme_b = (const Base16Scheme *)b;
    return strcmp(scheme_a->name, scheme_b->name);
}

// List all available schemes
int schemes_list_all(const char *schemes_dir, const char *variant_filter, const char *search_term, int show_preview) {
    DIR *dir = opendir(schemes_dir);
    if (!dir) {
        fprintf(stderr, "Failed to open schemes directory: %s\n", schemes_dir);
        return -1;
    }
    
    // First pass: count schemes
    int capacity = 100;
    int count = 0;
    Base16Scheme *schemes = malloc(capacity * sizeof(Base16Scheme));
    if (!schemes) {
        closedir(dir);
        return -1;
    }
    
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        // Skip non-yaml files
        if (strstr(entry->d_name, ".yaml") == NULL) {
            continue;
        }
        
        // Skip README
        if (strcmp(entry->d_name, "README.md") == 0) {
            continue;
        }
        
        // Expand array if needed
        if (count >= capacity) {
            capacity *= 2;
            Base16Scheme *new_schemes = realloc(schemes, capacity * sizeof(Base16Scheme));
            if (!new_schemes) {
                free(schemes);
                closedir(dir);
                return -1;
            }
            schemes = new_schemes;
        }
        
        // Load the scheme
        char filepath[2048];
        snprintf(filepath, sizeof(filepath), "%s/%s", schemes_dir, entry->d_name);
        
        Base16Scheme *scheme = &schemes[count];
        memset(scheme, 0, sizeof(Base16Scheme));
        
        if (base16_scheme_load(scheme, filepath) == 0) {
            // Apply filters
            int include = 1;
            
            // Variant filter
            if (variant_filter && scheme->variant[0]) {
                if (strcasecmp(scheme->variant, variant_filter) != 0) {
                    include = 0;
                }
            }
            
            // Search filter
            if (search_term && include) {
                if (!contains_ignore_case(scheme->name, search_term) &&
                    !contains_ignore_case(scheme->author, search_term) &&
                    !contains_ignore_case(scheme->slug, search_term)) {
                    include = 0;
                }
            }
            
            if (include) {
                count++;
            }
        }
    }
    
    closedir(dir);
    
    // Sort schemes alphabetically by name
    qsort(schemes, count, sizeof(Base16Scheme), compare_schemes);
    
    // Display results
    if (count == 0) {
        printf("No schemes found matching criteria.\n");
    } else {
        printf("Found %d scheme%s:\n\n", count, count == 1 ? "" : "s");
        
        for (int i = 0; i < count; i++) {
            Base16Scheme *scheme = &schemes[i];
            
            printf(ANSI_BOLD "%-30s" ANSI_RESET, scheme->name);
            
            if (scheme->variant[0]) {
                printf(" [%s]", scheme->variant);
            }
            printf("\n");
            
            if (scheme->author[0]) {
                printf("  Author: %s\n", scheme->author);
            }
            
            if (scheme->slug[0]) {
                printf("  Slug:   %s\n", scheme->slug);
            }
            
            if (show_preview) {
                schemes_show_preview(scheme);
            }
            
            printf("\n");
        }
    }
    
    free(schemes);
    return 0;
}
