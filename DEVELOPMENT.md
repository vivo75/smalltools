# Development Guide for st-zfs-send-recv

This guide covers architecture, code organization, and development practices for the Rust implementation.

## Project Structure

```
st-zfs-send-recv/
├── Cargo.toml              # Project manifest and dependencies
├── Cargo.lock              # Locked dependency versions
├── README.md               # User documentation
├── INSTALL.md              # Installation guide
├── DEVELOPMENT.md          # This file
├── LICENSE                 # GPL v2 license
├── src/
│   ├── main.rs             # Entry point and orchestration
│   ├── config.rs           # Configuration parsing (YAML)
│   ├── db.rs               # SQLite database layer
│   ├── ssh.rs              # SSH session management
│   ├── backup.rs           # Core backup logic
│   ├── logger.rs           # Logging initialization
│   └── error.rs            # Error types and context
├── tests/
│   └── integration_test.rs # Integration tests
├── etc/
│   └── st-zfs-send-recv.d/
│       └── conf1.pool.example  # Configuration template
└── .gitignore              # Git ignore rules
```

## Architecture Overview

### Module Responsibilities

#### main.rs
- Application entry point
- Orchestrates the overall workflow:
  1. Initialize logging
  2. Load pool configurations
  3. Process each pool sequentially
  4. Handle errors and cleanup
- Bridges configuration, database, SSH, and backup modules

#### config.rs
- YAML configuration file loading and parsing
- Configuration validation
- Default value application
- Supports multiple `*.pool` files in configuration directory

#### db.rs
- SQLite database operations
- Schema management
- Query layer:
  - Filesystem discovery queries
  - Snapshot metadata queries
  - Common snapshot detection
- Temporary database file management

#### ssh.rs
- SSH session establishment
- Remote command execution
- ZFS command wrapper methods
- Automatic sudo detection and fallback
- Stream handling for binary ZFS data

#### backup.rs
- Core backup logic:
  - Filesystem discovery (local and remote)
  - Snapshot discovery and filtering
  - Incremental transfer calculation
  - Compression pipeline coordination
- State-based decision making (no common snapshot → error, up-to-date → skip, etc.)

#### logger.rs
- Structured logging configuration
- Tracing subscriber setup
- Environment variable support for log levels
- JSON output support (optional)

#### error.rs
- Custom error types with `thiserror`
- Error context wrapping with `anyhow`
- Error propagation via Result<T>

## Async Design

The application uses `tokio` for async operations:

- **SSH operations** run asynchronously via `openssh` crate
- **Database operations** are synchronous (blocking OK for this use case)
- **File I/O** uses both tokio and std::fs depending on context

### Why Async?

- SSH operations can be slow - async avoids blocking
- Future extensibility for parallel pool processing
- Modern Rust best practices

## Key Data Structures

### Filesystem
```rust
pub struct Filesystem {
    pub pool: String,
    pub guid: String,           // Unique identifier
    pub createtxg: String,      // Creation transaction
    pub creation: String,       // Creation timestamp
    pub fs_type: String,        // filesystem or volume
    pub used: String,          // Space usage
    pub name: String,          // Full filesystem path
}
```

### Snapshot
```rust
pub struct Snapshot {
    pub parent_guid: String,    // Parent filesystem GUID
    pub guid: String,          // Snapshot GUID
    pub createtxg: String,
    pub creation: String,
    pub snap_type: String,
    pub used: String,
    pub name: String,          // Full snapshot path
    pub snapshot: String,      // Snapshot name part after @
}
```

### PoolConfig
```rust
pub struct PoolConfig {
    pub name: String,               // Configuration name
    pub remote_host: String,        // SSH host
    pub remote_pool: String,        // Source ZFS pool
    pub local_pool: String,         // Destination ZFS pool
    pub compress: String,           // Compression algorithm
    pub compress_flags: String,     // Compression options
    pub uncompress_flags: String,   // Decompression options
    pub keep_snapshots: Vec<KeepPolicy>,
    pub skip_filesystems: Vec<String>,
}
```

## Workflow

### Normal Backup Process

