use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

mod backup;
mod config;
mod db;
mod error;
mod logger;
mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    logger::init();

    info!("st-zfs-send-recv starting");

    // Parse configuration files
    let config_dir = PathBuf::from("/etc/st-zfs-send-recv.d");
    let configs = config::load_pool_configs(&config_dir)?;

    if configs.is_empty() {
        anyhow::bail!("No pool configuration files found in {:?}", config_dir);
    }

    info!("Loaded {} pool configuration(s)", configs.len());

    // Process each pool configuration
    for pool_config in configs {
        info!("Processing pool: {}", pool_config.name);

        match process_pool(&pool_config).await {
            Ok(_) => info!("Successfully processed pool: {}", pool_config.name),
            Err(e) => {
                tracing::error!("Error processing pool {}: {:#}", pool_config.name, e);
            }
        }
    }

    info!("st-zfs-send-recv completed");
    Ok(())
}

async fn process_pool(pool_config: &config::PoolConfig) -> Result<()> {
    // Verify root privileges
    let uid = unsafe { libc::geteuid() };
    if uid != 0 {
        anyhow::bail!("This tool requires root privileges");
    }

    // Create temporary database
    let db_path = db::create_temp_db(&pool_config.local_pool)?;
    info!("Created temporary database: {}", db_path.display());

    // Establish SSH connection
    let mut ssh = ssh::SshSession::new(&pool_config.remote_host).await?;
    verify_remote_privileges(&mut ssh, &pool_config.remote_host).await?;

    // Initialize database schema
    db::initialize_db(&db_path)?;

    // Discover filesystems and snapshots
    discover_and_backup(&pool_config, &mut ssh, &db_path).await?;

    // Cleanup
    std::fs::remove_file(&db_path)?;
    info!("Cleaned up temporary database: {}", db_path.display());

    Ok(())
}

async fn verify_remote_privileges(ssh: &mut ssh::SshSession, host: &str) -> Result<()> {
    if !ssh.check_root_privileges().await? {
        anyhow::bail!(
            "Remote user on {} is not root and sudo is not available",
            host
        );
    }
    Ok(())
}

async fn discover_and_backup(
    pool_config: &config::PoolConfig,
    ssh: &mut ssh::SshSession,
    db_path: &std::path::Path,
) -> Result<()> {
    // Discover remote filesystems and snapshots
    backup::discover_remote_fs(ssh, db_path, &pool_config.remote_pool).await?;
    backup::discover_local_fs(db_path, &pool_config.local_pool)?;

    backup::discover_snapshots(ssh, db_path, "src", &pool_config.remote_pool).await?;
    backup::discover_snapshots_local(db_path, &pool_config.local_pool)?;

    // Perform incremental backup
    backup::backup_pool(ssh, db_path, &pool_config.remote_pool, &pool_config.local_pool)
        .await?;

    Ok(())
}
