# Installation Guide for st-zfs-send-recv

This guide covers building, installing, and configuring the professional Rust implementation of st-zfs-send-recv.

## Prerequisites

### Build Requirements

- **Rust 1.70 or later** - Install from https://rustup.rs/
- **Git** - For cloning and version control
- **Standard build tools** (gcc, make, pkg-config)

Installation on Ubuntu/Debian:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
sudo apt-get install -y build-essential pkg-config
```

### Runtime Requirements

- **ZFS tools** - Installed with ZFS
- **OpenSSH** - For SSH operations
- **SQLite3 libraries** (optional - bundled with Cargo build)

Installation on Ubuntu/Debian:
```bash
sudo apt-get install -y zfsutils-linux openssh-client sqlite3
```

Installation on RHEL/CentOS:
```bash
sudo yum install -y zfs openssh-clients sqlite
```

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/vivo75/smalltools.git
cd smalltools
```

### 2. Build the Release Binary

```bash
cargo build --release
```

The optimized binary will be at: `target/release/st-zfs-send-recv`

Compile time: 2-5 minutes depending on system

### 3. Verify the Build

```bash
./target/release/st-zfs-send-recv --version  # (if implemented)
./target/release/st-zfs-send-recv --help      # (if implemented)
```

## Installation

### 1. Install the Binary

```bash
sudo install -m 755 target/release/st-zfs-send-recv /usr/local/bin/
```

Or system-wide installation:
```bash
sudo install -m 755 target/release/st-zfs-send-recv /usr/sbin/
```

### 2. Create Configuration Directory

```bash
sudo mkdir -p /etc/st-zfs-send-recv.d
sudo chmod 755 /etc/st-zfs-send-recv.d
```

