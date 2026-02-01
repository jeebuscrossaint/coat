//
// Created by coat
//

#ifndef COAT_SWAY_H
#define COAT_SWAY_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a sway window manager theme file from a Base16 scheme
int sway_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply sway theme to current sway configuration
int sway_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_SWAY_H
