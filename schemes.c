//
// Created by amarnath on 1/19/26.
//

#include "schemes.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Check if schemes repository exists
bool schemes_exists(const char *config_dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/%s", config_dir, SCHEMES_DIR_NAME);
    
    struct stat st;
    if (stat(path, &st) == 0 && S_ISDIR(st.st_mode)) {
        // Check if .git directory exists
        char git_path[1024];
        snprintf(git_path, sizeof(git_path), "%s/.git", path);
        if (stat(git_path, &st) == 0 && S_ISDIR(st.st_mode)) {
            return true;
        }
    }
    return false;
}

// Clone the schemes repository
int schemes_clone(const char *config_dir) {
    char command[2048];
    
    printf("Cloning tinted-theming/schemes repository...\n");
    printf("This may take a moment...\n\n");
    
    snprintf(command, sizeof(command), 
             "git clone --depth 1 %s %s/%s 2>&1", 
             SCHEMES_REPO_URL, config_dir, SCHEMES_DIR_NAME);
    
    int result = system(command);
    
    if (result == 0) {
        printf("Successfully cloned schemes repository!\n");
        return 0;
    } else {
        fprintf(stderr, "Failed to clone schemes repository\n");
        return -1;
    }
}

// Update the schemes repository
int schemes_update(const char *config_dir) {
    char path[1024];
    snprintf(path, sizeof(path), "%s/%s", config_dir, SCHEMES_DIR_NAME);
    
    if (!schemes_exists(config_dir)) {
        fprintf(stderr, "Schemes repository not found. Cloning instead...\n");
        return schemes_clone(config_dir);
    }
    
    printf("Updating schemes repository...\n");
    
    char command[2048];
    snprintf(command, sizeof(command), 
             "cd %s && git pull 2>&1", path);
    
    int result = system(command);
    
    if (result == 0) {
        printf("Successfully updated schemes repository!\n");
        return 0;
    } else {
        fprintf(stderr, "Failed to update schemes repository\n");
        return -1;
    }
}

// Get the full path to the schemes directory
char* schemes_get_path(const char *config_dir) {
    char *path = malloc(1024);
    if (!path) {
        return NULL;
    }
    
    snprintf(path, 1024, "%s/%s/base16", config_dir, SCHEMES_DIR_NAME);
    return path;
}
