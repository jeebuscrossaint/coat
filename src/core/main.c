#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pwd.h>

#include "yaml.h"
#include "schemes.h"
#include "tinted_parser.h"
#include "schemes_list.h"

// Application modules
#include "avizo.h"
#include "bat.h"
#include "bemenu.h"
#include "btop.h"
#include "cava.h"
#include "dunst.h"
#include "firefox.h"
#include "fish.h"
#include "gtk.h"
#include "helix.h"
#include "i3.h"
#include "kitty.h"
#include "mangowc.h"
#include "rofi.h"
#include "sway.h"
#include "swaylock.h"
#include "tty.h"
#include "vesktop.h"
#include "vscode.h"
#include "xresources.h"
#include "yazi.h"
#include "zathura.h"

// Application module definition
typedef struct {
    const char *name;
    const char *aliases[4];  // Additional names this module can match
    int (*apply_fn)(Base16Scheme *, FontConfig *, OpacityConfig *);
    int needs_font;
    int needs_opacity;
} AppModule;

// Helper functions for module application
static int apply_simple(Base16Scheme *s, FontConfig *f, OpacityConfig *o, int (*fn)(Base16Scheme *)) {
    (void)f; (void)o;
    return fn(s);
}

static int apply_font(Base16Scheme *s, FontConfig *f, OpacityConfig *o, int (*fn)(Base16Scheme *, FontConfig *)) {
    (void)o;
    return fn(s, f);
}

static int apply_opacity(Base16Scheme *s, FontConfig *f, OpacityConfig *o, int (*fn)(Base16Scheme *, OpacityConfig *)) {
    (void)f;
    return fn(s, o);
}

static int apply_font_opacity(Base16Scheme *s, FontConfig *f, OpacityConfig *o, int (*fn)(Base16Scheme *, FontConfig *, OpacityConfig *)) {
    return fn(s, f, o);
}

// Application modules table
static const AppModule app_modules[] = {
    {"avizo", {NULL}, (void*)avizo_apply_theme, 0, 0},
    {"bat", {NULL}, (void*)bat_apply_theme, 0, 0},
    {"bemenu", {NULL}, (void*)bemenu_apply_theme, 0, 0},
    {"btop", {NULL}, (void*)btop_apply_theme, 0, 0},
    {"cava", {NULL}, (void*)cava_apply_theme, 0, 0},
    {"dunst", {NULL}, (void*)dunst_apply_theme, 1, 0},
    {"firefox", {NULL}, (void*)firefox_apply_theme, 1, 0},
    {"fish", {NULL}, (void*)fish_apply_theme, 0, 0},
    {"gtk", {NULL}, (void*)gtk_apply_theme, 1, 0},
    {"helix", {NULL}, (void*)helix_apply_theme, 0, 0},
    {"i3", {NULL}, (void*)i3_apply_theme, 1, 0},
    {"kitty", {NULL}, (void*)kitty_apply_theme, 1, 1},
    {"mangowc", {NULL}, (void*)mangowc_apply_theme, 1, 0},
    {"rofi", {NULL}, (void*)rofi_apply_theme, 1, 0},
    {"sway", {NULL}, (void*)sway_apply_theme, 1, 0},
    {"swaylock", {NULL}, (void*)swaylock_apply_theme, 0, 1},
    {"tty", {"console", NULL}, (void*)tty_apply_theme, 0, 0},
    {"vesktop", {"vencord", "discord", NULL}, (void*)vesktop_apply_theme, 1, 0},
    {"vscode", {NULL}, (void*)vscode_apply_theme, 1, 0},
    {"xresources", {NULL}, (void*)xresources_apply_theme, 1, 0},
    {"yazi", {NULL}, (void*)yazi_apply_theme, 0, 0},
    {"zathura", {NULL}, (void*)zathura_apply_theme, 1, 0},
};

static const int num_app_modules = sizeof(app_modules) / sizeof(app_modules[0]);

static const AppModule* find_app_module(const char *name) {
    for (int i = 0; i < num_app_modules; i++) {
        if (strcmp(app_modules[i].name, name) == 0) {
            return &app_modules[i];
        }
        // Check aliases
        for (int j = 0; j < 4 && app_modules[i].aliases[j]; j++) {
            if (strcmp(app_modules[i].aliases[j], name) == 0) {
                return &app_modules[i];
            }
        }
    }
    return NULL;
}

