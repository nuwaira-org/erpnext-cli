use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// A single ERPNext connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base URL of the ERPNext instance (e.g. "https://erp.example.com").
    #[serde(default)]
    pub url: String,

    /// Authentication type: "token" or "password".
    #[serde(default = "default_auth_type")]
    pub auth_type: String,

    /// API key (for token auth).
    #[serde(default)]
    pub api_key: String,

    /// API secret (for token auth).
    #[serde(default)]
    pub api_secret: String,

    /// Session ID cookie (for password auth).
    #[serde(default)]
    pub session_id: String,

    /// HTTP request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_auth_type() -> String {
    "token".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl Config {
    /// Load configuration from the default config file path,
    /// then apply environment variable overrides.
    pub fn load() -> Result<Self, crate::error::ClientError> {
        let mut config = Self::load_file().unwrap_or_default();
        config.merge_env();
        Ok(config)
    }

    fn load_file() -> Option<Self> {
        let path = config_path()?;
        let content = fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Merge environment variables: ERPNEXT_URL, ERPNEXT_TOKEN, ERPNEXT_API_KEY, ERPNEXT_API_SECRET
    fn merge_env(&mut self) {
        if let Ok(url) = env::var("ERPNEXT_URL") {
            self.url = url;
        }
        if let Ok(token) = env::var("ERPNEXT_TOKEN") {
            if let Some((key, secret)) = token.split_once(':') {
                self.api_key = key.to_string();
                self.api_secret = secret.to_string();
                self.auth_type = "token".to_string();
            }
        }
        if let Ok(key) = env::var("ERPNEXT_API_KEY") {
            self.api_key = key;
        }
        if let Ok(secret) = env::var("ERPNEXT_API_SECRET") {
            self.api_secret = secret;
        }
    }

    /// Write the configuration to disk.
    pub fn save(&self) -> Result<(), crate::error::ClientError> {
        let path = config_path().ok_or_else(|| {
            crate::error::ClientError::Config("could not determine config directory".into())
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::error::ClientError::Config(format!("failed to create config directory: {e}"))
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            crate::error::ClientError::Config(format!("failed to serialize config: {e}"))
        })?;

        fs::write(&path, content).map_err(|e| {
            crate::error::ClientError::Config(format!("failed to write config: {e}"))
        })?;

        Ok(())
    }

    /// Check if the config has enough information to make API calls.
    pub fn is_ready(&self) -> bool {
        if self.url.is_empty() {
            return false;
        }
        match self.auth_type.as_str() {
            "token" => !self.api_key.is_empty() && !self.api_secret.is_empty(),
            "password" => !self.session_id.is_empty(),
            _ => false,
        }
    }

    /// Check if password-based login can be attempted (URL configured).
    pub fn can_login(&self) -> bool {
        !self.url.is_empty() && self.auth_type == "password"
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: String::new(),
            auth_type: default_auth_type(),
            api_key: String::new(),
            api_secret: String::new(),
            session_id: String::new(),
            timeout_secs: default_timeout(),
        }
    }
}

/// Path to the configuration file: ~/.config/erpnext-cli/config.toml
fn config_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("ERPNEXT_CONFIG_FILE") {
        return Some(PathBuf::from(path));
    }
    dirs::config_dir().map(|d| d.join("erpnext-cli").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_not_ready() {
        let config = Config::default();
        assert!(!config.is_ready());
    }

    #[test]
    fn config_with_url_and_token_is_ready() {
        let config = Config {
            url: "https://erp.example.com".into(),
            auth_type: "token".into(),
            api_key: "key123".into(),
            api_secret: "secret456".into(),
            session_id: String::new(),
            timeout_secs: 30,
        };
        assert!(config.is_ready());
    }

    #[test]
    fn config_with_url_and_session_is_ready() {
        let config = Config {
            url: "https://erp.example.com".into(),
            auth_type: "password".into(),
            api_key: String::new(),
            api_secret: String::new(),
            session_id: "sid123".into(),
            timeout_secs: 30,
        };
        assert!(config.is_ready());
        assert!(config.can_login());
    }

    #[test]
    fn config_with_missing_key_is_not_ready() {
        let config = Config {
            url: "https://erp.example.com".into(),
            auth_type: "token".into(),
            api_key: "key123".into(),
            api_secret: String::new(),
            session_id: String::new(),
            timeout_secs: 30,
        };
        assert!(!config.is_ready());
    }

    #[test]
    fn env_token_parses_correctly() {
        let mut config = Config::default();
        let token = "mykey:mysecret";
        if let Some((key, secret)) = token.split_once(':') {
            config.api_key = key.to_string();
            config.api_secret = secret.to_string();
        }
        assert_eq!(config.api_key, "mykey");
        assert_eq!(config.api_secret, "mysecret");
    }

    #[test]
    fn roundtrip_toml() {
        let config = Config {
            url: "https://erp.example.com".into(),
            auth_type: "token".into(),
            api_key: "abc".into(),
            api_secret: "xyz".into(),
            session_id: String::new(),
            timeout_secs: 45,
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.url, config.url);
        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.timeout_secs, config.timeout_secs);
    }
}
