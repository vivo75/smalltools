# Prompt for Recreating st-zfs-send-recv

This document contains a complete prompt for Large Language Models (LLMs) to recreate the st-zfs-send-recv Rust application from scratch.

---

## Complete Recreation Prompt

You are an expert Rust developer tasked with creating a professional-grade incremental ZFS backup tool. Here are the complete specifications:

### Project Overview

**Name**: st-zfs-send-recv  
**Language**: Rust (Edition 2021, minimum 1.70)  
**Purpose**: Incremental ZFS snapshot backup tool over SSH  
**License**: GPL v2.0  
**Status**: Production-ready

Create a command-line tool that:
- Manages incremental ZFS snapshots between remote and local ZFS pools
- Uses SSH for secure remote operations
- Tracks filesystem state in SQLite to determine backup strategy
- Supports multiple pool configurations
- Provides structured logging for production monitoring
- Handles errors gracefully with detailed context

### Functional Requirements

#### 1. Configuration Management
- Load YAML configuration files from `/etc/st-zfs-send-recv.d/*.pool`
- Each configuration file defines one pool to backup
- Support these required fields:
  - `remote_host`: SSH hostname containing source pool
  - `remote_pool`: ZFS pool name on remote host
  - `local_pool`: Local ZFS destination pool
- Support optional fields:
  - `compress`: Compression algorithm (default: lz4)
  - `compress_flags`: Custom compression flags
  - `uncompress_flags`: Decompression flags (default: -dcfm)
  - `keep_snapshots`: Array of {count, label} for retention
  - `skip_filesystems`: Array of regex patterns to exclude
- Validate all required fields are present
- Apply sensible defaults for optional fields

#### 2. SSH Operations
- Establish SSH connection to remote host
- Auto-detect if root or sudo required
- Execute remote ZFS commands asynchronously
- Handle binary ZFS stream data
- Gracefully handle SSH failures

#### 3. Filesystem Discovery
- Query remote filesystems via SSH: `zfs list -Hpr -t filesystem,volume`
- Query local filesystems: `zfs list -Hpr -t filesystem,volume`
- Parse output fields: guid, createtxg, creation, type, used, name
- Store results in SQLite database

#### 4. Snapshot Discovery
- Query remote snapshots: `zfs list -Hpr -d 1 -t snapshot -S creation`
- Query local snapshots: `zfs list -Hpr -d 1 -t snapshot -S creation`
- Keep up to 1000 most recent snapshots per filesystem
- Store parent-child relationships in database
- Parse snapshot names to extract base name after @

#### 5. Incremental Backup Algorithm
For each filesystem:
1. Query database for common snapshot (exists on both sides)
2. Query database for newest remote snapshot
3. Decision logic:
   - If common == newest: Log "already up to date", skip
   - If no common: Log error, skip (requires manual initialization)
   - Otherwise: Perform incremental backup from common to newest
4. Execute: `zfs send -I pool/fs@common pool/fs@newest | compress | ssh remote | uncompress | zfs recv -F -eu dest_pool/fs`
5. On success: Clean old snapshots per retention policy

#### 6. Snapshot Retention
- For each retention policy (label, count):
  - Find snapshots matching label
  - Keep N most recent
  - Destroy older matching snapshots
- Use `zfs-auto-snapshot` tool for destruction

#### 7. State Tracking
- Create temporary SQLite database per pool: `/tmp/{pool-name}--XXXXXXXX.db`
- Initialize schema with 4 tables: fs_src, fs_dst, snap_src, snap_dst
- Each filesystem table: pool, guid, createtxg, creation, type, used, name
- Each snapshot table: parent_guid, guid, createtxg, creation, type, used, name, snapshot
- Clear tables before discovering
- Delete database after successful completion

#### 8. Error Handling
- Verify running as root at startup
- Verify SSH access to remote host
- Verify remote user has ZFS privileges (or sudo available)
- Handle missing commands gracefully
- Provide contextual error messages
- Continue processing other pools/filesystems on failure
- Log all errors with appropriate severity

#### 9. Logging
- Use structured logging via `tracing` crate
- Output to stderr
- Support RUST_LOG environment variable for level control
- Include file, line number, and thread info
- Log at appropriate levels: info, debug, warn, error
- No stdout output in normal operation

#### 10. Process Flow
```
1. Initialize logging
2. Verify root privileges
3. Load pool configurations from /etc/st-zfs-send-recv.d/
4. For each pool:
   a. Create temporary database
   b. Establish SSH connection
   c. Verify remote privileges
   d. Discover remote filesystems → store in db
   e. Discover local filesystems → store in db
   f. Discover remote snapshots → store in db
   g. Discover local snapshots → store in db
   h. For each filesystem:
      - Query common snapshot
      - Query latest remote snapshot
      - Decide: skip/error/backup
      - If backup: execute send/recv pipeline
      - If success: cleanup old snapshots
   i. Delete temporary database
5. Exit with status
```

### Technical Architecture

