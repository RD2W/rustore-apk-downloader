# RuStore APK Downloader

A Rust CLI for downloading APK files from RuStore.ru and querying app metadata.

## Features

- Download APKs from RuStore via the mobile showcase API (direct APKs, fallback ZIP extraction for legacy apps)
- Query app metadata without downloading (`--info`, `-v`, `--json-info`)
- External source detection — `--info` shows when an app is loaded from third-parties and cannot be downloaded directly
- Device profile emulation via `[device]` in config.toml — choose ABI, DPI, SDK version, locale, mobile services
- JSON output for scripting and automation
- SHA-256 file integrity verification
- Automatic temp file cleanup on errors
- Path sanitization against traversal attacks
- OS-native TLS certificate verification (Windows, Linux, macOS)

## Usage

### Download an APK

```bash
rustore_apk_downloader <package> [path]
# path is optional — falls back to config.toml download.default_path
# or with cargo:
cargo run -- <package> [path]
```

Example:
```bash
rustore_apk_downloader ru.yandex.yandexmaps ./downloads
```

### Query app metadata (no download)

```bash
# Full app info
rustore_apk_downloader --info ru.yandex.yandexmaps    # or -i

# Version only
rustore_apk_downloader -v ru.yandex.yandexmaps

# JSON output (for scripting)
rustore_apk_downloader --json-info ru.yandex.yandexmaps | jq .rating
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '.rating.average'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '{name: .app_name, ver: .version_name, size_mb: (.file_size / 1048576 | floor)}'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.signature'
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.whats_new'
rustore_apk_downloader -j ru.yandex.yandexmaps > app.json
```

### Flags

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Show help |
| `-V`, `--version` | Show program version |
| `-i`, `--info` | Full app info without download |
| `-v` | App version (name + code) |
| `-j`, `--json-info` | App info as JSON |
| `--config <path>` | Use a specific config.toml |

## Configuration (config.toml)

An optional `config.toml` allows overriding settings without rebuilding. The tool
looks for it in this order:

1. `--config <path>` (must exist)
2. `config.toml` next to the binary
3. `config.toml` in the current working directory
4. Built-in defaults

Every key is optional; omitted keys use the defaults shown in `config.example.toml`:

```toml
[api]
base_url = "https://backapi.rustore.ru"
rustore_ver_code = "1000"

[network]
request_timeout_secs = 30
download_timeout_secs = 300

[download]
default_path = "./downloads"
# Placeholders: {package}, {version}, {version_code}, {app_name}
file_name_template = "{package}-{version}.apk"

[device]
# Android device profile — sent to the showcase API to choose the right APK variant.
# ABI list, first entry is preferred: arm64-v8a, armeabi-v7a, x86_64, x86
supported_abis = ["arm64-v8a"]
# Locale codes (e.g. "ru", "en", "en-US")
supported_locales = ["ru"]
# Screen density in DPI: 120, 160, 240, 320, 480, 640
screen_density = 480
# Android SDK level (21+)
sdk_version = 33
# Request a unified APK without split bundles
without_splits = false
# Mobile services: GMS (Google), HMS (Huawei). Empty = none
mobile_services = ["GMS"]

[log]
# off | error | warn | info | debug | trace  (RUST_LOG env var overrides this)
level = "error"
```

## Scripting with jq

