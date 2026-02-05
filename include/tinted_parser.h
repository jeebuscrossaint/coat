//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_TINTED_PARSER_H
#define COAT_TINTED_PARSER_H

#define MAX_SCHEME_NAME 256
#define MAX_SCHEME_DESC 1024
#define MAX_COLOR_VALUE 16

typedef struct {
    char system[64];
    char name[MAX_SCHEME_NAME];
    char slug[MAX_SCHEME_NAME];
    char author[MAX_SCHEME_NAME];
    char description[MAX_SCHEME_DESC];
    char variant[32];
    
    // Base16 color palette
    char base00[MAX_COLOR_VALUE];  // Default Background
    char base01[MAX_COLOR_VALUE];  // Lighter Background
    char base02[MAX_COLOR_VALUE];  // Selection Background
    char base03[MAX_COLOR_VALUE];  // Comments, Invisibles
    char base04[MAX_COLOR_VALUE];  // Dark Foreground
    char base05[MAX_COLOR_VALUE];  // Default Foreground
    char base06[MAX_COLOR_VALUE];  // Light Foreground
    char base07[MAX_COLOR_VALUE];  // Light Background
    char base08[MAX_COLOR_VALUE];  // Variables, XML Tags
    char base09[MAX_COLOR_VALUE];  // Integers, Booleans
    char base0A[MAX_COLOR_VALUE];  // Classes, Search Text
    char base0B[MAX_COLOR_VALUE];  // Strings, Inherited Class
    char base0C[MAX_COLOR_VALUE];  // Support, Regex
    char base0D[MAX_COLOR_VALUE];  // Functions, Methods
    char base0E[MAX_COLOR_VALUE];  // Keywords, Storage
    char base0F[MAX_COLOR_VALUE];  // Deprecated, Embedded
    
    // Base24 extended palette (optional, for Base24 schemes)
    char base10[MAX_COLOR_VALUE];  // Base24 extended color
    char base11[MAX_COLOR_VALUE];  // Base24 extended color
    char base12[MAX_COLOR_VALUE];  // Base24 extended color
    char base13[MAX_COLOR_VALUE];  // Base24 extended color
    char base14[MAX_COLOR_VALUE];  // Base24 extended color
    char base15[MAX_COLOR_VALUE];  // Base24 extended color
    char base16[MAX_COLOR_VALUE];  // Base24 extended color
    char base17[MAX_COLOR_VALUE];  // Base24 extended color
    
    int is_base24;  // Flag to indicate if this is a Base24 scheme
} Base16Scheme;

// Create a new Base16 scheme structure
Base16Scheme* base16_scheme_new(void);

// Free a Base16 scheme structure
void base16_scheme_free(Base16Scheme *scheme);

// Load a Base16 scheme from a file
int base16_scheme_load(Base16Scheme *scheme, const char *filepath);

// Find and load a scheme by name from the schemes directory
int base16_scheme_load_by_name(Base16Scheme *scheme, const char *scheme_name, const char *schemes_dir);

#endif //COAT_TINTED_PARSER_H