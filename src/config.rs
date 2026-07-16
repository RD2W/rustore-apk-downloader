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

const ALLOWED_PLACEHOLDERS: [&str; 4] = ["package", "version", "version_code", "app_name"];

/// Checks that a file name template contains only known placeholders.
pub fn validate_template(template: &str) -> Result<(), ConfigError> {
    let re = regex::Regex::new(r"\{([^{}]*)\}").expect("static regex");
    for cap in re.captures_iter(template) {
        let name = &cap[1];
        if !ALLOWED_PLACEHOLDERS.contains(&name) {
            return Err(ConfigError::ValidationError(
                "download.file_name_template".to_string(),
                format!(
                    "unknown placeholder '{{{}}}'; allowed: {{package}}, {{version}}, {{version_code}}, {{app_name}}",
                    name
                ),
            ));
        }
    }
    Ok(())
}

/// Renders the output APK file name from a template and app info.
/// Substituted values are sanitized so the result cannot escape the
/// download directory or contain characters invalid on Windows.
pub fn render_file_name(template: &str, app: &crate::domain::AppInfo) -> String {
    let rendered = template
        .replace("{package}", &sanitize_component(&app.package_name))
        .replace("{version}", &sanitize_component(&app.version_name))
        .replace("{version_code}", &app.version_code.to_string())
        .replace("{app_name}", &sanitize_component(&app.app_name));

    if rendered.to_lowercase().ends_with(".apk") {
        rendered
    } else {
        format!("{}.apk", rendered)
    }
}

fn sanitize_component(value: &str) -> String {
    let mut result: String = value
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '"' | '*' | '?' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    while result.contains("..") {
        result = result.replace("..", "_");
    }

    result
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

    fn mock_app() -> crate::domain::AppInfo {
        crate::domain::AppInfo {
            app_name: "My App: The/Best?".to_string(),
            package_name: "com.example.app".to_string(),
            version_name: "1.2.3".to_string(),
            version_code: 42,
            short_description: String::new(),
            file_size: 0,
            min_sdk_version: 0,
            max_sdk_version: 0,
            target_sdk_version: 0,
            icon_url: String::new(),
            download_url: String::new(),
            integration_type: "rustore".to_string(),
            rating: None,
            whats_new: None,
            age_restriction: None,
            app_ver_updated_at: None,
            signature: None,
        }
    }

    #[test]
    fn test_validate_template_accepts_known_placeholders() {
        assert!(validate_template("{package}-{version}-{version_code}-{app_name}.apk").is_ok());
        assert!(validate_template("no_placeholders.apk").is_ok());
    }

    #[test]
    fn test_validate_template_rejects_unknown_placeholder() {
        let err = validate_template("{package}-{foo}.apk").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("{foo}"),
            "message must name the bad placeholder: {}",
            msg
        );
        assert!(
            msg.contains("{package}"),
            "message must list allowed placeholders: {}",
            msg
        );
    }

    #[test]
    fn test_render_file_name_substitutes_all_placeholders() {
        let name = render_file_name("{package}_{version}_{version_code}.apk", &mock_app());
        assert_eq!(name, "com.example.app_1.2.3_42.apk");
    }

    #[test]
    fn test_render_file_name_appends_apk_extension() {
        let name = render_file_name("{package}", &mock_app());
        assert_eq!(name, "com.example.app.apk");
    }

    #[test]
    fn test_render_file_name_sanitizes_dangerous_characters() {
        let name = render_file_name("{app_name}.apk", &mock_app());
        assert!(!name.contains('/'), "slash must be sanitized: {}", name);
        assert!(!name.contains(':'), "colon must be sanitized: {}", name);
        assert!(
            !name.contains('?'),
            "question mark must be sanitized: {}",
            name
        );
    }

    #[test]
    fn test_render_file_name_neutralizes_path_traversal() {
        let mut app = mock_app();
        app.version_name = "../../etc/passwd".to_string();
        let name = render_file_name("{package}-{version}.apk", &app);
        assert!(!name.contains(".."), "dot-dot must be removed: {}", name);
        assert!(!name.contains('/'), "slash must be removed: {}", name);
    }
}
