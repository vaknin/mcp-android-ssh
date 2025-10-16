use crate::ssh::SshClient;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ErrorData as McpError},
    schemars::JsonSchema,
    tool, tool_router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

// Read-only commands whitelist (81 commands from Python implementation)
const READ_ONLY_COMMANDS: &[&str] = &[
    // File viewing
    "ls",
    "cat",
    "head",
    "tail",
    "less",
    "more",
    "grep",
    "rg",
    "find",
    "fd",
    "tree",
    "bat",
    "eza",
    "exa",
    "locate",
    // Path operations
    "cd",
    "pwd",
    "readlink",
    "realpath",
    "basename",
    "dirname",
    // Identity/system info
    "whoami",
    "id",
    "groups",
    "which",
    "whereis",
    "type",
    "hostname",
    "uname",
    "date",
    "uptime",
    // Display/output
    "echo",
    "printf",
    // Process monitoring
    "ps",
    "top",
    "htop",
    "btop",
    "lsof",
    // Disk/filesystem info
    "df",
    "du",
    "lsblk",
    "blkid",
    "stat",
    "file",
    // Memory/performance monitoring
    "free",
    "vmstat",
    "iostat",
    "iotop",
    "lsmem",
    "lshw",
    "lscpu",
    // Network monitoring
    "netstat",
    "ss",
    "ping",
    "traceroute",
    "nslookup",
    "dig",
    "host",
    // Text processing
    "wc",
    "sort",
    "uniq",
    "cut",
    "paste",
    "tr",
    "column",
    // Comparison tools
    "diff",
    "cmp",
    "comm",
    // Checksums
    "md5sum",
    "sha1sum",
    "sha256sum",
    "sha512sum",
    // Environment info
    "env",
    "printenv",
    "getent",
    "getconf",
    // Binary/hex viewers
    "xxd",
    "hexdump",
    "od",
    "strings",
    // Compressed file viewers
    "zcat",
    "bzcat",
    "xzcat",
    "gunzip",
    "bunzip2",
    "unxz",
    // Data parsers
    "jq",
    "yq",
    "xmllint",
    // Log viewing
    "journalctl",
    // Hardware/module info
    "lsmod",
    "modinfo",
    "lspci",
    "lsusb",
    // Shell info
    "history",
    "alias",
    // Font info
    "fc-list",
    "fc-match",
    // Test/null commands
    "test",
    "true",
    "false",
];

fn is_read_only(command: &str) -> bool {
    let cmd = command.split_whitespace().next().unwrap_or("");
    READ_ONLY_COMMANDS.contains(&cmd)
}

#[derive(Clone)]
pub struct AndroidSshService {
    pub(crate) ssh_client: Arc<Mutex<Option<SshClient>>>,
    pub tool_router: ToolRouter<Self>,
}

