# Makefile для компиляции RuStore_ApkDownloader под различные платформы и архитектуры

# Добавляем ~/.cargo/bin в PATH (нужно для macOS и некоторых Linux-систем)
export PATH := $(HOME)/.cargo/bin:$(PATH)

BINARY_NAME = rustore_apk_downloader
OUTPUT_DIR = builds
# Версия приложения из Cargo.toml
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
ARCHIVE_PREFIX = RuStore_ApkDownloader_v$(VERSION)

# Цели для различных платформ и архитектур (только те, которые можно собрать на Linux)
LINUX_TARGETS = \
    x86_64-unknown-linux-gnu \
    aarch64-unknown-linux-gnu

WINDOWS_TARGETS = \
    x86_64-pc-windows-gnu

MACOS_TARGETS = \
    x86_64-apple-darwin \
    aarch64-apple-darwin

# Все цели, кроме macOS (так как они не могут быть собраны на Linux)
TARGETS = $(LINUX_TARGETS) $(WINDOWS_TARGETS)

.PHONY: all clean linux windows

all: $(addprefix build-, $(TARGETS))

clean:
	rm -rf $(OUTPUT_DIR)

$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

define create_build_dir
	mkdir -p $(OUTPUT_DIR)/$(1)
endef

# ---- build-*: локальная разработка ----

# Сборка для Linux x86_64 (нативный cargo — cross-образ имеет старый GCC с багом memcmp)
build-x86_64-unknown-linux-gnu:
	$(call create_build_dir,linux-x86_64)
	cargo build --release
	cp target/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME) 2>/dev/null || true

# Сборка для Linux aarch64
build-aarch64-unknown-linux-gnu:
	$(call create_build_dir,linux-aarch64)
	cross build --target aarch64-unknown-linux-gnu --release
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME)
	aarch64-linux-gnu-strip $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME) 2>/dev/null || true

# Сборка для Windows x86_64
build-x86_64-pc-windows-gnu:
	$(call create_build_dir,windows-x86_64)
	cross build --target x86_64-pc-windows-gnu --release
	cp target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe $(OUTPUT_DIR)/windows-x86_64/$(BINARY_NAME).exe

linux: build-x86_64-unknown-linux-gnu build-aarch64-unknown-linux-gnu

windows: build-x86_64-pc-windows-gnu

# ---- release-*: CI/релизные сборки (с --locked) ----

# Релизная сборка для Linux x86_64 (нативный cargo — cross-образ имеет старый GCC)
release-x86_64-unknown-linux-gnu:
	$(call create_build_dir,linux-x86_64)
	cargo build --release --locked
	cp target/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME) 2>/dev/null || true

# Релизная сборка для Linux aarch64
release-aarch64-unknown-linux-gnu:
	$(call create_build_dir,linux-aarch64)
	cross build --target aarch64-unknown-linux-gnu --release --locked
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME)
	aarch64-linux-gnu-strip $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME) 2>/dev/null || true

# Релизная сборка для Windows x86_64
release-x86_64-pc-windows-gnu:
	$(call create_build_dir,windows-x86_64)
	cross build --target x86_64-pc-windows-gnu --release --locked
	cp target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe $(OUTPUT_DIR)/windows-x86_64/$(BINARY_NAME).exe

release-x86_64-apple-darwin:
	$(call create_build_dir,macos-x86_64)
	cargo build --target x86_64-apple-darwin --release --locked
	cp target/x86_64-apple-darwin/release/$(BINARY_NAME) $(OUTPUT_DIR)/macos-x86_64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/macos-x86_64/$(BINARY_NAME) 2>/dev/null || true

release-aarch64-apple-darwin:
	$(call create_build_dir,macos-aarch64)
	cargo build --target aarch64-apple-darwin --release --locked
	cp target/aarch64-apple-darwin/release/$(BINARY_NAME) $(OUTPUT_DIR)/macos-aarch64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/macos-aarch64/$(BINARY_NAME) 2>/dev/null || true

release-all: $(addprefix release-, $(TARGETS))

macos-native: $(addprefix release-, $(MACOS_TARGETS))

# ---- Сборка и упаковка ----

package: all
	tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_linux-x86_64.tar.gz -C $(OUTPUT_DIR)/linux-x86_64 .
	tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_linux-aarch64.tar.gz -C $(OUTPUT_DIR)/linux-aarch64 .
	zip -r $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_windows-x86_64.zip $(OUTPUT_DIR)/windows-x86_64 -j
	@if [ -d $(OUTPUT_DIR)/macos-x86_64 ]; then \
		tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_macos-x86_64.tar.gz -C $(OUTPUT_DIR)/macos-x86_64 .; \
	fi
	@if [ -d $(OUTPUT_DIR)/macos-aarch64 ]; then \
		tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX)_macos-aarch64.tar.gz -C $(OUTPUT_DIR)/macos-aarch64 .; \
	fi

# ---- Установка инструментов ----

install-targets:
	cargo install cross || echo "Cross is already installed"
	rustup target add $(LINUX_TARGETS) $(WINDOWS_TARGETS)

help:
	@echo "Доступные цели:"
	@echo "  all              - Собрать для всех поддерживаемых платформ"
	@echo "  linux            - Собрать для Linux (x86_64, aarch64)"
	@echo "  windows          - Собрать для Windows (x86_64)"
	@echo "  macos-native     - Собрать для macOS (только на macOS системе)"
	@echo "  clean            - Удалить все собранные файлы"
	@echo "  install-targets  - Установить целевые архитектуры"
	@echo "  package          - Собрать всё и упаковать в архивы (tar.gz / zip)"
	@echo "  help             - Показать это сообщение"
