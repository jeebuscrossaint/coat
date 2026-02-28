//
// Created by amarnath on 2/27/26.
//

#ifndef COAT_MATERIAL_YOU_H
#define COAT_MATERIAL_YOU_H

#include "tinted_parser.h"

// Material You color transformation parameters
typedef struct {
    float saturation_boost;    // Multiplier for saturation (1.0 = no change, >1.0 = more vibrant)
    float lightness_adjust;    // Additive adjustment for lightness (-100 to +100)
    float contrast_enhance;    // Contrast enhancement factor (1.0 = no change)
    int harmonize;             // Apply color harmonization (0 = off, 1 = on)
} MaterialYouConfig;

// Apply Material You transformation to a Base16 scheme
// Transforms colors to be more vibrant, harmonious, and Material You-like
void material_you_transform(Base16Scheme *scheme, MaterialYouConfig *config);

// Apply Material You transformation with default settings
void material_you_transform_default(Base16Scheme *scheme);

#endif //COAT_MATERIAL_YOU_H
