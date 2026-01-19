# Makefile for coat
# Created by amarnath on 1/19/26

CC = gcc
CFLAGS = -Wall -Wextra -std=c11 -O2
LDFLAGS = $(shell pkg-config --libs yaml-0.1 json-c)
CFLAGS += $(shell pkg-config --cflags yaml-0.1 json-c)

BUILD_DIR = build
TARGET = coat
SRCS = main.c fish.c yaml.c tinted_parser.c schemes.c schemes_list.c kitty.c i3.c helix.c rofi.c bat.c tty.c avizo.c bemenu.c btop.c cava.c zathura.c yazi.c vscode.c gtk.c dunst.c
OBJS = $(addprefix $(BUILD_DIR)/, $(SRCS:.c=.o))
HEADERS = fish.h yaml.h tinted_parser.h schemes.h schemes_list.h kitty.h i3.h helix.h rofi.h bat.h tty.h avizo.h bemenu.h btop.h cava.h zathura.h yazi.h vscode.h gtk.h dunst.h

.PHONY: all clean install uninstall

all: $(BUILD_DIR) $(TARGET)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(TARGET): $(OBJS)
	$(CC) $(OBJS) -o $(TARGET) $(LDFLAGS)

$(BUILD_DIR)/%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -rf $(BUILD_DIR)
	rm -f $(TARGET)

install: $(TARGET)
	install -Dm755 $(TARGET) $(DESTDIR)/usr/local/bin/$(TARGET)

uninstall:
	rm -f $(DESTDIR)/usr/local/bin/$(TARGET)
