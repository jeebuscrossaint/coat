//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_VSCODE_H
#define COAT_VSCODE_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a VSCode theme from a Base16 scheme
int vscode_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply VSCode theme to current VSCode configuration
int vscode_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_VSCODE_H