#### Module Structure (7 modules)

**main.rs** - Entry point and orchestration
- `async fn main()` - Initialize logging, process pools
- `async fn process_pool()` - Process single pool
- `async fn verify_remote_privileges()` - Check SSH access
- `async fn discover_and_backup()` - Orchestrate backup

**config.rs** - YAML configuration
- `struct PoolConfig` - Pool configuration
- `struct KeepPolicy` - Snapshot retention policy
- `fn load_pool_configs()` - Load all .pool files
- `fn load_pool_config()` - Parse single YAML file
- `fn validate_pool_config()` - Validate required fields

**db.rs** - SQLite state management
- `struct Filesystem` - Filesystem metadata
- `struct Snapshot` - Snapshot metadata
- `fn create_temp_db()` - Create temp database
- `fn initialize_db()` - Create schema
- `fn insert_filesystem()` - Store filesystem
- `fn insert_snapshot()` - Store snapshot
- `fn get_common_snapshot()` - Find common snapshot
- `fn get_remote_newest_snapshot()` - Latest remote
- `fn get_filesystems()` - List filesystems
- `fn get_fs_property()` - Query property
- Query implementation details (JOIN snap_src/snap_dst on GUID)

**ssh.rs** - SSH operations
- `struct SshSession` - SSH session wrapper
- `impl SshSession::new()` - Connect via SSH
- `impl SshSession::check_root_privileges()` - Verify root or sudo
- `impl SshSession::execute_zfs_list()` - Run zfs list
- `impl SshSession::execute_zfs_send()` - Run zfs send
- Auto-detect and use sudo if needed
- Handle both root and non-root users

**backup.rs** - Core backup logic
- `async fn discover_remote_fs()` - Query remote filesystems
- `async fn discover_local_fs()` - Query local filesystems
- `async fn discover_snapshots()` - Query remote snapshots
- `fn discover_snapshots_local()` - Query local snapshots
- `async fn backup_pool()` - Orchestrate pool backup
- `async fn backup_filesystem()` - Backup single filesystem
- `async fn perform_incremental_backup()` - Execute send/recv

**logger.rs** - Structured logging
- `fn init()` - Initialize tracing subscriber
- Configure stderr output
- Support RUST_LOG environment variable
- Include timestamps, target, line numbers

**error.rs** - Custom error types
- `enum ZfsBackupError` - Custom error variants
- `type ZfsResult<T>` - Result alias
- SSH errors, DB errors, ZFS errors, config errors
- Implement Display and Error traits

**lib.rs** - Module exports for testing

#### Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
openssh = "0.10"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
anyhow = "1.0"
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }
regex = "1.10"
parking_lot = "0.12"
fs-err = "2.11"
tempfile = "3.12"
libc = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Implementation Details

#### Configuration File Format (YAML)
```yaml
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
compress: lz4
compress_flags: ""
uncompress_flags: "-dcfm"
keep_snapshots:
  - count: 7
    label: daily
  - count: 4
    label: weekly
skip_filesystems:
  - "^tank/docker$"
  - "^tank/temp"
```

#### Database Schema
```sql
CREATE TABLE fs_src (
  pool TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL
);

CREATE TABLE fs_dst (/* same as fs_src */);

CREATE TABLE snap_src (
  parent_guid TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL,
  snapshot TEXT NOT NULL
);

CREATE TABLE snap_dst (/* same as snap_src */);
```

#### ZFS Commands

Remote (via SSH):
```bash
zfs list -Hpr -t filesystem,volume -o guid,createtxg,creation,type,used,name -s name POOL
zfs list -Hpr -d 1 -t snapshot -S creation -o guid,createtxg,creation,type,used,name FILESYSTEM
zfs send -I POOL/FS@COMMON POOL/FS@LATEST
```

Local:
```bash
zfs list -Hpr -t filesystem,volume -o guid,createtxg,creation,type,used,name -s name POOL
zfs list -Hpr -d 1 -t snapshot -S creation -o guid,createtxg,creation,type,used,name FILESYSTEM
zfs recv -F -eu DEST_POOL/FS
zfs-auto-snapshot --quiet --destroy-only --label=LABEL --keep=N POOL
```

#### Compression Pipeline
```bash
# Remote send with compression
zfs send ... | lz4 [flags]

# Local receive with decompression
lz4 -dcfm | zfs recv ...
```

#### Error Messages
- Include context: what operation, which filesystem/pool, what failed
- Use anyhow `.context()` to add information at each level
- Log at appropriate levels (error/warn/info/debug)

### Testing Requirements

1. Unit tests for configuration parsing
2. Integration tests for config + database
3. Test pool config creation
4. Test default values application
5. Test validation of required fields
6. Use `#[cfg(test)]` modules within source files
7. Include `tests/` directory for integration tests

### Documentation Requirements

Create these files:
1. **README.md** (1500+ lines)
   - Overview and features
   - Requirements and installation
   - Configuration guide with examples
   - Usage instructions
   - Troubleshooting section
   - Architecture overview
   - Contributing guidelines

