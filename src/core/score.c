#include "score.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// Calculate circular hue difference (0-360 wraps around)
static double hue_difference(double a, double b) {
    double diff = fabs(a - b);
    return fmin(diff, 360.0 - diff);
}

// Compare function for sorting by score (descending)
static int compare_scored_colors(const void *a, const void *b) {
    ScoredColor *ca = (ScoredColor*)a;
    ScoredColor *cb = (ScoredColor*)b;
    if (cb->score > ca->score) return 1;
    if (cb->score < ca->score) return -1;
    return 0;
}

ScoredColor *score_colors(QuantizeResult *quant_result, int desired, int *out_count) {
    if (!quant_result || quant_result->count == 0) {
        *out_count = 0;
        return NULL;
    }
    
    // 1. Convert to HCT and build hue histogram
    HCT *colors_hct = malloc(quant_result->count * sizeof(HCT));
    int hue_population[360] = {0};
    double population_sum = 0.0;
    double max_chroma = 0.0;
    
    for (int i = 0; i < quant_result->count; i++) {
        colors_hct[i] = rgb_to_hct(quant_result->colors[i]);
        int population = quant_result->populations[i];
        
        int hue_bucket = (int)floor(colors_hct[i].h);
        if (hue_bucket < 0) hue_bucket = 0;
        if (hue_bucket > 359) hue_bucket = 359;
        
        hue_population[hue_bucket] += population;
        population_sum += population;
        
        // Track maximum chroma for low-chroma detection
        if (colors_hct[i].c > max_chroma) {
            max_chroma = colors_hct[i].c;
        }
    }
    
    // Detect low-chroma images (e.g., black & white photos, classical art)
    int is_low_chroma = (max_chroma < 48.0);
    double chroma_cutoff = is_low_chroma ? 1.0 : CUTOFF_CHROMA;
    double tone_min = is_low_chroma ? 5.0 : 15.0;
    double proportion_cutoff = is_low_chroma ? 0.0025 : CUTOFF_EXCITED_PROPORTION;
    
    // 2. Calculate "excited proportions" (popularity in ±14° neighborhood)
    double hue_excited_proportions[360] = {0};
    for (int hue = 0; hue < 360; hue++) {
        double proportion = hue_population[hue] / population_sum;
        
        // Spread to ±14 degree neighbors (29 degree window)
        for (int i = hue - 14; i < hue + 16; i++) {
            int neighbor_hue = i;
            while (neighbor_hue < 0) neighbor_hue += 360;
            while (neighbor_hue >= 360) neighbor_hue -= 360;
            
            hue_excited_proportions[neighbor_hue] += proportion;
        }
    }
    
    // 3. Score each color
    ScoredColor *scored_colors = malloc(quant_result->count * sizeof(ScoredColor));
    int scored_count = 0;
    
    for (int i = 0; i < quant_result->count; i++) {
        HCT hct = colors_hct[i];
        int hue_bucket = (int)round(hct.h);
        if (hue_bucket < 0) hue_bucket = 0;
        if (hue_bucket > 359) hue_bucket = 359;
        
        double proportion = hue_excited_proportions[hue_bucket];
       
        // Filter unsuitable colors
        // CRITICAL: Reject very dark or very light tones (unstable HCT)
        if (hct.t < tone_min || hct.t > 95.0) {
            continue;
        }
        
        // Reject low chroma colors (use adaptive cutoff)
        if (hct.c < chroma_cutoff || proportion <= proportion_cutoff) {
            continue;
        }
        
        // Calculate score
        double proportion_score = proportion * 100.0 * WEIGHT_PROPORTION;
        
        double chroma_weight = (hct.c < TARGET_CHROMA) 
            ? WEIGHT_CHROMA_BELOW 
            : WEIGHT_CHROMA_ABOVE;
        double chroma_score = (hct.c - TARGET_CHROMA) * chroma_weight;
        
        double total_score = proportion_score + chroma_score;
        
        scored_colors[scored_count].color = quant_result->colors[i];
        scored_colors[scored_count].score = total_score;
        scored_colors[scored_count].hue = hct.h;
        scored_colors[scored_count].chroma = hct.c;
        scored_count++;
    }
    
    free(colors_hct);
    
    // If no colors passed filtering, return empty
    if (scored_count == 0) {
        free(scored_colors);
        *out_count = 0;
        return NULL;
    }
    
    // 4. Sort by score (highest first)
    qsort(scored_colors, scored_count, sizeof(ScoredColor), compare_scored_colors);
    
    // 5. Select colors with diverse hues
    // For low-chroma images, skip hue diversity (all colors are similar anyway)
    // For normal images, try different minimum hue separations (90° down to 15°)
    ScoredColor *chosen_colors = malloc(desired * sizeof(ScoredColor));
    int chosen_count = 0;
    
    if (is_low_chroma) {
        // No hue diversity requirement - just take top scored colors
        for (int i = 0; i < scored_count && i < desired; i++) {
            chosen_colors[i] = scored_colors[i];
            chosen_count++;
        }
    } else {
        // Normal hue diversity selection
        for (int hue_diff = 90; hue_diff >= 15; hue_diff -= 15) {
            chosen_count = 0;
        
            for (int i = 0; i < scored_count && chosen_count < desired; i++) {
                // Check if hue is too similar to already chosen colors
                int duplicate_hue = 0;
                for (int j = 0; j < chosen_count; j++) {
                    double diff = hue_difference(scored_colors[i].hue, chosen_colors[j].hue);
                    if (diff < hue_diff) {
                        duplicate_hue = 1;
                        break;
                    }
                }
                
                if (!duplicate_hue) {
                    chosen_colors[chosen_count] = scored_colors[i];
                    chosen_count++;
                }
            }
            
            if (chosen_count >= desired) break;
        }
    }
    
    // 6. Return chosen colors (with fallback)
    if (chosen_count == 0) {
        // Fallback: return highest scored color
        chosen_colors[0] = scored_colors[0];
        chosen_count = 1;
    }
    
    free(scored_colors);
    
    *out_count = chosen_count;
    return chosen_colors;
}

void score_colors_free(ScoredColor *colors) {
    free(colors);
}
