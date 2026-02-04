//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_KITTY_H
#define COAT_KITTY_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a kitty terminal theme file from a Base16 scheme
int kitty_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font, const OpacityConfig *opacity);

// Apply kitty theme to current kitty configuration
int kitty_apply_theme(const Base16Scheme *scheme, const FontConfig *font, const OpacityConfig *opacity);

#endif //COAT_KITTY_H
