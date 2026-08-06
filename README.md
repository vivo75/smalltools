# st-zfs-send-recv - Professional ZFS Backup Tool

A production-grade incremental ZFS backup utility written in Rust for efficient remote ZFS snapshot synchronization over SSH.

## Overview

`st-zfs-send-recv` is a Rust-rewritten version of the original bash script, providing a robust, maintainable, and performant solution for managing incremental ZFS snapshots between remote and local storage pools. It maintains local state using SQLite to intelligently perform incremental backups, minimizing bandwidth usage and transfer time.

## Features

- **Incremental Snapshots**: Efficiently transfers only changed data between snapshots
- **SSH Remote Operations**: Secure remote-to-local backup over SSH with automatic privilege detection
- **State Tracking**: SQLite database maintains accurate filesystem and snapshot metadata
- **Compression Support**: Configurable compression algorithms (lz4, gzip, zstd, etc.)
- **Multi-Pool Configuration**: Manage multiple backup pools with independent configurations
- **Retention Policies**: Automatic cleanup of old snapshots based on label matching
- **Async Operations**: Non-blocking async/await runtime with Tokio for improved performance
- **Structured Logging**: Production-grade logging with tracing, configurable levels
- **Error Recovery**: Comprehensive error handling with detailed diagnostics
- **Type Safety**: Leverages Rust's type system for correctness guarantees

## Requirements

### System Dependencies
- Rust 1.70+ (for building from source)
- ZFS filesystem tools (`zfs` command)
- OpenSSH client
- SQLite3 libraries (or bundled with Cargo)
- zfs-auto-snapshot (optional, for automated snapshot creation)

### Runtime Requirements
- Root privileges (locally and on remote host for ZFS operations)
- SSH key-based authentication recommended
- Network connectivity to remote ZFS pool

## Installation

### Building from Source

```bash
cd /path/to/smalltools
cargo build --release
sudo install -m 755 target/release/st-zfs-send-recv /usr/local/bin/
```

### Create Configuration Directory

```bash
sudo mkdir -p /etc/st-zfs-send-recv.d
sudo chmod 750 /etc/st-zfs-send-recv.d
```

## Configuration

Pool configurations are YAML files located in `/etc/st-zfs-send-recv.d/*.pool`.

### Minimal Configuration Example

```yaml
# /etc/st-zfs-send-recv.d/tank.pool
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
```

### Full Configuration Example

```yaml
# /etc/st-zfs-send-recv.d/tank.pool

# SSH host containing source ZFS pool
remote_host: backup.example.com

# Source ZFS pool name (on remote host)
remote_pool: tank

# Local destination pool/filesystem
local_pool: backup/tank

# Compression algorithm (optional, default: lz4)
compress: lz4

# Compression flags (optional, default: empty)
compress_flags: "-c"

# Decompression flags (optional, default: "-dcfm")
uncompress_flags: "-dcfm"

# Snapshot retention policies (optional)
# Matches snapshots by label and keeps N most recent
keep_snapshots:
  - count: 7
    label: daily
  - count: 4
    label: weekly
  - count: 12
    label: monthly

# Filesystems to skip (regex patterns, optional)
skip_filesystems:
  - "^tank/docker$"
  - "^tank/temp"
  - ".*\\.cache"
```

### Configuration Parameters

| Parameter | Required | Type | Description |
|-----------|----------|------|-------------|
| `remote_host` | ✓ | String | SSH hostname or IP address |
| `remote_pool` | ✓ | String | Source ZFS pool name |
| `local_pool` | ✓ | String | Destination ZFS pool or filesystem |
| `compress` | - | String | Compression algorithm (default: `lz4`) |
| `compress_flags` | - | String | Flags passed to compression command |
| `uncompress_flags` | - | String | Flags for decompression (default: `-dcfm`) |
| `keep_snapshots` | - | Array | Snapshot retention policies |
| `skip_filesystems` | - | Array | Regex patterns for filesystems to exclude |

