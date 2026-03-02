#ifndef SCORE_H
#define SCORE_H

#include "hct.h"
#include "quantize_celebi.h"

// Material You color scoring algorithm
// Based on material-color-utilities Score.score()

#define TARGET_CHROMA 48.0
#define WEIGHT_PROPORTION 0.7
#define WEIGHT_CHROMA_ABOVE 0.3
#define WEIGHT_CHROMA_BELOW 0.1
#define CUTOFF_CHROMA 5.0
#define CUTOFF_EXCITED_PROPORTION 0.01

// Scored result
typedef struct {
    RGB color;
    double score;
    double hue;
    double chroma;
} ScoredColor;

// Score colors from quantization result
// Returns best colors sorted by score (highest first)
ScoredColor *score_colors(QuantizeResult *quant_result, int desired, int *out_count);

// Free scored colors
void score_colors_free(ScoredColor *colors);

#endif // SCORE_H
