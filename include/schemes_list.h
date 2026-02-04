//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_SCHEMES_LIST_H
#define COAT_SCHEMES_LIST_H

#include "tinted_parser.h"

// List all available schemes
int schemes_list_all(const char *schemes_dir, const char *variant_filter, const char *search_term, int show_preview);

// Show interactive preview of a scheme
void schemes_show_preview(const Base16Scheme *scheme);

#endif //COAT_SCHEMES_LIST_H
