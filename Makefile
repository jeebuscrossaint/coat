# Makefile for coat - Configuration Tool
# Created by amarnath on 1/19/26

# Auto-detect compiler: prefer cc (system default), fall back to gcc then clang
CC ?= $(shell command -v cc 2>/dev/null || command -v gcc 2>/dev/null || command -v clang 2>/dev/null || echo gcc)

CFLAGS = -Wall -Wextra -std=c11 -O2 -Iinclude

# pkg-config for yaml and json-c, with /usr/local fallback for OpenBSD
PKG_CONFIG ?= pkg-config
YAML_CFLAGS  := $(shell $(PKG_CONFIG) --cflags yaml-0.1 2>/dev/null || echo -I/usr/local/include)
YAML_LIBS    := $(shell $(PKG_CONFIG) --libs   yaml-0.1 2>/dev/null || echo -L/usr/local/lib -lyaml)
JSONC_CFLAGS := $(shell $(PKG_CONFIG) --cflags json-c   2>/dev/null || echo -I/usr/local/include)
JSONC_LIBS   := $(shell $(PKG_CONFIG) --libs   json-c   2>/dev/null || echo -L/usr/local/lib -ljson-c)

CFLAGS  += $(YAML_CFLAGS) $(JSONC_CFLAGS)
LDFLAGS  = $(YAML_LIBS) $(JSONC_LIBS)

BUILD_DIR = build
TARGET = coat
SRC_DIR = src
INCLUDE_DIR = include

# Core source files
CORE_SRCS = $(SRC_DIR)/core/main.c $(SRC_DIR)/core/yaml.c $(SRC_DIR)/core/tinted_parser.c \
            $(SRC_DIR)/core/schemes.c $(SRC_DIR)/core/schemes_list.c

# Application module source files (alphabetically ordered)
APP_SRCS = $(SRC_DIR)/modules/bat.c $(SRC_DIR)/modules/btop.c \
           $(SRC_DIR)/modules/dunst.c $(SRC_DIR)/modules/fish.c $(SRC_DIR)/modules/foot.c \
           $(SRC_DIR)/modules/fuzzel.c $(SRC_DIR)/modules/gtk.c $(SRC_DIR)/modules/helix.c \
           $(SRC_DIR)/modules/i3.c $(SRC_DIR)/modules/kitty.c $(SRC_DIR)/modules/labwc.c \
           $(SRC_DIR)/modules/lf.c $(SRC_DIR)/modules/qt.c $(SRC_DIR)/modules/ranger.c \
           $(SRC_DIR)/modules/rofi.c $(SRC_DIR)/modules/sway.c $(SRC_DIR)/modules/swaylock.c \
           $(SRC_DIR)/modules/vesktop.c $(SRC_DIR)/modules/vscode.c $(SRC_DIR)/modules/waybar.c \
           $(SRC_DIR)/modules/xresources.c $(SRC_DIR)/modules/zathura.c

SRCS = $(CORE_SRCS) $(APP_SRCS)
OBJS = $(patsubst $(SRC_DIR)/%.c,$(BUILD_DIR)/%.o,$(SRCS))

# Header files
HEADERS = $(wildcard $(INCLUDE_DIR)/*.h)

.PHONY: all clean install uninstall

all: $(BUILD_DIR) $(TARGET)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)/core $(BUILD_DIR)/modules

$(TARGET): $(OBJS)
	$(CC) $(OBJS) -o $(TARGET) $(LDFLAGS)

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -rf $(BUILD_DIR)
	rm -f $(TARGET)

install: $(TARGET)
	install -Dm755 $(TARGET) $(DESTDIR)/usr/local/bin/$(TARGET)

uninstall:
	rm -f $(DESTDIR)/usr/local/bin/$(TARGET)
