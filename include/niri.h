//
// Created by coat
//

#ifndef COAT_NIRI_H
#define COAT_NIRI_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a niri compositor theme file from a Base16 scheme
int niri_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply niri theme to current niri configuration
int niri_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_NIRI_H
