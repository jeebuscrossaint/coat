#include "quantize_celebi.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <limits.h>

// Wu quantization constants
#define INDEX_BITS 5
#define INDEX_COUNT 33  // (1 << INDEX_BITS) + 1
#define TOTAL_SIZE 35937  // INDEX_COUNT^3

// Color utilities
static inline int rgb_to_int(RGB rgb) {
    return (rgb.r << 16) | (rgb.g << 8) | rgb.b;
}

static inline RGB int_to_rgb(int argb) {
    RGB rgb = {
        (argb >> 16) & 0xFF,
        (argb >> 8) & 0xFF,
        argb & 0xFF
    };
    return rgb;
}

// Get Wu index for color
static inline int get_index(int r, int g, int b) {
    r = r >> (8 - INDEX_BITS);
    g = g >> (8 - INDEX_BITS);
    b = b >> (8 - INDEX_BITS);
    return (r * INDEX_COUNT * INDEX_COUNT) + (g * INDEX_COUNT) + b;
}

// Box structure for Wu algorithm
typedef struct {
    int r0, r1;
    int g0, g1;
    int b0, b1;
    int volume;
} Box;

// Wu quantization data
typedef struct {
    long long weights[TOTAL_SIZE];
    long long moments_r[TOTAL_SIZE];
    long long moments_g[TOTAL_SIZE];
    long long moments_b[TOTAL_SIZE];
    double moments[TOTAL_SIZE];
    int cubes_count;
} WuData;

// Volume of a box
static long long volume(Box *box, long long *moment) {
    return (moment[get_index(box->r1, box->g1, box->b1)] -
            moment[get_index(box->r1, box->g1, box->b0)] -
            moment[get_index(box->r1, box->g0, box->b1)] +
            moment[get_index(box->r1, box->g0, box->b0)] -
            moment[get_index(box->r0, box->g1, box->b1)] +
            moment[get_index(box->r0, box->g1, box->b0)] +
            moment[get_index(box->r0, box->g0, box->b1)] -
            moment[get_index(box->r0, box->g0, box->b0)]);
}

// Volume of a box (for double moments)
static double volume_double(Box *box, double *moment) {
    return (moment[get_index(box->r1, box->g1, box->b1)] -
            moment[get_index(box->r1, box->g1, box->b0)] -
            moment[get_index(box->r1, box->g0, box->b1)] +
            moment[get_index(box->r1, box->g0, box->b0)] -
            moment[get_index(box->r0, box->g1, box->b1)] +
            moment[get_index(box->r0, box->g1, box->b0)] +
            moment[get_index(box->r0, box->g0, box->b1)] -
            moment[get_index(box->r0, box->g0, box->b0)]);
}

// Bottom of a box
static long long bottom(Box *box, int direction, long long *moment) {
    switch (direction) {
        case 0:  // Red
            return (-moment[get_index(box->r0, box->g1, box->b1)] +
                    moment[get_index(box->r0, box->g1, box->b0)] +
                    moment[get_index(box->r0, box->g0, box->b1)] -
                    moment[get_index(box->r0, box->g0, box->b0)]);
        case 1:  // Green
            return (-moment[get_index(box->r1, box->g0, box->b1)] +
                    moment[get_index(box->r1, box->g0, box->b0)] +
                    moment[get_index(box->r0, box->g0, box->b1)] -
                    moment[get_index(box->r0, box->g0, box->b0)]);
        case 2:  // Blue
            return (-moment[get_index(box->r1, box->g1, box->b0)] +
                    moment[get_index(box->r1, box->g0, box->b0)] +
                    moment[get_index(box->r0, box->g1, box->b0)] -
                    moment[get_index(box->r0, box->g0, box->b0)]);
        default:
            return 0;
    }
}

// Top of a box
static long long top(Box *box, int direction, int position, long long *moment) {
    switch (direction) {
        case 0:  // Red
            return (moment[get_index(position, box->g1, box->b1)] -
                    moment[get_index(position, box->g1, box->b0)] -
                    moment[get_index(position, box->g0, box->b1)] +
                    moment[get_index(position, box->g0, box->b0)]);
        case 1:  // Green
            return (moment[get_index(box->r1, position, box->b1)] -
                    moment[get_index(box->r1, position, box->b0)] -
                    moment[get_index(box->r0, position, box->b1)] +
                    moment[get_index(box->r0, position, box->b0)]);
        case 2:  // Blue
            return (moment[get_index(box->r1, box->g1, position)] -
                    moment[get_index(box->r1, box->g0, position)] -
                    moment[get_index(box->r0, box->g1, position)] +
                    moment[get_index(box->r0, box->g0, position)]);
        default:
            return 0;
    }
}