static int apply_module(const AppModule *mod, Base16Scheme *scheme, FontConfig *font, OpacityConfig *opacity) {
    if (!mod->needs_font && !mod->needs_opacity) {
        return apply_simple(scheme, font, opacity, (void*)mod->apply_fn);
    } else if (mod->needs_font && !mod->needs_opacity) {
        return apply_font(scheme, font, opacity, (void*)mod->apply_fn);
    } else if (!mod->needs_font && mod->needs_opacity) {
        return apply_opacity(scheme, font, opacity, (void*)mod->apply_fn);
    } else {
        return apply_font_opacity(scheme, font, opacity, mod->apply_fn);
    }
}

static char* get_config_dir(void) {
    const char *home = getenv("HOME");
    if (!home) {
        struct passwd *pw = getpwuid(getuid());
        if (pw) {
            home = pw->pw_dir;
        } else {
            fprintf(stderr, "Could not determine home directory\n");
            return NULL;
        }
    }
    
    char *config_dir = malloc(1024);
    if (!config_dir) {
        return NULL;
    }
    
    snprintf(config_dir, 1024, "%s/.config/coat", home);
    return config_dir;
}

static void print_usage(const char *prog_name) {
    printf("Usage: %s [COMMAND] [OPTIONS]\n\n", prog_name);
    printf("Commands:\n");
    printf("  update              Update the schemes repository\n");
    printf("  clone               Clone the schemes repository\n");
    printf("  list [OPTIONS]      List available color schemes\n");
    printf("  search <term>       Search for schemes by name/author\n");
    printf("  apply [app]         Apply current scheme to all apps or specific app\n");
    printf("  docs <app>          Show setup instructions for specific app\n");
    printf("  help                Show this help message\n");
    printf("\n");
    printf("List Options:\n");
    printf("  --dark              Show only dark variant schemes\n");
    printf("  --light             Show only light variant schemes\n");
    printf("  --no-preview        Disable color preview (enabled by default)\n");
    printf("\n");
    printf("Apply Examples:\n");
    printf("  %s apply            Apply to all enabled apps in config\n", prog_name);
    printf("  %s apply kitty      Apply only to kitty\n", prog_name);
    printf("  %s apply fish       Apply only to fish shell\n", prog_name);
    printf("\n");
    printf("If no command is provided, displays current configuration.\n");
}

