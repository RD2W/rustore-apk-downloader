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
    
    println!("Fetching app info for {}...", package_name);
    let app_info = match app_service.get_app_info(package_name).await {
        Ok(info) => info,
        Err(e) => {
            log::error!("Failed to get app info: {}", e);
            std::process::exit(1);
        }
    };

    println!();
    println!("=== Application Information ===");
    println!("Name:      {}", app_info.app_name);
    println!("Package:   {}", app_info.package_name);
    println!("Version:   {} (code: {})", app_info.version_name, app_info.version_code);
    println!("Size:      {} ({:.2} MB)", app_info.file_size, app_info.file_size as f64 / 1_048_576.0);
    if let Some(ref rating) = app_info.rating {
        println!("Rating:    {}", rating);
    }
    if let Some(ref age) = app_info.age_restriction {
        println!("Age:       {}", age);
    }
    println!("Min SDK:   {}", app_info.min_sdk_version);
    println!("Target SDK: {}", app_info.target_sdk_version);
    if app_info.max_sdk_version > 0 {
        println!("Max SDK:   {}", app_info.max_sdk_version);
    }
    if let Some(ref updated) = app_info.app_ver_updated_at {
        let date = updated.split('T').next().unwrap_or(updated);
        println!("Updated:   {}", date);
    }
    if let Some(ref sig) = app_info.signature {
        println!("Signature: {}", sig);
    }
    println!("———————————————");
    if let Some(ref whats_new) = app_info.whats_new {
        println!("What's new: {}", whats_new);
        println!("———————————————");
    }
    println!();

    println!("Downloading...");
    match app_service.download_app(&app_info, download_path).await {
        Ok(path) => {
            println!("Apk downloaded: {}", path);
        },
        Err(e) => {
            log::error!("Failed to download app: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
