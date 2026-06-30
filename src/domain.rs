use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub average: f64,
    pub votes: u64,
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} ({} votes)", self.average, self.votes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub app_name: String,
    pub package_name: String,
    pub version_name: String,
    pub version_code: i64,
    pub short_description: String,
    pub file_size: u64,
    pub min_sdk_version: i64,
    pub max_sdk_version: i64,
    pub target_sdk_version: i64,
    pub icon_url: String,
    pub download_url: String,
    pub integration_type: String,
    pub rating: Option<Rating>,
    pub whats_new: Option<String>,
    pub age_restriction: Option<String>,
    pub app_ver_updated_at: Option<String>,
    pub signature: Option<String>,
}

impl fmt::Display for AppInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) v{} — {}",
            self.app_name, self.package_name, self.version_name,
            self.short_description
        )
    }
}

/// Error types for the domain layer
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Download error: {0}")]
    DownloadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

}

/// Interface for application repository (to be implemented by infrastructure layer)
pub trait AppRepository {
    async fn get_app_info(&self, package_name: &str) -> Result<AppInfo, DomainError>;
    async fn download_app(&self, app_info: &AppInfo, download_path: &str) -> Result<String, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_app_info() -> AppInfo {
        AppInfo {
            app_name: "Example App".to_string(),
            package_name: "com.example.app".to_string(),
            version_name: "1.2.3".to_string(),
            version_code: 123,
            short_description: "An example application".to_string(),
            file_size: 50_000_000,
            min_sdk_version: 21,
            max_sdk_version: 34,
            target_sdk_version: 33,
            icon_url: "https://example.com/icon.png".to_string(),
            download_url: "https://example.com/app.apk".to_string(),
            integration_type: "rustore".to_string(),
            rating: Some(Rating { average: 4.5, votes: 1000 }),
            whats_new: Some("Bug fixes".to_string()),
            age_restriction: Some("0+".to_string()),
            app_ver_updated_at: None,
            signature: None,
        }
    }

    #[test]
    fn test_app_info_display() {
        let app_info = mock_app_info();

        let display = format!("{}", app_info);
        assert!(display.contains("Example App"));
        assert!(display.contains("com.example.app"));
        assert!(display.contains("1.2.3"));
    }

    #[test]
    fn test_domain_error_messages() {
        assert_eq!(
            format!("{}", DomainError::InvalidPackageName("test".to_string())),
            "Invalid package name: test"
        );
        assert_eq!(
            format!("{}", DomainError::NetworkError("timeout".to_string())),
            "Network error: timeout"
        );
        assert_eq!(
            format!("{}", DomainError::ApiError("404".to_string())),
            "API error: 404"
        );
        assert_eq!(
            format!("{}", DomainError::DownloadError("interrupted".to_string())),
            "Download error: interrupted"
        );
        assert_eq!(
            format!("{}", DomainError::ValidationError("corrupt".to_string())),
            "Validation error: corrupt"
        );
        assert_eq!(
            format!("{}", DomainError::FileSystemError("no space".to_string())),
            "File system error: no space"
        );
    }

    #[test]
    fn test_app_info_clone() {
        let app_info = mock_app_info();
        let cloned = app_info.clone();
        assert_eq!(cloned.package_name, app_info.package_name);
        assert_eq!(cloned.version_name, app_info.version_name);
        assert_eq!(cloned.file_size, app_info.file_size);
    }

    #[test]
    fn test_app_info_debug() {
        let app_info = mock_app_info();
        let debug = format!("{:?}", app_info);
        assert!(debug.contains("AppInfo"));
        assert!(debug.contains("com.example.app"));
    }

    #[test]
    fn test_rating_display() {
        let rating = Rating { average: 4.26, votes: 40474 };
        let display = format!("{}", rating);
        assert!(display.contains("4.26"));
        assert!(display.contains("40474"));
    }
}
