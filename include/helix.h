//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_HELIX_H
#define COAT_HELIX_H

#include "tinted_parser.h"

// Generate a helix editor theme file from a Base16 scheme
int helix_generate_theme(const Base16Scheme *scheme, const char *output_path);

// Apply helix theme to current helix configuration
int helix_apply_theme(const Base16Scheme *scheme);

#endif //COAT_HELIX_H
