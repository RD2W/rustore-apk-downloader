# Загрузчик приложений RuStore

Консольная утилита на Rust для скачивания APK-файлов с RuStore.ru и получения метаданных приложений.

## Возможности

- Загрузка APK с RuStore с индикацией прогресса
- Просмотр информации о приложении без скачивания (`--info`, `-v`, `--json-info`)
- Вывод в JSON для скриптов и автоматизации
- Проверка целостности файлов по SHA-256
- Обработка ZIP-архивов (RuStore заворачивает APK в ZIP)
- Автоматическая очистка временных файлов при ошибках
- Защита от path traversal атак
- TLS с системными сертификатами (Windows, Linux, macOS)

## Использование

### Скачивание APK

```bash
rustore_apk_downloader <package> <path>
# или через cargo:
cargo run -- <package> <path>
```

Пример:
```bash
rustore_apk_downloader ru.yandex.yandexmaps ./downloads
```

### Просмотр метаданных (без скачивания)

```bash
# Полная информация
rustore_apk_downloader --info ru.yandex.yandexmaps    # или -i

# Только версия
rustore_apk_downloader -v ru.yandex.yandexmaps

# JSON для скриптов
rustore_apk_downloader --json-info ru.yandex.yandexmaps | jq .rating
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '.rating.average'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '{name: .app_name, ver: .version_name, size_mb: (.file_size / 1048576 | floor)}'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.signature'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.whats_new'
rustore_apk_downloader -j ru.yandex.yandexmaps > app.json
```

### Флаги

| Флаг | Описание |
|------|----------|
| `-h`, `--help` | Показать справку |
| `-V`, `--version` | Версия программы |
| `-i`, `--info` | Информация о приложении без скачивания |
| `-v` | Версия приложения (название + код) |
| `-j`, `--json-info` | Информация в JSON |

## Скрипты с jq

```bash
# Строка с версией
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '"v\(.version_name) (\(.version_code))"'

# Рейтинг с количеством голосов
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '"\(.rating.average)/5 (\(.rating.votes) голосов)"'

# Размер в МБ
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '"\(.file_size) байт ≈ \(.file_size / 1048576 | floor) МБ"'

# Проверить существование пакета
rustore_apk_downloader -j ru.yandex.yandexmaps > /dev/null && echo "существует"

# Сохранить метаданные и скачать отдельно
rustore_apk_downloader -j ru.yandex.yandexmaps > meta.json
rustore_apk_downloader ru.yandex.yandexmaps ./out

# Проверить версии нескольких пакетов
for pkg in ru.yandex.yandexmaps com.example.app; do
  ver=$(rustore_apk_downloader -j "$pkg" 2>/dev/null | jq -r .version_name)
  echo "$pkg → $ver"
done
```

## Сборка

```bash
cargo build --release
```

### Кроссплатформенная сборка

```bash
make install-targets    # установка cross и rustup целей
make linux              # x86_64 + aarch64
make windows            # x86_64
make all                # все платформы
```

На macOS — нативная сборка:
```bash
make macos-native       # x86_64 + aarch64
cargo build --release   # или напрямую
```

Бинарники помещаются в `builds/`. Архивы включают версию: `RuStore_ApkDownloader_v1.1.0_linux-x86_64.tar.gz`.

> **Примечание:** для Linux x86_64 используется нативный `cargo build` вместо `cross build` из-за бага GCC (memcmp) в Docker-образе cross. CI-воркфлоу (`release.yml`) учитывает это автоматически.

## Архитектура

```
src/
  main.rs            # Точка входа и диспетчеризация
  cli.rs             # Парсинг аргументов (enum Action)
  display.rs         # Форматирование вывода (справка, информация)
  domain.rs          # AppInfo, DomainError, трейт AppRepository
  application.rs     # AppDownloadService — оркестрация
  infrastructure.rs  # RuStoreDownloader — HTTP, файлы, ZIP
  util.rs            # SHA-256, валидация, проверки ZIP/APK
```

| Слой | Файл | Назначение |
|------|------|------------|
| Domain | `domain.rs` | `AppInfo`, `Rating`, `DomainError`, `AppRepository` |
| Application | `application.rs` | `AppDownloadService<R: AppRepository>` |
| Infrastructure | `infrastructure.rs` | `RuStoreDownloader` — API, загрузка, ZIP |
| CLI | `cli.rs` | Парсинг аргументов, `Action` |
| Display | `display.rs` | `print_help()`, `print_app_info()` |
| Utility | `util.rs` | Хеширование, валидация, проверки ZIP/APK |

## Зависимости

- `reqwest` 0.13 + `rustls` (чистый Rust TLS, системные сертификаты)
- `tokio` 1.52 (асинхронный рантайм)
- `serde` / `serde_json` (сериализация)
- `zip` 8.6 (работа с ZIP-архивами)
- `sha2` 0.11 (SHA-256)
- `regex` 1.12 (валидация имени пакета)
- `log` + `env_logger` (логирование)

## Безопасность

- Валидация имени пакета (защита от path traversal)
- Проверка ZIP-архивов на опасные пути
- Проверка APK по содержимому (AndroidManifest.xml + classes.dex)
- TempFileGuard — автоудаление временных файлов при ошибках
- Нормализация путей через `std::path::absolute()`

## Совместимость с Windows

Приложение использует `rustls` с нативными сертификатами (SChannel), проблем с сертификатами российских УЦ нет.

При указании пути загрузки используйте абсолютные пути: `C:\Downloads`. Подробнее см. `README_WINDOWS.md`.

## Лицензия

MIT
