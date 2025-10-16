<div align="center">

# Android SSH MCP Server

**Control your Android device from AI assistants via SSH**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/mcp-android-ssh.svg)](https://crates.io/crates/mcp-android-ssh)

A high-performance [MCP](https://modelcontextprotocol.io/) server written in Rust that provides secure SSH access to Android devices through Termux.

<img src="assets/usage-example.png" alt="Usage Example" width="50%">

*Example: AI assistant listing files and exploring directories on Android*

</div>

---

## Features

- **Lightweight** - 401 KiB binary
- **High Performance** - Memory-safe Rust implementation
- **Secure SSH Connection** - Key-based or password authentication
- **Safe Read Commands** - 81 whitelisted read-only commands (ls, cat, ps, etc.)
- **Full Write Access** - Execute any command including system operations
- **Auto-Reconnect** - Persistent connections with retry logic
- **Security** - SSH key auth preferred, command whitelist for read-only operations

## Prerequisites

**On Android:**
- Termux ([F-Droid](https://f-droid.org/packages/com.termux/) recommended)
- OpenSSH server running on port 8022

**On Host:**
- Rust toolchain (for `cargo install`)
- MCP-compatible client (Claude Code, etc.)

### Setup Android Device

**In Termux:**
```bash
# Install and start SSH server
pkg update && pkg install openssh && sshd

# Find your username and IP
whoami          # Example: u0_a555
ifconfig wlan0  # Example: 192.168.1.100
```

### Setup SSH Key Authentication (Recommended)

**Host machine:**
```bash
# Generate key (skip if you already have one)
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N ""

# Copy to Android (enter password once, then done!)
ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub u0_a555@192.168.1.100
```

## Quick Start

### Installation

```bash
cargo install mcp-android-ssh
```

### Configure Connection

Create a `.env` file in your working directory:

```bash
ANDROID_SSH_HOST=192.168.1.100
ANDROID_SSH_PORT=8022
ANDROID_SSH_USER=u0_a555
ANDROID_SSH_KEY_PATH=~/.ssh/id_ed25519
# ANDROID_SSH_PASSWORD=your_password
```

### Add to Claude Code

```bash
claude mcp add --scope user --transport stdio mcp-android-ssh mcp-android-ssh
```

That's it! Start asking your AI assistant to interact with your Android device.

## Usage Examples

**Read-only operations:**
- "List files in /sdcard/Download"
- "Show running processes on my Android"
- "Check disk usage on my phone"
- "Read my Termux .bashrc file"

**Write operations:**
- "Create a backup directory in /sdcard"
- "Install git on my Android device"
- "Download a file with curl"
- "Run system diagnostics with dumpsys"

## Tools

### `execute_read`
Execute safe, read-only commands. 81 whitelisted commands including: ls, cat, ps, grep, find, df, top, and more.

**Parameters:**
- `command` (string) - The command to execute
- `timeout` (number, optional) - Timeout in seconds (default: 30, max: 300)

### `execute`
Execute any command including write operations.

**Parameters:**
- `command` (string) - The command to execute
- `timeout` (number, optional) - Timeout in seconds (default: 30, max: 300)

---

<div align="center">

**[Report Bug](https://github.com/vaknin/mcp-android-ssh/issues)** · **[Request Feature](https://github.com/vaknin/mcp-android-ssh/issues)**

Built with [MCP](https://modelcontextprotocol.io/) • Powered by [russh](https://github.com/warp-tech/russh)

</div>
