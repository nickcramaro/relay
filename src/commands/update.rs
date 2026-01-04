use crate::cli::OutputFormat;
use anyhow::{anyhow, Context, Result};
use owo_colors::OwoColorize;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const REPO: &str = "nickcramaro/relay";

pub async fn update(format: OutputFormat) -> Result<()> {
    let (os, arch) = detect_platform()?;
    let asset_name = format!("relay-{}-{}", os, arch);
    let download_url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        REPO, asset_name
    );

    match format {
        OutputFormat::Human => {
            println!("Updating relay...");
            println!("  OS: {}", os);
            println!("  Arch: {}", arch);
            println!();
        }
        OutputFormat::Json => {}
    }

    // Get current executable path
    let current_exe = env::current_exe().context("Failed to get current executable path")?;

    // Download new binary
    match format {
        OutputFormat::Human => println!("Downloading from GitHub releases..."),
        OutputFormat::Json => {}
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", download_url))?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to download: HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    // Write to temp file first
    let temp_path = current_exe.with_extension("new");
    fs::write(&temp_path, &bytes).context("Failed to write temporary file")?;

    // Make executable
    let mut perms = fs::metadata(&temp_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&temp_path, perms)?;

    // Replace current executable
    fs::rename(&temp_path, &current_exe).context("Failed to replace executable")?;

    match format {
        OutputFormat::Human => {
            println!();
            println!("{}", "Successfully updated relay!".green().bold());
        }
        OutputFormat::Json => {
            println!(r#"{{"success": true}}"#);
        }
    }

    Ok(())
}

fn detect_platform() -> Result<(&'static str, &'static str)> {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        _ => return Err(anyhow!("Unsupported OS: {}", env::consts::OS)),
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(anyhow!("Unsupported architecture: {}", env::consts::ARCH)),
    };

    Ok((os, arch))
}
