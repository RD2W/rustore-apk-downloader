# Загрузчик приложений RuStore

Консольная утилита на Rust для скачивания APK-файлов с RuStore.ru и получения метаданных приложений.

## Возможности

- Загрузка APK с RuStore через мобильный showcase API (прямые APK, для старых приложений — извлечение из ZIP)
- Просмотр метаданных приложения без скачивания (`--info`, `-v`, `--json-info`)
- Определение внешних источников — `--info` показывает, когда приложение загружено из стороннего источника и недоступно для прямой загрузки
- Эмуляция профиля устройства через `[device]` в config.toml — выбор ABI, DPI, версии SDK, локали, мобильных сервисов
- Вывод в JSON для скриптов и автоматизации
- Проверка целостности файлов по SHA-256
- Автоматическая очистка временных файлов при ошибках
- Защита от path traversal атак
- TLS с системными сертификатами (Windows, Linux, macOS)

## Использование

### Скачивание APK

```bash
rustore_apk_downloader <package> [path]
# path необязателен — по умолчанию из config.toml download.default_path
# или через cargo:
cargo run -- <package> [path]
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
| `--config <path>` | Использовать указанный config.toml |

## Конфигурация (config.toml)

Опциональный `config.toml` позволяет менять настройки без пересборки. Поиск файла
ведётся в следующем порядке:

1. `--config <path>` (файл должен существовать)
2. `config.toml` рядом с бинарником
3. `config.toml` в текущей рабочей директории
4. Встроенные значения по умолчанию

Все ключи опциональны; неуказанные используют значения из `config.example.toml`:

```toml
[api]
base_url = "https://backapi.rustore.ru"
rustore_ver_code = "1000"

[network]
request_timeout_secs = 30
download_timeout_secs = 300

[download]
default_path = "./downloads"
# Плейсхолдеры: {package}, {version}, {version_code}, {app_name}
file_name_template = "{package}-{version}.apk"

[device]
# Профиль Android-устройства — отправляется в showcase API для выбора нужного варианта APK.
# Список ABI, первый — предпочтительный: arm64-v8a, armeabi-v7a, x86_64, x86
supported_abis = ["arm64-v8a"]
# Коды локалей (например, "ru", "en", "en-US")
supported_locales = ["ru"]
# Плотность экрана в DPI: 120, 160, 240, 320, 480, 640
screen_density = 480
# Уровень Android SDK (21+)
sdk_version = 33
# Запросить unified APK без split-пакетов
without_splits = false
# Мобильные сервисы: GMS (Google), HMS (Huawei). Пусто — без сервисов
mobile_services = ["GMS"]

[log]
# off | error | warn | info | debug | trace  (RUST_LOG имеет приоритет)
level = "error"
```

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

# Проверить, доступно ли приложение для скачивания (у внешних download_url = null)
url=$(rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.download_url // "внешний источник"')
echo "Ссылка: $url"

# Получить app_id для использования в API
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '.app_id'

# Определить внешние приложения (integration_type != "rustore")
rustore_apk_downloader -j com.eshare.clientv2 | jq '{src: .integration_type, dl: .download_url}'

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
make linux-upx          # linux + UPX-сжатие (2,7 МБ на бинарник)
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
  config.rs           # Config-структуры, загрузка config.toml, шаблоны имён файлов
  util.rs             # SHA-256, валидация, проверки ZIP/APK
```

| Слой | Файл | Назначение |
|------|------|------------|
| Domain | `domain.rs` | `AppInfo`, `Rating`, `DomainError`, `AppRepository` |
| Application | `application.rs` | `AppDownloadService<R: AppRepository>` |
| Infrastructure | `infrastructure.rs` | `RuStoreDownloader` — API, загрузка, ZIP |
| CLI | `cli.rs` | Парсинг аргументов, `Action` |
| Display | `display.rs` | `print_help()`, `print_app_info()` |
| Utility | `util.rs` | Хеширование, валидация, проверки ZIP/APK |
| Utility | `config.rs` | `Config`-структуры, загрузка config.toml, шаблоны имён файлов |

## Зависимости

- `reqwest` 0.13 + `rustls` (чистый Rust TLS, системные сертификаты)
- `tokio` 1.53 (асинхронный рантайм)
- `serde` / `serde_json` (сериализация)
- `zip` 8.6 (работа с ZIP-архивами)
- `sha2` 0.11 (SHA-256)
- `regex` 1.12 (валидация имени пакета)
- `toml` 1 (парсинг config.toml)
- `anyhow` 1.0 (обработка ошибок)
- `thiserror` 2.0 (derive Error)
- `log` + `env_logger` (логирование)
- `futures-util` 0.3 (потоковая загрузка)

## Решение проблем

### Приложения из внешних источников

Некоторые приложения в RuStore загружены из внешних источников (агрегаторов) и не имеют прямой ссылки на APK. Проверить можно через `--info`:

```
Source:    Загружено из внешнего источника
```

Такие приложения можно установить только через мобильное приложение RuStore — API не предоставляет URL для скачивания. Запросы метаданных (`--info`, `--json-info`) работают.

### APK не найден для выбранного ABI

Не для всех приложений есть сборки под каждую архитектуру. Если при кастомном ABI в `[device]` появляется ошибка *«No downloadable APK found»*, переключитесь на самый распространённый:

```toml
[device]
supported_abis = ["arm64-v8a"]
```

### Отладка

Установите переменную окружения `RUST_LOG` для просмотра деталей API-запросов:

```bash
RUST_LOG=debug rustore_apk_downloader ru.yandex.yandexmaps ./out
```

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
