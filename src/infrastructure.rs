use crate::domain::{AppInfo, AppRepository, DomainError};
use crate::util;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverallInfoResponse {
    body: AppBody,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppBody {
    app_id: i64,
    #[serde(rename = "appName")]
    app_name: String,
    package_name: String,
    version_name: String,
    #[serde(rename = "shortDescription")]
    short_description: String,
    company_name: Option<String>,
    version_code: i64,
    min_sdk_version: i64,
    max_sdk_version: i64,
    target_sdk_version: i64,
    file_size: u64,
    icon_url: String,
    #[serde(default)]
    rating: Option<RatingBody>,
    #[serde(rename = "whatsNew", default)]
    whats_new: Option<String>,
    #[serde(rename = "ageRestriction", default)]
    age_restriction: Option<AgeRestrictionBody>,
    #[serde(rename = "appVerUpdatedAt", default)]
    app_ver_updated_at: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    aggregator_info: Option<AggregatorInfoResponse>,
}

#[derive(Deserialize)]
struct RatingBody {
    average: f64,
    votes: u64,
}

#[derive(Deserialize)]
struct AgeRestrictionBody {
    category: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AggregatorInfoResponse {
    aggregator_app_id: i64,
    company_name: String,
    source: String,
    #[serde(default)]
    app_coins: bool,
    #[serde(default)]
    displayed_source: Option<String>,
    #[serde(default)]
    source_informer_text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadLinkResponse {
    download_urls: Vec<DownloadUrl>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadUrl {
    url: String,
    #[allow(dead_code)]
    size: u64,
}

/// Guard that removes a temporary file on drop unless consumed (renamed).
struct TempFileGuard {
    path: String,
    consumed: bool,
}

impl TempFileGuard {
    fn new(path: String) -> Self {
        Self {
            path,
            consumed: false,
        }
    }

    fn consume(&mut self) {
        self.consumed = true;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.consumed {
            if let Err(e) = std::fs::remove_file(&self.path) {
                log::warn!("Failed to remove temp file '{}': {}", self.path, e);
            } else {
                log::info!("Cleaned up temp file: {}", self.path);
            }
        }
    }
}

/// RuStore client version code sent with API requests.
/// The backend rejects requests without this header (400 Bad Request).
pub(crate) const RUSTORE_VER_CODE: &str = "1000";

/// HTTP header name for the RuStore version code.
const HEADER_RUSTORE_VER_CODE: &str = "ruStoreVerCode";

/// HTTP content type for JSON API requests.
const CONTENT_TYPE_JSON: &str = "application/json; charset=utf-8";

/// Base URL of the RuStore backend API.
pub(crate) const RUSTORE_BASE_URL: &str = "https://backapi.rustore.ru";

/// API path: fetches app metadata by package name.
const OVERALL_INFO_PATH: &str = "/applicationData/overallInfo";

/// API path: requests a download link with device-specific parameters.
const DOWNLOAD_LINK_PATH: &str = "/v3/showcase/apps/download-link";

/// Default ABI sent to the showcase API to emulate a real device.
const DEFAULT_ABI: &str = "arm64-v8a";

/// Default locale sent to the showcase API.
const DEFAULT_LOCALE: &str = "ru";

/// Default screen density (DPI) sent to the showcase API.
const DEFAULT_SCREEN_DENSITY: i64 = 480;

/// Default Android SDK version sent to the showcase API.
const DEFAULT_SDK_VERSION: i64 = 33;

/// Implementation of AppRepository that interacts with RuStore API
pub struct RuStoreDownloader {
    client: reqwest::Client,
    base_url: String,
    download_timeout_secs: u64,
    file_name_template: String,
}

impl RuStoreDownloader {
    /// Creates a new instance of the downloader configured via Config.
    pub fn new(config: &crate::config::Config) -> Result<Self, DomainError> {
        let ver_code = reqwest::header::HeaderValue::from_str(&config.api.rustore_ver_code)
            .map_err(|e| {
                DomainError::ValidationError(format!("Invalid api.rustore_ver_code: {}", e))
            })?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(HEADER_RUSTORE_VER_CODE, ver_code);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.network.request_timeout_secs,
            ))
            .default_headers(headers)
            .build()
            .map_err(|e| {
                DomainError::NetworkError(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            base_url: config.api.base_url.clone(),
            download_timeout_secs: config.network.download_timeout_secs,
            file_name_template: config.download.file_name_template.clone(),
        })
    }

    /// Resolves path to absolute form without requiring it to exist.
    /// Also checks for unresolved path traversal patterns.
    fn sanitize_path(&self, path: &str) -> Result<String, DomainError> {
        let abs_path = std::path::absolute(path)
            .map_err(|e| DomainError::FileSystemError(format!("Invalid path '{}': {}", path, e)))?;

        let normalized = abs_path.to_string_lossy().to_string();

        let has_traversal = normalized.contains("/../")
            || normalized.contains("\\..\\")
            || normalized.ends_with("/..")
            || normalized.ends_with("\\..");

        if has_traversal {
            return Err(DomainError::FileSystemError(format!(
                "Path traversal detected: {}",
                path
            )));
        }

        log::info!("Sanitized path: {}", normalized);
        Ok(normalized)
    }

    /// Checks that a file path is within a given base directory.
    fn ensure_within_base(&self, file_path: &str, base_dir: &str) -> Result<(), DomainError> {
        let file = std::path::Path::new(file_path);
        let base = std::path::Path::new(base_dir);
        if !file.starts_with(base) {
            return Err(DomainError::FileSystemError(format!(
                "File path '{}' is outside the download directory",
                file_path
            )));
        }
        Ok(())
    }

    fn finalize_apk(
        &self,
        temp_file_path: &str,
        sanitized_download_path: &str,
        app_info: &AppInfo,
        temp_guard: &mut TempFileGuard,
    ) -> Result<String, DomainError> {
        let final_file_path = std::path::Path::new(sanitized_download_path)
            .join(crate::config::render_file_name(
                &self.file_name_template,
                app_info,
            ))
            .to_string_lossy()
            .to_string();

        self.ensure_within_base(&final_file_path, sanitized_download_path)?;

        std::fs::rename(temp_file_path, &final_file_path).map_err(|e| {
            log::error!(
                "Cannot rename temporary file from '{}' to '{}': {}",
                temp_file_path,
                final_file_path,
                e
            );
            DomainError::FileSystemError(format!("Cannot rename temporary file: {}", e))
        })?;

        temp_guard.consume();
        log::info!("File renamed to {}", final_file_path);

        Ok(util::clean_windows_path(&final_file_path))
    }
}

impl RuStoreDownloader {
    async fn fetch_download_link(&self, app_id: i64) -> Result<String, DomainError> {
        let download_response = self
            .client
            .post(format!("{}{}", self.base_url, DOWNLOAD_LINK_PATH))
            .header("Content-Type", CONTENT_TYPE_JSON)
            .json(&serde_json::json!({
                "appId": app_id,
                "firstInstall": true,
                "supportedAbis": [DEFAULT_ABI],
                "screenDensity": DEFAULT_SCREEN_DENSITY,
                "supportedLocales": [DEFAULT_LOCALE],
                "sdkVersion": DEFAULT_SDK_VERSION,
                "withoutSplits": false
            }))
            .send()
            .await
            .map_err(|e| {
                DomainError::NetworkError(format!("Download link request failed: {}", e))
            })?;

        let download_status = download_response.status();
        log::info!("Download link request status: {}", download_status);

        if download_status != 200 {
            return Err(DomainError::ApiError(format!(
                "Failed to get application download link. HTTP {}",
                download_status
            )));
        }

        let dl: DownloadLinkResponse = download_response.json().await.map_err(|e| {
            DomainError::ApiError(format!("Invalid download response format: {}", e))
        })?;

        dl.download_urls
            .first()
            .map(|du| du.url.clone())
            .ok_or_else(|| {
                DomainError::ApiError(
                    "No downloadable APK found for this app. It may be loaded from an external source and require installation through the RuStore mobile app.".to_string(),
                )
            })
    }
}

impl AppRepository for RuStoreDownloader {
    async fn get_app_info(&self, package_name: &str) -> Result<AppInfo, DomainError> {
        log::info!("Attempting to get app info for package: {}", package_name);

        let url = format!("{}{}/{}", self.base_url, OVERALL_INFO_PATH, package_name);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Request failed: {}", e)))?;

        let status_code = response.status();
        log::info!("Overall info request status: {}", status_code);

        if status_code != 200 {
            return Err(DomainError::ApiError(format!(
                "Failed to get application info. Request returned status code: {}",
                status_code
            )));
        }

        let info: OverallInfoResponse = response
            .json()
            .await
            .map_err(|e| DomainError::ApiError(format!("Invalid response format: {}", e)))?;

        let body = info.body;

        log::info!(
            "Successfully found app: {}, version: {}, company: {}",
            body.package_name,
            body.version_name,
            body.company_name.as_deref().unwrap_or("N/A")
        );

        let download_url = match self.fetch_download_link(body.app_id).await {
            Ok(url) => {
                log::info!("Got download link for {}", body.package_name);
                Some(url)
            }
            Err(e) => {
                log::warn!(
                    "Could not get download link for {}: {}",
                    body.package_name,
                    e
                );
                None
            }
        };

        let integration_type = body
            .aggregator_info
            .as_ref()
            .and_then(|a| a.displayed_source.clone())
            .unwrap_or_else(|| "rustore".to_string());

        Ok(AppInfo {
            app_id: body.app_id,
            app_name: body.app_name,
            package_name: body.package_name,
            version_name: body.version_name,
            version_code: body.version_code,
            short_description: body.short_description,
            file_size: body.file_size,
            min_sdk_version: body.min_sdk_version,
            max_sdk_version: body.max_sdk_version,
            target_sdk_version: body.target_sdk_version,
            icon_url: body.icon_url,
            download_url,
            integration_type,
            rating: body.rating.map(|r| crate::domain::Rating {
                average: r.average,
                votes: r.votes,
            }),
            whats_new: body.whats_new,
            age_restriction: body.age_restriction.map(|a| a.category),
            app_ver_updated_at: body.app_ver_updated_at,
            signature: body.signature,
        })
    }

    async fn download_app(
        &self,
        app_info: &AppInfo,
        download_path: &str,
    ) -> Result<String, DomainError> {
        log::info!(
            "Starting download of application: {}",
            app_info.package_name
        );
        log::info!("Download path: {}", download_path);

        // Sanitize the download path
        let sanitized_download_path = self.sanitize_path(download_path)?;

        // Create download directory if it doesn't exist
        log::info!("Creating directory: {}", sanitized_download_path);
        fs::create_dir_all(&sanitized_download_path)
            .await
            .map_err(|e| {
                log::error!(
                    "Cannot create download directory '{}': {}",
                    sanitized_download_path,
                    e
                );
                DomainError::FileSystemError(format!("Cannot create download directory: {}", e))
            })?;

        log::info!(
            "Created directory {} for downloading app",
            sanitized_download_path
        );

        // Create temporary filename
        let temp_filename = format!("{}-{}.tmp", app_info.package_name, app_info.version_name);
        let temp_file_path = std::path::Path::new(&sanitized_download_path)
            .join(&temp_filename)
            .to_string_lossy()
            .to_string();

        log::info!("Temporary file path: {}", temp_file_path);

        let download_url = match &app_info.download_url {
            Some(url) => url.clone(),
            None => {
                log::info!("No download URL cached, fetching from API...");
                self.fetch_download_link(app_info.app_id).await?
            }
        };

        // Download the file
        log::info!("Downloading from: {}", download_url);
        let response = self
            .client
            .get(&download_url)
            .timeout(std::time::Duration::from_secs(self.download_timeout_secs))
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Download request failed: {}", e)))?;

        let status = response.status();
        if status != 200 {
            return if status == 401 {
                Err(DomainError::DownloadError(format!(
                    "Failed to download application. Unauthorized access. Request returned status code: {}",
                    status
                )))
            } else {
                let response_text = response.text().await.unwrap_or_default();
                Err(DomainError::DownloadError(format!(
                    "Failed to download application. Request returned status code: {}, Response: {}",
                    status, response_text
                )))
            };
        }

        // Stream the response to a temporary file
        log::info!("Creating temporary file: {}", temp_file_path);
        let mut file = fs::File::create(&temp_file_path).await.map_err(|e| {
            log::error!("Cannot create temporary file '{}': {}", temp_file_path, e);
            DomainError::FileSystemError(format!("Cannot create temporary file: {}", e))
        })?;

        let mut temp_guard = TempFileGuard::new(temp_file_path.clone());

        let mut stream = response.bytes_stream();
        log::info!("Starting to stream download data to file");
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                DomainError::DownloadError(format!("Error reading download stream: {}", e))
            })?;
            file.write_all(&chunk).await.map_err(|e| {
                log::error!("Error writing to file '{}': {}", temp_file_path, e);
                DomainError::FileSystemError(format!("Error writing to file: {}", e))
            })?;
        }

        file.flush().await.map_err(|e| {
            log::error!("Error flushing file '{}': {}", temp_file_path, e);
            DomainError::FileSystemError(format!("Error flushing file: {}", e))
        })?;

        log::info!("File was downloaded from RuStore to {}", temp_file_path);

        // Check file size
        let downloaded_size = fs::metadata(&temp_file_path)
            .await
            .map_err(|e| {
                DomainError::FileSystemError(format!(
                    "Cannot access downloaded file metadata: {}",
                    e
                ))
            })?
            .len();

        if app_info.file_size > 0 && downloaded_size != app_info.file_size {
            log::warn!(
                "File size mismatch: expected {}, got {}",
                app_info.file_size,
                downloaded_size
            );
        }

        // Calculate file hash for integrity check
        let file_hash = util::calculate_file_hash(&temp_file_path).await?;
        log::info!("Downloaded file SHA-256 hash: {}", file_hash);

        // Check if the downloaded file is a ZIP archive
        log::info!("Checking if downloaded file is a ZIP archive");
        if util::is_zip_file(&temp_file_path)? {
            log::info!("Downloaded file is a ZIP archive, extracting APK...");

            // Validate ZIP file
            util::validate_zip_file(&temp_file_path)?;

            // Open the ZIP file and look for APK files
            let zip_file = std::fs::File::open(&temp_file_path).map_err(|e| {
                DomainError::FileSystemError(format!("Cannot open ZIP file: {}", e))
            })?;

            let mut archive = ZipArchive::new(zip_file).map_err(|e| {
                DomainError::ValidationError(format!("Cannot read ZIP archive: {}", e))
            })?;

            // Find APK files in the archive
            let mut apk_files = Vec::new();
            for i in 0..archive.len() {
                let file = archive.by_index(i).map_err(|e| {
                    DomainError::ValidationError(format!("Cannot access ZIP entry: {}", e))
                })?;

                let file_name = file.name().to_string();
                if file_name.to_lowercase().ends_with(".apk") {
                    apk_files.push(file_name);
                }
            }

            if apk_files.is_empty() {
                if util::is_valid_apk_file(&temp_file_path)? {
                    log::info!("ZIP file itself is a valid APK, renaming...");
                    return self.finalize_apk(
                        &temp_file_path,
                        &sanitized_download_path,
                        app_info,
                        &mut temp_guard,
                    );
                }
                return Err(DomainError::DownloadError(
                    "No APK file found inside the ZIP archive".to_string(),
                ));
            }

            // Take the first APK file found
            let apk_filename = &apk_files[0];

            // Check for dangerous file paths
            if apk_filename.contains("..")
                || apk_filename.starts_with('/')
                || apk_filename.contains("../")
            {
                return Err(DomainError::ValidationError(format!(
                    "Dangerous file path detected in ZIP: {}",
                    apk_filename
                )));
            }

            // Create safe path for extracted APK
            let extracted_apk_filename =
                crate::config::render_file_name(&self.file_name_template, app_info);
            let extracted_apk_path = std::path::Path::new(&sanitized_download_path)
                .join(&extracted_apk_filename)
                .to_string_lossy()
                .to_string();

            log::info!("Extracting APK file to: {}", extracted_apk_path);

            // Extract the APK file (stream directly to disk to avoid buffering in memory)
            {
                let mut apk_file_in_zip = archive.by_name(apk_filename).map_err(|e| {
                    DomainError::ValidationError(format!("Cannot find APK in ZIP: {}", e))
                })?;

                let mut out_file = std::fs::File::create(&extracted_apk_path).map_err(|e| {
                    log::error!("Cannot create APK file '{}': {}", extracted_apk_path, e);
                    DomainError::FileSystemError(format!("Cannot create APK file: {}", e))
                })?;

                std::io::copy(&mut apk_file_in_zip, &mut out_file).map_err(|e| {
                    DomainError::FileSystemError(format!("Cannot extract APK from ZIP: {}", e))
                })?;
            } // End scope to drop the ZipFile before async operations

            log::info!("APK extracted to {}", extracted_apk_path);

            // Verify the extracted APK
            if !util::is_valid_apk_file(&extracted_apk_path)? {
                return Err(DomainError::ValidationError(format!(
                    "Extracted file is not a valid APK: {}",
                    extracted_apk_path
                )));
            }

            // Check hash of extracted APK
            let extracted_apk_hash = util::calculate_file_hash(&extracted_apk_path).await?;
            log::info!("Extracted APK SHA-256 hash: {}", extracted_apk_hash);

            // Remove temporary ZIP file
            std::fs::remove_file(&temp_file_path).map_err(|e| {
                DomainError::FileSystemError(format!("Cannot remove temporary ZIP file: {}", e))
            })?;

            temp_guard.consume();
            log::info!("Temporary ZIP archive {} removed", temp_file_path);

            Ok(util::clean_windows_path(&extracted_apk_path))
        } else {
            log::info!("Downloaded file is not a ZIP archive, checking if it's a valid APK");

            if !util::is_valid_apk_file(&temp_file_path)? {
                return Err(DomainError::ValidationError(format!(
                    "Downloaded file is not a valid APK: {}",
                    temp_file_path
                )));
            }

            self.finalize_apk(
                &temp_file_path,
                &sanitized_download_path,
                app_info,
                &mut temp_guard,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn spawn_test_server() -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let response =
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            socket.write_all(response.as_bytes()).await.unwrap();
            request
        });

        (format!("http://{}", addr), server)
    }

    #[tokio::test]
    async fn test_get_app_info_sends_rustore_ver_code_header() {
        let (base_url, server) = spawn_test_server().await;

        let mut config = crate::config::Config::default();
        config.api.base_url = base_url;
        config.api.rustore_ver_code = "1000".to_string();
        let downloader = RuStoreDownloader::new(&config).unwrap();

        let _ = downloader.get_app_info("ru.example.app").await;

        let request = server.await.unwrap();
        let expected_header = format!("{}: 1000", HEADER_RUSTORE_VER_CODE.to_lowercase());
        assert!(
            request.to_lowercase().contains(&expected_header),
            "Request must include {}: 1000 header. Actual request:\n{}",
            HEADER_RUSTORE_VER_CODE,
            request
        );
    }

    #[tokio::test]
    async fn test_rustore_ver_code_header_value_comes_from_config() {
        let (base_url, server) = spawn_test_server().await;

        let mut config = crate::config::Config::default();
        config.api.base_url = base_url;
        config.api.rustore_ver_code = "9999".to_string();
        let downloader = RuStoreDownloader::new(&config).unwrap();

        let _ = downloader.get_app_info("ru.example.app").await;

        let request = server.await.unwrap();
        let expected_header = format!("{}: 9999", HEADER_RUSTORE_VER_CODE.to_lowercase());
        assert!(
            request.to_lowercase().contains(&expected_header),
            "Header must use configured value. Actual request:\n{}",
            request
        );
    }
}
