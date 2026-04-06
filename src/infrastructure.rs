use crate::domain::{AppInfo, AppRepository, DomainError};
use crate::util;
use tokio::io::AsyncWriteExt;
use tokio::fs;
use zip::ZipArchive;
use futures_util::StreamExt;

/// Implementation of AppRepository that interacts with RuStore API
pub struct RuStoreDownloader {
    client: reqwest::Client,
}

impl RuStoreDownloader {
    /// Creates a new instance of the downloader
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)) // 30 seconds timeout for API calls
            .build()
            .expect("Failed to build HTTP client");
            
        Self { client }
    }

    /// Sanitizes a path to prevent path traversal attacks
    fn sanitize_path(&self, path: &str) -> Result<String, DomainError> {
        log::info!("Sanitizing path: {}", path);
        
        let path_obj = std::path::Path::new(path);
        log::info!("Path object created: {:?}", path_obj);

        // On Windows, canonicalize can fail if the path is on a different drive
        // So we'll handle this differently
        let normalized_path = if cfg!(target_os = "windows") {
            // For Windows, we'll use a more lenient approach
            match path_obj.canonicalize() {
                Ok(canonical_path) => canonical_path.to_string_lossy().to_string(),
                Err(_) => {
                    // If canonicalization fails, use the absolute path instead
                    let abs_path = std::env::current_dir()
                        .map_err(|e| DomainError::FileSystemError(format!("Cannot get current directory: {}", e)))?
                        .join(path_obj)
                        .canonicalize()
                        .map_err(|e| {
                            log::error!("Path canonicalization failed for '{}': {}", path, e);
                            DomainError::FileSystemError(format!("Path canonicalization failed: {}", e))
                        })?
                        .to_string_lossy()
                        .to_string();
                    
                    log::info!("Used absolute path approach for Windows: {}", abs_path);
                    abs_path
                }
            }
        } else {
            // For Unix-like systems, continue with the original approach
            path_obj
                .canonicalize()
                .map_err(|e| {
                    log::error!("Path canonicalization failed for '{}': {}", path, e);
                    DomainError::FileSystemError(format!("Path canonicalization failed: {}", e))
                })?
                .to_string_lossy()
                .to_string()
        };
            
        log::info!("Canonicalized path: {}", normalized_path);

        let base_dir = std::env::current_dir()
            .map_err(|e| {
                log::error!("Cannot get current directory: {}", e);
                DomainError::FileSystemError(format!("Cannot get current directory: {}", e))
            })?
            .to_string_lossy()
            .to_string();
            
        log::info!("Base directory: {}", base_dir);

        // For Windows, we need to handle paths on different drives
        if cfg!(target_os = "windows") {
            // On Windows, we'll check if the path is safe by ensuring it doesn't contain dangerous patterns
            if path.contains("../") || path.contains("..\\") || path.ends_with("/..") || path.ends_with("\\..") {
                log::error!("Path traversal detected in path: {}", path);
                return Err(DomainError::FileSystemError(
                    format!("Path traversal detected: {}", path),
                ));
            }
        } else {
            // For Unix-like systems, continue with the original approach
            if !normalized_path.starts_with(&base_dir) {
                log::error!("Path traversal detected: '{}' does not start with base dir '{}'", normalized_path, base_dir);
                return Err(DomainError::FileSystemError(
                    format!("Path traversal detected: {}", path),
                ));
            }
        }

        log::info!("Path sanitization successful: {}", normalized_path);
        Ok(normalized_path)
    }
}

#[async_trait::async_trait]
impl AppRepository for RuStoreDownloader {
    async fn get_app_info(&self, package_name: &str) -> Result<AppInfo, DomainError> {
        log::info!("Attempting to get app info for package: {}", package_name);

        // Make request to get overall app info
        let url = format!("https://backapi.rustore.ru/applicationData/overallInfo/{}", package_name);
        let response = self.client
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

        let response_text = response
            .text()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Failed to read response: {}", e)))?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| DomainError::ApiError(format!("Invalid response format: {}", e)))?;

        // Extract body info
        let body_info = response_json
            .get("body")
            .ok_or_else(|| DomainError::ApiError("Response missing 'body' field".to_string()))?;

        let app_id = body_info
            .get("appId")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| DomainError::ApiError("Missing appId in response".to_string()))?;

