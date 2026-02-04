//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_I3_H
#define COAT_I3_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate an i3 window manager theme file from a Base16 scheme
int i3_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply i3 theme to current i3 configuration
int i3_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_I3_H
