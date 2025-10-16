use crate::error::{Result, SshMcpError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = "mcp-android-ssh";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
}

fn default_port() -> u16 {
    8022
}

impl Config {
    /// Get the config directory path (~/.config/mcp-android-ssh)
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|p| p.join(CONFIG_DIR_NAME))
            .ok_or_else(|| SshMcpError::Config("Cannot determine config directory".to_string()))
    }

    /// Get the config file path (~/.config/mcp-android-ssh/config.toml)
    pub fn config_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(CONFIG_FILE_NAME))
    }

    /// Create config directory and template if they don't exist
    /// Returns Ok(Some(path)) if config exists or was created successfully
    /// Returns Ok(None) if config was just created and needs to be edited
    pub fn ensure_config_exists() -> Result<Option<PathBuf>> {
        let config_path = Self::config_file_path()?;

        if !config_path.exists() {
            let config_dir = Self::config_dir()?;
            std::fs::create_dir_all(&config_dir).map_err(|e| {
                SshMcpError::Config(format!("Failed to create config directory: {}", e))
            })?;

            let template = Self::default_template();
            std::fs::write(&config_path, template).map_err(|e| {
                SshMcpError::Config(format!("Failed to create config template: {}", e))
            })?;

            tracing::info!("Created config template at: {}", config_path.display());
            return Ok(None);
        }

        Ok(Some(config_path))
    }

    /// Generate default config template
    fn default_template() -> String {
        let example = Config {
            host: "192.168.1.100".to_string(),
            port: 8022,
            user: "u0_a555".to_string(),
            password: None,
            key_path: Some("~/.ssh/id_ed25519".to_string()),
        };

        format!(
            "# Android SSH MCP Server Configuration\n\
             # Edit with your Android device credentials\n\
             \n\
             # Connection Settings\n\
             {}\
             \n\
             # Authentication (choose one method)\n\
             # key_path = \"~/.ssh/id_ed25519\"  # Recommended: SSH key auth\n\
             # password = \"your_password\"       # Alternative: password auth\n\
             \n\
             # Quick Setup:\n\
             # 1. Find your device IP: Run 'ip -4 addr show wlan0' in Termux\n\
             # 2. Find your username: Run 'whoami' in Termux\n\
             # 3. Generate SSH key: ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N \"\"\n\
             # 4. Copy to device: ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub USER@HOST\n\
             # 5. Update host and user above\n",
            toml::to_string_pretty(&example)
                .unwrap()
                .lines()
                .map(|line| {
                    if line.starts_with("host") {
                        format!("{}  # Find with: ip -4 addr show wlan0 (in Termux)\n", line)
                    } else if line.starts_with("port") {
                        format!("{}             # Termux SSH default\n", line)
                    } else if line.starts_with("user") {
                        format!("{}        # Find with: whoami (in Termux)\n", line)
                    } else {
                        format!("{}\n", line)
                    }
                })
                .collect::<String>()
        )
    }

    /// Load configuration from file with environment variable overrides
    /// Returns Ok(None) if config doesn't exist yet (first run)
    pub fn load() -> Result<Option<Self>> {
        let config_path = match Self::ensure_config_exists()? {
            Some(path) => path,
            None => return Ok(None), // Config template created, needs editing
        };

        // Read and parse TOML
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            SshMcpError::Config(format!("Failed to read config file: {}", e))
        })?;

        let mut config: Config = toml::from_str(&content).map_err(|e| {
            SshMcpError::Config(format!("Failed to parse config file: {}", e))
        })?;

        // Environment variables override config file
        if let Ok(host) = std::env::var("ANDROID_SSH_HOST") {
            config.host = host;
        }
        if let Ok(port) = std::env::var("ANDROID_SSH_PORT") {
            config.port = port.parse().map_err(|e| {
                SshMcpError::Config(format!("Invalid ANDROID_SSH_PORT: {}", e))
            })?;
        }
        if let Ok(user) = std::env::var("ANDROID_SSH_USER") {
            config.user = user;
        }
        if let Ok(password) = std::env::var("ANDROID_SSH_PASSWORD") {
            config.password = Some(password);
        }
        if let Ok(key_path) = std::env::var("ANDROID_SSH_KEY_PATH") {
            config.key_path = Some(key_path);
        }

        // Validate configuration
        config.validate()?;

        Ok(Some(config))
    }

    /// Generate a helpful first-run error message
    pub fn first_run_message() -> String {
        let config_path = Self::config_file_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.config/mcp-android-ssh/config.toml".to_string());

        format!(
            "Configuration Setup Required\n\n\
             Config file created at: {}\n\n\
             Please edit this file with your Android device credentials:\n\
             - host: Your device IP (run 'ip -4 addr show wlan0' in Termux)\n\
             - user: Your Termux username (run 'whoami' in Termux)\n\
             - key_path: Path to SSH key (recommended: ~/.ssh/id_ed25519)\n\
             - password: Only if not using key auth\n\n\
             Quick SSH key setup:\n\
             1. ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N \"\"\n\
             2. ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub USER@HOST\n\
             3. Update config file with your credentials\n\n\
             Alternatively, set environment variables:\n\
             ANDROID_SSH_HOST, ANDROID_SSH_USER, ANDROID_SSH_KEY_PATH\n\n\
             Full setup guide: https://github.com/vaknin/mcp-android-ssh#setup",
            config_path
        )
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Must have at least one auth method
        if self.password.is_none() && self.key_path.is_none() {
            return Err(SshMcpError::Config(
                "Must provide either 'password' or 'key_path' for authentication".to_string(),
            ));
        }

        // If key_path is provided, expand tilde and validate
        if let Some(ref key_path) = self.key_path {
            let expanded_path = PathBuf::from(shellexpand::tilde(key_path).to_string());

            if !expanded_path.exists() {
                return Err(SshMcpError::Config(format!(
                    "SSH key file not found: {}",
                    expanded_path.display()
                )));
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&expanded_path) {
                    let mode = metadata.permissions().mode();
                    if mode & 0o777 != 0o600 {
                        tracing::warn!(
                            "SSH key file has permissions {:o}, recommended 600: {}",
                            mode & 0o777,
                            expanded_path.display()
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the expanded key path (with ~ replaced)
    pub fn expanded_key_path(&self) -> Option<PathBuf> {
        self.key_path
            .as_ref()
            .map(|p| PathBuf::from(shellexpand::tilde(p).to_string()))
    }
}
