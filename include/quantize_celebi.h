#ifndef QUANTIZE_CELEBI_H
#define QUANTIZE_CELEBI_H

#include "hct.h"

// QuantizeCelebi: Wu + Wsmeans color quantization
// Based on Material Color Utilities implementation

#define MAX_COLORS 128  // Maximum colors to extract

typedef struct {
    RGB *colors;        // Extracted colors
    int *populations;   // Pixel count for each color
    int count;          // Number of colors
} QuantizeResult;

// Main quantization function
// pixels: Array of RGB pixels
// pixel_count: Number of pixels
// max_colors: Maximum colors to extract (typically 128)
QuantizeResult quantize_celebi(RGB *pixels, int pixel_count, int max_colors);

// Free quantization result
void quantize_result_free(QuantizeResult *result);

#endif // QUANTIZE_CELEBI_H
