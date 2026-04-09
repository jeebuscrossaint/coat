#ifndef COAT_UTIL_H
#define COAT_UTIL_H

#include <stdlib.h>
#include <pwd.h>
#include <unistd.h>

/*
 * Return the user's home directory.
 * Falls back to getpwuid() when $HOME is unset (cron, sudo, etc.).
 */
static inline const char *get_home(void) {
    const char *home = getenv("HOME");
    if (!home || !home[0]) {
        const struct passwd *pw = getpwuid(getuid());
        if (pw) home = pw->pw_dir;
    }
    return home;
}

/*
 * Strip a leading '#' from a hex color string.
 */
static inline const char *strip_hash(const char *color) {
    return (color && color[0] == '#') ? color + 1 : color;
}

#endif /* COAT_UTIL_H */
