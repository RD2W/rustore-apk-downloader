use crate::domain::{AppRepository, DomainError};

/// Service that orchestrates the application download process
pub struct AppDownloadService<R: AppRepository> {
    repo: R,
}

impl<R: AppRepository> AppDownloadService<R> {
    /// Creates a new instance of the service
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Downloads an app by its package name to the specified path
    pub async fn download_app_by_package_name(
        &self,
        package_name: &str,
        download_path: &str,
    ) -> Result<String, DomainError> {
        log::info!("Getting app info for package: {}", package_name);

        // Get app information
        let app_info = self.repo.get_app_info(package_name).await?;
        log::info!("Retrieved app info: {}", app_info);

        // Print app information before starting download
        println!("=== Application Information ===");
        println!("Package Name: {}", app_info.package_name);
        println!("Version: {} (Code: {})", app_info.version_name, app_info.version_code);
        println!("File Size: {} bytes", app_info.file_size);
        println!("Min SDK Version: {}", app_info.min_sdk_version);
        println!("Target SDK Version: {}", app_info.target_sdk_version);
        println!("Max SDK Version: {}", app_info.max_sdk_version);
        println!("Download URL: {}", app_info.download_url);
        println!("===============================");
        println!();
        println!("Downloading file...");
        println!();

        // Download the app
        let downloaded_path = self.repo.download_app(&app_info, download_path).await?;
        
        log::info!("Successfully downloaded app to: {}", downloaded_path);
        Ok(downloaded_path)
    }
}

/// Mock repository for testing
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AppInfo, DomainError};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockRepository {
        call_count: AtomicUsize,
        should_fail_get_info: bool,
        should_fail_download: bool,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                should_fail_get_info: false,
                should_fail_download: false,
            }
        }

        fn failing_get_info() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                should_fail_get_info: true,
                should_fail_download: false,
            }
        }

        fn failing_download() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                should_fail_get_info: false,
                should_fail_download: true,
            }
        }

        fn mock_app_info() -> AppInfo {
            AppInfo {
                integration_type: "rustore".to_string(),
                download_url: "https://example.com/app.apk".to_string(),
                package_name: "com.test.app".to_string(),
                version_name: "1.0.0".to_string(),
                version_code: 1,
                min_sdk_version: 21,
                max_sdk_version: 34,
                target_sdk_version: 33,
                file_size: 1000,
                icon_url: "https://example.com/icon.png".to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl AppRepository for MockRepository {
        async fn get_app_info(&self, _package_name: &str) -> Result<AppInfo, DomainError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.should_fail_get_info {
                return Err(DomainError::ApiError("Mock: app not found".to_string()));
            }
            Ok(Self::mock_app_info())
        }

        async fn download_app(&self, _app_info: &AppInfo, _download_path: &str) -> Result<String, DomainError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.should_fail_download {
                return Err(DomainError::DownloadError("Mock: download failed".to_string()));
            }
            Ok("/mock/path/app-1.0.0.apk".to_string())
        }
    }

    #[tokio::test]
    async fn test_successful_download() {
        let mock_repo = MockRepository::new();
        let service = AppDownloadService::new(mock_repo);

        let result = service
            .download_app_by_package_name("com.test.app", "/tmp/downloads")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/mock/path/app-1.0.0.apk");
        assert_eq!(service.repo.call_count.load(Ordering::SeqCst), 2); // get_info + download
    }

    #[tokio::test]
    async fn test_download_fails_on_get_info_error() {
        let mock_repo = MockRepository::failing_get_info();
        let service = AppDownloadService::new(mock_repo);

        let result = service
            .download_app_by_package_name("com.test.app", "/tmp/downloads")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::ApiError(msg) => {
                assert!(msg.contains("Mock: app not found"));
            }
            other => panic!("Expected ApiError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_download_fails_on_download_error() {
        let mock_repo = MockRepository::failing_download();
        let service = AppDownloadService::new(mock_repo);

        let result = service
            .download_app_by_package_name("com.test.app", "/tmp/downloads")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::DownloadError(msg) => {
                assert!(msg.contains("Mock: download failed"));
            }
            other => panic!("Expected DownloadError, got: {:?}", other),
        }
    }
}
