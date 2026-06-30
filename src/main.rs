use anyhow::Result;

mod application;
mod cli;
mod display;
mod domain;
mod infrastructure;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let action = cli::parse_args(&args);

    let pkg = match &action {
        cli::Action::Info(pkg) | cli::Action::ApkVersion(pkg) | cli::Action::JsonInfo(pkg) => {
            pkg.clone()
        }
        cli::Action::Download { package, .. } => package.clone(),
    };

    util::validate_package_name(&pkg)?;

    let downloader = infrastructure::RuStoreDownloader::new()?;
    let app_service = application::AppDownloadService::new(downloader);

    match action {
        cli::Action::Info(_) => {
            println!("Fetching app info for {}...", pkg);
            let info = app_service.get_app_info(&pkg).await?;
            println!();
            display::print_app_info(&info);
        }
        cli::Action::ApkVersion(_) => {
            let info = app_service.get_app_info(&pkg).await?;
            println!("{} ({})", info.version_name, info.version_code);
        }
        cli::Action::JsonInfo(_) => {
            let info = app_service.get_app_info(&pkg).await?;
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
        cli::Action::Download { path, .. } => {
            println!("Fetching app info for {}...", pkg);
            let info = app_service.get_app_info(&pkg).await?;
            println!();
            display::print_app_info(&info);
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
