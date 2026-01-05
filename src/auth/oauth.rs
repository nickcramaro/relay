use super::storage::{AuthStore, StoredClient, StoredToken};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// Protected Resource Metadata (RFC 9728)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

/// Authorization Server Metadata (RFC 8414)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(default)]
    pub registration_endpoint: Option<String>,
}

/// Dynamic Client Registration Response
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// Token Response
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
}

/// OAuth error from server
#[derive(Debug, Deserialize)]
pub struct OAuthError {
    pub error: String,
    pub error_description: Option<String>,
}

/// Check if a response indicates auth is required
pub fn parse_www_authenticate(header: &str) -> Option<String> {
    // Parse: Bearer realm="mcp", resource_metadata="https://..."
    if !header.to_lowercase().starts_with("bearer") {
        return None;
    }

    for part in header.split(',') {
        let part = part.trim();
        if part.starts_with("resource_metadata=") {
            let url = part
                .strip_prefix("resource_metadata=")?
                .trim_matches('"')
                .to_string();
            return Some(url);
        }
    }
    None
}

/// Generate PKCE code verifier and challenge
fn generate_pkce() -> (String, String) {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // Generate random verifier (43-128 characters, we use 64)
    let mut verifier = String::new();
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let hasher = RandomState::new();
    for i in 0..64 {
        let mut h = hasher.build_hasher();
        h.write_usize(i);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        let idx = (h.finish() as usize) % chars.len();
        verifier.push(chars.chars().nth(idx).unwrap());
    }

    // Generate challenge using SHA-256 (S256 method)
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Generate a random state parameter
fn generate_state() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let hasher = RandomState::new();
    let mut h = hasher.build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    format!("{:016x}", h.finish())
}

/// Extract port number from a redirect URI like "http://localhost:12345/callback"
fn extract_port_from_uri(uri: &str) -> Option<u16> {
    // Parse URI to extract port
    if let Some(host_start) = uri.find("://") {
        let rest = &uri[host_start + 3..];
        if let Some(colon_pos) = rest.find(':') {
            let after_colon = &rest[colon_pos + 1..];
            // Find end of port (either '/' or end of string)
            let port_end = after_colon.find('/').unwrap_or(after_colon.len());
            let port_str = &after_colon[..port_end];
            return port_str.parse().ok();
        }
    }
    None
}

#[allow(dead_code)]
pub struct OAuthFlow {
    client: Client,
    server_name: String,
    server_url: String,
}

impl OAuthFlow {
    pub fn new(server_name: String, server_url: String) -> Self {
        Self {
            client: Client::new(),
            server_name,
            server_url,
        }
    }

    /// Fetch protected resource metadata
    pub async fn fetch_resource_metadata(&self, url: &str) -> Result<ProtectedResourceMetadata> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch resource metadata from {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch resource metadata: HTTP {}",
                response.status()
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse resource metadata")
    }

    /// Fetch authorization server metadata
    pub async fn fetch_auth_server_metadata(
        &self,
        auth_server: &str,
    ) -> Result<AuthServerMetadata> {
        // Try OpenID Connect discovery first
        let oidc_url = format!("{}/.well-known/openid-configuration", auth_server);
        if let Ok(response) = self.client.get(&oidc_url).send().await {
            if response.status().is_success() {
                if let Ok(metadata) = response.json().await {
                    return Ok(metadata);
                }
            }
        }

        // Fall back to OAuth 2.0 discovery
        let oauth_url = format!("{}/.well-known/oauth-authorization-server", auth_server);
        let response =
            self.client.get(&oauth_url).send().await.with_context(|| {
                format!("Failed to fetch auth server metadata from {}", oauth_url)
            })?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch auth server metadata: HTTP {}",
                response.status()
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse auth server metadata")
    }

    /// Register as a dynamic client
    pub async fn register_client(
        &self,
        registration_endpoint: &str,
        redirect_uri: &str,
    ) -> Result<ClientRegistrationResponse> {
        #[derive(Serialize)]
        struct RegistrationRequest<'a> {
            client_name: &'a str,
            redirect_uris: Vec<&'a str>,
            grant_types: Vec<&'a str>,
            response_types: Vec<&'a str>,
        }

