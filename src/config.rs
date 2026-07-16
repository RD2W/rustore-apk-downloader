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

impl Config {
    /// Loads config with lookup order:
    /// 1. explicit --config path (must exist)
    /// 2. config.toml next to the binary
    /// 3. config.toml in the current working directory
    /// 4. built-in defaults
    pub fn load(explicit_path: Option<&str>) -> Result<Config, ConfigError> {
        if let Some(path) = explicit_path {
            if !std::path::Path::new(path).is_file() {
                return Err(ConfigError::NotFound(path.to_string()));
            }
            return Self::from_file(path);
        }

        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let candidate = dir.join("config.toml");
            if candidate.is_file() {
                return Self::from_file(&candidate.to_string_lossy());
            }
        }

        if std::path::Path::new("config.toml").is_file() {
            return Self::from_file("config.toml");
        }

        Ok(Config::default())
    }

    /// Reads, parses and validates a specific config file.
    pub fn from_file(path: &str) -> Result<Config, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(path.to_string(), e.to_string()))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(path.to_string(), e.to_string()))?;

        config.validate(path)?;
        log::info!("Loaded config from {}", path);
        Ok(config)
    }

    fn validate(&self, path: &str) -> Result<(), ConfigError> {
        validate_template(&self.download.file_name_template).map_err(|e| match e {
            ConfigError::ValidationError(_, msg) => {
                ConfigError::ValidationError(path.to_string(), msg)
            }
            other => other,
        })?;

        self.log.level.parse::<log::LevelFilter>().map_err(|_| {
            ConfigError::ValidationError(
                path.to_string(),
                format!(
                    "invalid log.level '{}'; allowed: off, error, warn, info, debug, trace",
                    self.log.level
                ),
            )
        })?;

        Ok(())
    }
}

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

    fn write_temp_config(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_explicit_path_missing_is_error() {
        let err = Config::load(Some("/nonexistent/rustore-test-config.toml")).unwrap_err();
        assert!(matches!(err, ConfigError::NotFound(_)));
    }

    #[test]
    fn test_load_explicit_path_parses_file() {
        let path = write_temp_config(
            "rustore-test-load-ok.toml",
            "[api]\nrustore_ver_code = \"2000\"\n",
        );
        let c = Config::load(Some(path.to_str().unwrap())).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(c.api.rustore_ver_code, "2000");
    }

    #[test]
    fn test_load_broken_file_is_parse_error() {
        let path = write_temp_config("rustore-test-load-broken.toml", "[api\n");
        let result = Config::load(Some(path.to_str().unwrap()));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result.unwrap_err(), ConfigError::ParseError(_, _)));
    }

    #[test]
    fn test_load_invalid_template_is_validation_error() {
        let path = write_temp_config(
            "rustore-test-load-badtpl.toml",
            "[download]\nfile_name_template = \"{bad}.apk\"\n",
        );
        let result = Config::load(Some(path.to_str().unwrap()));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ValidationError(_, _)
        ));
    }

    #[test]
    fn test_load_invalid_log_level_is_validation_error() {
        let path = write_temp_config("rustore-test-load-badlog.toml", "[log]\nlevel = \"loud\"\n");
        let result = Config::load(Some(path.to_str().unwrap()));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ValidationError(_, _)
        ));
    }

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
