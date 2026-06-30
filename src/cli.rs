pub enum Action {
    Info(String),
    ApkVersion(String),
    JsonInfo(String),
    Download { package: String, path: String },
}

pub fn parse_args(args: &[String]) -> Action {
    match args {
        [_, flag] if ["-V", "--version"].contains(&flag.as_str()) => {
            println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        [_, flag] if ["-h", "--help"].contains(&flag.as_str()) => {
            crate::display::print_help(&args[0]);
            std::process::exit(0);
        }
        [_, flag, pkg] if ["-i", "--info"].contains(&flag.as_str()) => {
            Action::Info(pkg.clone())
        }
        [_, flag, pkg] if ["-v"].contains(&flag.as_str()) => {
            Action::ApkVersion(pkg.clone())
        }
        [_, flag, pkg] if ["-j", "--json-info"].contains(&flag.as_str()) => {
            Action::JsonInfo(pkg.clone())
        }
        [_, _flag, pkg] if ["--version"].contains(&_flag.as_str()) => {
            Action::ApkVersion(pkg.clone())
        }
        [_, pkg, path] => Action::Download {
            package: pkg.clone(),
            path: path.clone(),
        },
        _ => {
            crate::display::print_help(&args[0]);
            std::process::exit(1);
        }
    }
}
