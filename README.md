<div align="center">

# Android SSH MCP Server

**Control your Android device from AI assistants via SSH**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/mcp-android-ssh.svg)](https://crates.io/crates/mcp-android-ssh)

A high-performance [MCP](https://modelcontextprotocol.io/) server written in Rust that provides secure SSH access to Android devices through Termux.

<img src="assets/usage-example.png" alt="Usage Example" width="63%">

*Example: AI assistant listing files and exploring directories on Android*

</div>

---

## Why Use This?

Turn your Android device into a programmable computer that AI can interact with naturally. Instead of manually SSH'ing into your phone every time you need to check something or run a command, just ask Claude:

- "Show me my recent photos"
- "Check if my Termux background service is running"
- "Download this paper to my tablet"
- "Find all PDFs on my device larger than 10MB"

**vs. Manual SSH:** While you *could* SSH manually every time, this MCP server provides:
- **Natural language interface** - No need to remember SSH commands or device IPs
- **Persistent connection** - Auto-reconnects when dropped, handles network issues
- **Safety guardrails** - Read-only operations require no approval, write operations are explicit
- **Zero context switching** - Stay in your Claude conversation, don't break flow
- **Cross-device workflows** - Combine Android commands with other MCP tools seamlessly

## Features

- **Zero-config first run** - Creates config template automatically on first use
- **Lightweight & fast** - 401 KiB binary, memory-safe Rust implementation
- **Smart safety** - 81 whitelisted read-only commands run freely, writes require explicit tool
- **Bulletproof connectivity** - Auto-reconnect with retry logic, handles network drops
- **Flexible auth** - SSH key (recommended) or password authentication
- **Privacy-first** - Local-only connection, no data leaves your network

## Quick Start

### TL;DR (Experienced Users)

```bash
# Install
cargo install mcp-android-ssh
claude mcp add --scope user --transport stdio mcp-android-ssh mcp-android-ssh

# Setup Termux (on Android)
pkg install openssh && sshd

# Copy your SSH key
ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub YOUR_USER@YOUR_ANDROID_IP

# Ask Claude anything about your Android device!
```

The server will create `~/.config/mcp-android-ssh/config.toml` on first use - just edit with your device credentials.

<details>
<summary><b>Detailed Setup Guide</b> (click to expand)</summary>

## Detailed Setup

### Installation & Configuration

**Step 1: Install and add to Claude Code**

```bash
cargo install mcp-android-ssh
claude mcp add --scope user --transport stdio mcp-android-ssh mcp-android-ssh
```

**Step 2: Setup your Android device**

In Termux on your Android device:
```bash
# Install and start SSH server
pkg update && pkg install openssh && sshd

# Find your username and IP address
whoami          # Example: u0_a555
ifconfig wlan0  # Example: 192.168.1.100
```

**Step 3: Setup SSH key authentication (recommended)**

On your host machine:
```bash
# Generate SSH key (skip if you already have one)
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N ""

# Copy key to Android (replace with your username and IP)
ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub u0_a555@192.168.1.100
```

**Step 4: Configure the server**

On first use, when you ask Claude to interact with your Android device, the server will create a config template at `~/.config/mcp-android-ssh/config.toml` with instructions. Edit it with your credentials:

```toml
host = "192.168.1.100"        # Your Android device IP
port = 8022                    # SSH port (default 8022 for Termux)
user = "u0_a555"               # Your Termux username
key_path = "~/.ssh/id_ed25519" # Path to your SSH private key

# Optional: password authentication (not recommended)
# password = "your_password"
```

**Alternative: Use environment variables**

You can also configure via environment variables (useful for testing):
```bash
export ANDROID_SSH_HOST=192.168.1.100
export ANDROID_SSH_USER=u0_a555
export ANDROID_SSH_KEY_PATH=~/.ssh/id_ed25519
```

That's it! Start asking your AI assistant to interact with your Android device.

</details>

---

## Usage Examples

### File Management
- "Show me my most recent photos from /sdcard/DCIM"
- "Find all PDF files on my device larger than 10MB"
- "What's taking up the most space on my phone?"
- "List all APK files I've downloaded"

### Development & Monitoring
- "Show me running processes sorted by memory usage"
- "Check if my PostgreSQL server is running in Termux"
- "What's the CPU temperature right now?"
- "Show me the last 50 lines of my server logs"

### System Administration
- "Install python and pip on my Android device"
- "Create a backup of my Termux home directory"
- "Download the latest dataset from example.com to /sdcard"
- "Set up a cron job to run my backup script daily"

### Data Analysis
- "Parse my call log and show me my top 10 contacts by call count"
- "Extract all URLs from my browser history"
- "Analyze my app usage statistics from dumpsys"
- "Find duplicate files in my photos directory"

### Quick Utilities
- "What's my Android device's IP address?"
- "Show me battery information"
- "Check if specific packages are installed"
- "Read my termux startup script"

---

## API Reference

The server exposes two MCP tools that Claude can use:

### `execute_read` - Safe Read-Only Commands

Executes whitelisted read-only commands without user approval. Perfect for browsing files, checking system status, and gathering information.

**81 Whitelisted Commands Include:**
- File operations: `ls`, `cat`, `head`, `tail`, `grep`, `find`, `tree`
- System monitoring: `ps`, `top`, `df`, `du`, `free`, `uptime`
- Network: `ping`, `netstat`, `ss`, `ifconfig`
- Text processing: `wc`, `sort`, `cut`, `jq`
- [See full list in source](src/tools.rs#L13-L135)

**Parameters:**
- `command` (string, required) - The shell command to execute
- `timeout` (number, optional) - Timeout in seconds (default: 30, max: 300)

**Example:** `ls -lah /sdcard/Download`

---

### `execute` - Full Command Access

Executes any command including write/modify/delete operations. Use this for installing packages, creating files, or modifying system state.

**Parameters:**
- `command` (string, required) - The shell command to execute
- `timeout` (number, optional) - Timeout in seconds (default: 30, max: 300)

**Example:** `pkg install git`

**Note:** Commands that aren't whitelisted in `execute_read` will automatically suggest using this tool instead.

---

<div align="center">

**[Report Bug](https://github.com/vaknin/mcp-android-ssh/issues)** · **[Request Feature](https://github.com/vaknin/mcp-android-ssh/issues)**

Built with [MCP](https://modelcontextprotocol.io/) • Powered by [russh](https://github.com/warp-tech/russh)

</div>
