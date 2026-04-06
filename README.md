# RuStore App Downloader

A Rust application for downloading APK files from RuStore.ru following clean architecture principles.

## Features

- Downloads APK files from RuStore using the official API
- Implements clean architecture with separation of concerns
- Package name validation to prevent path traversal attacks
- File integrity verification using SHA-256 hashes
- ZIP archive validation and extraction
- APK file validation

## Architecture

The application follows clean architecture principles with three main layers plus utilities:

### Domain Layer (`src/domain.rs`)
- Contains business entities and interfaces
- Defines the `AppRepository` trait for repository abstraction
- Contains error types

### Application Layer (`src/application.rs`)
- Contains business logic orchestration
- Implements the `AppDownloadService` for coordinating operations

### Infrastructure Layer (`src/infrastructure.rs`)
- Handles external concerns like HTTP requests and file operations
- Implements the `AppRepository` trait with RuStore API interaction
- Manages network requests and file system operations

### Utilities (`src/util.rs`)
- Contains helper functions for file operations
- Package name validation
- File hashing (SHA-256)
- ZIP and APK file validation

## Usage

### Running Directly with Cargo

```bash
cargo run -- <package_name> <download_path>
```

Example:
```bash
cargo run -- ru.yandex.searchplugin ./downloads
```

### Building and Running

First, build the application:

```bash
cargo build --release
```

Then run the built binary:

```bash
./target/release/rustore_apk_downloader <package_name> <download_path>
```

### Cross-platform Builds with Make

Use the provided Makefile to build for various platforms:

```bash
# Build for all supported platforms
make all

# Build for Linux only
make linux

# Build for Windows only
make windows

# Build for macOS only
make macos
```

Built binaries will be placed in the `builds/` directory.

## Dependencies

- `reqwest`: HTTP client with TLS support
- `tokio`: Async runtime
- `serde`: Serialization/deserialization
- `zip`: ZIP archive handling
- `sha2`: Cryptographic hash functions
- `regex`: Regular expressions for validation
- `log` and `env_logger`: Logging functionality

## Security Features

- Package name validation to prevent path traversal
- ZIP archive validation to prevent malicious archives
- APK file validation to ensure downloaded files are legitimate
- Secure temporary file handling
- Safe path sanitization

## License

MIT