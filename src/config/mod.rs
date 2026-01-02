mod store;
mod types;

pub use store::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            servers: [
                ("linear".to_string(), ServerConfig {
                    transport: TransportConfig::Stdio {
                        command: "npx @linear/mcp-server".to_string(),
                    },
                    env: [("LINEAR_API_KEY".to_string(), "${env:LINEAR_API_KEY}".to_string())]
                        .into_iter()
                        .collect(),
                }),
            ]
            .into_iter()
            .collect(),
            default_server: Some("linear".to_string()),
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(parsed.default_server, Some("linear".to_string()));
        assert!(parsed.servers.contains_key("linear"));
    }
}
