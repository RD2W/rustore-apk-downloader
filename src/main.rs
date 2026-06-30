use anyhow::Result;

mod domain;
mod application;
mod infrastructure;
mod util;

fn print_help(program_name: &str) {
    println!("RuStore APK Downloader");
    println!("Author: RD2W");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  {} <package> <path>         Download an APK", program_name);
    println!("  {} --info <package>          Show full app information", program_name);
    println!("  {} --version <package>       Show app version only", program_name);
    println!("  {} --json-info <package>     Show app info as JSON", program_name);
    println!();
    println!("Flags:");
    println!("  -h, --help          Show this help message");
    println!("  -V, --version       Show program version");
    println!("  -i, --info          App info without downloading");
    println!("  -v                  App version (short)");
    println!("  -j, --json-info     App info as JSON");
}

fn print_app_info(info: &domain::AppInfo) {
    println!("=== Application Information ===");
    println!("Name:      {}", info.app_name);
    println!("Package:   {}", info.package_name);
    println!("Version:   {} (code: {})", info.version_name, info.version_code);
    println!("Size:      {} ({:.2} MB)", info.file_size, info.file_size as f64 / 1_048_576.0);
    if let Some(ref rating) = info.rating {
        println!("Rating:    {}", rating);
    }
    if let Some(ref age) = info.age_restriction {
        println!("Age:       {}", age);
    }
    println!("Min SDK:   {}", info.min_sdk_version);
    println!("Target SDK: {}", info.target_sdk_version);
    if info.max_sdk_version > 0 {
        println!("Max SDK:   {}", info.max_sdk_version);
    }
    if let Some(ref updated) = info.app_ver_updated_at {
        let date = updated.split('T').next().unwrap_or(updated);
        println!("Updated:   {}", date);
    }
    if let Some(ref sig) = info.signature {
        println!("Signature: {}", sig);
    }
    if let Some(ref whats_new) = info.whats_new {
        println!("———————————————");
        println!("What's new:");
        println!("{}", whats_new);
    }
    println!("———————————————");
}

enum Action {
    Info(String),
    ApkVersion(String),
    JsonInfo(String),
    Download { package: String, path: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    let action = match &args[..] {
        [_, flag] if ["-V", "--version"].contains(&flag.as_str()) => {
            println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        [_, flag] if ["-h", "--help"].contains(&flag.as_str()) => {
            print_help(&args[0]);
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
            print_help(&args[0]);
            std::process::exit(1);
        }
    };

    let pkg = match &action {
        Action::Info(pkg) | Action::ApkVersion(pkg) | Action::JsonInfo(pkg) => pkg.clone(),
        Action::Download { package, .. } => package.clone(),
    };

    util::validate_package_name(&pkg)?;

    let downloader = infrastructure::RuStoreDownloader::new()?;
    let app_service = application::AppDownloadService::new(downloader);

    match action {
        Action::Info(_) => {
            println!("Fetching app info for {}...", pkg);
            let info = app_service.get_app_info(&pkg).await?;
            println!();
            print_app_info(&info);
        }
        Action::ApkVersion(_) => {
            let info = app_service.get_app_info(&pkg).await?;
            println!("{} ({})", info.version_name, info.version_code);
        }
        Action::JsonInfo(_) => {
            let info = app_service.get_app_info(&pkg).await?;
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
        Action::Download { path, .. } => {
            println!("Fetching app info for {}...", pkg);
            let info = app_service.get_app_info(&pkg).await?;
            println!();
            print_app_info(&info);
            println!();
            println!("Downloading...");
            match app_service.download_app(&info, &path).await {
                Ok(path) => println!("Apk downloaded: {}", path),
                Err(e) => {
                    log::error!("Failed to download app: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
