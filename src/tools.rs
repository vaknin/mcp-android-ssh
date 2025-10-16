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
        let ssh_client = config.map(|cfg| SshClient::new(cfg));
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

fn default_timeout() -> u64 {
    30
}

#[tool_router]
impl AndroidSshService {
    #[tool(description = "Execute safe read-only shell commands on Android via SSH (81 whitelisted commands)")]
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

    #[tool(description = "Execute any shell command on Android via SSH, including write/modify/delete operations")]
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
}