`/etc/st-zfs-send-recv.d` is the default location. To use a different directory instead,
pass `--config-dir <path>` on the command line, or set the `ST_CONFIG_DIR` environment
variable (the `--config-dir` flag takes precedence if both are set). See
[Configuration Directory](#configuration-directory) below.

### 3. Create Log Directory (Optional)

```bash
sudo mkdir -p /var/log/st-zfs-send-recv
sudo chmod 755 /var/log/st-zfs-send-recv
```

## Configuration

### Configuration Directory

By default, pool configuration files are read from `/etc/st-zfs-send-recv.d`. Override it with:

| Method | Example | Precedence |
|--------|---------|------------|
| `--config-dir` flag | `st-zfs-send-recv --config-dir /opt/st-zfs/conf.d` | Highest |
| `ST_CONFIG_DIR` env var | `ST_CONFIG_DIR=/opt/st-zfs/conf.d st-zfs-send-recv` | Used if flag is absent |
| Default | `/etc/st-zfs-send-recv.d` | Used if neither is set |

If a non-default directory is used, substitute it for `/etc/st-zfs-send-recv.d` in the
remaining steps of this guide.

### 1. Create Pool Configuration Files

Copy the example configuration:

```bash
sudo cp etc/st-zfs-send-recv.d/conf1.pool.example \
     /etc/st-zfs-send-recv.d/tank.pool
```

### 2. Edit Configuration

```bash
sudo vi /etc/st-zfs-send-recv.d/tank.pool
```

Modify these required fields:
- `remote_host`: SSH hostname of backup source
- `remote_pool`: ZFS pool name on remote
- `local_pool`: Local ZFS pool for backups

### 3. Secure Configuration Files

```bash
sudo chmod 600 /etc/st-zfs-send-recv.d/*.pool
```

### 4. Set Up SSH Access

Configure passwordless SSH access using keys:

#### On the backup server (as root):

```bash
# Generate SSH key if needed
ssh-keygen -t ed25519 -f /root/.ssh/id_ed25519 -N ""

# Copy public key to remote host
ssh-copy-id -i /root/.ssh/id_ed25519.pub backup.example.com

# Test SSH access
ssh backup.example.com zfs list
```

#### SSH Config (~/.ssh/config for root):

```
Host backup.example.com
    Hostname backup.example.com
    User root
    IdentityFile /root/.ssh/id_ed25519
    ControlMaster auto
    ControlPath /tmp/ssh-master-%r@%h:%p.socket
    ControlPersist 5m
    ServerAliveInterval 60
    ConnectTimeout 10
```

## Testing

### 1. Manual Test Run

```bash
# Test with debug logging
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv

# Test with debug logging against a non-default config directory
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv --config-dir /opt/st-zfs/conf.d
```

Watch for:
- Successful SSH connection to remote host
- Filesystem discovery messages
- Snapshot synchronization
- Completion without errors

### 2. Verify Backup

Check if snapshots were transferred:

```bash
# On backup destination
zfs list -r backup/tank
zfs list -H -t snapshot backup/tank
```

### 3. Check Log Output

```bash
# View recent logs
sudo journalctl -u backup.service -n 50  # if using systemd
dmesg | tail -20                         # kernel logs
```

## Scheduling with Cron

### 1. Create Cron Job

```bash
sudo crontab -e
```

Add one of these scheduling options:

**Hourly backups:**
```
0 * * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

**Daily backups at 2 AM:**
```
0 2 * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

**Every 6 hours:**
```
0 */6 * * * /usr/local/bin/st-zfs-send-recv >> /var/log/st-zfs-send-recv.log 2>&1
```

### 2. Monitor Cron Execution

```bash
# View cron mail (if configured)
sudo mail

# Check cron logs
sudo grep CRON /var/log/syslog
journalctl -u cron
```

## Systemd Timer (Alternative to Cron)

### 1. Create Systemd Service Unit

File: `/etc/systemd/system/st-zfs-send-recv.service`

```ini
[Unit]
Description=ZFS Incremental Backup Service
After=network.target zfs-mount.service
Wants=st-zfs-send-recv.timer

[Service]
Type=oneshot
ExecStart=/usr/local/bin/st-zfs-send-recv
StandardOutput=journal
StandardError=journal
SyslogIdentifier=st-zfs-send-recv
```

### 2. Create Systemd Timer

File: `/etc/systemd/system/st-zfs-send-recv.timer`

```ini
[Unit]
Description=ZFS Incremental Backup Timer
Requires=st-zfs-send-recv.service

[Timer]
OnBootSec=10min
OnUnitActiveSec=6h
AccuracySec=1min
Persistent=true

[Install]
WantedBy=timers.target
```

### 3. Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable st-zfs-send-recv.timer
sudo systemctl start st-zfs-send-recv.timer

# Verify
sudo systemctl status st-zfs-send-recv.timer
sudo systemctl list-timers st-zfs-send-recv.timer
```

## Post-Installation Verification

### 1. Check Installation

```bash
which st-zfs-send-recv
ls -l /usr/local/bin/st-zfs-send-recv
file /usr/local/bin/st-zfs-send-recv
```

### 2. Verify Configuration

```bash
ls -la /etc/st-zfs-send-recv.d/
cat /etc/st-zfs-send-recv.d/*.pool
```

### 3. Test SSH Connectivity

```bash
sudo ssh backup.example.com "zfs list"
sudo ssh backup.example.com "zfs list -t snapshot tank | head"
```

### 4. First Backup Run

```bash
# Enable verbose logging
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv
```

Expected output:
- Connection to remote host
- Discovery of filesystems
- Creation of incremental snapshots
- Summary of backed-up data

## Troubleshooting Installation

### "Command not found"

```bash
# Verify installation location
ls -l /usr/local/bin/st-zfs-send-recv

# Add to PATH if needed
export PATH=/usr/local/bin:$PATH
```

### "Permission denied"

```bash
# Check permissions
ls -l /usr/local/bin/st-zfs-send-recv
# Should show: -rwxr-xr-x

# Fix if needed
sudo chmod 755 /usr/local/bin/st-zfs-send-recv
```

### SSH Authentication Fails

```bash
# Test SSH connection
ssh -v backup.example.com zfs list

# Check SSH key permissions
ls -l /root/.ssh/
# Should be: 700 for directory, 600 for keys

# Fix permissions
chmod 700 /root/.ssh
chmod 600 /root/.ssh/id_*
chmod 644 /root/.ssh/*.pub
```

### Compilation Errors

```bash
# Ensure Rust is up-to-date
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Upgrading

### 1. Backup Current Version

```bash
sudo cp /usr/local/bin/st-zfs-send-recv \
        /usr/local/bin/st-zfs-send-recv.backup
```

### 2. Build New Version

```bash
cd /path/to/smalltools
git pull
cargo build --release
```

### 3. Install New Version

```bash
sudo install -m 755 target/release/st-zfs-send-recv /usr/local/bin/
```

### 4. Verify

```bash
# Test new version
RUST_LOG=debug sudo /usr/local/bin/st-zfs-send-recv
```

## Uninstallation

```bash
# Remove binary
sudo rm /usr/local/bin/st-zfs-send-recv

# Remove configuration (optional - keep backups!)
sudo rm -rf /etc/st-zfs-send-recv.d/

# Remove cron job
sudo crontab -e  # Remove the line

# Or if using systemd timer
sudo systemctl stop st-zfs-send-recv.timer
sudo systemctl disable st-zfs-send-recv.timer
sudo rm /etc/systemd/system/st-zfs-send-recv.*
sudo systemctl daemon-reload
```

## Security Hardening

### 1. Restrict Binary Permissions

```bash
sudo chmod 750 /usr/local/bin/st-zfs-send-recv
sudo chown root:root /usr/local/bin/st-zfs-send-recv
```

### 2. Restrict Configuration

```bash
sudo chmod 750 /etc/st-zfs-send-recv.d
sudo chmod 600 /etc/st-zfs-send-recv.d/*.pool
sudo chown root:root /etc/st-zfs-send-recv.d
```

### 3. Audit Logging

```bash
# Check execution
sudo ausearch -m EXECVE -F exe=/usr/local/bin/st-zfs-send-recv

# Monitor configuration changes
sudo auditctl -w /etc/st-zfs-send-recv.d/ -p wa -k zfs-backup-config
```

## Next Steps

1. Verify the first backup completes successfully
2. Monitor cron/systemd execution
3. Set up log rotation for `/var/log/st-zfs-send-recv.log`
4. Configure email alerts for failures (via cron or systemd)
5. Test disaster recovery procedures regularly

## Support

For issues during installation:

1. Check that Rust is installed: `rustc --version`
2. Verify ZFS is working: `zfs list`
3. Test SSH access: `ssh backup.example.com zfs list`
4. Enable debug logging: `RUST_LOG=debug`
5. Review configuration syntax in the example files

---

**Congratulations!** You now have a professional ZFS backup system.
