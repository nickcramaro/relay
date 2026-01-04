use crate::auth::{AuthStore, OAuthFlow, StoredToken};
use crate::cli::OutputFormat;
use crate::config::ConfigStore;
use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;

pub async fn authenticate(
    store: &ConfigStore,
    name: &str,
    manual_token: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    // Handle manual token
    if let Some(token) = manual_token {
        let mut auth_store = AuthStore::load()?;
        auth_store.set_token(
            name.to_string(),
            StoredToken {
                access_token: token,
                refresh_token: None,
                expires_at: None,
                token_type: "Bearer".to_string(),
            },
        );
        auth_store.save()?;

        match format {
            OutputFormat::Human => {
                println!("{} Token saved for server: {}", "✓".green(), name.cyan());
            }
            OutputFormat::Json => {
                println!(r#"{{"success": true, "server": "{}"}}"#, name);
            }
        }
        return Ok(());
    }
    let config = store.load()?;
    let server_config = config
        .servers
        .get(name)
        .ok_or_else(|| anyhow!("Server '{}' not found", name))?;

    let url = match &server_config.transport {
        crate::config::TransportConfig::Http { url } => url.clone(),
        crate::config::TransportConfig::Stdio { .. } => {
            return Err(anyhow!(
                "OAuth authentication is only supported for HTTP servers"
            ));
        }
    };

    // First, probe the server to get the resource metadata URL
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json, text/event-stream")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        // Look for WWW-Authenticate header with resource_metadata URL
        if let Some(www_auth) = response.headers().get("www-authenticate") {
            let www_auth_str = www_auth.to_str().unwrap_or("");
            if let Some(metadata_url) = crate::auth::parse_www_authenticate(www_auth_str) {
                let flow = OAuthFlow::new(name.to_string(), url);
                let _token = flow.authenticate(&metadata_url).await?;

                match format {
                    OutputFormat::Human => {
                        println!("{} Authenticated with server: {}", "✓".green(), name.cyan());
                    }
                    OutputFormat::Json => {
                        println!(r#"{{"success": true, "server": "{}"}}"#, name);
                    }
                }
                return Ok(());
            }
        }

        // Extract the origin from the URL
        let base_url = url.trim_end_matches('/');
        let origin = if let Some(pos) = base_url.find("://") {
            let after_scheme = &base_url[pos + 3..];
            if let Some(path_pos) = after_scheme.find('/') {
                &base_url[..pos + 3 + path_pos]
            } else {
                base_url
            }
        } else {
            base_url
        };

        // Try OAuth authorization server discovery first (more common)
        let auth_server_url = format!("{}/.well-known/oauth-authorization-server", origin);
        let flow = OAuthFlow::new(name.to_string(), url.clone());

        match flow.authenticate_with_auth_server(&auth_server_url).await {
            Ok(_token) => {
                match format {
                    OutputFormat::Human => {
                        println!("{} Authenticated with server: {}", "✓".green(), name.cyan());
                    }
                    OutputFormat::Json => {
                        println!(r#"{{"success": true, "server": "{}"}}"#, name);
                    }
                }
                return Ok(());
            }
            Err(e) => {
                // Try protected resource metadata as fallback
                let resource_metadata_url =
                    format!("{}/.well-known/oauth-protected-resource", origin);
                match flow.authenticate(&resource_metadata_url).await {
                    Ok(_token) => {
                        match format {
                            OutputFormat::Human => {
                                println!(
                                    "{} Authenticated with server: {}",
                                    "✓".green(),
                                    name.cyan()
                                );
                            }
                            OutputFormat::Json => {
                                println!(r#"{{"success": true, "server": "{}"}}"#, name);
                            }
                        }
                        return Ok(());
                    }
                    Err(_) => {
                        return Err(anyhow!(
                            "OAuth discovery failed: {}\n\n\
                            You can manually provide a token with: relay auth {} --token <TOKEN>",
                            e,
                            name
                        ));
                    }
                }
            }
        }
    } else if response.status().is_success() {
        match format {
            OutputFormat::Human => {
                println!(
                    "{} Server '{}' does not require authentication",
                    "ℹ".blue(),
                    name.cyan()
                );
            }
            OutputFormat::Json => {
                println!(
                    r#"{{"success": true, "server": "{}", "message": "no auth required"}}"#,
                    name
                );
            }
        }
        return Ok(());
    }

    Err(anyhow!(
        "Unexpected response from server: HTTP {}",
        response.status()
    ))
}

pub fn logout(name: &str, format: OutputFormat) -> Result<()> {
    let mut auth_store = AuthStore::load()?;

    if auth_store.get_token(name).is_none() {
        match format {
            OutputFormat::Human => {
                println!("{} No authentication found for '{}'", "ℹ".blue(), name);
            }
            OutputFormat::Json => {
                println!(
                    r#"{{"success": false, "server": "{}", "message": "not authenticated"}}"#,
                    name
                );
            }
        }
        return Ok(());
    }

    auth_store.remove_token(name);
    auth_store.save()?;

    match format {
        OutputFormat::Human => {
            println!("{} Logged out from server: {}", "✓".green(), name.cyan());
        }
        OutputFormat::Json => {
            println!(r#"{{"success": true, "server": "{}"}}"#, name);
        }
    }

    Ok(())
}
