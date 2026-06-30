# RuStore APK Downloader

A Rust CLI for downloading APK files from RuStore.ru and querying app metadata.

## Features

- Download APKs from RuStore with progress indication
- Query app information without downloading (`--info`, `-v`, `--json-info`)
- JSON output for scripting and automation
- SHA-256 file integrity verification
- ZIP archive handling (RuStore wraps APKs in ZIP)
- Automatic temp file cleanup on errors
- Path sanitization against traversal attacks
- OS-native TLS certificate verification (Windows, Linux, macOS)

## Usage

### Download an APK

```bash
rustore_apk_downloader <package> <path>
# or with cargo:
cargo run -- <package> <path>
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
  util.rs            # SHA-256, package validation, ZIP/APK checks

```

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `domain.rs` | `AppInfo`, `Rating`, `DomainError`, `AppRepository` trait |
| Application | `application.rs` | `AppDownloadService<R: AppRepository>` |
| Infrastructure | `infrastructure.rs` | `RuStoreDownloader` — API calls, download stream, ZIP extraction |
| CLI | `cli.rs` | Argument parsing, `Action` enum |
| Display | `display.rs` | `print_help()`, `print_app_info()` |
| Utility | `util.rs` | Hashing, package validation, ZIP/APK checks |

## Dependencies

- `reqwest` 0.13 + `rustls` (pure Rust TLS, OS cert store)
- `tokio` 1.52 (async runtime)
- `serde` / `serde_json` (serialization)
- `zip` 8.6 (ZIP archive handling)
- `sha2` 0.11 (SHA-256 hashing)
- `regex` 1.12 (package name validation)
- `log` + `env_logger` (logging)

## License

MIT
