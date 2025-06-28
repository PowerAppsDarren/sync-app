use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _out_dir = env::var("OUT_DIR")?;
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")?;

    // PocketBase version to download
    let pocketbase_version = "0.22.0";

    // Determine the correct PocketBase binary for the target platform
    let (platform, extension) = match (target_os.as_str(), target_arch.as_str()) {
        ("windows", "x86_64") => ("windows_amd64", ".zip"),
        ("linux", "x86_64") => ("linux_amd64", ".zip"),
        ("macos", "x86_64") => ("darwin_amd64", ".zip"),
        ("macos", "aarch64") => ("darwin_arm64", ".zip"),
        _ => {
            println!(
                "cargo:warning=PocketBase binary not available for {target_os}-{target_arch}"
            );
            return Ok(());
        }
    };

    let filename = format!("pocketbase_{pocketbase_version}_{platform}{extension}");
    let url = format!(
        "https://github.com/pocketbase/pocketbase/releases/download/v{pocketbase_version}/{filename}"
    );

    let pocketbase_dir = Path::new("../pocketbase");

    // Create pocketbase directory if it doesn't exist
    if !pocketbase_dir.exists() {
        fs::create_dir_all(pocketbase_dir)?;
    }

    let download_path = pocketbase_dir.join(&filename);

    // Only download if the file doesn't already exist
    if !download_path.exists() {
        println!(
            "cargo:warning=Downloading PocketBase {pocketbase_version} for {target_os}-{target_arch}"
        );
        println!("cargo:warning=URL: {url}");
        println!(
            "cargo:warning=This will be downloaded to: {}",
            download_path.display()
        );
        println!("cargo:warning=Note: Actual download requires network access during build");

        // Create a placeholder file for now - in a real implementation,
        // you would use reqwest or similar to download the actual binary
        fs::write(&download_path, b"placeholder")?;
    }

    // Tell cargo to re-run this build script if the PocketBase directory changes
    println!("cargo:rerun-if-changed=../pocketbase");

    Ok(())
}
