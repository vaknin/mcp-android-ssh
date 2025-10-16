use crate::config::Config;
use crate::error::{Result, SshMcpError};
use russh::keys::{self, decode_secret_key, PublicKey};
use russh::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(2);

pub struct SshClient {
    config: Config,
    session: Option<client::Handle<ClientHandler>>,
}

impl SshClient {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            session: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut last_error = None;

        for attempt in 1..=MAX_RETRIES {
            match self.try_connect().await {
                Ok(session) => {
                    self.session = Some(session);
                    tracing::info!(
                        "Successfully connected to {}:{} (attempt {})",
                        self.config.host,
                        self.config.port,
                        attempt
                    );
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        tracing::warn!(
                            "Connection attempt {}/{} failed, retrying in {:?}",
                            attempt,
                            MAX_RETRIES,
                            RETRY_DELAY
                        );
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            SshMcpError::SshConnection("Failed to connect after retries".to_string())
        }))
    }

    async fn try_connect(&self) -> Result<client::Handle<ClientHandler>> {
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(60)),
            ..Default::default()
        });

        let handler = ClientHandler {};

        let mut session = client::connect(
            config,
            (self.config.host.as_str(), self.config.port),
            handler,
        )
        .await
        .map_err(|e| SshMcpError::SshConnection(format!("Connection failed: {}", e)))?;

        // Try authentication: key first, then password
        let auth_success = if let Some(ref key_path) = self.config.key_path {
            match self.try_key_auth(&mut session, key_path).await {
                Ok(success) if success => {
                    tracing::info!("Authenticated with SSH key");
                    true
                }
                Ok(_) => {
                    tracing::warn!("Key auth failed, trying password");
                    if let Some(ref password) = self.config.password {
                        self.try_password_auth(&mut session, password).await?
                    } else {
                        return Err(SshMcpError::Authentication(
                            "Key authentication failed and no password provided".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!("Key auth error: {}, trying password", e);
                    if let Some(ref password) = self.config.password {
                        self.try_password_auth(&mut session, password).await?
                    } else {
                        return Err(e);
                    }
                }
            }
        } else if let Some(ref password) = self.config.password {
            self.try_password_auth(&mut session, password).await?
        } else {
            return Err(SshMcpError::Authentication(
                "No authentication method available".to_string(),
            ));
        };

        if !auth_success {
            return Err(SshMcpError::Authentication(
                "Authentication failed".to_string(),
            ));
        }

        Ok(session)
    }

    async fn try_key_auth(
        &self,
        session: &mut client::Handle<ClientHandler>,
        key_path: &std::path::Path,
    ) -> Result<bool> {
        let key_pair = decode_secret_key(&std::fs::read_to_string(key_path)?, None)
            .map_err(|e| SshMcpError::Authentication(format!("Failed to load key: {}", e)))?;

        let key_with_hash = keys::PrivateKeyWithHashAlg::new(Arc::new(key_pair), None);

        let auth_result = session
            .authenticate_publickey(&self.config.username, key_with_hash)
            .await
            .map_err(|e| SshMcpError::Authentication(format!("Key auth failed: {}", e)))?;

        Ok(matches!(auth_result, client::AuthResult::Success))
    }

    async fn try_password_auth(
        &self,
        session: &mut client::Handle<ClientHandler>,
        password: &str,
    ) -> Result<bool> {
        let auth_result = session
            .authenticate_password(&self.config.username, password)
            .await
            .map_err(|e| SshMcpError::Authentication(format!("Password auth failed: {}", e)))?;

        let success = matches!(auth_result, client::AuthResult::Success);
        if success {
            tracing::info!("Authenticated with password");
        }

        Ok(success)
    }

    async fn ensure_connected(&mut self) -> Result<()> {
        // Check if session exists and is active
        if let Some(ref session) = self.session {
            if session.is_closed() {
                tracing::warn!("Session closed, reconnecting...");
                self.session = None;
                self.connect().await?;
            }
        } else {
            tracing::info!("No active session, connecting...");
            self.connect().await?;
        }

        Ok(())
    }

    pub async fn execute_command(
        &mut self,
        command: &str,
        timeout_secs: u64,
    ) -> Result<CommandResult> {
        self.ensure_connected().await?;

        let session = self
            .session
            .as_ref()
            .ok_or_else(|| SshMcpError::SshConnection("No active session".to_string()))?;

        let exec_timeout = Duration::from_secs(timeout_secs);

        let result = timeout(exec_timeout, self.exec_command_inner(session, command))
            .await
            .map_err(|_| {
                SshMcpError::Timeout(format!("Command timed out after {} seconds", timeout_secs))
            })??;

        Ok(result)
    }

    async fn exec_command_inner(
        &self,
        session: &client::Handle<ClientHandler>,
        command: &str,
    ) -> Result<CommandResult> {
        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|e| SshMcpError::CommandExecution(format!("Failed to open channel: {}", e)))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| SshMcpError::CommandExecution(format!("Failed to exec command: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

        // Collect output
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    stdout.extend_from_slice(&data);
                }
                ChannelMsg::ExtendedData { data, ext } => {
                    if ext == 1 {
                        // SSH_EXTENDED_DATA_STDERR
                        stderr.extend_from_slice(&data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = exit_status as i32;
                }
                ChannelMsg::Eof => {
                    break;
                }
                _ => {}
            }
        }

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            exit_code,
        })
    }

    #[allow(dead_code)]
    pub async fn disconnect(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session
                .disconnect(Disconnect::ByApplication, "", "en")
                .await;
            tracing::info!("Disconnected from SSH server");
        }
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct ClientHandler {}

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    #[allow(refining_impl_trait_reachable, clippy::manual_async_fn)]
    fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> impl std::future::Future<Output = std::result::Result<bool, Self::Error>> + Send + '_ {
        // Accept all server keys (similar to AutoAddPolicy in Python)
        // In production, you might want to verify against known_hosts
        async { Ok(true) }
    }
}