```
1. Initialize Logging
   └─> Configure structured logging with tracing

2. Load Configuration
   └─> Parse all *.pool files from /etc/st-zfs-send-recv.d/

3. For Each Pool:
   a. Verify Root Privileges
   b. Create Temporary SQLite Database
   c. Establish SSH Connection
   d. Discover Filesystems:
      ├─> Remote via SSH
      └─> Local via direct ZFS commands
   e. Discover Snapshots:
      ├─> Remote (1000 most recent per filesystem)
      └─> Local (1000 most recent per filesystem)
   f. Backup Each Filesystem:
      ├─> Find common snapshot between source and destination
      ├─> If no common: Error (requires manual initialization)
      ├─> If up-to-date: Skip
      └─> If incremental needed:
          ├─> SSH: zfs send -I common...latest | compress
          ├─> Local: uncompress | zfs recv -F -eu
          └─> Cleanup old snapshots per retention policy
   g. Cleanup Temporary Database

4. Completion
   └─> Log summary and exit
```

## Development Workflow

### Setting Up Development Environment

```bash
# Install Rust (if not already done)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repository
git clone https://github.com/vivo75/smalltools.git
cd smalltools

# Install development tools
rustup component add rustfmt clippy
cargo install cargo-watch  # For auto-rebuilding
```

### Building

```bash
# Debug build (faster compilation, slower execution)
cargo build

# Release build (slower compilation, optimized execution)
cargo build --release

# Watch for changes and rebuild
cargo watch -x build
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_pool_config_creation

# Show output
cargo test -- --nocapture

# Run tests with specific log level
RUST_LOG=debug cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without changing
cargo fmt -- --check

# Run linter
cargo clippy

# Strict clippy (deny warnings)
cargo clippy -- -D warnings

# Combined quality check
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Adding New Features

### Example: Add Bandwidth Limiting

1. **Update config.rs:**
```rust
pub struct PoolConfig {
    // ... existing fields ...
    pub bandwidth_limit: Option<u64>,  // MB/s
}
```

2. **Update Cargo.toml:**
```toml
# Add rate limiting crate if needed
token-bucket = "0.1"
```

3. **Update backup.rs:**
```rust
async fn perform_incremental_backup(
    ssh: &SshSession,
    remote_fs: &str,
    local_fs: &str,
    common_snap: &str,
    remote_snap: &str,
    rate_limit: Option<u64>,  // Add parameter
) -> Result<()> {
    // Implement rate limiting
}
```

4. **Update main.rs to pass the parameter**

5. **Test the feature**

### Example: Add Snapshot Verification

1. Implement verification function in backup.rs
2. Call after successful backup
3. Log results
4. Add configuration option if needed

## Error Handling

The project uses three layers of error handling:

### 1. Custom Errors (error.rs)
```rust
pub enum ZfsBackupError {
    #[error("SSH error: {0}")]
    SshError(String),
    // ... more variants
}
```

### 2. Context Wrapping (anyhow)
```rust
connection.execute(query, [])
    .context("Failed to execute query")?;
```

### 3. Result Types
```rust
pub type ZfsResult<T> = Result<T, ZfsBackupError>;
```

### Best Practices

- Use `context()` to add descriptive information
- Propagate errors with `?` operator
- Log errors at appropriate levels
- Recover gracefully when possible

## Testing

### Unit Tests

Located in each module with `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Integration Tests

Located in `tests/` directory:

```rust
// tests/integration_test.rs
use st_zfs_send_recv::*;

#[test]
fn test_e2e_backup() {
    // End-to-end test
}
```

### Testing Strategy

1. **Unit tests**: Module functionality in isolation
2. **Integration tests**: Multi-module interactions
3. **Manual testing**: Actual ZFS operations (requires test environment)

### Mock Testing

For SSH and ZFS operations, consider using:
- `mockito` for HTTP mocking (if API layer added)
- `tempfile` for temporary test databases
- Docker containers for test ZFS environments

## Performance Considerations

### Current Bottlenecks

1. **SSH stream transfer** - Network bound
2. **Compression** - CPU bound
3. **ZFS operations** - I/O bound

### Optimization Opportunities

1. **Parallel pool processing** - Process multiple pools concurrently
2. **SSH connection pooling** - Reuse SSH connections
3. **Compression selection** - Auto-tune based on network speed
4. **Snapshot batching** - Send multiple snapshots in one operation

### Profiling

