//
// Created by amarnath on 1/19/26.
//

#ifndef COAT_SCHEMES_H
#define COAT_SCHEMES_H

#include <stdbool.h>

#define SCHEMES_REPO_URL "https://github.com/tinted-theming/schemes.git"
#define SCHEMES_DIR_NAME "schemes"

// Check if schemes repository exists
bool schemes_exists(const char *config_dir);

// Clone the schemes repository
int schemes_clone(const char *config_dir);

// Update the schemes repository
int schemes_update(const char *config_dir);

// Get the full path to the schemes directory
char* schemes_get_path(const char *config_dir);

#endif //COAT_SCHEMES_H
