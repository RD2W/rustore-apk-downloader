use crate::domain;

pub fn print_help(program_name: &str) {
    println!("RuStore APK Downloader");
    println!("Author: RD2W");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!(
        "  {} <package> <path>          Download an APK",
        program_name
    );
    println!(
        "  {} --info <package>          Show full app information",
        program_name
    );
    println!(
        "  {} --version <package>       Show app version only",
        program_name
    );
    println!(
        "  {} --json-info <package>     Show app info as JSON",
        program_name
    );
    println!();
    println!("Flags:");
    println!("  -h, --help          Show this help message");
    println!("  -V, --version       Show program version");
    println!("  -i, --info          App info without downloading");
    println!("  -v                  App version (short)");
    println!("  -j, --json-info     App info as JSON");
}

pub fn print_app_info(info: &domain::AppInfo) {
    println!("=== Application Information ===");
    println!("Name:      {}", info.app_name);
    println!("Package:   {}", info.package_name);
    println!(
        "Version:   {} (code: {})",
        info.version_name, info.version_code
    );
    println!(
        "Size:      {} ({:.2} MB)",
        info.file_size,
        info.file_size as f64 / 1_048_576.0
    );
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
