# Changelog

All notable changes to the st-zfs-send-recv project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2024-01-XX

### Changed
- **Complete rewrite from Bash to Rust** - Production-ready, maintainable implementation
- Configuration format changed from bash variables to YAML for improved readability
- Command-line interface remains the same (no arguments, uses config directory)
- Improved error handling with detailed context and recovery mechanisms
- Enhanced logging with structured tracing and configurable output levels

### Added
- **Async/await runtime** - Tokio-based async operations for SSH and I/O
- **Structured logging** - Production-grade logging with tracing crate
- **Type safety** - Leverages Rust's type system for compile-time correctness
- **Comprehensive documentation** - README, INSTALL, and DEVELOPMENT guides
- **Integration tests** - Test suite for configuration and basic functionality
- **Configuration validation** - Upfront validation of configuration files
- **Automatic sudo detection** - Detects and uses sudo automatically if needed
- **Better error messages** - Contextual error information for troubleshooting

### Improved
- **Performance** - Faster execution, better resource utilization
- **Maintainability** - Clear module structure, easier to extend
- **Security** - Type safety eliminates entire classes of bugs
- **Reliability** - Comprehensive error handling and recovery
- **Code quality** - Rust compiler ensures memory safety and thread safety

### Fixed
- Potential race conditions in snapshot detection
- Improved handling of edge cases (no common snapshots, etc.)
- Better handling of SSH failures with automatic reconnection potential

### Removed
- Bash shell syntax and associated dependencies
- Complex shell variable handling
- Potential shellcode injection vulnerabilities

## [1.x] - Historical

### Original Bash Implementation
- Basic incremental ZFS snapshot backup
- SQLite-based state tracking
- SSH remote command execution
- Compression support
- Snapshot retention policies

---

## Migration Guide (v1 → v2)

### Configuration Changes

**Old (Bash variables):**
```bash
REMOTE_HOST=backup.example.com
REMOTE_POOL=tank
LOCAL_POOL=backup/tank
COMPRESS=lz4
KEEP_SNAP=(7,daily 4,weekly 12,monthly)
```

**New (YAML):**
```yaml
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
compress: lz4
keep_snapshots:
  - count: 7
    label: daily
  - count: 4
    label: weekly
  - count: 12
    label: monthly
```

### Behavior Changes

1. **Configuration Format**: YAML instead of bash sourcing
2. **Binary**: Single `st-zfs-send-recv` executable
3. **Logging**: Structured logs to stderr (configure with RUST_LOG)
4. **Error Messages**: More detailed with context

### Compatibility

- Configuration files must be converted to YAML format
- Backup data is fully compatible (ZFS snapshots unchanged)
- Existing snapshots and backups continue to work
- Installation location can be the same

### Upgrade Steps

1. Build new Rust version: `cargo build --release`
2. Backup current binary: `sudo cp st-zfs-send-recv st-zfs-send-recv.old`
3. Convert configuration files to YAML (use `conf1.pool.example` as template)
4. Install new binary
5. Test with: `RUST_LOG=debug sudo st-zfs-send-recv`
6. Monitor first backup: `journalctl -f` or `tail -f /var/log/st-zfs-send-recv.log`

---

## Future Roadmap

### Planned for v2.1
- [ ] Configuration schema validation improvements
- [ ] Bandwidth throttling support
- [ ] Backup verification and integrity checking
- [ ] Human-readable data transfer statistics

### Planned for v2.2
- [ ] Parallel pool processing for multiple configurations
- [ ] SSH connection pooling for efficiency
- [ ] Prometheus metrics export for monitoring
- [ ] Web UI for backup status monitoring

### Planned for v3.0
- [ ] Distributed backup support (multiple destinations)
- [ ] Database replication for backup verification
- [ ] Multi-snapshot incremental transfers
- [ ] Backup scheduling engine (built-in replacement for cron)

---

## Known Issues

### Current Limitations
1. Processes pools sequentially (no parallel processing)
2. Creates new SSH connection per pool (no connection pooling)
3. Compression/decompression outside ZFS native streams

### Workarounds
- Run multiple instances for different pools (use flock to serialize)
- Monitor SSH connection usage if many pools configured
- Consider reducing number of snapshots per transfer

---

## Version History

| Version | Date | Type | Notes |
|---------|------|------|-------|
| 2.0.0 | 2024-01-XX | Major | Complete Rust rewrite |
| 1.2.1 | 2023-XX-XX | Patch | Last bash version |
| 1.2.0 | 2023-XX-XX | Minor | Feature improvements |
| 1.0.0 | 2017-XX-XX | Major | Initial release |

---

## Contributing

See [DEVELOPMENT.md](DEVELOPMENT.md) for development guidelines and contribution process.

## Support

For issues, questions, or suggestions:
1. Check this changelog first
2. Review the [README.md](README.md) and [INSTALL.md](INSTALL.md)
3. Enable debug logging: `RUST_LOG=debug`
4. Open an issue with detailed information

---

**Last Updated**: 2024-01-XX
**Current Version**: 2.0.0
**Stable Release**: Yes