        let request = RegistrationRequest {
            client_name: "relay",
            redirect_uris: vec![redirect_uri],
            grant_types: vec!["authorization_code", "refresh_token"],
            response_types: vec!["code"],
        };

        let response = self
            .client
            .post(registration_endpoint)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("Failed to register client at {}", registration_endpoint))?;

        if !response.status().is_success() {
            let error: OAuthError = response.json().await.unwrap_or(OAuthError {
                error: "unknown".to_string(),
                error_description: None,
            });
            return Err(anyhow!(
                "Client registration failed: {} - {}",
                error.error,
                error.error_description.unwrap_or_default()
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse client registration response")
    }

    /// Register a new OAuth client and return the listener, redirect_uri, and client
    async fn register_new_client(
        &self,
        registration_endpoint: &Option<String>,
        auth_server: &str,
        auth_store: &mut AuthStore,
    ) -> Result<(TcpListener, String, StoredClient)> {
        let registration_endpoint = registration_endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("No stored client and no registration endpoint available"))?;

        // Start local callback server with random port
        let listener =
            TcpListener::bind("127.0.0.1:0").context("Failed to bind callback server")?;
        let port = listener.local_addr()?.port();
        let redirect_uri = format!("http://localhost:{}/callback", port);

        println!("Registering client...");
        let response = self
            .register_client(registration_endpoint, &redirect_uri)
            .await?;

        let client = StoredClient {
            client_id: response.client_id,
            client_secret: response.client_secret,
            redirect_uri: Some(redirect_uri.clone()),
        };
        auth_store.set_client(auth_server.to_string(), client.clone());
        auth_store.save()?;

        Ok((listener, redirect_uri, client))
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        token_endpoint: &str,
        code: &str,
        client_id: &str,
        client_secret: Option<&str>,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse> {
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ];

        let secret_string;
        if let Some(secret) = client_secret {
            secret_string = secret.to_string();
            params.push(("client_secret", &secret_string));
        }

        let response = self
            .client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .with_context(|| format!("Failed to exchange code at {}", token_endpoint))?;

        if !response.status().is_success() {
            let error: OAuthError = response.json().await.unwrap_or(OAuthError {
                error: "unknown".to_string(),
                error_description: None,
            });
            return Err(anyhow!(
                "Token exchange failed: {} - {}",
                error.error,
                error.error_description.unwrap_or_default()
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse token response")
    }

    /// Refresh an access token
    #[allow(dead_code)]
    pub async fn refresh_token(
        &self,
        token_endpoint: &str,
        refresh_token: &str,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> Result<TokenResponse> {
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ];

        let secret_string;
        if let Some(secret) = client_secret {
            secret_string = secret.to_string();
            params.push(("client_secret", &secret_string));
        }

        let response = self
            .client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .with_context(|| format!("Failed to refresh token at {}", token_endpoint))?;

        if !response.status().is_success() {
            let error: OAuthError = response.json().await.unwrap_or(OAuthError {
                error: "unknown".to_string(),
                error_description: None,
            });
            return Err(anyhow!(
                "Token refresh failed: {} - {}",
                error.error,
                error.error_description.unwrap_or_default()
            ));
        }

        response
            .json()
            .await
            .context("Failed to parse token response")
    }

    /// Run the full OAuth flow
    pub async fn authenticate(&self, resource_metadata_url: &str) -> Result<StoredToken> {
        println!("Fetching resource metadata...");
        let resource_metadata = self.fetch_resource_metadata(resource_metadata_url).await?;

        if resource_metadata.authorization_servers.is_empty() {
            return Err(anyhow!("No authorization servers found"));
        }

        let auth_server = &resource_metadata.authorization_servers[0];
        println!("Using authorization server: {}", auth_server);

        let auth_metadata = self.fetch_auth_server_metadata(auth_server).await?;

        // Get or register client (need to handle redirect_uri matching)
        let mut auth_store = AuthStore::load()?;

        // Try to use stored client with its original redirect_uri
        let (listener, redirect_uri, client) =
            if let Some(stored) = auth_store.get_client(auth_server) {
                if let Some(stored_uri) = &stored.redirect_uri {
                    // Extract port from stored redirect_uri and try to bind to it
                    if let Some(port) = extract_port_from_uri(stored_uri) {
                        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
                            Ok(listener) => {
                                // Successfully bound to the same port
                                (listener, stored_uri.clone(), stored.clone())
                            }
                            Err(_) => {
                                // Port in use - need to re-register with new redirect_uri
                                println!(
                                    "Previous callback port {} is in use, re-registering client...",
                                    port
                                );
                                auth_store.remove_client(auth_server);
                                let (listener, uri, client) = self
                                    .register_new_client(
                                        &auth_metadata.registration_endpoint,
                                        auth_server,
                                        &mut auth_store,
                                    )
                                    .await?;
                                (listener, uri, client)
                            }
                        }
                    } else {
                        // Invalid stored redirect_uri - re-register
                        auth_store.remove_client(auth_server);
                        let (listener, uri, client) = self
                            .register_new_client(
                                &auth_metadata.registration_endpoint,
                                auth_server,
                                &mut auth_store,
                            )
                            .await?;
                        (listener, uri, client)
                    }
                } else {
                    // No stored redirect_uri (legacy client) - re-register
                    auth_store.remove_client(auth_server);
                    let (listener, uri, client) = self
                        .register_new_client(
                            &auth_metadata.registration_endpoint,
                            auth_server,
                            &mut auth_store,
                        )
                        .await?;
                    (listener, uri, client)
                }
            } else {
                // No stored client - register new one
                let (listener, uri, client) = self
                    .register_new_client(
                        &auth_metadata.registration_endpoint,
                        auth_server,
                        &mut auth_store,
                    )
                    .await?;
                (listener, uri, client)
            };

        // Generate PKCE
        let (code_verifier, code_challenge) = generate_pkce();
        let state = generate_state();

        // Build authorization URL
        let scopes = if resource_metadata.scopes_supported.is_empty() {
            "read write".to_string()
        } else {
            resource_metadata.scopes_supported.join(" ")
        };

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            auth_metadata.authorization_endpoint,
            urlencoding::encode(&client.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge),
        );

        println!("Opening browser for authentication...\n");

        // Open browser automatically
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&auth_url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(&auth_url)
            .spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", &auth_url])
            .spawn();

        println!("If the browser didn't open, visit:\n  {}\n", auth_url);
        println!("Waiting for authorization...");

        // Wait for callback
        let (mut stream, _) = listener.accept().context("Failed to accept callback")?;
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        // Parse the authorization code from callback
        let code = parse_callback(&request_line, &state)?;

        // Send success response to browser
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication successful!</h1><p>You can close this window.</p></body></html>";
        stream.write_all(response.as_bytes())?;

        // Exchange code for tokens
        println!("Exchanging code for tokens...");
        let token_response = self
            .exchange_code(
                &auth_metadata.token_endpoint,
                &code,
                &client.client_id,
                client.client_secret.as_deref(),
                &redirect_uri,
                &code_verifier,
            )
            .await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let stored_token = StoredToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: token_response.expires_in.map(|e| now + e),
            token_type: token_response.token_type,
        };

        // Store token
        auth_store.set_token(self.server_name.clone(), stored_token.clone());
        auth_store.save()?;

        Ok(stored_token)
    }

    /// Run OAuth flow using auth server metadata URL directly
    pub async fn authenticate_with_auth_server(
        &self,
        auth_server_metadata_url: &str,
    ) -> Result<StoredToken> {
        println!("Fetching authorization server metadata...");
        let auth_metadata = self
            .client
            .get(auth_server_metadata_url)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch auth server metadata from {}",
                    auth_server_metadata_url
                )
            })?;

        if !auth_metadata.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch auth server metadata: HTTP {}",
                auth_metadata.status()
            ));
        }

        let auth_metadata: AuthServerMetadata = auth_metadata
            .json()
            .await
            .context("Failed to parse auth server metadata")?;

        // Extract issuer as auth server identifier
        let auth_server = &auth_metadata.issuer;

        // Get or register client (need to handle redirect_uri matching)
        let mut auth_store = AuthStore::load()?;

        // Try to use stored client with its original redirect_uri
        let (listener, redirect_uri, client) =
            if let Some(stored) = auth_store.get_client(auth_server) {
                if let Some(stored_uri) = &stored.redirect_uri {
                    // Extract port from stored redirect_uri and try to bind to it
                    if let Some(port) = extract_port_from_uri(stored_uri) {
                        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
                            Ok(listener) => {
                                // Successfully bound to the same port
                                (listener, stored_uri.clone(), stored.clone())
                            }
                            Err(_) => {
                                // Port in use - need to re-register with new redirect_uri
                                println!(
                                    "Previous callback port {} is in use, re-registering client...",
                                    port
                                );
                                auth_store.remove_client(auth_server);
                                let (listener, uri, client) = self
                                    .register_new_client(
                                        &auth_metadata.registration_endpoint,
                                        auth_server,
                                        &mut auth_store,
                                    )
                                    .await?;
                                (listener, uri, client)
                            }
                        }
                    } else {
                        // Invalid stored redirect_uri - re-register
                        auth_store.remove_client(auth_server);
                        let (listener, uri, client) = self
                            .register_new_client(
                                &auth_metadata.registration_endpoint,
                                auth_server,
                                &mut auth_store,
                            )
                            .await?;
                        (listener, uri, client)
                    }
                } else {
                    // No stored redirect_uri (legacy client) - re-register
                    auth_store.remove_client(auth_server);
                    let (listener, uri, client) = self
                        .register_new_client(
                            &auth_metadata.registration_endpoint,
                            auth_server,
                            &mut auth_store,
                        )
                        .await?;
                    (listener, uri, client)
                }
            } else {
                // No stored client - register new one
                let (listener, uri, client) = self
                    .register_new_client(
                        &auth_metadata.registration_endpoint,
                        auth_server,
                        &mut auth_store,
                    )
                    .await?;
                (listener, uri, client)
            };

        // Generate PKCE
        let (code_verifier, code_challenge) = generate_pkce();
        let state = generate_state();

        // Build authorization URL
        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&state={}&code_challenge={}&code_challenge_method=S256",
            auth_metadata.authorization_endpoint,
            urlencoding::encode(&client.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge),
        );

        println!("Opening browser for authentication...\n");

        // Open browser automatically
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&auth_url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(&auth_url)
            .spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", &auth_url])
            .spawn();

        println!("If the browser didn't open, visit:\n  {}\n", auth_url);
        println!("Waiting for authorization...");

        // Wait for callback
        let (mut stream, _) = listener.accept().context("Failed to accept callback")?;
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        // Parse the authorization code from callback
        let code = parse_callback(&request_line, &state)?;

        // Send success response to browser
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Authentication successful!</h1><p>You can close this window.</p></body></html>";
        stream.write_all(response.as_bytes())?;

        // Exchange code for tokens
        println!("Exchanging code for tokens...");
        let token_response = self
            .exchange_code(
                &auth_metadata.token_endpoint,
                &code,
                &client.client_id,
                client.client_secret.as_deref(),
                &redirect_uri,
                &code_verifier,
            )
            .await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let stored_token = StoredToken {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: token_response.expires_in.map(|e| now + e),
            token_type: token_response.token_type,
        };

        // Store token
        auth_store.set_token(self.server_name.clone(), stored_token.clone());
        auth_store.save()?;

        Ok(stored_token)
    }
}

fn parse_callback(request_line: &str, expected_state: &str) -> Result<String> {
    // Parse: GET /callback?code=xxx&state=yyy HTTP/1.1
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("Invalid callback request"))?;

    let query = path
        .split('?')
        .nth(1)
        .ok_or_else(|| anyhow!("No query string in callback"))?;

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let value = urlencoding::decode(value).unwrap_or_default().to_string();

        match key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            "error_description" => error_description = Some(value),
            _ => {}
        }
    }

    if let Some(err) = error {
        return Err(anyhow!(
            "Authorization failed: {} - {}",
            err,
            error_description.unwrap_or_default()
        ));
    }

    let state = state.ok_or_else(|| anyhow!("Missing state in callback"))?;
    if state != expected_state {
        return Err(anyhow!("State mismatch - possible CSRF attack"));
    }

    code.ok_or_else(|| anyhow!("Missing authorization code in callback"))
}
