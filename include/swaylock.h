//
// Created by coat
//

#ifndef COAT_SWAYLOCK_H
#define COAT_SWAYLOCK_H

#include "tinted_parser.h"
#include "yaml.h"

// Generate a swaylock config from a Base16 scheme
int swaylock_generate_config(const Base16Scheme *scheme, const char *output_path, const OpacityConfig *opacity);

// Apply swaylock theme to current swaylock configuration
int swaylock_apply_theme(const Base16Scheme *scheme, const OpacityConfig *opacity);

#endif //COAT_SWAYLOCK_H
