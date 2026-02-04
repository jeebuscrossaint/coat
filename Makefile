# Makefile for coat - Configuration Tool
# Created by amarnath on 1/19/26

CC = gcc
CFLAGS = -Wall -Wextra -std=c11 -O2 -Iinclude
LDFLAGS = $(shell pkg-config --libs yaml-0.1 json-c)
CFLAGS += $(shell pkg-config --cflags yaml-0.1 json-c)

BUILD_DIR = build
TARGET = coat
SRC_DIR = src
INCLUDE_DIR = include

# Core source files
CORE_SRCS = $(SRC_DIR)/core/main.c $(SRC_DIR)/core/yaml.c $(SRC_DIR)/core/tinted_parser.c \
            $(SRC_DIR)/core/schemes.c $(SRC_DIR)/core/schemes_list.c

# Application module source files (alphabetically ordered)
APP_SRCS = $(SRC_DIR)/modules/avizo.c $(SRC_DIR)/modules/bat.c $(SRC_DIR)/modules/bemenu.c \
           $(SRC_DIR)/modules/btop.c $(SRC_DIR)/modules/cava.c $(SRC_DIR)/modules/dunst.c \
           $(SRC_DIR)/modules/firefox.c $(SRC_DIR)/modules/fish.c $(SRC_DIR)/modules/gtk.c \
           $(SRC_DIR)/modules/helix.c $(SRC_DIR)/modules/i3.c $(SRC_DIR)/modules/kitty.c \
           $(SRC_DIR)/modules/mangowc.c $(SRC_DIR)/modules/rofi.c $(SRC_DIR)/modules/sway.c \
           $(SRC_DIR)/modules/swaylock.c $(SRC_DIR)/modules/tty.c $(SRC_DIR)/modules/vesktop.c \
           $(SRC_DIR)/modules/vscode.c $(SRC_DIR)/modules/xresources.c $(SRC_DIR)/modules/yazi.c \
           $(SRC_DIR)/modules/zathura.c

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
