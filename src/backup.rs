use crate::{db, ssh::SshSession};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

pub async fn discover_remote_fs(
    ssh: &SshSession,
    db_path: &Path,
    pool: &str,
) -> Result<()> {
    info!("Discovering remote filesystems in pool: {}", pool);

    let output = ssh
        .execute_zfs_list(&[
            "list", "-Hpr", "-t", "filesystem,volume", "-o",
            "guid,createtxg,creation,type,used,name", "-s", "name", pool,
        ])
        .await?;

    let conn = db::get_connection(db_path)?;
    db::clear_tables(&conn)?;

    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 6 {
            let fs = db::Filesystem {
                pool: pool.to_string(),
                guid: parts[0].to_string(),
                createtxg: parts[1].to_string(),
                creation: parts[2].to_string(),
                fs_type: parts[3].to_string(),
                used: parts[4].to_string(),
                name: parts[5].to_string(),
            };

            db::insert_filesystem(&conn, &fs)?;
            debug!("Discovered remote filesystem: {}", fs.name);
        }
    }

    Ok(())
}

pub fn discover_local_fs(db_path: &Path, pool: &str) -> Result<()> {
    info!("Discovering local filesystems in pool: {}", pool);

    let output = std::process::Command::new("zfs")
        .args(&[
            "list", "-Hpr", "-t", "filesystem,volume", "-o",
            "guid,createtxg,creation,type,used,name", "-s", "name", pool,
        ])
        .output()
        .context("Failed to execute zfs list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("zfs list failed: {}", stderr);
    }

    let conn = db::get_connection(db_path)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 6 {
            let query = "INSERT INTO fs_dst (pool, guid, createtxg, creation, type, used, name) \
                        VALUES (?, ?, ?, ?, ?, ?, ?)";

            conn.execute(
                query,
                rusqlite::params![
                    pool,
                    parts[0],
                    parts[1],
                    parts[2],
                    parts[3],
                    parts[4],
                    parts[5]
                ],
            )
            .context("Failed to insert filesystem")?;

            debug!("Discovered local filesystem: {}", parts[5]);
        }
    }

    Ok(())
}

pub async fn discover_snapshots(
    ssh: &SshSession,
    db_path: &Path,
    table_suffix: &str,
    pool: &str,
) -> Result<()> {
    info!("Discovering remote snapshots");

    let conn = db::get_connection(db_path)?;
    let filesystems = db::get_filesystems(&conn, &format!("fs_{}", table_suffix), pool)?;

    for fs_name in filesystems {
        discover_fs_snapshots(ssh, db_path, table_suffix, &fs_name, pool).await?;
    }

    Ok(())
}

pub fn discover_snapshots_local(db_path: &Path, pool: &str) -> Result<()> {
    info!("Discovering local snapshots");

    let conn = db::get_connection(db_path)?;
    let filesystems = db::get_filesystems(&conn, "fs_dst", pool)?;

    for fs_name in filesystems {
        let output = std::process::Command::new("zfs")
            .args(&[
                "list", "-Hpr", "-d", "1", "-t", "snapshot", "-S", "creation", "-o",
                "guid,createtxg,creation,type,used,name", &fs_name,
            ])
            .output()
            .context("Failed to execute zfs list for snapshots")?;

        if !output.status.success() {
            warn!("Failed to list snapshots for {}", fs_name);
            continue;
        }

        let parent_guid = db::get_fs_property(&conn, "fs_dst", "guid", &fs_name)?
            .context("Failed to find filesystem")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for (idx, line) in stdout.lines().enumerate() {
            if idx >= 1000 {
                break; // Keep only 1000 most recent
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                let snap_name = parts[5];
                let snapshot_part = snap_name.split('@').nth(1).unwrap_or("");

                let snap = db::Snapshot {
                    parent_guid: parent_guid.clone(),
                    guid: parts[0].to_string(),
                    createtxg: parts[1].to_string(),
                    creation: parts[2].to_string(),
                    snap_type: parts[3].to_string(),
                    used: parts[4].to_string(),
                    name: snap_name.to_string(),
                    snapshot: snapshot_part.to_string(),
                };

                db::insert_snapshot(&conn, "snap_dst", &snap)?;
                debug!("Discovered local snapshot: {}", snap_name);
            }
        }
    }

    Ok(())
}

