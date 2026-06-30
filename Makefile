# Makefile для компиляции RuStore_ApkDownloader под различные платформы и архитектуры

# Добавляем ~/.cargo/bin в PATH (нужно для macOS и некоторых Linux-систем)
export PATH := $(HOME)/.cargo/bin:$(PATH)

# Префикс для имени архива
ARCHIVE_PREFIX_NAME = RuStore_ApkDownloader

# Имя бинарного файла
BINARY_NAME = rustore_apk_downloader

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

# Папка для вывода бинарных файлов
OUTPUT_DIR = builds

.PHONY: all clean linux windows macos

# Компиляция для всех целей (Linux и Windows, доступных на этой платформе)
all: $(addprefix build-, $(TARGETS))

# Удаление папки с билдами
clean:
	rm -rf $(OUTPUT_DIR)

# Создание директории для сборок
$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

# Функция для создания директории вывода
define create_build_dir
	mkdir -p $(OUTPUT_DIR)/$(1)
endef

# Сборка для Linux x86_64
build-x86_64-unknown-linux-gnu:
	$(call create_build_dir,linux-x86_64)
	cross build --target x86_64-unknown-linux-gnu --release
	cp target/x86_64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME) 2>/dev/null || true

# Сборка для Linux aarch64
build-aarch64-unknown-linux-gnu:
	$(call create_build_dir,linux-aarch64)
	cross build --target aarch64-unknown-linux-gnu --release
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME) 2>/dev/null || true

# Сборка для Windows x86_64
build-x86_64-pc-windows-gnu:
	$(call create_build_dir,windows-x86_64)
	cross build --target x86_64-pc-windows-gnu --release
	cp target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe $(OUTPUT_DIR)/windows-x86_64/$(BINARY_NAME).exe

# Алиасы для конкретных платформ
linux: build-x86_64-unknown-linux-gnu build-aarch64-unknown-linux-gnu

windows: build-x86_64-pc-windows-gnu

# Для сборки под macOS используйте нативную систему macOS
macos-native: $(addprefix release-, $(MACOS_TARGETS))

# Установка целевых архитектур если они не установлены
install-targets:
	cargo install cross || echo "Cross is already installed"
	rustup target add $(LINUX_TARGETS) $(WINDOWS_TARGETS)

# Сборка с оптимизациями для релиза
release-all: $(addprefix release-, $(TARGETS))

release-x86_64-unknown-linux-gnu:
	$(call create_build_dir,linux-x86_64)
	cross build --target x86_64-unknown-linux-gnu --release --locked
	cp target/x86_64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-x86_64/$(BINARY_NAME) 2>/dev/null || true

release-aarch64-unknown-linux-gnu:
	$(call create_build_dir,linux-aarch64)
	cross build --target aarch64-unknown-linux-gnu --release --locked
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY_NAME) $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME)
	strip $(OUTPUT_DIR)/linux-aarch64/$(BINARY_NAME) 2>/dev/null || true

release-x86_64-pc-windows-gnu:
	$(call create_build_dir,windows-x86_64)
	cross build --target x86_64-pc-windows-gnu --release --locked
	cp target/x86_64-pc-windows-gnu/release/$(BINARY_NAME).exe $(OUTPUT_DIR)/windows-x86_64/$(BINARY_NAME).exe

release-x86_64-apple-darwin:
	$(call create_build_dir,macos-x86_64)
	cargo build --target x86_64-apple-darwin --release --locked
	cp target/x86_64-apple-darwin/release/$(BINARY_NAME) $(OUTPUT_DIR)/macos-x86_64/$(BINARY_NAME)

release-aarch64-apple-darwin:
	$(call create_build_dir,macos-aarch64)
	cargo build --target aarch64-apple-darwin --release --locked
	cp target/aarch64-apple-darwin/release/$(BINARY_NAME) $(OUTPUT_DIR)/macos-aarch64/$(BINARY_NAME)

# Упаковка артефактов
package: all
	tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX_NAME)_linux-x86_64.tar.gz -C $(OUTPUT_DIR)/linux-x86_64 .
	tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX_NAME)_linux-aarch64.tar.gz -C $(OUTPUT_DIR)/linux-aarch64 .
	zip -r $(OUTPUT_DIR)/$(ARCHIVE_PREFIX_NAME)_windows-x86_64.zip $(OUTPUT_DIR)/windows-x86_64 -j
	@if [ -d $(OUTPUT_DIR)/macos-x86_64 ]; then \
		tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX_NAME)_macos-x86_64.tar.gz -C $(OUTPUT_DIR)/macos-x86_64 .; \
	fi
	@if [ -d $(OUTPUT_DIR)/macos-aarch64 ]; then \
		tar -czf $(OUTPUT_DIR)/$(ARCHIVE_PREFIX_NAME)_macos-aarch64.tar.gz -C $(OUTPUT_DIR)/macos-aarch64 .; \
	fi

# Краткая информация о возможных целях
help:
	@echo "Доступные цели:"
	@echo "  all              - Собрать для всех поддерживаемых платформ"
	@echo "  linux            - Собрать для Linux (x86_64, aarch64)"
	@echo "  windows          - Собрать для Windows (x86_64)"
	@echo "  macos-native     - Собрать для macOS (только на macOS системе)"
	@echo "  clean            - Удалить все собранные файлы"
	@echo "  install-targets  - Установить целевые архитектуры"
	@echo "  package          - Упаковать все бинарные файлы в архивы"
	@echo "  help             - Показать это сообщение"
