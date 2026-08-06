# Quick Start Guide

Get st-zfs-send-recv running in 5 minutes.

## Prerequisites Checklist

- [ ] Running as root or with sudo
- [ ] ZFS installed and working (`zfs list` returns pools)
- [ ] OpenSSH installed
- [ ] SSH key access to remote host ready
- [ ] Rust 1.70+ installed (for building) or pre-built binary available

## Quick Installation

### 1. Build (If Building from Source)

```bash
cd /path/to/smalltools
cargo build --release
```

Takes 2-5 minutes. Binary will be in `target/release/st-zfs-send-recv`

### 2. Install Binary

```bash
sudo cp target/release/st-zfs-send-recv /usr/local/bin/
```

### 3. Create Config Directory

```bash
sudo mkdir -p /etc/st-zfs-send-recv.d
```

### 4. Create Pool Configuration

```bash
# Copy example
sudo cp etc/st-zfs-send-recv.d/conf1.pool.example \
        /etc/st-zfs-send-recv.d/mypool.pool

# Edit with your values
sudo vi /etc/st-zfs-send-recv.d/mypool.pool
```

Minimum required changes:
```yaml
remote_host: your-backup-server.example.com
remote_pool: tank                          # Pool on remote
local_pool: backup/tank                    # Local destination
```

### 5. Test SSH Access

```bash
sudo ssh your-backup-server.example.com zfs list
```

If this works, continue. If not, fix SSH keys first:

```bash
# Generate key if needed
ssh-keygen -t ed25519 -f /root/.ssh/id_ed25519 -N ""

# Copy to remote
ssh-copy-id -i /root/.ssh/id_ed25519.pub your-backup-server.example.com
```

### 6. First Backup Run

```bash
# Test with verbose output
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv
```

Expected output:
```
[INFO] st-zfs-send-recv starting
[INFO] Loaded 1 pool configuration(s)
[INFO] Processing pool: mypool
[INFO] Created temporary database
[INFO] Discovering remote filesystems in pool: tank
[INFO] Discovering local filesystems in pool: backup/tank
[INFO] Starting backup of pool: tank
[INFO] Successfully backed up tank to backup/tank
[INFO] Cleaned up temporary database
[INFO] st-zfs-send-recv completed
```

### 7. Schedule Automatic Backups

**Option A: Cron (Simple)**

```bash
sudo crontab -e

# Add line for hourly backups:
0 * * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

**Option B: Systemd Timer (Advanced)**

See [INSTALL.md](INSTALL.md) for detailed systemd setup.

## Verify Success

After first backup completes:

```bash
# Check snapshots arrived
sudo zfs list -r backup/tank
sudo zfs list -H -t snapshot backup/tank | head -10

# View backup log (if using cron)
sudo tail -20 /var/log/st-zfs-send-recv.log

# Check with debug output
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv 2>&1 | head -50
```

## Common Issues and Fixes

### "This tool requires root privileges"
**Fix**: Run with `sudo`
```bash
sudo /usr/local/bin/st-zfs-send-recv
```

### "No pool configuration files found"
**Fix**: Create config directory and files
```bash
sudo mkdir -p /etc/st-zfs-send-recv.d
sudo cp etc/st-zfs-send-recv.d/conf1.pool.example \
        /etc/st-zfs-send-recv.d/mypool.pool
```

### SSH Connection Fails
**Fix**: Test SSH directly and fix keys
```bash
# Test connection
sudo ssh your-backup-server.example.com zfs list

# If fails, copy SSH key
sudo ssh-copy-id -i /root/.ssh/id_ed25519.pub your-backup-server.example.com

# Or manually add to authorized_keys on remote server
ssh your-backup-server.example.com 'cat >> ~/.ssh/authorized_keys' \
  < /root/.ssh/id_ed25519.pub
```

### "Cannot find common snapshot"
**Fix**: Create initial snapshot on destination
```bash
sudo zfs snapshot -r backup/tank@initial
```

### Slow Performance
**Fix**: Try faster compression
```yaml
# In config file
compress: none  # Disable compression for LAN
```

Or try:
```yaml
compress: zstd  # Fast modern compression
```

## Next Steps

1. **Wait for first backup**: Monitor with `sudo tail -f /var/log/st-zfs-send-recv.log`

2. **Configure retention**: Edit config file to keep old snapshots
   ```yaml
   keep_snapshots:
     - count: 7
       label: daily
   ```

3. **Monitor regularly**: Check backups are completing
   ```bash
   sudo zfs list -H -t snapshot backup/tank | wc -l
   ```

4. **Document**: Keep records of backup schedule and settings

5. **Test restore**: Verify you can recover data if needed
   ```bash
   # List backed up filesystems
   sudo zfs list backup/tank
   ```

## Configuration Tips

### Exclude Filesystems
```yaml
# Skip temporary or cache filesystems
skip_filesystems:
  - "^tank/docker$"
  - "^tank/tmp"
  - ".*\\.cache"
```

### Use Compression
```yaml
# Fast compression (default)
compress: lz4

# Or try modern compression
compress: zstd

# No compression for LAN
compress: none
```

### Fine-tune Schedule
```bash
# Every 6 hours
0 */6 * * * /usr/local/bin/st-zfs-send-recv

# Every 4 hours
0 */4 * * * /usr/local/bin/st-zfs-send-recv

# Multiple times daily (frequent backups)
0 6,12,18,0 * * * /usr/local/bin/st-zfs-send-recv
```

## Troubleshooting Commands

```bash
# Check if binary exists
which st-zfs-send-recv
ls -l /usr/local/bin/st-zfs-send-recv

# Test config syntax
cat /etc/st-zfs-send-recv.d/*.pool

# Check ZFS pools
sudo zfs list

# Check SSH access
sudo ssh your-backup-server.example.com zfs list

# View recent logs
sudo journalctl -u cron
tail -50 /var/log/st-zfs-send-recv.log

# Enable verbose logging
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv
```

## Getting Help

1. **Check logs**: `RUST_LOG=debug` shows detailed info
2. **Test SSH**: Manual SSH command shows auth issues
3. **Verify ZFS**: Both `zfs list` commands should work
4. **Review README**: Full documentation in README.md
5. **Check INSTALL**: Detailed setup in INSTALL.md

## What's Next?

- Add more pools by creating additional `.pool` files in `/etc/st-zfs-send-recv.d/`
- Configure snapshot retention policies
- Set up monitoring/alerting for backup completion
- Document backup strategy and RPO/RTO requirements
- Test disaster recovery procedures regularly

---

**You're done!** Your ZFS backups should now be running automatically.

For detailed information, see [README.md](README.md) and [INSTALL.md](INSTALL.md).