        let package_name_response = body_info
            .get("packageName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::ApiError("Missing packageName in response".to_string()))?
            .to_string();

        let version_name = body_info
            .get("versionName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::ApiError("Missing versionName in response".to_string()))?
            .to_string();

        let company_name = body_info
            .get("companyName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::ApiError("Missing companyName in response".to_string()))?;

        log::info!(
            "Successfully found app with package name: {}, version: {}, company: {}",
            package_name_response, version_name, company_name
        );

        // Now get download link
        let download_link_url = "https://backapi.rustore.ru/applicationData/download-link";
        let download_request_body = serde_json::json!({
            "appId": app_id,
            "firstInstall": true
        });

        let download_response = self.client
            .post(download_link_url)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&download_request_body)
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Download link request failed: {}", e)))?;

        let download_status = download_response.status();
        log::info!("Download link request status: {}", download_status);

        if download_status != 200 {
            return Err(DomainError::ApiError(format!(
                "Failed to get application download link. Request returned status code: {}",
                download_status
            )));
        }

        let download_response_text = download_response
            .text()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Failed to read download response: {}", e)))?;

        let download_response_json: serde_json::Value = serde_json::from_str(&download_response_text)
            .map_err(|e| DomainError::ApiError(format!("Invalid download response format: {}", e)))?;

        let download_link = download_response_json
            .get("body")
            .and_then(|v| v.get("apkUrl"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| DomainError::ApiError("Missing download URL in response".to_string()))?
            .to_string();

        // Create AppInfo from response data
        let app_info = AppInfo {
            integration_type: "rustore".to_string(),
            download_url: download_link,
            package_name: body_info
                .get("packageName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DomainError::ApiError("Missing packageName in response".to_string()))?
                .to_string(),
            version_name: body_info
                .get("versionName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DomainError::ApiError("Missing versionName in response".to_string()))?
                .to_string(),
            version_code: body_info
                .get("versionCode")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| DomainError::ApiError("Missing versionCode in response".to_string()))? as i32,
            min_sdk_version: body_info
                .get("minSdkVersion")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| DomainError::ApiError("Missing minSdkVersion in response".to_string()))? as i32,
            max_sdk_version: body_info
                .get("maxSdkVersion")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| DomainError::ApiError("Missing maxSdkVersion in response".to_string()))? as i32,
            target_sdk_version: body_info
                .get("targetSdkVersion")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| DomainError::ApiError("Missing targetSdkVersion in response".to_string()))? as i32,
            file_size: body_info
                .get("fileSize")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| DomainError::ApiError("Missing fileSize in response".to_string()))?,
            icon_url: body_info
                .get("iconUrl")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DomainError::ApiError("Missing iconUrl in response".to_string()))?
                .to_string(),
        };

        Ok(app_info)
    }

    async fn download_app(&self, app_info: &AppInfo, download_path: &str) -> Result<String, DomainError> {
        log::info!("Starting download of application: {}", app_info.package_name);
        log::info!("Download path: {}", download_path);

        // Sanitize the download path
        let sanitized_download_path = self.sanitize_path(download_path)?;
        
        // Create download directory if it doesn't exist
        log::info!("Creating directory: {}", sanitized_download_path);
        fs::create_dir_all(&sanitized_download_path)
            .await
            .map_err(|e| {
                log::error!("Cannot create download directory '{}': {}", sanitized_download_path, e);
                DomainError::FileSystemError(format!("Cannot create download directory: {}", e))
            })?;
        
        log::info!("Created directory {} for downloading app", sanitized_download_path);

        // Create temporary filename
        let temp_filename = format!("{}-{}.tmp", app_info.package_name, app_info.version_name);
        let temp_file_path = std::path::Path::new(&sanitized_download_path)
            .join(&temp_filename)
            .to_string_lossy()
            .to_string();
        
        log::info!("Temporary file path: {}", temp_file_path);

        // Download the file
        log::info!("Downloading from: {}", app_info.download_url);
        let response = self.client
            .get(&app_info.download_url)
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes timeout for download
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("Download request failed: {}", e)))?;

        let status = response.status();
        if status != 200 {
            if status == 401 {
                return Err(DomainError::DownloadError(
                    format!("Failed to download application. Unauthorized access. Request returned status code: {}", status)
                ));
            } else {
                let response_text = response.text().await.unwrap_or_default();
                return Err(DomainError::DownloadError(
                    format!("Failed to download application. Request returned status code: {}, Response: {}", status, response_text)
                ));
            }
        }

        // Stream the response to a temporary file
        log::info!("Creating temporary file: {}", temp_file_path);
        let mut file = tokio::fs::File::create(&temp_file_path)
            .await
            .map_err(|e| {
                log::error!("Cannot create temporary file '{}': {}", temp_file_path, e);
                DomainError::FileSystemError(format!("Cannot create temporary file: {}", e))
            })?;

        let mut stream = response.bytes_stream();
        log::info!("Starting to stream download data to file");
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| DomainError::DownloadError(format!("Error reading download stream: {}", e)))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| {
                    log::error!("Error writing to file '{}': {}", temp_file_path, e);
                    DomainError::FileSystemError(format!("Error writing to file: {}", e))
                })?;
        }

        file.flush()
            .await
            .map_err(|e| {
                log::error!("Error flushing file '{}': {}", temp_file_path, e);
                DomainError::FileSystemError(format!("Error flushing file: {}", e))
            })?;

        log::info!("File was downloaded from RuStore to {}", temp_file_path);

        // Check file size
        let downloaded_size = fs::metadata(&temp_file_path)
            .await
            .map_err(|e| DomainError::FileSystemError(format!("Cannot access downloaded file metadata: {}", e)))?
            .len();
        
        if app_info.file_size > 0 && downloaded_size != app_info.file_size {
            log::warn!("File size mismatch: expected {}, got {}", app_info.file_size, downloaded_size);
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
            let zip_file = std::fs::File::open(&temp_file_path)
                .map_err(|e| DomainError::FileSystemError(format!("Cannot open ZIP file: {}", e)))?;

            let mut archive = ZipArchive::new(zip_file)
                .map_err(|e| DomainError::ValidationError(format!("Cannot read ZIP archive: {}", e)))?;
            
            // Find APK files in the archive
            let mut apk_files = Vec::new();
            for i in 0..archive.len() {
                let file = archive.by_index(i)
                    .map_err(|e| DomainError::ValidationError(format!("Cannot access ZIP entry: {}", e)))?;
                
                let file_name = file.name().to_string();
                if file_name.to_lowercase().ends_with(".apk") {
                    apk_files.push(file_name);
                }
            }
            
            if apk_files.is_empty() {
                return Err(DomainError::DownloadError("No APK file found inside the ZIP archive".to_string()));
            }
            
            // Take the first APK file found
            let apk_filename = &apk_files[0];
            
            // Check for dangerous file paths
            if apk_filename.contains("..") || apk_filename.starts_with('/') || apk_filename.contains("../") {
                return Err(DomainError::ValidationError(
                    format!("Dangerous file path detected in ZIP: {}", apk_filename)
                ));
            }
            
            // Create safe path for extracted APK
            let extracted_apk_filename = format!("{}-{}.apk", app_info.package_name, app_info.version_name);
            let extracted_apk_path = std::path::Path::new(&sanitized_download_path)
                .join(&extracted_apk_filename)
                .to_string_lossy()
                .to_string();
            
            log::info!("Extracting APK file to: {}", extracted_apk_path);
            
            // Extract the APK file
            {
                let mut apk_file_in_zip = archive
                    .by_name(apk_filename)
                    .map_err(|e| DomainError::ValidationError(format!("Cannot find APK in ZIP: {}", e)))?;
                
                let mut extracted_apk_data = Vec::new();
                std::io::copy(&mut apk_file_in_zip, &mut extracted_apk_data)
                    .map_err(|e| DomainError::FileSystemError(format!("Cannot extract APK from ZIP: {}", e)))?;
                
                std::fs::write(&extracted_apk_path, &extracted_apk_data)
                    .map_err(|e| {
                        log::error!("Cannot write extracted APK to '{}': {}", extracted_apk_path, e);
                        DomainError::FileSystemError(format!("Cannot write extracted APK: {}", e))
                    })?;
            } // End scope to drop the ZipFile before async operations
            
            log::info!("APK extracted to {}", extracted_apk_path);

            // Verify the extracted APK
            if !util::is_valid_apk_file(&extracted_apk_path)? {
                return Err(DomainError::ValidationError(
                    format!("Extracted file is not a valid APK: {}", extracted_apk_path)
                ));
            }

            // Check hash of extracted APK
            let extracted_apk_hash = util::calculate_file_hash(&extracted_apk_path).await?;
            log::info!("Extracted APK SHA-256 hash: {}", extracted_apk_hash);
            
            // Remove temporary ZIP file
            std::fs::remove_file(&temp_file_path)
                .map_err(|e| DomainError::FileSystemError(format!("Cannot remove temporary ZIP file: {}", e)))?;
            
            log::info!("Temporary ZIP archive {} removed", temp_file_path);

            Ok(util::clean_windows_path(&extracted_apk_path))
        } else {
            log::info!("Downloaded file is not a ZIP archive, checking if it's a valid APK");

            // The file is not a ZIP, check if it's a valid APK
            if !util::is_valid_apk_file(&temp_file_path)? {
                return Err(DomainError::ValidationError(
                    format!("Downloaded file is not a valid APK: {}", temp_file_path)
                ));
            }
            
            // Rename the temporary file to APK
            let final_file_path = std::path::Path::new(&sanitized_download_path)
                .join(format!("{}-{}.apk", app_info.package_name, app_info.version_name))
                .to_string_lossy()
                .to_string();
            
            log::info!("Renaming temporary file to final APK path: {}", final_file_path);
            
            // Ensure the final path is safe
            let _ = self.sanitize_path(&final_file_path)?;
            
            std::fs::rename(&temp_file_path, &final_file_path)
                .map_err(|e| {
                    log::error!("Cannot rename temporary file from '{}' to '{}': {}", temp_file_path, final_file_path, e);
                    DomainError::FileSystemError(format!("Cannot rename temporary file: {}", e))
                })?;
            
            log::info!("File is already an APK, renamed to {}", final_file_path);

            Ok(util::clean_windows_path(&final_file_path))
        }
    }
}
