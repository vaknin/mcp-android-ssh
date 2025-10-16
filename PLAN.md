# MCP Android SSH - Rust Implementation Plan

## Project Overview

This document outlines the plan to rewrite the Android SSH MCP Server from Python to Rust, providing secure SSH access to Android devices through the Model Context Protocol (MCP).

### Goals
- Create a performant, memory-safe MCP server in Rust
- Maintain feature parity with the Python implementation
- Improve type safety and error handling
- Reduce runtime dependencies and improve deployment

### Original Python Implementation
- **Source**: `android_ssh_mcp/` (Python)
- **Main modules**: `android_mcp.py`, `ssh_client.py`, `validation.py`, `__main__.py`
- **Dependencies**: `mcp`, `paramiko`, `python-dotenv`

## Architecture Overview

### MCP Server Fundamentals (Based on Research)
- **Binary application**: MCP servers are standalone executables, not libraries
- **Communication**: JSON-RPC over stdio (stdin/stdout)
- **Runtime**: Launched as subprocess by MCP clients (e.g., Claude Desktop)
- **Protocol**: Async request/response model with tools, resources, and prompts

### Project Structure (Implemented)

```
mcp-android-ssh/
├── Cargo.toml                 # Project manifest
├── Cargo.lock                 # Dependency lock file (gitignored)
├── PLAN.md                    # This file
├── README.md                  # Documentation
├── .env                       # Environment variables (gitignored)
├── .env.example               # Environment variable template
├── .gitignore                 # Git ignore rules
└── src/
    ├── main.rs                # Entry point, MCP server setup
    ├── config.rs              # Environment configuration
    ├── error.rs               # Custom error types
    ├── tools.rs               # MCP tools (execute_read, execute) + whitelist
    └── ssh/
        ├── mod.rs             # SSH module exports
        └── client.rs          # SSH client implementation

Note: Simplified from original plan - validation integrated into tools.rs,
no separate lib.rs or tests directory (deferred to Phase 5).
```

## Dependencies Mapping

### Python → Rust Dependencies

| Python Package | Rust Crate | Purpose |
|----------------|------------|---------|
| `mcp` | `rmcp` | MCP protocol implementation |
| `paramiko` | `russh` or `ssh2` | SSH client library |
| `python-dotenv` | `dotenvy` | Environment variable loading |
| `asyncio` | `tokio` | Async runtime |
| N/A | `serde` | Serialization/deserialization |
| N/A | `serde_json` | JSON handling |
| N/A | `anyhow` or `thiserror` | Error handling |
| N/A | `tracing` / `tracing-subscriber` | Logging |

### Installing Dependencies

```bash
cargo add rmcp --features server,transport-io,macros
cargo add russh russh-keys tokio --features full
cargo add serde --features derive serde_json dotenvy anyhow thiserror async-trait
cargo add tracing tracing-subscriber --features env-filter
cargo add --dev tokio-test mockall
```

## ⚠️ Critical Implementation Notes (From 2025 Best Practices)

### 1. **Logging MUST Use stderr (NOT stdout!)**
   - stdout is reserved exclusively for JSON-RPC MCP protocol communication
   - All logging, tracing, and debug output MUST go to stderr
   - Configuration:
   ```rust
   tracing_subscriber::fmt()
       .with_writer(std::io::stderr)
       .init();
   ```

### 2. **Never Block the Async Runtime**
   - NEVER use blocking operations in async functions
   - Use `tokio::spawn_blocking` for CPU-intensive or blocking I/O work
   - Blocking will stall the entire Tokio runtime and cause hangs/deadlocks

### 3. **Tool Implementation Pattern**
   - Use `#[tool_router]` macro on the impl block
   - Use `#[tool]` macro on individual tool methods
   - Reference example:
   ```rust
   #[derive(Clone)]
   pub struct AndroidSshService {
       ssh_client: Arc<Mutex<SshClient>>,
       tool_router: ToolRouter<Self>,
   }

   #[tool_router]
   impl AndroidSshService {
       #[tool(description = "Execute read-only commands")]
       async fn execute_read(&self, Parameters(request): Parameters<ExecuteRequest>)
           -> Result<CallToolResult, McpError> {
           // Implementation
       }
   }
   ```

### 4. **SSH Library Security**
   - russh had CVE-2025-54804 (integer overflow) in versions ≤ 0.54.0
   - Using `cargo add` will automatically get the patched version (0.54.1+)
   - Alternative: `async-ssh2-tokio` (simpler API, built on russh)

### 5. **Server Lifecycle**
   ```rust
   #[tokio::main]
   async fn main() -> Result<()> {
       // Initialize logging to stderr
       tracing_subscriber::fmt()
           .with_writer(std::io::stderr)
           .init();

       // Create service and serve on stdio
       let service = AndroidSshService::new().serve(stdio()).await?;

       // Wait for completion
       service.waiting().await?;
       Ok(())
   }
   ```

