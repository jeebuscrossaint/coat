//
// Created by amarnath on 2/27/26.
//

#ifndef COAT_HYPRLAND_H
#define COAT_HYPRLAND_H

#include "tinted_parser.h"
#include "yaml.h"

// Apply Base16 color scheme to Hyprland configuration
int hyprland_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif //COAT_HYPRLAND_H
