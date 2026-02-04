//
// Created by amarnath on 1/19/26.
//

#ifndef DUNST_H
#define DUNST_H

#include "tinted_parser.h"
#include "yaml.h"

// Apply dunst theme
int dunst_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif // DUNST_H