int main(int argc, char *argv[]) {
    printf("coat - Configuration Tool\n\n");
    
    char *config_dir = get_config_dir();
    if (!config_dir) {
        return 1;
    }
    
    // Handle command-line arguments
    if (argc > 1) {
        if (strcmp(argv[1], "update") == 0) {
            int result = schemes_update(config_dir);
            free(config_dir);
            return result == 0 ? 0 : 1;
        } else if (strcmp(argv[1], "clone") == 0) {
            int result = schemes_clone(config_dir);
            free(config_dir);
            return result == 0 ? 0 : 1;
        } else if (strcmp(argv[1], "docs") == 0) {
            if (argc < 3) {
                fprintf(stderr, "Error: docs command requires an app name\\n\\n");
                fprintf(stderr, "Usage: %s docs <app>\\n\\n", argv[0]);
                fprintf(stderr, "Examples:\\n");
                fprintf(stderr, "  %s docs fish\\n", argv[0]);
                fprintf(stderr, "  %s docs kitty\\n", argv[0]);
                free(config_dir);
                return 1;
            }
            
            const char *app = argv[2];
            const AppModule *mod = find_app_module(app);
            
            if (!mod) {
                fprintf(stderr, "Error: No module found for '%s'\n\n", app);
                fprintf(stderr, "Available modules:\n");
                for (int i = 0; i < num_app_modules; i++) {
                    fprintf(stderr, "  - %s", app_modules[i].name);
                    if (app_modules[i].aliases[0]) {
                        fprintf(stderr, " (aliases: ");
                        for (int j = 0; j < 4 && app_modules[i].aliases[j]; j++) {
                            if (j > 0) fprintf(stderr, ", ");
                            fprintf(stderr, "%s", app_modules[i].aliases[j]);
                        }
                        fprintf(stderr, ")");
                    }
                    fprintf(stderr, "\n");
                }
                free(config_dir);
                return 1;
            }
            
            // Show documentation for the app
            printf("=== %s Setup Instructions ===\n\n", mod->name);
            
            // App-specific documentation
            if (strcmp(mod->name, "fish") == 0) {
                printf("To activate the fish theme:\n\n");
                printf("  fish_config theme save coat\n\n");
                printf("Or add to ~/.config/fish/config.fish:\n\n");
                printf("  fish_config theme choose coat\n");
            } else if (strcmp(mod->name, "kitty") == 0) {
                printf("Add to ~/.config/kitty/kitty.conf:\n\n");
                printf("  include coat-theme.conf\n\n");
                printf("Then reload: Ctrl+Shift+F5\n");
            } else if (strcmp(mod->name, "helix") == 0) {
                printf("Add to ~/.config/helix/config.toml:\n\n");
                printf("  theme = \"coat\"\n\n");
                printf("Then restart helix or run :config-reload\n");
            } else if (strcmp(mod->name, "i3") == 0) {
                printf("Add to ~/.config/i3/config:\n\n");
                printf("  include coat-theme.conf\n\n");
                printf("Then reload: $mod+Shift+r\n");
            } else if (strcmp(mod->name, "rofi") == 0) {
                printf("Add to ~/.config/rofi/config.rasi:\n\n");
                printf("  @theme \"coat\"\n\n");
                printf("Or test with: rofi -show drun -theme coat\n");
            } else if (strcmp(mod->name, "bat") == 0) {
                printf("Add to ~/.config/bat/config:\n\n");
                printf("  --theme=\"coat\"\n\n");
                printf("Or use temporarily: bat --theme=coat <file>\n");
            } else if (strcmp(mod->name, "sway") == 0) {
                printf("Add to ~/.config/sway/config:\n\n");
                printf("  include ~/.config/sway/coat-theme\n\n");
                printf("IMPORTANT: Remove any 'bar { }' blocks!\n");
                printf("The coat-theme includes a complete bar configuration.\n\n");
                printf("Then reload: swaymsg reload\n");
            } else if (strcmp(mod->name, "vscode") == 0) {
                printf("The theme is automatically activated.\n\n");
                printf("If it doesn't appear, reload VSCode:\n");
                printf("  Ctrl+Shift+P → Reload Window\n");
            } else if (strcmp(mod->name, "gtk") == 0) {
                printf("Theme is applied via gsettings automatically.\n\n");
                printf("Ensure 'adw-gtk3' and 'adw-gtk3-dark' are installed.\n");
                printf("Some apps may require a restart.\n");
            } else if (strcmp(mod->name, "firefox") == 0) {
                printf("Files created:\n");
                printf("  - userChrome.css (UI theme)\n");
                printf("  - userContent.css (web content theme)\n\n");
                printf("To enable, in about:config set:\n");
                printf("  toolkit.legacyUserProfileCustomizations.stylesheets = true\n\n");
                printf("Then restart Firefox.\n");
            } else if (strcmp(mod->name, "swaylock") == 0) {
                printf("Theme is applied automatically.\n\n");
                printf("Test with: swaylock\n\n");
                printf("To bind to a key, add to Sway config:\n");
                printf("  bindsym $mod+l exec swaylock\n");
            } else if (strcmp(mod->name, "dunst") == 0) {
                printf("Dunst is restarted automatically to apply the theme.\n\n");
                printf("If it doesn't restart, run manually:\n");
                printf("  killall dunst && dunst &\n");
            } else if (strcmp(mod->name, "btop") == 0) {
                printf("To activate:\n\n");
                printf("1. Open btop\n");
                printf("2. Press ESC to open menu\n");
                printf("3. Navigate to 'Options' > 'Color theme'\n");
                printf("4. Select 'coat'\n\n");
                printf("Or set in ~/.config/btop/btop.conf:\n");
                printf("  color_theme = \"coat\"\n");
            } else if (strcmp(mod->name, "yazi") == 0) {
                printf("Restart yazi to see the changes.\n");
            } else if (strcmp(mod->name, "zathura") == 0) {
                printf("Restart zathura or open a new PDF to see the changes.\n");
            } else if (strcmp(mod->name, "vesktop") == 0) {
                printf("Enable the theme in Discord/Vesktop:\n\n");
                printf("  Settings → Vencord → Themes → coat.theme.css\n\n");
                printf("Or restart if auto-loading is enabled.\n");
            } else if (strcmp(mod->name, "tty") == 0) {
                printf("To make permanent, choose one:\n\n");
                printf("1. Add to ~/.bashrc or ~/.zshrc:\n");
                printf("   ~/.config/coat/tty-theme.sh\n\n");
                printf("2. Copy to /etc/profile.d/ (requires root):\n");
                printf("   sudo cp ~/.config/coat/tty-theme.sh /etc/profile.d/\n\n");
                printf("3. Create systemd service (see USAGE.md)\n");
            } else if (strcmp(mod->name, "xresources") == 0) {
                printf("Theme is automatically merged.\n\n");
                printf("To make permanent, add to ~/.xinitrc or ~/.xprofile:\n");
                printf("  xrdb -merge ~/.Xresources\n");
            } else {
                printf("The %s theme has been applied.\n", mod->name);
                printf("See USAGE.md for detailed information.\n");
            }
            
            printf("\n");
            free(config_dir);
            return 0;
        } else if (strcmp(argv[1], "list") == 0) {
            // Ensure schemes exist
            if (!schemes_exists(config_dir)) {
                printf("Schemes repository not found. Cloning...\n");
                if (schemes_clone(config_dir) != 0) {
                    free(config_dir);
                    return 1;
                }
                printf("\n");
            }
            
            // Parse list options
            const char *variant_filter = NULL;
            int show_preview = 1;
            
            for (int i = 2; i < argc; i++) {
                if (strcmp(argv[i], "--dark") == 0) {
                    variant_filter = "dark";
                } else if (strcmp(argv[i], "--light") == 0) {
                    variant_filter = "light";
                } else if (strcmp(argv[i], "--no-preview") == 0) {
                    show_preview = 0;
                }
            }
            
            char *schemes_path = schemes_get_path(config_dir);
            if (!schemes_path) {
                fprintf(stderr, "Failed to get schemes path\n");
                free(config_dir);
                return 1;
            }
            
            int result = schemes_list_all(schemes_path, variant_filter, NULL, show_preview);
            
            free(schemes_path);
            free(config_dir);
            return result == 0 ? 0 : 1;
        } else if (strcmp(argv[1], "search") == 0) {
            if (argc < 3) {
                fprintf(stderr, "Error: search command requires a search term\n\n");
                print_usage(argv[0]);
                free(config_dir);
                return 1;
            }
            
            // Ensure schemes exist
            if (!schemes_exists(config_dir)) {
                printf("Schemes repository not found. Cloning...\n");
                if (schemes_clone(config_dir) != 0) {
                    free(config_dir);
                    return 1;
                }
                printf("\n");
            }
            
            // Parse search options
            const char *search_term = argv[2];
            const char *variant_filter = NULL;
            int show_preview = 1;
            
            for (int i = 3; i < argc; i++) {
                if (strcmp(argv[i], "--dark") == 0) {
                    variant_filter = "dark";
                } else if (strcmp(argv[i], "--light") == 0) {
                    variant_filter = "light";
                } else if (strcmp(argv[i], "--no-preview") == 0) {
                    show_preview = 0;
                }
            }
            
            char *schemes_path = schemes_get_path(config_dir);
            if (!schemes_path) {
                fprintf(stderr, "Failed to get schemes path\n");
                free(config_dir);
                return 1;
            }
            
            int result = schemes_list_all(schemes_path, variant_filter, search_term, show_preview);
            
            free(schemes_path);
            free(config_dir);
            return result == 0 ? 0 : 1;
        } else if (strcmp(argv[1], "apply") == 0) {
            // Check if specific app was provided
            const char *specific_app = (argc > 2) ? argv[2] : NULL;
            
            // If specific app provided, validate it exists first
            if (specific_app) {
                const AppModule *test_mod = find_app_module(specific_app);
                if (!test_mod) {
                    fprintf(stderr, "Error: No module found for '%s'\n\n", specific_app);
                    fprintf(stderr, "Available modules:\n");
                    for (int i = 0; i < num_app_modules; i++) {
                        fprintf(stderr, "  - %s", app_modules[i].name);
                        if (app_modules[i].aliases[0]) {
                            fprintf(stderr, " (aliases: ");
                            for (int j = 0; j < 4 && app_modules[i].aliases[j]; j++) {
                                if (j > 0) fprintf(stderr, ", ");
                                fprintf(stderr, "%s", app_modules[i].aliases[j]);
                            }
                            fprintf(stderr, ")");
                        }
                        fprintf(stderr, "\n");
                    }
                    free(config_dir);
                    return 1;
                }
            }
            
            // Ensure schemes exist
            if (!schemes_exists(config_dir)) {
                printf("Schemes repository not found. Cloning...\n");
                if (schemes_clone(config_dir) != 0) {
                    free(config_dir);
                    return 1;
                }
                printf("\n");
            }
            
            // Load config to get scheme and enabled apps
            CoatConfig *config = coat_config_new();
            if (!config) {
                fprintf(stderr, "Failed to allocate config structure\n");
                free(config_dir);
                return 1;
            }
            
            if (coat_config_load_default(config) != 0) {
                fprintf(stderr, "Failed to load config file\n");
                coat_config_free(config);
                free(config_dir);
                return 1;
            }
            
            if (!config->scheme[0]) {
                fprintf(stderr, "No scheme specified in config file\n");
                coat_config_free(config);
                free(config_dir);
                return 1;
            }
            
            // Load the scheme
            Base16Scheme *scheme = base16_scheme_new();
            if (!scheme) {
                fprintf(stderr, "Failed to allocate scheme structure\n");
                coat_config_free(config);
                free(config_dir);
                return 1;
            }
            
            char *schemes_path = schemes_get_path(config_dir);
            if (!schemes_path) {
                fprintf(stderr, "Failed to get schemes path\n");
                base16_scheme_free(scheme);
                coat_config_free(config);
                free(config_dir);
                return 1;
            }
            
            printf("Loading scheme: %s\n", config->scheme);
            if (base16_scheme_load_by_name(scheme, config->scheme, schemes_path, config->prefer_base24) != 0) {
                fprintf(stderr, "Failed to load scheme: %s\n", config->scheme);
                free(schemes_path);
                base16_scheme_free(scheme);
                coat_config_free(config);
                free(config_dir);
                return 1;
            }
            
            printf("Applying scheme: %s\n", scheme->name);
            
            // If specific app is provided, only apply to that app
            if (specific_app) {
                printf("Target: %s only\n\n", specific_app);
                const AppModule *mod = find_app_module(specific_app);
                
                printf("Applying to %s...\n", mod->name);
                if (apply_module(mod, scheme, &config->font, &config->opacity) == 0) {
                    printf("\n✓ Successfully applied scheme to %s!\n", mod->name);
                } else {
                    fprintf(stderr, "\n✗ Failed to apply scheme to %s\n", mod->name);
                }
            } else {
                // Apply to all enabled applications
                printf("\n");
                int applied = 0;
                for (int i = 0; i < config->enabled_count; i++) {
                    const char *app = config->enabled[i];
                    const AppModule *mod = find_app_module(app);
                    
                    if (mod) {
                        printf("Applying to %s...\n", mod->name);
                        if (apply_module(mod, scheme, &config->font, &config->opacity) == 0) {
                            applied++;
                        }
                        printf("\n");
                    } else {
                        printf("Warning: No module for '%s'\n\n", app);
                    }
                }
                
                if (applied > 0) {
                    printf("Successfully applied scheme to %d application%s!\n", 
                           applied, applied == 1 ? "" : "s");
                } else {
                    printf("No applications were configured.\n");
                }
            }
            
            free(schemes_path);
            base16_scheme_free(scheme);
            coat_config_free(config);
            free(config_dir);
            return 0;
        } else if (strcmp(argv[1], "help") == 0 || strcmp(argv[1], "--help") == 0) {
            print_usage(argv[0]);
            free(config_dir);
            return 0;
        } else {
            fprintf(stderr, "Unknown command: %s\n\n", argv[1]);
            print_usage(argv[0]);
            free(config_dir);
            return 1;
        }
    }
    
    // Check if schemes repository exists, if not, clone it
    if (!schemes_exists(config_dir)) {
        printf("Schemes repository not found.\n");
        if (schemes_clone(config_dir) != 0) {
            free(config_dir);
            return 1;
        }
        printf("\n");
    }

    // Create config structure
    CoatConfig *config = coat_config_new();
    if (!config) {
        fprintf(stderr, "Failed to allocate config structure\n");
        return 1;
    }

    // Load config from default location
    printf("Loading config from ~/.config/coat/coat.yaml...\n");
    if (coat_config_load_default(config) != 0) {
        fprintf(stderr, "Failed to load config file\n");
        coat_config_free(config);
        return 1;
    }

    // Display loaded configuration
    printf("\nLoaded Configuration:\n");
    printf("====================\n\n");

    printf("Scheme: %s\n\n", config->scheme[0] ? config->scheme : "(not set)");

    printf("Enabled items (%d):\n", config->enabled_count);
    for (int i = 0; i < config->enabled_count; i++) {
        printf("  - %s\n", config->enabled[i]);
    }
    printf("\n");

    printf("Font Configuration:\n");
    printf("  emoji:      %s\n", config->font.emoji[0] ? config->font.emoji : "(not set)");
    printf("  monospace:  %s\n", config->font.monospace[0] ? config->font.monospace : "(not set)");
    printf("  sansserif:  %s\n", config->font.sansserif[0] ? config->font.sansserif : "(not set)");
    printf("  serif:      %s\n", config->font.serif[0] ? config->font.serif : "(not set)");
    printf("\n");

    // Load and display Base16 scheme if specified
    if (config->scheme[0]) {
        Base16Scheme *scheme = base16_scheme_new();
        if (!scheme) {
            fprintf(stderr, "Failed to allocate scheme structure\n");
            coat_config_free(config);
            free(config_dir);
            return 1;
        }

        char *schemes_path = schemes_get_path(config_dir);
        if (!schemes_path) {
            fprintf(stderr, "Failed to get schemes path\n");
            base16_scheme_free(scheme);
            coat_config_free(config);
            free(config_dir);
            return 1;
        }

        printf("Loading scheme: %s\n", config->scheme);
        if (base16_scheme_load_by_name(scheme, config->scheme, schemes_path, config->prefer_base24) == 0) {
            printf("\nBase16 Color Scheme:\n");
            printf("====================\n");
            printf("System:      %s\n", scheme->system);
            printf("Name:        %s\n", scheme->name);
            printf("Slug:        %s\n", scheme->slug);
            printf("Author:      %s\n", scheme->author);
            printf("Variant:     %s\n", scheme->variant);
            if (scheme->description[0]) {
                printf("Description: %s\n", scheme->description);
            }
            printf("\nPalette:\n");
            printf("  base00: %s  (Default Background)\n", scheme->base00);
            printf("  base01: %s  (Lighter Background)\n", scheme->base01);
            printf("  base02: %s  (Selection Background)\n", scheme->base02);
            printf("  base03: %s  (Comments)\n", scheme->base03);
            printf("  base04: %s  (Dark Foreground)\n", scheme->base04);
            printf("  base05: %s  (Default Foreground)\n", scheme->base05);
            printf("  base06: %s  (Light Foreground)\n", scheme->base06);
            printf("  base07: %s  (Light Background)\n", scheme->base07);
            printf("  base08: %s  (Variables)\n", scheme->base08);
            printf("  base09: %s  (Integers)\n", scheme->base09);
            printf("  base0A: %s  (Classes)\n", scheme->base0A);
            printf("  base0B: %s  (Strings)\n", scheme->base0B);
            printf("  base0C: %s  (Support)\n", scheme->base0C);
            printf("  base0D: %s  (Functions)\n", scheme->base0D);
            printf("  base0E: %s  (Keywords)\n", scheme->base0E);
            printf("  base0F: %s  (Deprecated)\n", scheme->base0F);
        } else {
            fprintf(stderr, "Failed to load scheme: %s\n", config->scheme);
            fprintf(stderr, "Make sure the scheme file exists at: %s/%s.yaml\n", schemes_path, config->scheme);
        }

        free(schemes_path);
        base16_scheme_free(scheme);
    }

    // Clean up
    coat_config_free(config);
    free(config_dir);

    return 0;
}