// Variance of a box
static double variance(Box *box, WuData *data) {
    long long dr = volume(box, data->moments_r);
    long long dg = volume(box, data->moments_g);
    long long db = volume(box, data->moments_b);
    double xx = volume_double(box, data->moments);
    
    long long weight = volume(box, data->weights);
    if (weight == 0) return 0.0;
    
    double variance_value = xx - ((double)dr * dr + (double)dg * dg + (double)db * db) / weight;
    return variance_value;
}

// Maximize variance for box splitting
static double maximize(Box *box, int direction, int first, int last, int *cut,
                       long long whole_w, long long whole_r, long long whole_g, long long whole_b, WuData *data) {
    long long base_w = bottom(box, direction, data->weights);
    long long base_r = bottom(box, direction, data->moments_r);
    long long base_g = bottom(box, direction, data->moments_g);
    long long base_b = bottom(box, direction, data->moments_b);
    
    double max_variance = 0.0;
    *cut = -1;
    
    for (int i = first; i < last; i++) {
        long long half_w = base_w + top(box, direction, i, data->weights);
        long long half_r = base_r + top(box, direction, i, data->moments_r);
        long long half_g = base_g + top(box, direction, i, data->moments_g);
        long long half_b = base_b + top(box, direction, i, data->moments_b);
        
        if (half_w == 0) continue;
        
        double temp = ((double)half_r * half_r + 
                      (double)half_g * half_g + 
                      (double)half_b * half_b) / half_w;
        
        half_w = whole_w - half_w;
        if (half_w == 0) continue;
        
        half_r = whole_r - half_r;
        half_g = whole_g - half_g;
        half_b = whole_b - half_b;
        
        temp += ((double)half_r * half_r + 
                (double)half_g * half_g + 
                (double)half_b * half_b) / half_w;
        
        if (temp > max_variance) {
            max_variance = temp;
            *cut = i;
        }
    }
    
    return max_variance;
}

// Cut box
static int cut(Box *set1, Box *set2, WuData *data) {
    long long whole_w = volume(set1, data->weights);
    long long whole_r = volume(set1, data->moments_r);
    long long whole_g = volume(set1, data->moments_g);
    long long whole_b = volume(set1, data->moments_b);
    
    int cutr, cutg, cutb;
    double maxr = maximize(set1, 0, set1->r0 + 1, set1->r1, &cutr,
                          whole_w, whole_r, whole_g, whole_b, data);
    double maxg = maximize(set1, 1, set1->g0 + 1, set1->g1, &cutg,
                          whole_w, whole_r, whole_g, whole_b, data);
    double maxb = maximize(set1, 2, set1->b0 + 1, set1->b1, &cutb,
                          whole_w, whole_r, whole_g, whole_b, data);
    
    int direction;
    if (maxr >= maxg && maxr >= maxb) {
        direction = 0;
        if (cutr < 0) return 0;
    } else if (maxg >= maxr && maxg >= maxb) {
        direction = 1;
        if (cutg < 0) return 0;
    } else {
        direction = 2;
        if (cutb < 0) return 0;
    }
    
    set2->r1 = set1->r1;
    set2->g1 = set1->g1;
    set2->b1 = set1->b1;
    
    switch (direction) {
        case 0:
            set2->r0 = set1->r1 = cutr;
            set2->g0 = set1->g0;
            set2->b0 = set1->b0;
            break;
        case 1:
            set2->g0 = set1->g1 = cutg;
            set2->r0 = set1->r0;
            set2->b0 = set1->b0;
            break;
        case 2:
            set2->b0 = set1->b1 = cutb;
            set2->r0 = set1->r0;
            set2->g0 = set1->g0;
            break;
    }
    
    set1->volume = (set1->r1 - set1->r0) * (set1->g1 - set1->g0) * (set1->b1 - set1->b0);
    set2->volume = (set2->r1 - set2->r0) * (set2->g1 - set2->g0) * (set2->b1 - set2->b0);
    
    return 1;
}

