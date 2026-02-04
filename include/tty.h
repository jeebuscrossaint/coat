//
// Created for coat TTY theming module
//

#ifndef COAT_TTY_H
#define COAT_TTY_H

#include "tinted_parser.h"

// Generate a TTY color script from a Base16 scheme
int tty_generate_script(const Base16Scheme *scheme, const char *output_path);

// Apply TTY theme by executing the generated script
int tty_apply_theme(const Base16Scheme *scheme);

#endif //COAT_TTY_H