impl AndroidSshService {
    pub fn new(config: Option<crate::config::Config>) -> Self {
        let ssh_client = config.map(SshClient::new);
        Self {
            ssh_client: Arc::new(Mutex::new(ssh_client)),
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecuteRequest {
    /// The shell command to execute
    pub command: String,
    /// Command timeout in seconds (default: 30, max: 300)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetupRequest {
    /// Android device IP address (e.g., 192.168.1.100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// SSH port (default: 8022 for Termux)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Termux username (run 'whoami' in Termux)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Path to SSH private key (recommended, e.g., ~/.ssh/id_ed25519)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    /// SSH password (alternative to key_path, not recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

fn default_timeout() -> u64 {
    30
}

#[tool_router]
impl AndroidSshService {
    #[tool(
        description = "Execute safe read-only shell commands on Android via SSH (81 whitelisted commands)"
    )]
    async fn execute_read(
        &self,
        Parameters(request): Parameters<ExecuteRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if client exists (config was loaded)
        let mut client_guard = self.ssh_client.lock().await;
        if client_guard.is_none() {
            return Ok(CallToolResult::error(vec![Content::text(
                crate::config::Config::first_run_message(),
            )]));
        }

        // Validate timeout
        if request.timeout == 0 || request.timeout > 300 {
            return Ok(CallToolResult::error(vec![Content::text(
                "Timeout must be between 1 and 300 seconds".to_string(),
            )]));
        }

        // Check whitelist
        if !is_read_only(&request.command) {
            let cmd_name = request.command.split_whitespace().next().unwrap_or("");
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Command '{}' is not whitelisted as read-only. Use execute tool instead.",
                cmd_name
            ))]));
        }

        // Execute command
        let client = client_guard.as_mut().unwrap();
        match client
            .execute_command(&request.command, request.timeout)
            .await
        {
            Ok(result) => {
                // Format output nicely
                let mut output = String::new();

                // Add stdout if present
                if !result.stdout.is_empty() {
                    output.push_str(&result.stdout);
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }

                // Add stderr if present
                if !result.stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str("stderr:\n");
                    output.push_str(&result.stderr);
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }

                // Always show status line
                if !output.is_empty() {
                    output.push('\n');
                }

                if result.exit_code == 0 {
                    output.push_str("✓ Success");
                } else {
                    output.push_str(&format!("✗ Failed (exit code: {})", result.exit_code));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Command execution failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Execute any shell command on Android via SSH, including write/modify/delete operations"
    )]
    async fn execute(
        &self,
        Parameters(request): Parameters<ExecuteRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if client exists (config was loaded)
        let mut client_guard = self.ssh_client.lock().await;
        if client_guard.is_none() {
            return Ok(CallToolResult::error(vec![Content::text(
                crate::config::Config::first_run_message(),
            )]));
        }

        // Validate timeout
        if request.timeout == 0 || request.timeout > 300 {
            return Ok(CallToolResult::error(vec![Content::text(
                "Timeout must be between 1 and 300 seconds".to_string(),
            )]));
        }

        // Execute command
        let client = client_guard.as_mut().unwrap();
        match client
            .execute_command(&request.command, request.timeout)
            .await
        {
            Ok(result) => {
                // Format output nicely
                let mut output = String::new();

                // Add stdout if present
                if !result.stdout.is_empty() {
                    output.push_str(&result.stdout);
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }

                // Add stderr if present
                if !result.stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str("stderr:\n");
                    output.push_str(&result.stderr);
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }

                // Always show status line
                if !output.is_empty() {
                    output.push('\n');
                }

                if result.exit_code == 0 {
                    output.push_str("✓ Success");
                } else {
                    output.push_str(&format!("✗ Failed (exit code: {})", result.exit_code));
                }

                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Command execution failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Configure Android SSH connection - provide credentials to connect to your Android device"
    )]
    async fn setup(
        &self,
        Parameters(request): Parameters<SetupRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Try to load existing config, or create empty one
        let existing_config = crate::config::Config::load_existing().ok();

        // Merge with provided values
        let host = request
            .host
            .or_else(|| existing_config.as_ref().map(|c| c.host.clone()));
        let port = request
            .port
            .or_else(|| existing_config.as_ref().map(|c| c.port));
        let user = request
            .user
            .or_else(|| existing_config.as_ref().map(|c| c.user.clone()));
        let key_path = request
            .key_path
            .or_else(|| existing_config.as_ref().and_then(|c| c.key_path.clone()));
        let password = request
            .password
            .or_else(|| existing_config.as_ref().and_then(|c| c.password.clone()));

        // Check what's missing
        let mut missing = Vec::new();
        if host.is_none() {
            missing.push("host");
        }
        if user.is_none() {
            missing.push("user");
        }
        if key_path.is_none() && password.is_none() {
            missing.push("key_path or password");
        }

        // If anything is missing, return helpful message
        if !missing.is_empty() {
            let mut msg = String::from("Setup incomplete. Missing:\n\n");

            if missing.contains(&"host") {
                msg.push_str("• host - Your Android device IP\n");
                msg.push_str("  Find it: Run 'ifconfig wlan0' in Termux\n\n");
            }

            if missing.contains(&"user") {
                msg.push_str("• user - Your Termux username\n");
                msg.push_str("  Find it: Run 'whoami' in Termux\n\n");
            }

            if missing.contains(&"key_path or password") {
                msg.push_str("• Authentication - Choose one:\n");
                msg.push_str("  SSH key (recommended):\n");
                msg.push_str("    Generate: ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519\n");
                msg.push_str(
                    "    Copy to device: ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub USER@HOST\n",
                );
                msg.push_str("    Then provide: key_path = \"~/.ssh/id_ed25519\"\n\n");
                msg.push_str("  OR password (less secure):\n");
                msg.push_str("    Set Termux password: Run 'passwd' in Termux\n");
                msg.push_str("    Then provide: password = \"your_password\"\n\n");
            }

            if let Some(ref h) = host {
                msg.push_str(&format!("Current: host = \"{}\"\n", h));
            }
            if let Some(ref u) = user {
                msg.push_str(&format!("Current: user = \"{}\"\n", u));
            }
            if let Some(ref k) = key_path {
                msg.push_str(&format!("Current: key_path = \"{}\"\n", k));
            }
            if password.is_some() {
                msg.push_str("Current: password = \"***\"\n");
            }

            return Ok(CallToolResult::error(vec![Content::text(msg)]));
        }

        // All required fields present - create config
        let config = crate::config::Config {
            host: host.unwrap(),
            port: port.unwrap_or(8022),
            user: user.unwrap(),
            password,
            key_path,
        };

        // Save config
        match crate::config::Config::save(&config) {
            Ok(path) => {
                let msg = format!(
                    "✓ Configuration saved to: {}\n\n\
                     Connection details:\n\
                     • Host: {}:{}\n\
                     • User: {}\n\
                     • Auth: {}\n\n\
                     To activate, restart the MCP server:\n\
                     1. Type /mcp\n\
                     2. Find mcp-android-ssh in the list\n\
                     3. Click restart\n\n\
                     Then try: \"list files in /sdcard\"",
                    path.display(),
                    config.host,
                    config.port,
                    config.user,
                    if config.key_path.is_some() {
                        "SSH key"
                    } else {
                        "Password"
                    }
                );
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to save config: {}",
                e
            ))])),
        }
    }
}