```bash
# Version string
rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '"v\(.version_name) (\(.version_code))"'

# Rating with vote count
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '"\(.rating.average)/5 (\(.rating.votes) votes)"'

# Size in MB
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '"\(.file_size) bytes ≈ \(.file_size / 1048576 | floor) MB"'

# Check if a specific package exists
rustore_apk_downloader -j ru.yandex.yandexmaps > /dev/null && echo "exists"

# Check if an app is downloadable (external apps have null download_url)
url=$(rustore_apk_downloader -j ru.yandex.yandexmaps | jq -r '.download_url // "external"')
echo "Download URL: $url"

# Get app_id for custom API use
rustore_apk_downloader -j ru.yandex.yandexmaps | jq '.app_id'

# Detect external-source apps (integration_type != "rustore")
rustore_apk_downloader -j com.eshare.clientv2 | jq '{src: .integration_type, dl: .download_url}'

# Save metadata and download separately
rustore_apk_downloader -j ru.yandex.yandexmaps > meta.json
rustore_apk_downloader ru.yandex.yandexmaps ./out

# Batch check versions for multiple packages
for pkg in ru.yandex.yandexmaps com.example.app; do
  ver=$(rustore_apk_downloader -j "$pkg" 2>/dev/null | jq -r .version_name)
  echo "$pkg → $ver"
done
```

## Build

```bash
cargo build --release
```

### Cross-platform builds

```bash
make install-targets    # one-time: install cross and rustup targets
make linux              # x86_64 + aarch64
make windows            # x86_64
make all                # all platforms
make linux-upx          # linux + UPX compression (2.7 MB per binary)
```

On macOS, build natively:
```bash
make macos-native       # x86_64 + aarch64
cargo build --release   # or directly
```

Built binaries are placed in `builds/`. Archives include version: `RuStore_ApkDownloader_v1.1.0_linux-x86_64.tar.gz`.

> **Note:** x86_64 Linux uses native `cargo build` instead of `cross build` due to a GCC memcmp bug in the cross Docker image. The CI workflow (`release.yml`) handles this automatically.

## Architecture

```
src/
  main.rs            # Bootstrap and dispatch
  cli.rs             # CLI argument parsing (Action enum)
  display.rs         # Output formatting (help, app info)
  domain.rs          # AppInfo, DomainError, AppRepository trait
  application.rs     # AppDownloadService orchestrator
  infrastructure.rs  # RuStoreDownloader: HTTP, file ops, ZIP extraction
  config.rs           # Config structs, config.toml loading/validation
  util.rs             # SHA-256, package validation, ZIP/APK checks

```

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `domain.rs` | `AppInfo`, `Rating`, `DomainError`, `AppRepository` trait |
| Application | `application.rs` | `AppDownloadService<R: AppRepository>` |
| Infrastructure | `infrastructure.rs` | `RuStoreDownloader` — API calls, download stream, ZIP extraction |
| CLI | `cli.rs` | Argument parsing, `Action` enum |
| Display | `display.rs` | `print_help()`, `print_app_info()` |
| Utility | `util.rs` | Hashing, package validation, ZIP/APK checks |
| Utility | `config.rs` | `Config` structs, config.toml loading, template rendering |

## Dependencies

- `reqwest` 0.13 + `rustls` (pure Rust TLS, native OS cert store)
- `tokio` 1.53 (async runtime)
- `serde` / `serde_json` (serialization)
- `zip` 8.6 (ZIP archive handling)
- `sha2` 0.11 (SHA-256 hashing)
- `regex` 1.12 (package name validation)
- `toml` 1 (config file parsing)
- `anyhow` 1.0 (error handling)
- `thiserror` 2.0 (derive Error)
- `log` + `env_logger` (logging)
- `futures-util` 0.3 (async stream helpers)

## Troubleshooting

### External-source apps

Some apps on RuStore are loaded from external sources (aggregators) and do not have a direct APK download. Use `--info` to check:

```
Source:    Загружено из внешнего источника
```

These apps can only be installed through the RuStore mobile app — the API does not provide a download URL. Metadata queries (`--info`, `--json-info`) still work.

### No APK found for a specific ABI

Not all apps provide builds for every architecture. If you get *"No downloadable APK found"* with a custom ABI in `[device]`, try switching to the most common one:

```toml
[device]
supported_abis = ["arm64-v8a"]
```

### Debug logging

Set the `RUST_LOG` environment variable to see API request/response details:

```bash
RUST_LOG=debug rustore_apk_downloader ru.yandex.yandexmaps ./out
```

## License

MIT