async fn discover_fs_snapshots(
    ssh: &SshSession,
    db_path: &Path,
    table_suffix: &str,
    fs_name: &str,
    _pool: &str,
) -> Result<()> {
    let output = ssh
        .execute_zfs_list(&[
            "list", "-Hpr", "-d", "1", "-t", "snapshot", "-S", "creation", "-o",
            "guid,createtxg,creation,type,used,name", fs_name,
        ])
        .await?;

    let conn = db::get_connection(db_path)?;

    let parent_guid = db::get_fs_property(&conn, &format!("fs_{}", table_suffix), "guid", fs_name)?
        .context("Failed to find filesystem")?;

    for (idx, line) in output.lines().enumerate() {
        if idx >= 1000 {
            break;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 6 {
            let snap_name = parts[5];
            let snapshot_part = snap_name.split('@').nth(1).unwrap_or("");

            let snap = db::Snapshot {
                parent_guid: parent_guid.clone(),
                guid: parts[0].to_string(),
                createtxg: parts[1].to_string(),
                creation: parts[2].to_string(),
                snap_type: parts[3].to_string(),
                used: parts[4].to_string(),
                name: snap_name.to_string(),
                snapshot: snapshot_part.to_string(),
            };

            db::insert_snapshot(&conn, &format!("snap_{}", table_suffix), &snap)?;
            debug!("Discovered remote snapshot: {}", snap_name);
        }
    }

    Ok(())
}

pub async fn backup_pool(
    ssh: &SshSession,
    db_path: &Path,
    remote_pool: &str,
    local_pool: &str,
) -> Result<()> {
    info!("Starting backup of pool: {}", remote_pool);

    let conn = db::get_connection(db_path)?;

    // Backup pool root
    backup_filesystem(ssh, &conn, db_path, remote_pool, local_pool).await?;

    // Backup all filesystems
    let filesystems = db::get_filesystems(&conn, "fs_src", remote_pool)?;
    for fs_short_name in filesystems {
        let fs_name = format!("{}/{}", remote_pool, fs_short_name);
        let dst_name = format!("{}/{}", local_pool, fs_short_name);

        backup_filesystem(ssh, &conn, db_path, &fs_name, &dst_name).await?;
    }

    info!("Backup completed for pool: {}", remote_pool);
    Ok(())
}

async fn backup_filesystem(
    ssh: &SshSession,
    conn: &rusqlite::Connection,
    _db_path: &Path,
    remote_fs: &str,
    local_fs: &str,
) -> Result<()> {
    // Extract pool name from filesystem name
    let pool = remote_fs.split('/').next().unwrap_or("");
    
    let common_snap = db::get_common_snapshot(conn, pool, remote_fs)?;
    let remote_snap = db::get_remote_newest_snapshot(conn, pool, remote_fs)?;

    match (common_snap, remote_snap) {
        (Some(common), Some(remote)) if common == remote => {
            info!("{}{} is already up to date", remote_fs, common);
        }
        (None, _) => {
            error!("{} no common snapshot found", remote_fs);
        }
        (Some(common), Some(remote)) => {
            info!("{}{} => {}", remote_fs, common, remote);

            match perform_incremental_backup(ssh, remote_fs, local_fs, &common, &remote).await {
                Ok(_) => {
                    info!("Successfully backed up {} to {}", remote_fs, local_fs);
                }
                Err(e) => {
                    error!("Failed to backup {} to {}: {}", remote_fs, local_fs, e);
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn perform_incremental_backup(
    ssh: &SshSession,
    remote_fs: &str,
    local_fs: &str,
    common_snap: &str,
    remote_snap: &str,
) -> Result<()> {
    // Stream: remote `zfs send -I` stdout -> local `zfs recv` stdin, so `zfs recv`
    // starts consuming data as soon as it arrives instead of waiting for the whole
    // send stream to be buffered in memory first.
    let send_cmd = format!(
        "zfs send -I {} {}",
        format!("{}{}", remote_fs, common_snap),
        format!("{}{}", remote_fs, remote_snap)
    );

    debug!("Executing remote command: {}", send_cmd);

    let mut send_child = ssh
        .spawn_zfs_send(&[
            "send",
            "-I",
            &format!("{}{}", remote_fs, common_snap),
            &format!("{}{}", remote_fs, remote_snap),
        ])
        .await?;

    let mut send_stdout = send_child
        .stdout()
        .take()
        .context("zfs send child is missing stdout")?;
    let mut send_stderr = send_child
        .stderr()
        .take()
        .context("zfs send child is missing stderr")?;

    // Receive on local side
    let mut recv_child = Command::new("zfs")
        .args(&["recv", "-F", "-eu", local_fs])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn zfs recv")?;

    let mut recv_stdin = recv_child
        .stdin
        .take()
        .context("zfs recv child is missing stdin")?;
    let mut recv_stdout = recv_child
        .stdout
        .take()
        .context("zfs recv child is missing stdout")?;
    let mut recv_stderr = recv_child
        .stderr
        .take()
        .context("zfs recv child is missing stderr")?;

    use tokio::io::AsyncReadExt;

    let copy_fut = async move {
        let result = tokio::io::copy(&mut send_stdout, &mut recv_stdin).await;
        // Close stdin so `zfs recv` sees EOF even if the stream ended early.
        drop(recv_stdin);
        result
    };
    let send_stderr_fut = async move {
        let mut buf = Vec::new();
        let _ = send_stderr.read_to_end(&mut buf).await;
        buf
    };
    let recv_stdout_fut = async move {
        let mut buf = Vec::new();
        let _ = recv_stdout.read_to_end(&mut buf).await;
        buf
    };
    let recv_stderr_fut = async move {
        let mut buf = Vec::new();
        let _ = recv_stderr.read_to_end(&mut buf).await;
        buf
    };

    let (copy_result, send_stderr_buf, _recv_stdout_buf, recv_stderr_buf) =
        tokio::join!(copy_fut, send_stderr_fut, recv_stdout_fut, recv_stderr_fut);

    let recv_status = recv_child.wait().await?;
    let send_status = send_child.wait().await?;

    if !send_status.success() {
        anyhow::bail!(
            "zfs send failed: {}",
            String::from_utf8_lossy(&send_stderr_buf).trim()
        );
    }

    if !recv_status.success() {
        anyhow::bail!(
            "zfs recv failed with status {}: {}",
            recv_status,
            String::from_utf8_lossy(&recv_stderr_buf).trim()
        );
    }

    let bytes_streamed = copy_result.context("Failed to stream zfs send data into zfs recv")?;
    debug!("Streamed {} bytes from zfs send to zfs recv", bytes_streamed);

    Ok(())
}
