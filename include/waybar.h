#ifndef WAYBAR_H
#define WAYBAR_H

#include "tinted_parser.h"
#include "coat_yaml.h"

// Generate waybar CSS theme from a Base16 scheme
int waybar_generate_theme(const Base16Scheme *scheme, const char *output_path, const FontConfig *font);

// Apply waybar theme
int waybar_apply_theme(const Base16Scheme *scheme, const FontConfig *font);

#endif // WAYBAR_H
