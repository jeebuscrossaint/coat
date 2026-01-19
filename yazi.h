//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_YAZI_H
#define COAT_YAZI_H

#include "tinted_parser.h"

// Generate a yazi theme file from a Base16 scheme
int yazi_generate_theme(const Base16Scheme *scheme, const char *output_path);

// Apply yazi theme to current yazi configuration
int yazi_apply_theme(const Base16Scheme *scheme);

#endif //COAT_YAZI_H
