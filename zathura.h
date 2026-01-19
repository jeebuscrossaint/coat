//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_ZATHURA_H
#define COAT_ZATHURA_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a zathura PDF viewer theme file from a Base16 scheme
int zathura_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply zathura theme to current zathura configuration
int zathura_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_ZATHURA_H
