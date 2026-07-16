pub enum Action {
    Info(String),
    ApkVersion(String),
    JsonInfo(String),
    Download {
        package: String,
        path: Option<String>,
    },
}

pub struct ParsedArgs {
    pub config_path: Option<String>,
    pub action: Action,
}

pub fn parse_args(args: &[String]) -> ParsedArgs {
    let mut rest: Vec<String> = Vec::with_capacity(args.len());
    let mut config_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--config" {
            if i + 1 >= args.len() {
                eprintln!("Error: --config requires a path argument");
                std::process::exit(1);
            }
            config_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            rest.push(args[i].clone());
            i += 1;
        }
    }

    ParsedArgs {
        config_path,
        action: parse_action(&rest),
    }
}

fn parse_action(args: &[String]) -> Action {
    match args {
        [_, flag] if ["-V", "--version"].contains(&flag.as_str()) => {
            println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        [_, flag] if ["-h", "--help"].contains(&flag.as_str()) => {
            crate::display::print_help(&args[0]);
            std::process::exit(0);
        }
        [_, flag, pkg] if ["-i", "--info"].contains(&flag.as_str()) => Action::Info(pkg.clone()),
        [_, flag, pkg] if ["-v"].contains(&flag.as_str()) => Action::ApkVersion(pkg.clone()),
        [_, flag, pkg] if ["-j", "--json-info"].contains(&flag.as_str()) => {
            Action::JsonInfo(pkg.clone())
        }
        [_, _flag, pkg] if ["--version"].contains(&_flag.as_str()) => {
            Action::ApkVersion(pkg.clone())
        }
        [_, pkg] if !pkg.starts_with('-') => Action::Download {
            package: pkg.clone(),
            path: None,
        },
        [_, pkg, path] if !pkg.starts_with('-') => Action::Download {
            package: pkg.clone(),
            path: Some(path.clone()),
        },
        _ => {
            crate::display::print_help(&args[0]);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn test_download_with_explicit_path() {
        let parsed = parse_args(&s(&["prog", "com.example.app", "/tmp/out"]));
        assert!(parsed.config_path.is_none());
        assert!(matches!(
            parsed.action,
            Action::Download { ref package, ref path }
                if package == "com.example.app" && path.as_deref() == Some("/tmp/out")
        ));
    }

    #[test]
    fn test_download_without_path() {
        let parsed = parse_args(&s(&["prog", "com.example.app"]));
        assert!(matches!(
            parsed.action,
            Action::Download { ref package, ref path }
                if package == "com.example.app" && path.is_none()
        ));
    }

    #[test]
    fn test_config_flag_is_extracted_before_action() {
        let parsed = parse_args(&s(&[
            "prog",
            "--config",
            "/etc/r.toml",
            "-i",
            "com.example.app",
        ]));
        assert_eq!(parsed.config_path.as_deref(), Some("/etc/r.toml"));
        assert!(matches!(parsed.action, Action::Info(ref p) if p == "com.example.app"));
    }

    #[test]
    fn test_config_flag_with_download_and_no_path() {
        let parsed = parse_args(&s(&["prog", "com.example.app", "--config", "cfg.toml"]));
        assert_eq!(parsed.config_path.as_deref(), Some("cfg.toml"));
        assert!(matches!(
            parsed.action,
            Action::Download { ref path, .. } if path.is_none()
        ));
    }
}
