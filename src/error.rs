//! Error types for the Android SSH MCP Server

use thiserror::Error;

/// Errors that can occur in the SSH MCP server
#[derive(Error, Debug)]
pub enum SshMcpError {
    #[error("SSH connection failed: {0}")]
    SshConnection(String),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Convenience Result type that uses SshMcpError as the error type
pub type Result<T> = std::result::Result<T, SshMcpError>;
