//! Android SSH MCP Server
//!
//! A high-performance MCP (Model Context Protocol) server written in Rust
//! that provides secure SSH access to Android devices.
//!
//! This server exposes two tools:
//! - `execute_read`: Execute whitelisted read-only commands
//! - `execute`: Execute any command (with user approval)
//!
//! The server communicates via JSON-RPC over stdin/stdout and is designed
//! to be run as a subprocess by MCP clients like Claude Code.

mod config;
mod error;
mod ssh;
mod tools;

use config::Config;
use rmcp::{
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler, ServerHandler, ServiceExt,
};
use ssh::SshClient;
use tools::AndroidSshService;

#[tokio::main]
async fn main() -> error::Result<()> {
    // Initialize logging to stderr (stdout is for JSON-RPC protocol)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Android SSH MCP Server starting...");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!(
        "Loaded config: host={}:{}, user={}",
        config.host,
        config.port,
        config.username
    );

    // Create SSH client and connect (fail fast on startup)
    let mut ssh_client = SshClient::new(config);
    ssh_client.connect().await?;
    tracing::info!("Initial SSH connection successful");

    // Create MCP service
    let service = AndroidSshService::new(ssh_client);

    // Serve on stdio
    tracing::info!("Starting MCP server on stdio...");
    let server = service
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| error::SshMcpError::Other(format!("Failed to start server: {}", e)))?;

    // Wait for completion
    server
        .waiting()
        .await
        .map_err(|e| error::SshMcpError::Other(format!("Server error: {}", e)))?;

    tracing::info!("Android SSH MCP Server shutting down");
    Ok(())
}

#[tool_handler]
impl ServerHandler for AndroidSshService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Android SSH MCP Server - Secure SSH access to Android devices.\n\n\
                Use execute_read for safe read-only commands (ls, cat, ps, etc.).\n\
                Use execute for commands that modify the system (rm, mkdir, curl, etc.).\n\n\
                ## execute_read Tool\n\
                Execute SAFE shell commands on Android via SSH. Whitelisted commands only - cannot write/delete.\n\
                Returns stdout, stderr, and exit code.\n\n\
                **Whitelisted commands (81 total):**\n\
                - File viewing: ls, cat, head, tail, less, more, grep, rg, find, fd, tree, bat, eza, exa, locate\n\
                - Path operations: cd, pwd, readlink, realpath, basename, dirname\n\
                - System info: whoami, id, groups, which, whereis, type, hostname, uname, date, uptime\n\
                - Display: echo, printf\n\
                - Process monitoring: ps, top, htop, btop, lsof\n\
                - Disk/filesystem: df, du, lsblk, blkid, stat, file\n\
                - Memory/performance: free, vmstat, iostat, iotop, lsmem, lshw, lscpu\n\
                - Network monitoring: netstat, ss, ping, traceroute, nslookup, dig, host\n\
                - Text processing: wc, sort, uniq, cut, paste, tr, column\n\
                - Comparison: diff, cmp, comm\n\
                - Checksums: md5sum, sha1sum, sha256sum, sha512sum\n\
                - Environment: env, printenv, getent, getconf\n\
                - Binary viewers: xxd, hexdump, od, strings\n\
                - Compressed viewers: zcat, bzcat, xzcat, gunzip, bunzip2, unxz\n\
                - Data parsers: jq, yq, xmllint\n\
                - Logs: journalctl\n\
                - Hardware: lsmod, modinfo, lspci, lsusb\n\
                - Shell: history, alias\n\
                - Fonts: fc-list, fc-match\n\
                - Test: test, true, false\n\n\
                **Examples:**\n\
                - List files: ls -la\n\
                - Read file: cat ~/.bashrc\n\
                - System info: uname -a\n\
                - Disk usage: df -h\n\
                - Running processes: ps aux\n\n\
                If a command isn't whitelisted, you'll get an error telling you to use 'execute' tool instead.\n\n\
                ## execute Tool\n\
                Execute ANY shell command on Android via SSH. Use for commands that write/modify/delete.\n\
                Returns stdout, stderr, and exit code.\n\n\
                **Use this for:**\n\
                - System diagnostics: dumpsys (Android system information)\n\
                - File operations: rm, mv, cp, mkdir, chmod, touch\n\
                - Package management: pkg install, apt install, npm install\n\
                - Downloads: curl, wget\n\
                - Git operations: git clone, git pull, git commit\n\
                - Service management: systemctl start/stop\n\
                - File writing: echo > file, cat > file\n\n\
                **Examples:**\n\
                - System diagnostics: dumpsys package com.termux\n\
                - Create directory: mkdir ~/newdir\n\
                - Remove file: rm oldfile.txt\n\
                - Install package: pkg install git\n\
                - Write file: echo 'content' > file.txt\n\
                - Download: curl -O https://example.com/file\n\n\
                **IMPORTANT:** Always prefer execute_read for safe commands (ls, cat, ps, grep, etc.).\n\n\
                ## Command Timeout\n\
                Both tools accept an optional 'timeout' parameter (1-300 seconds, default: 30).\n\
                Use longer timeouts for package installations or long-running operations."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}
