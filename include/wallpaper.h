#ifndef WALLPAPER_H
#define WALLPAPER_H

#include "tinted_parser.h"

// Extract colors from current wallpaper and generate a Base16 scheme
// Returns 0 on success, -1 on error
int wallpaper_extract_scheme(Base16Scheme *scheme, int apply_material_you);

// Helper: Get wallpaper path from swww
char* wallpaper_get_path(void);

#endif // WALLPAPER_H
