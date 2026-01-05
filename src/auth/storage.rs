use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Stored OAuth tokens for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub token_type: String,
}

/// OAuth client registration for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredClient {
    pub client_id: String,
    pub client_secret: Option<String>,
    /// The redirect_uri used during registration (needed for subsequent auth flows)
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

/// All auth data for servers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthStore {
    /// Tokens by server name
    pub tokens: HashMap<String, StoredToken>,
    /// Client registrations by authorization server URL
    pub clients: HashMap<String, StoredClient>,
}

impl AuthStore {
    fn path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let config_dir = PathBuf::from(home).join(".config").join("relay");
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
        Ok(config_dir.join("auth.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read auth store from {:?}", path))?;
        let store: AuthStore =
            serde_json::from_str(&contents).with_context(|| "Failed to parse auth store")?;
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize auth store")?;
        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write auth store to {:?}", path))?;
        Ok(())
    }

    pub fn get_token(&self, server_name: &str) -> Option<&StoredToken> {
        self.tokens.get(server_name)
    }

    pub fn set_token(&mut self, server_name: String, token: StoredToken) {
        self.tokens.insert(server_name, token);
    }

    pub fn remove_token(&mut self, server_name: &str) {
        self.tokens.remove(server_name);
    }

    pub fn get_client(&self, auth_server: &str) -> Option<&StoredClient> {
        self.clients.get(auth_server)
    }

    pub fn set_client(&mut self, auth_server: String, client: StoredClient) {
        self.clients.insert(auth_server, client);
    }

    pub fn remove_client(&mut self, auth_server: &str) {
        self.clients.remove(auth_server);
    }

    #[allow(dead_code)]
    pub fn is_token_expired(&self, server_name: &str) -> bool {
        if let Some(token) = self.tokens.get(server_name) {
            if let Some(expires_at) = token.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                // Consider expired if within 5 minutes of expiry
                return now + 300 >= expires_at;
            }
        }
        false
    }
}
