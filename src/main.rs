use anyhow::Result;

mod domain;
mod application;
mod infrastructure;
mod util;

// Add a function to print help information
fn print_help(program_name: &str) {
    println!("RuStore APK Downloader");
    println!("Author: RD2W");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: {} <package_name> <download_path>", program_name);
    println!();
    println!("Arguments:");
    println!("  <package_name>\tThe package name of the app to download (e.g., com.example.app)");
    println!("  <download_path>\tThe directory path where the APK will be saved");
    println!();
    println!("Flags:");
    println!("  -h, --help\t\tShow this help message");
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flags
    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help") {
        print_help(&args[0]);
        std::process::exit(0);
    }

    if args.len() < 3 {
        print_help(&args[0]);
        std::process::exit(1);
    }

    let package_name = &args[1];
    let download_path = &args[2];

    log::info!("Starting RuStore download for package: {}", package_name);

    // Validate package name first
    util::validate_package_name(package_name)?;

    let downloader = infrastructure::RuStoreDownloader::new()?;
    let app_service = application::AppDownloadService::new(downloader);
    
    let result = app_service.download_app_by_package_name(package_name, download_path).await;
    
    match result {
        Ok((app_info, path)) => {
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
            println!("Apk downloaded: {}", path);
        },
        Err(e) => {
            log::error!("Failed to download app: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