```bash
# Build with profiling support
cargo build --release

# Profile with perf (Linux)
perf record -g ./target/release/st-zfs-send-recv
perf report

# Memory profiling
valgrind --leak-check=full ./target/release/st-zfs-send-recv
```

## Logging

### Adding Logs

```rust
use tracing::{debug, info, warn, error};

// Structured logging
info!("Starting backup of pool: {}", pool_name);
debug!("Pool config: {:?}", config);
warn!("No snapshots found on destination");
error!("Backup failed: {:#}", error);

// With context
info!(
    pool = pool_name,
    fs_count = filesystems.len(),
    "Pool discovery completed"
);
```

### Viewing Logs

```bash
# Default (info level)
sudo st-zfs-send-recv

# Debug
RUST_LOG=debug sudo st-zfs-send-recv

# Specific module debug
RUST_LOG=st_zfs_send_recv::backup=trace sudo st-zfs-send-recv

# JSON format
RUST_LOG=info RUST_LOG_FORMAT=json sudo st-zfs-send-recv
```

## Documentation

### Code Documentation

```rust
/// Performs incremental ZFS backup between remote and local pools.
///
/// # Arguments
///
/// * `ssh` - SSH session to remote host
/// * `db_path` - Path to temporary SQLite database
/// * `remote_pool` - Source ZFS pool name
/// * `local_pool` - Destination ZFS pool name
///
/// # Errors
///
/// Returns an error if SSH operations fail or ZFS commands error out.
///
/// # Example
///
/// ```ignore
/// backup_pool(&mut ssh, &db_path, "tank", "backup/tank").await?;
/// ```
pub async fn backup_pool(
    ssh: &mut SshSession,
    db_path: &Path,
    remote_pool: &str,
    local_pool: &str,
) -> Result<()> {
    // Implementation
}
```

### Documentation Tests

```bash
cargo test --doc
```

## Contributing Guidelines

1. **Code Style**: Use `cargo fmt` before committing
2. **Linting**: Ensure `cargo clippy` passes
3. **Tests**: Add tests for new functionality
4. **Documentation**: Update docs for API changes
5. **Commits**: Clear, descriptive commit messages
6. **Branches**: Feature branches from main

## Dependencies

### Production Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| tokio | Async runtime | 1.40+ |
| openssh | SSH operations | 0.10 |
| rusqlite | SQLite database | 0.32 |
| serde | Serialization | 1.0 |
| serde_yaml | YAML parsing | 0.9 |
| tracing | Structured logging | 0.1 |
| anyhow | Error handling | 1.0 |
| thiserror | Custom errors | 1.0 |

### Development Dependencies

```bash
# Installed via rustup
rustfmt   # Code formatting
clippy    # Linting

# Optional tools
cargo-watch  # Auto rebuild
cargo-tarpaulin  # Code coverage
```

## Maintenance Tasks

### Regular Checks

```bash
# Update dependencies (carefully)
cargo update
cargo tree --duplicates

# Check for security vulnerabilities
cargo audit

# Check outdated dependencies
cargo outdated
```

### Release Checklist

- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md
- [ ] Run all tests
- [ ] Run clippy with strict settings
- [ ] Update documentation
- [ ] Tag release in git
- [ ] Build release binary
- [ ] Test on clean installation

## Troubleshooting Development

### Compilation Errors

```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build

# Check for unstable features
cargo check
```

### Test Failures

```bash
# Verbose test output
RUST_LOG=trace cargo test -- --nocapture

# Single test
cargo test test_name -- --exact --nocapture
```

### SSH Issues in Development

```bash
# Test SSH manually
ssh -v backup.example.com zfs list

# Debug SSH in code
RUST_LOG=debug cargo run -- 2>&1 | grep -i ssh
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/)
- [Rusqlite Documentation](https://docs.rs/rusqlite/)
- [Tracing Guide](https://docs.rs/tracing/)
- [ZFS Administration](https://openzfs.github.io/openzfs-docs/)

## Getting Help

1. Check existing issues on GitHub
2. Search Rust forums: https://users.rust-lang.org/
3. Ask on Rust Discord
4. Open an issue with reproducible steps

---

**Happy coding!** This is a well-structured, maintainable project. Follow these guidelines to keep it that way.
