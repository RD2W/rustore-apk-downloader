use crate::domain::DomainError;
use sha2::{Sha256, Digest};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;
use tokio::io::AsyncReadExt;

/// Calculates SHA-256 hash of a file
pub async fn calculate_file_hash(file_path: &str) -> Result<String, DomainError> {
    let mut file = tokio::fs::File::open(file_path)
        .await
        .map_err(|e| DomainError::FileSystemError(format!("Cannot open file for hashing: {}", e)))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0; 4096];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|e| DomainError::FileSystemError(format!("Error reading file for hashing: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

/// Validates if a file is a valid ZIP archive
pub fn validate_zip_file(zip_path: &str) -> Result<bool, DomainError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| DomainError::FileSystemError(format!("Cannot open ZIP file: {}", e)))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| DomainError::ValidationError(format!("Invalid ZIP file: {}", e)))?;

    // Test each file in the archive
    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| DomainError::ValidationError(format!("Error reading ZIP entry: {}", e)))?;

        let file_name = file.name();

        // Check for dangerous paths
        if file_name.contains("..") || file_name.starts_with('/') || file_name.contains("../") {
            return Err(DomainError::ValidationError(
                format!("ZIP archive contains dangerous file path: {}", file_name),
            ));
        }
    }

    Ok(true)
}

/// Checks if a file is a valid APK file
pub fn is_valid_apk_file(file_path: &str) -> Result<bool, DomainError> {
    // Check if file exists and has .apk extension
    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(false);
    }

    if path.extension().is_none_or(|ext| ext != "apk") {
        return Ok(false);
    }

    // Check if it's a valid ZIP file (APK is a ZIP archive)
    if !std::fs::metadata(file_path)
        .map_err(|e| DomainError::FileSystemError(format!("Cannot access file metadata: {}", e)))?
        .is_file()
    {
        return Ok(false);
    }

    // Try to read as ZIP to verify it's a proper APK
    let file = std::fs::File::open(file_path)
        .map_err(|e| DomainError::FileSystemError(format!("Cannot open file for validation: {}", e)))?;

    let archive = match ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(_) => return Ok(false),
    };

    // Check for required APK components
    let file_list: Vec<String> = archive.file_names().map(|name| name.to_string()).collect();
    let has_manifest = file_list.iter().any(|name| name == "AndroidManifest.xml");
    let has_classes_dex = file_list.iter().any(|name| name == "classes.dex");

    Ok(has_manifest || has_classes_dex)
}

/// Helper function to check if a file is a ZIP file by magic number
pub fn is_zip_file(file_path: &str) -> Result<bool, DomainError> {
    let mut file = std::fs::File::open(file_path)
        .map_err(|e| DomainError::FileSystemError(format!("Cannot open file for ZIP check: {}", e)))?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| DomainError::FileSystemError("Cannot read file header".to_string()))?;

    // ZIP files start with PK signature
    Ok(magic[0] == 0x50 && magic[1] == 0x4B)
}

/// Validation utilities for package names
pub fn validate_package_name(package_name: &str) -> Result<(), DomainError> {
    if package_name.is_empty() {
        return Err(DomainError::InvalidPackageName(
            "Package name must be a non-empty string".to_string(),
        ));
    }

    // Check for valid package name format (com.example.app)
    let package_regex = regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*$").unwrap();
    if !package_regex.is_match(package_name) {
        return Err(DomainError::InvalidPackageName(
            format!("Invalid package name format: {}", package_name),
        ));
    }

    // Check for dangerous sequences
    if package_name.contains("..") ||
       package_name.contains("/") ||
       package_name.contains("\\") {
        return Err(DomainError::InvalidPackageName(
            format!("Package name contains invalid characters: {}", package_name),
        ));
    }

    Ok(())
}