## Module Implementation Details

### 1. `src/main.rs` - MCP Server Entry Point

- Initialize logging (stderr), load .env config, create SSH client manager
- Initialize MCP server with tools, serve on stdio transport, handle graceful shutdown

### 2. `src/ssh/client.rs` - SSH Client

**Core struct**: `SshClient` with host, port, username, auth (Password/PrivateKey), session, retry config

**Methods**: `new()`, `connect()` (with retry), `disconnect()`, `ensure_connected()`, `execute_command(command, timeout) -> (stdout, stderr, exit_code)`

**Library**: Use `russh` (pure Rust, async-first)

### 3. `src/validation/validators.rs` - Input Validation

**Functions**: `validate_command()`, `validate_path()`, `validate_timeout()`, `is_read_only_command()`

**Whitelist**: Port `READ_ONLY_COMMANDS` (81 commands) from Python, use `HashSet<&'static str>`

### 4. `src/tools/` - MCP Tool Implementations

**execute_read**: Whitelist check → validate → execute → return `CommandResult { stdout, stderr, exit_code }`

**execute**: Validate → execute → return `CommandResult`

### 5. `src/config.rs` - Configuration Management

**Config struct**: `android_ssh_host`, `port`, `user`, `password` (optional), `key_path` (optional), `mcp_debug`. Load from `.env` using `dotenvy`.

### 6. `src/error.rs` - Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("SSH connection failed: {0}")]
    SshConnection(String),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Implementation Phases

### Phase 1: Foundation ✅ COMPLETE
- [x] Create Cargo project structure
- [x] Set up basic dependencies in Cargo.toml (using `cargo add`)
- [x] Implement configuration loading (`src/config.rs`)
- [x] Set up logging with tracing (stderr only)
- [x] Define error types (`src/error.rs`)

### Phase 2: SSH Client ✅ COMPLETE
- [x] Research and choose SSH library (russh chosen)
- [x] Implement SSH client connection
- [x] Add authentication (password + key-based with fallback)
- [x] Implement command execution
- [x] Add retry logic and auto-reconnect
- [ ] Write unit tests for SSH client (deferred)

### Phase 3: Validation ✅ COMPLETE (Simplified)
- [x] Port command whitelist from Python (81 commands)
- [x] Implement command validation (integrated into tools.rs)
- [x] Implement timeout validation (1-300 seconds)
- [ ] Implement path validation (not needed)
- [ ] Write unit tests for validators (deferred)

### Phase 4: MCP Server Integration ✅ COMPLETE
- [x] Set up rmcp SDK
- [x] Define MCP tools (execute_read, execute)
- [x] Implement tool handlers
- [x] Set up stdio transport
- [x] Implement server lifecycle management
- [x] Handle graceful shutdown

### Phase 5: Testing & Documentation 🚧 IN PROGRESS
- [ ] Integration tests with mock SSH server
- [ ] Test with real Android device
- [x] Update README.md for Rust version
- [x] Add usage examples
- [x] Create .env.example
- [ ] Performance benchmarks vs Python

### Phase 6: Polish & Release 📋 PLANNED
- [x] Error message improvements
- [ ] Add CLI flags (--version, --help)
- [ ] Binary release builds (CI/CD)
- [ ] Docker support (optional)
- [ ] Migration guide from Python version

## Key Technical Decisions

- **SSH**: `russh` (pure Rust, async-first)
- **Error handling**: `thiserror` for library, `anyhow` for application
- **Logging**: `tracing` (structured logging, async support)
- **Whitelist**: `HashSet<&'static str>` (optimize with `phf` if needed)
- **Async runtime**: `tokio` (required by rmcp)

## Testing Strategy

**Unit**: SSH client (mocks), validation, config parsing
**Integration**: End-to-end MCP tool execution, SSH connection, error scenarios
**Manual**: Real Android device, Claude Desktop client, timeout/retry/validation edge cases

## Build Commands

```bash
cargo build                        # Development
cargo build --release              # Production
```

## Security Considerations

1. **Input Validation**: All user inputs validated before SSH execution
2. **Command Whitelist**: Strict enforcement for read-only operations
3. **SSH Key Permissions**: Check key file permissions (600)
4. **Secrets in Memory**: Use `zeroize` crate for sensitive data
5. **Dependency Audits**: Regular `cargo audit` runs

## References

- [MCP Specification](https://modelcontextprotocol.io/specification)
- [rmcp Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [russh Documentation](https://docs.rs/russh/)
- [Original Python Implementation](../android_ssh_mcp/)
