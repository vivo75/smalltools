// step-9: tutto insieme. Non abbiamo un host SSH né un pool ZFS veri
// (si vedano gli step 7 e 8 per il perché), quindi la "scoperta" di
// filesystem e snapshot è simulata con dati seminati a mano nel
// database — in produzione, quei dati arrivano da
// `ssh.execute_zfs_list(...).await` (step 8) e finiscono nelle stesse
// tabelle SQLite (step 7). Tutto il resto — caricare la
// configurazione, filtrare i filesystem, decidere l'azione di backup,
// loggare — è esattamente il flusso reale.

mod backup;
mod config;
mod db;
mod error;
mod logger;

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

fn seed_fake_discovery(conn: &Connection, pool: &str) -> Result<()> {
    // tank/data: sincronizzato (stesso snapshot su entrambi i lati).
    db::insert_fs(conn, pool, "guid-data", &format!("{pool}/data"))?;
    db::insert_snapshot(
        conn,
        "snap_src",
        "guid-data",
        "guid-common",
        "2024-01-01",
        "daily-01",
    )?;
    db::insert_snapshot(
        conn,
        "snap_dst",
        "guid-data",
        "guid-common",
        "2024-01-01",
        "daily-01",
    )?;

    // tank/docker: verrà escluso da skip_filesystems, non arriva mai a
    // essere interrogato — lo seminiamo comunque per dimostrarlo.
    db::insert_fs(conn, pool, "guid-docker", &format!("{pool}/docker"))?;

    // tank/logs: incrementale da fare (snapshot in comune + uno nuovo
    // solo remoto).
    db::insert_fs(conn, pool, "guid-logs", &format!("{pool}/logs"))?;
    db::insert_snapshot(
        conn,
        "snap_src",
        "guid-logs",
        "guid-logs-common",
        "2024-01-01",
        "daily-01",
    )?;
    db::insert_snapshot(
        conn,
        "snap_dst",
        "guid-logs",
        "guid-logs-common",
        "2024-01-01",
        "daily-01",
    )?;
    db::insert_snapshot(
        conn,
        "snap_src",
        "guid-logs",
        "guid-logs-new",
        "2024-01-02",
        "daily-02",
    )?;

    // tank/new: nessuno snapshot in comune, serve inizializzazione.
    db::insert_fs(conn, pool, "guid-new", &format!("{pool}/new"))?;
    db::insert_snapshot(
        conn,
        "snap_src",
        "guid-new",
        "guid-new-1",
        "2024-01-01",
        "daily-01",
    )?;

    Ok(())
}

fn process_pool(pool_config: &config::PoolConfig) -> Result<()> {
    info!("Processing pool: {}", pool_config.name);

    let conn = Connection::open_in_memory()?;
    db::initialize_db(&conn)?;
    seed_fake_discovery(&conn, &pool_config.remote_pool)?;

    let discovered = db::get_filesystems(&conn, &pool_config.remote_pool)?;

    // La stessa combinazione compila-una-volta + filtra vista in
    // backup.rs, qui applicata al risultato reale della "scoperta".
    let to_process = backup::filter_filesystems(discovered, &pool_config.skip_filesystems)?;

    for fs_name in to_process {
        let guid = match db::get_fs_guid(&conn, &fs_name)? {
            Some(g) => g,
            None => continue,
        };

        let common = db::get_common_snapshot(&conn, &guid)?;
        let newest = db::get_newest_snapshot(&conn, &guid)?;

        let decision = backup::decide(common, newest);
        backup::log_decision(&fs_name, &decision);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init();
    info!("st-zfs-send-recv (tutorial edition) starting");

    let config_dir = tempfile::tempdir()?;
    std::fs::write(
        config_dir.path().join("tank.pool"),
        r#"
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
skip_filesystems:
  - "^tank/docker$"
"#,
    )?;

    let configs = config::load_pool_configs(config_dir.path())?;
    info!("Loaded {} pool configuration(s)", configs.len());

    // Come nel vero main.rs: un pool che fallisce NON blocca gli
    // altri. `match` esplicito invece di lasciare che un panic o
    // un'eccezione non gestita interrompa l'intero processo.
    for pool_config in &configs {
        match process_pool(pool_config) {
            Ok(()) => info!("Successfully processed pool: {}", pool_config.name),
            Err(e) => tracing::error!("Error processing pool {}: {:#}", pool_config.name, e),
        }
    }

    info!("st-zfs-send-recv completed");
    Ok(())
}
