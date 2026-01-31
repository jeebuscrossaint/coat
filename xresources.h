//
// Created by coat
//

#ifndef XRESOURCES_H
#define XRESOURCES_H

#include "tinted_parser.h"
#include "yaml.h"

int xresources_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif // XRESOURCES_H