// Construct histogram
static void construct_histogram(RGB *pixels, int pixel_count, WuData *data) {
    memset(data,  0, sizeof(WuData));
    
    for (int i = 0; i < pixel_count; i++) {
        RGB rgb = pixels[i];
        int index = get_index(rgb.r, rgb.g, rgb.b);
        
        data->weights[index]++;
        data->moments_r[index] += rgb.r;
        data->moments_g[index] += rgb.g;
        data->moments_b[index] += rgb.b;
        data->moments[index] += (double)rgb.r * rgb.r + 
                                 (double)rgb.g * rgb.g + 
                                 (double)rgb.b * rgb.b;
    }
}

// Compute cumulative moments
static void compute_moments(WuData *data) {
    for (int r = 1; r < INDEX_COUNT; r++) {
        long long area[INDEX_COUNT] = {0};
        long long area_r[INDEX_COUNT] = {0};
        long long area_g[INDEX_COUNT] = {0};
        long long area_b[INDEX_COUNT] = {0};
        double area2[INDEX_COUNT] = {0};
        
        for (int g = 1; g < INDEX_COUNT; g++) {
            long long line = 0, line_r = 0, line_g = 0, line_b = 0;
            double line2 = 0.0;
            
            for (int b = 1; b < INDEX_COUNT; b++) {
                int idx = get_index(r, g, b);
                line += data->weights[idx];
                line_r += data->moments_r[idx];
                line_g += data->moments_g[idx];
                line_b += data->moments_b[idx];
                line2 += data->moments[idx];
                
                area[b] += line;
                area_r[b] += line_r;
                area_g[b] += line_g;
                area_b[b] += line_b;
                area2[b] += line2;
                
                int prev_idx = get_index(r - 1, g, b);
                data->weights[idx] = data->weights[prev_idx] + area[b];
                data->moments_r[idx] = data->moments_r[prev_idx] + area_r[b];
                data->moments_g[idx] = data->moments_g[prev_idx] + area_g[b];
                data->moments_b[idx] = data->moments_b[prev_idx] + area_b[b];
                data->moments[idx] = data->moments[prev_idx] + area2[b];
            }
        }
    }
}

// Wu quantization
static QuantizeResult quantize_wu(RGB *pixels, int pixel_count, int max_colors) {
    WuData *data = malloc(sizeof(WuData));
    
    // Build histogram and moments
    construct_histogram(pixels, pixel_count, data);
    compute_moments(data);
    
    // Create initial cube
    Box cubes[256];
    cubes[0].r0 = cubes[0].g0 = cubes[0].b0 = 0;
    cubes[0].r1 = cubes[0].g1 = cubes[0].b1 = INDEX_COUNT - 1;
    
    int next_cube = 1;
    
    // Cut cubes iteratively
    for (int i = 1; i < max_colors; i++) {
        if (next_cube >= max_colors) break;
        
        // Find box with maximum variance
        double max_var = variance(&cubes[0], data);
        int max_idx = 0;
        
        for (int j = 1; j < next_cube; j++) {
            double var = variance(&cubes[j], data);
            if (var > max_var) {
                max_var = var;
                max_idx = j;
            }
        }
        
        if (max_var <= 0.0) break;
        
        // Cut the box
        if (cut(&cubes[max_idx], &cubes[next_cube], data)) {
            next_cube++;
        } else {
            break;
        }
    }
    
    // Extract colors from cubes
    QuantizeResult result;
    result.count = next_cube;
    result.colors = malloc(next_cube * sizeof(RGB));
    result.populations = malloc(next_cube * sizeof(int));
    
    for (int i = 0; i < next_cube; i++) {
        long long weight = volume(&cubes[i], data->weights);
        if (weight > 0) {
            long long r = volume(&cubes[i], data->moments_r) / weight;
            long long g = volume(&cubes[i], data->moments_g) / weight;
            long long b = volume(&cubes[i], data->moments_b) / weight;
            
            result.colors[i].r = r & 0xFF;
            result.colors[i].g = g & 0xFF;
            result.colors[i].b = b & 0xFF;
            result.populations[i] = weight;
        }
    }
    
    free(data);
    return result;
}

// Main quantization (Wu only for now, Wsmeans TODO)
QuantizeResult quantize_celebi(RGB *pixels, int pixel_count, int max_colors) {
    if (max_colors > 256) max_colors = 256;
    if (max_colors < 1) max_colors = 128;
    
    return quantize_wu(pixels, pixel_count, max_colors);
}

void quantize_result_free(QuantizeResult *result) {
    if (result) {
        free(result->colors);
        free(result->populations);
        result->colors = NULL;
        result->populations = NULL;
        result->count = 0;
    }
}