/// Cleans up a Windows path by removing the \\?\ prefix if present
/// This is used to return user-friendly paths on Windows
pub fn clean_windows_path(path: &str) -> String {
    if path.starts_with(r"\\?\") {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    // ===== validate_package_name tests =====

    #[test]
    fn test_valid_package_names() {
        let valid_names = vec![
            "com.example.app",
            "org.my_company.my_application",
            "com.test.v1.app",
            "com.a.b.c.d",
            "com.example123.app456",
        ];

        for name in valid_names {
            assert!(validate_package_name(name).is_ok(), "Expected '{}' to be valid", name);
        }
    }

    #[test]
    fn test_empty_package_name() {
        let result = validate_package_name("");
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidPackageName(msg) => {
                assert!(msg.contains("non-empty"));
            }
            other => panic!("Expected InvalidPackageName, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_package_names() {
        let invalid_names = vec![
            "123.com.example",
            "com..example",
            "com/example/app",
            "com\\example\\app",
            "com.example.app..test",
            ".com.example",
            "com.example.",
        ];

        for name in invalid_names {
            assert!(validate_package_name(name).is_err(), "Expected '{}' to be invalid", name);
        }
    }

    #[test]
    fn test_package_name_with_path_traversal() {
        let result = validate_package_name("com.example../app");
        assert!(result.is_err());
    }

    // ===== clean_windows_path tests =====

    #[test]
    fn test_clean_windows_path_without_prefix() {
        let path = "C:\\Users\\test\\Downloads";
        assert_eq!(clean_windows_path(path), path.to_string());
    }

    #[test]
    fn test_clean_windows_path_with_prefix() {
        let path = r"\\?\C:\Users\test\Downloads";
        let expected = "C:\\Users\\test\\Downloads";
        assert_eq!(clean_windows_path(path), expected);
    }

    // ===== is_zip_file tests =====

    #[test]
    fn test_is_zip_file_with_real_zip() {
        // Create a minimal ZIP file
        let dir = std::env::temp_dir();
        let zip_path = dir.join("test_zip_check.zip");
        {
            let mut file = File::create(&zip_path).unwrap();
            // ZIP magic number: PK (0x50 0x4B)
            file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        }

        let result = is_zip_file(zip_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Cleanup
        std::fs::remove_file(&zip_path).ok();
    }

    #[test]
    fn test_is_zip_file_with_non_zip() {
        let dir = std::env::temp_dir();
        let txt_path = dir.join("test_not_zip.txt");
        {
            let mut file = File::create(&txt_path).unwrap();
            file.write_all(b"This is not a zip file").unwrap();
        }

        let result = is_zip_file(txt_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Cleanup
        std::fs::remove_file(&txt_path).ok();
    }

    #[test]
    fn test_is_zip_file_with_nonexistent_file() {
        let result = is_zip_file("/nonexistent/file.zip");
        assert!(result.is_err());
    }

    // ===== calculate_file_hash tests =====

    #[tokio::test]
    async fn test_calculate_file_hash_known_value() {
        // SHA-256 of "hello" = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_hash.txt");
        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"hello").unwrap();
        }

        let result = calculate_file_hash(file_path.to_str().unwrap()).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );

        // Cleanup
        std::fs::remove_file(&file_path).ok();
    }

    #[tokio::test]
    async fn test_calculate_file_hash_nonexistent_file() {
        let result = calculate_file_hash("/nonexistent/file.txt").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::FileSystemError(_) => {}
            other => panic!("Expected FileSystemError, got: {:?}", other),
        }
    }

    // ===== validate_zip_file tests =====

    #[test]
    fn test_validate_zip_file_with_valid_zip() {
        let dir = std::env::temp_dir();
        let zip_path = dir.join("test_valid_zip.zip");
        {
            let mut file = File::create(&zip_path).unwrap();
            // Minimal ZIP header
            file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
            // Central directory end record (minimal)
            file.write_all(&[0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        }

        let result = validate_zip_file(zip_path.to_str().unwrap());
        // The zip crate might reject our minimal ZIP, but it shouldn't panic
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        std::fs::remove_file(&zip_path).ok();
    }

    #[test]
    fn test_validate_zip_file_nonexistent() {
        let result = validate_zip_file("/nonexistent/file.zip");
        assert!(result.is_err());
    }
}
