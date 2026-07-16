use serde::Deserialize;

use crate::infrastructure::{RUSTORE_BASE_URL, RUSTORE_VER_CODE};

/// Errors produced while loading or validating config.toml.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Cannot read config file '{0}': {1}")]
    ReadError(String, String),

    #[error("Invalid config file '{0}': {1}")]
    ParseError(String, String),

    #[error("Invalid config value in '{0}': {1}")]
    ValidationError(String, String),
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub api: ApiConfig,
    pub network: NetworkConfig,
    pub download: DownloadConfig,
    pub log: LogConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    pub base_url: String,
    pub rustore_ver_code: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: RUSTORE_BASE_URL.to_string(),
            rustore_ver_code: RUSTORE_VER_CODE.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkConfig {
    pub request_timeout_secs: u64,
    pub download_timeout_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            download_timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadConfig {
    pub default_path: String,
    pub file_name_template: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            default_path: "./downloads".to_string(),
            file_name_template: "{package}-{version}.apk".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "error".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_matches_current_behavior() {
        let c = Config::default();
        assert_eq!(c.api.base_url, "https://backapi.rustore.ru");
        assert_eq!(c.api.rustore_ver_code, "1000");
        assert_eq!(c.network.request_timeout_secs, 30);
        assert_eq!(c.network.download_timeout_secs, 300);
        assert_eq!(c.download.default_path, "./downloads");
        assert_eq!(c.download.file_name_template, "{package}-{version}.apk");
        assert_eq!(c.log.level, "error");
    }

    #[test]
    fn test_parse_full_config() {
        let text = r#"
[api]
base_url = "https://example.com"
rustore_ver_code = "1001"

[network]
request_timeout_secs = 10
download_timeout_secs = 60

[download]
default_path = "/tmp/apks"
file_name_template = "{package}.apk"

[log]
level = "debug"
"#;
        let c: Config = toml::from_str(text).unwrap();
        assert_eq!(c.api.base_url, "https://example.com");
        assert_eq!(c.api.rustore_ver_code, "1001");
        assert_eq!(c.network.request_timeout_secs, 10);
        assert_eq!(c.network.download_timeout_secs, 60);
        assert_eq!(c.download.default_path, "/tmp/apks");
        assert_eq!(c.download.file_name_template, "{package}.apk");
        assert_eq!(c.log.level, "debug");
    }

    #[test]
    fn test_parse_partial_config_uses_defaults_for_rest() {
        let text = r#"
[api]
rustore_ver_code = "1001"
"#;
        let c: Config = toml::from_str(text).unwrap();
        assert_eq!(c.api.rustore_ver_code, "1001");
        assert_eq!(c.api.base_url, "https://backapi.rustore.ru");
        assert_eq!(c.network.request_timeout_secs, 30);
        assert_eq!(c.log.level, "error");
    }

    #[test]
    fn test_parse_empty_config_is_all_defaults() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c, Config::default());
    }

    #[test]
    fn test_parse_broken_toml_fails() {
        assert!(toml::from_str::<Config>("[api\nbase_url=").is_err());
    }

    #[test]
    fn test_unknown_key_is_rejected() {
        assert!(toml::from_str::<Config>("[api]\nbase_uri = \"typo\"").is_err());
    }
}
