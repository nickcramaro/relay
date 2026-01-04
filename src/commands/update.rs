use crate::cli::OutputFormat;
use anyhow::{anyhow, Context, Result};
use owo_colors::OwoColorize;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const REPO: &str = "nickcramaro/relay";
const INSTALL_PATH: &str = "/usr/local/bin/relay";

pub async fn update(format: OutputFormat) -> Result<()> {
    let (os, arch) = detect_platform()?;
    let asset_name = format!("relay-{}-{}", os, arch);
    let download_url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        REPO, asset_name
    );

    // Determine target path - prefer /usr/local/bin if it exists and is writable
    let current_exe = env::current_exe().context("Failed to get current executable path")?;
    let install_path = PathBuf::from(INSTALL_PATH);

    let target_path = if install_path.exists() {
        // Check if we can write to /usr/local/bin
        if is_writable(&install_path) {
            install_path
        } else {
            match format {
                OutputFormat::Human => {
                    eprintln!(
                        "{} Cannot write to {}. Run with sudo or update the current binary.",
                        "warning:".yellow(),
                        INSTALL_PATH
                    );
                }
                OutputFormat::Json => {}
            }
            current_exe.clone()
        }
    } else {
        current_exe.clone()
    };

    match format {
        OutputFormat::Human => {
            println!("Updating relay...");
            println!("  OS: {}", os);
            println!("  Arch: {}", arch);
            println!("  Target: {}", target_path.display());
            println!();
        }
        OutputFormat::Json => {}
    }

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
    let temp_path = target_path.with_extension("new");
    fs::write(&temp_path, &bytes).context("Failed to write temporary file")?;

    // Make executable
    let mut perms = fs::metadata(&temp_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&temp_path, perms)?;

    // Replace target executable
    fs::rename(&temp_path, &target_path).context("Failed to replace executable")?;

    match format {
        OutputFormat::Human => {
            println!();
            println!(
                "{} Updated {}",
                "✓".green(),
                target_path.display().to_string().cyan()
            );
        }
        OutputFormat::Json => {
            println!(
                r#"{{"success": true, "path": "{}"}}"#,
                target_path.display()
            );
        }
    }

    Ok(())
}

fn is_writable(path: &PathBuf) -> bool {
    if let Some(parent) = path.parent() {
        // Check if we can write to the directory
        let test_path = parent.join(".relay_write_test");
        if fs::write(&test_path, b"").is_ok() {
            let _ = fs::remove_file(&test_path);
            return true;
        }
    }
    false
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