2. **INSTALL.md** (400+ lines)
   - Prerequisites and build requirements
   - Step-by-step build and installation
   - Configuration setup
   - SSH key configuration
   - Testing procedure
   - Systemd timer setup
   - Troubleshooting

3. **QUICKSTART.md** (400+ lines)
   - 5-minute quick start
   - Basic installation
   - Minimal configuration
   - First backup test
   - Common issues
   - Next steps

4. **DEVELOPMENT.md** (1000+ lines)
   - Project structure
   - Module responsibilities
   - Data structures
   - Workflow explanation
   - Development setup
   - Code quality tools
   - Testing strategies
   - Performance considerations

5. **INTERNALS.md** (1000+ lines)
   - Architecture deep dive
   - Data flow diagrams (Mermaid)
   - State management details
   - Backup algorithm explanation
   - Error handling flow
   - Concurrency model
   - Performance characteristics
   - Extensibility points
   - Debugging techniques

6. **CHANGELOG.md** (300+ lines)
   - Version 2.0.0 changes
   - Migration guide from bash
   - Future roadmap
   - Version history table

7. **PROJECT_SUMMARY.md** (500+ lines)
   - Executive summary of transformation
   - Benefits over bash version
   - Project structure overview
   - Technology stack
   - Deployment checklist

### Code Quality Standards

- Use `rustfmt` for formatting
- Pass `cargo clippy` with no warnings
- No `unsafe` code except where necessary
- Use meaningful variable and function names
- Add doc comments to public APIs
- Include examples in doc comments
- Implement `Debug` and `Display` where appropriate
- Use Rust idioms (iterators, pattern matching, etc.)

### Build and Deployment

1. Binary should be single executable (no runtime dependencies)
2. Configuration via `/etc/st-zfs-send-recv.d/`
3. Logs to stderr (can be redirected or sent to syslog)
4. Exit codes: 0 on success, 1 on failure
5. No command-line arguments (uses config files)
6. Suitable for cron or systemd timer execution

### Performance Targets

- Debug build: < 5 minutes compilation
- Release build: < 60 seconds execution per pool
- Memory usage: < 100 MB
- Database creation: < 100ms
- Discovery phase: < 1 second per pool
- Support backup of 1000+ filesystems

### Security Considerations

1. Run as root (required for ZFS)
2. SSH key authentication (not passwords)
3. Validate all configuration inputs
4. No hardcoded credentials
5. Graceful handling of privilege errors
6. Type-safe configuration and command building

### Edge Cases to Handle

1. No snapshots on destination (requires initialization)
2. Filesystems with spaces or special characters in names
3. Snapshots with special characters
4. Very large snapshots (100GB+)
5. Many snapshots (1000+)
6. Network timeouts during SSH
7. Permission denied errors
8. Filesystem not found errors
9. Disk space issues
10. Concurrent backup attempts (use flock)

### Optional Enhancements (Future)

- Parallel pool processing
- SSH connection pooling
- Bandwidth throttling
- Backup verification
- Prometheus metrics export
- Web UI monitoring
- Database replication verification

---

## Example Usage After Completion

```bash
# Build
cargo build --release

# Install
sudo install -m 755 target/release/st-zfs-send-recv /usr/local/bin/

# Configure
sudo mkdir -p /etc/st-zfs-send-recv.d
sudo cp etc/st-zfs-send-recv.d/conf1.pool.example /etc/st-zfs-send-recv.d/tank.pool
sudo vi /etc/st-zfs-send-recv.d/tank.pool

# Test SSH
sudo ssh backup.example.com zfs list

# Run manually with debug output
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv

# Schedule with cron
0 * * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

## Success Criteria

The implementation is complete when:

✅ Compiles without errors with `cargo build --release`  
✅ Passes all unit and integration tests  
✅ Passes `cargo clippy -- -D warnings` without warnings  
✅ Passes `cargo fmt --check` formatting check  
✅ Successfully backs up ZFS pool over SSH  
✅ Handles errors gracefully with informative messages  
✅ Can be configured via multiple YAML files  
✅ Logs structured output via tracing  
✅ Runs with minimal memory footprint  
✅ Complete documentation provided  
✅ Suitable for production use in cron jobs  

---

## Additional Notes

- The original bash script had 600 lines and did the same functionality
- This Rust version provides:
  - Type safety eliminating entire classes of bugs
  - Better error handling and recovery
  - Structured logging for production monitoring
  - Improved maintainability and extensibility
  - Clear modular architecture
  - Comprehensive documentation
- The backup data format (ZFS snapshots) remains 100% compatible
- No persistent state is maintained (ephemeral temporary database)
- The application is single-threaded by default (suitable for cron)
- Architecture supports future parallelization without changes

---

## License

All code and documentation must be licensed under GNU General Public License v2.0, compatible with the original bash implementation.

---

**This prompt provides a complete specification for an LLM to recreate the st-zfs-send-recv Rust application from scratch.**