## Usage

### Manual Execution

```bash
# Standard run (requires root)
sudo st-zfs-send-recv

# With debug output
RUST_LOG=debug sudo st-zfs-send-recv

# With trace-level logging
RUST_LOG=st_zfs_send_recv=trace sudo st-zfs-send-recv
```

### Scheduled Backups

Add to root's crontab for automated backups:

```bash
# Hourly backups at the top of every hour
0 * * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1

# Daily backups at 2 AM
0 2 * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1

# Multiple backups throughout the day
0 */6 * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

### Logging

The tool outputs structured logs to stderr. Control verbosity via `RUST_LOG`:

```bash
# Info level (default)
RUST_LOG=info st-zfs-send-recv

# Debug - detailed operation information
RUST_LOG=debug st-zfs-send-recv

# Trace - very detailed, diagnostic output
RUST_LOG=trace st-zfs-send-recv

# Specific module debugging
RUST_LOG=st_zfs_send_recv::backup=debug st-zfs-send-recv

# Quiet (errors only)
RUST_LOG=error st-zfs-send-recv
```

## Architecture

### Module Structure

```
src/
├── main.rs      - Entry point, orchestration, error handling
├── config.rs    - YAML configuration parsing and validation
├── db.rs        - SQLite schema, queries, state management
├── ssh.rs       - SSH session management, remote command execution
├── backup.rs    - Core backup logic and incremental sync
├── logger.rs    - Structured logging with tracing
└── error.rs     - Custom error types and context
```

### Database Schema

The tool uses a temporary SQLite database per pool to maintain state:

```sql
fs_src      -- Remote filesystems (GUID, creation time, space used, etc.)
fs_dst      -- Local filesystems
snap_src    -- Remote snapshots (100 most recent per filesystem)
snap_dst    -- Local snapshots
```

### Workflow

1. **Load Configuration**: Parse all `*.pool` files from config directory
2. **SSH Connection**: Establish authenticated SSH session to remote host
3. **Initialize Database**: Create temporary SQLite database
4. **Discover Filesystems**: Query ZFS on both sides for filesystem metadata
5. **Discover Snapshots**: Query ZFS for recent snapshots (up to 1000 per filesystem)
6. **Incremental Backup**:
   - Find most recent common snapshot
   - Send incremental stream from common to latest remote
   - Receive and apply on local destination
   - Clean up old local snapshots per retention policy
7. **Cleanup**: Remove temporary database

## Security Considerations

### Root Privileges

The tool requires root for ZFS operations. Consider these practices:

1. **Dedicated Backup User**: Run via cron as root or dedicated user with ZFS sudo
2. **SSH Keys**: Use password-less SSH keys for remote authentication
3. **Configuration Permissions**: Restrict config file access:
   ```bash
   sudo chmod 600 /etc/st-zfs-send-recv.d/*.pool
   ```

### Sudo Configuration

For non-root backup users, configure sudoers:

```sudoers
# Allow specific ZFS operations
backupuser ALL=(ALL) NOPASSWD: /usr/sbin/zfs send, /usr/sbin/zfs recv, /usr/sbin/zfs list

# Or allow all ZFS operations
backupuser ALL=(ALL) NOPASSWD: /usr/sbin/zfs

# With no TTY requirement for automation
Defaults:backupuser !requiretty
```

### Network Security

- **SSH Only**: All remote operations over encrypted SSH
- **Host Keys**: Verify and cache SSH host keys in `/root/.ssh/known_hosts`
- **Firewall**: Restrict SSH access to backup server IPs
- **VPN**: Consider additional VPN encryption for sensitive data

## Troubleshooting

### "Cannot find common snapshot"

**Symptom**: Error message in logs about missing common snapshots

**Cause**: Destination pool has no snapshots yet

**Solution**:
```bash
# Create initial snapshot on destination
sudo zfs snapshot -r backup/tank@initial
```

### "Remote command failed"

**Symptom**: SSH or ZFS command execution errors

**Solutions**:
1. Test SSH access: `ssh backup.example.com zfs list`
2. Verify remote user permissions: `ssh backup.example.com sudo zfs list`
3. Check configuration for typos in hostname
4. Review SSH key permissions (should be 600)
5. Verify remote host in `~/.ssh/known_hosts`

### SSH Timeout or Connection Refused

**Solution**: Test SSH connectivity:
```bash
ssh -v backup.example.com echo "test"
```

### Slow Backup Speed

**Causes and Solutions**:
- **Network latency**: Check bandwidth with `iperf3`
- **Compression overhead**: Try `compress: none` or faster compression
- **Filesystem overhead**: Monitor with `iostat -x 1`
- **Large snapshots**: Consider more frequent backups with smaller increments

### Out of Disk Space

**Solution**: Check destination pool capacity:
```bash
zfs list -o name,used,avail backup/tank
```

Clean up old snapshots manually if retention policy is not working:
```bash
zfs destroy -r backup/tank@old-snapshot-name
```

## Development

### Building

```bash
cd smalltools
cargo build           # Debug build
cargo build --release # Optimized release build
```

### Code Quality

Format code with rustfmt:
```bash
cargo fmt
```

Check for common mistakes with clippy:
```bash
cargo clippy -- -D warnings
```

### Testing

```bash
cargo test                    # Run all tests
cargo test -- --nocapture    # Show output
cargo test --doc             # Run documentation tests
```

### Logging in Debug

Add logging to debug issues:
```rust
debug!("Variable value: {:?}", my_var);
info!("Checkpoint reached");
warn!("Potential issue: {}", message);
error!("Failed operation: {}", error);
```

## Performance Characteristics

- **Memory**: ~10-50 MB (mostly database and SSH buffers)
- **CPU**: Minimal overhead; limited by compression and ZFS I/O
- **Network**: Only changed blocks transferred (after first full snapshot)
- **Duration**: Depends on snapshot size and network speed
- **Concurrency**: Processes pools sequentially (run multiple instances for parallel backups)

## Differences from Bash Version

### Improvements

- **Type Safety**: Rust compiler catches many errors at compile time
- **Performance**: Faster execution, better resource usage
- **Maintainability**: Clearer code structure, easier to modify
- **Error Handling**: More robust error recovery
- **Async Operations**: Non-blocking I/O with Tokio
- **Testing**: Better testability with modular design
- **Documentation**: Comprehensive inline docs and examples

### API Compatibility

- Configuration format: Updated to YAML (more readable)
- Command-line: Identical to bash version (no arguments)
- Behavior: Functionally equivalent with enhanced error handling
- Log format: Structured logs instead of simple text

## Limitations

- Currently processes pools sequentially (not in parallel)
- Requires SSH connection per pool (no connection pooling)
- Compression/decompression handled outside ZFS stream (slightly less efficient than native compression)

## Future Enhancements

- Parallel pool processing
- SSH connection pooling
- Web UI for monitoring
- Prometheus metrics export
- Incremental backup verification
- Bandwidth throttling

## License

GNU General Public License v2.0 - See [LICENSE](LICENSE) for details

## Authors

- **Francesco Riosa** <fr@f1r.eu> - Original bash implementation and Rust rewrite

## Contributing

Contributions welcome! Please:

1. Follow Rust conventions (use `rustfmt`)
2. Add tests for new features
3. Update documentation
4. Maintain backward compatibility with existing configurations
5. Test on actual ZFS systems before submitting

## Support

For issues, questions, or suggestions:

1. Check existing documentation and logs
2. Enable debug logging (`RUST_LOG=debug`)
3. Test SSH access manually
4. Verify ZFS pool accessibility
5. Review configuration syntax

---

**v2.0.0** - Professional Rust rewrite with async/await, structured logging, and enhanced reliability
