use crate::error::{Result, SshMcpError};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let host = std::env::var("ANDROID_SSH_HOST")
            .map_err(|_| SshMcpError::Config("ANDROID_SSH_HOST not set".to_string()))?;

        let port = std::env::var("ANDROID_SSH_PORT")
            .unwrap_or_else(|_| "22".to_string())
            .parse::<u16>()
            .map_err(|e| SshMcpError::Config(format!("Invalid port: {}", e)))?;

        let username = std::env::var("ANDROID_SSH_USER")
            .map_err(|_| SshMcpError::Config("ANDROID_SSH_USER not set".to_string()))?;

        let password = std::env::var("ANDROID_SSH_PASSWORD").ok();

        let key_path = std::env::var("ANDROID_SSH_KEY_PATH")
            .ok()
            .map(|p| shellexpand::tilde(&p).to_string())
            .map(PathBuf::from);

        // Validate we have at least one auth method
        if password.is_none() && key_path.is_none() {
            return Err(SshMcpError::Config(
                "Must provide either ANDROID_SSH_PASSWORD or ANDROID_SSH_KEY_PATH".to_string(),
            ));
        }

        // Warn if key file has incorrect permissions (600 recommended)
        if let Some(ref path) = key_path {
            if path.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(path) {
                        let permissions = metadata.permissions();
                        let mode = permissions.mode();
                        if mode & 0o777 != 0o600 {
                            tracing::warn!(
                                "SSH key file {:?} has permissions {:o}, recommended 600",
                                path,
                                mode & 0o777
                            );
                        }
                    }
                }
            } else {
                return Err(SshMcpError::Config(format!(
                    "SSH key file not found: {:?}",
                    path
                )));
            }
        }

        Ok(Config {
            host,
            port,
            username,
            password,
            key_path,
        })
    }
}
