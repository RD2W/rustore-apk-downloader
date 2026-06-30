use anyhow::Result;

mod domain;
mod application;
mod infrastructure;
mod util;

// Add a function to print help information
fn print_help(program_name: &str) {
    println!("RuStore APK Downloader");
    println!("Author: RD2W");
    println!("Version: 1.0.0");
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
        Ok(path) => {
            log::info!("Successfully downloaded APK to: {}", path);
            println!("Apk downloaded: {}", path);
        },
        Err(e) => {
            log::error!("Failed to download app: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
