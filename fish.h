//
// Created by amarnath on 1/18/26.
//

#ifndef COAT_FISH_H
#define COAT_FISH_H

#include "tinted_parser.h"

// Generate a fish shell theme file from a Base16 scheme
int fish_generate_theme(const Base16Scheme *scheme, const char *output_path);

// Apply fish theme to current fish configuration
int fish_apply_theme(const Base16Scheme *scheme);

#endif //COAT_FISH_H