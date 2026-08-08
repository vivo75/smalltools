mod db;

use db::{Filesystem, Snapshot};
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    // In-memory: nessun file su disco. Il progetto reale usa invece un
    // file temporaneo (vedi il README) principalmente per poterlo
    // ispezionare con `sqlite3` durante un backup lungo o dopo un
    // crash; per questa demo la persistenza su disco non serve.
    let conn = Connection::open_in_memory()?;
    db::initialize_db(&conn)?;

    db::insert_filesystem(
        &conn,
        "fs_src",
        &Filesystem {
            pool: "tank".to_string(),
            guid: "guid-1".to_string(),
            name: "tank/data".to_string(),
        },
    )?;

    db::insert_snapshot(
        &conn,
        "snap_src",
        &Snapshot {
            parent_guid: "guid-1".to_string(),
            guid: "guid-common".to_string(),
            creation: "2024-01-01".to_string(),
            name: "tank/data@daily-2024-01-01".to_string(),
            snapshot: "daily-2024-01-01".to_string(),
        },
    )?;
    db::insert_snapshot(
        &conn,
        "snap_dst",
        &Snapshot {
            parent_guid: "guid-1".to_string(),
            guid: "guid-common".to_string(),
            creation: "2024-01-01".to_string(),
            name: "tank/data@daily-2024-01-01".to_string(),
            snapshot: "daily-2024-01-01".to_string(),
        },
    )?;
    db::insert_snapshot(
        &conn,
        "snap_src",
        &Snapshot {
            parent_guid: "guid-1".to_string(),
            guid: "guid-newer".to_string(),
            creation: "2024-01-02".to_string(),
            name: "tank/data@daily-2024-01-02".to_string(),
            snapshot: "daily-2024-01-02".to_string(),
        },
    )?;

    let filesystems = db::get_filesystems(&conn, "fs_src", "tank")?;
    println!("filesystems tracked for pool 'tank': {filesystems:?}");

    let common = db::get_common_snapshot(&conn, "tank", "tank/data")?;
    let newest = db::get_remote_newest_snapshot(&conn, "tank", "tank/data")?;

    match (common, newest) {
        (Some(c), Some(n)) if c == n => println!("tank/data is already up to date at {c}"),
        (Some(c), Some(n)) => println!("tank/data{c} => {n}  (incremental backup needed)"),
        (None, _) => println!("tank/data has no common snapshot: needs full initialization"),
        _ => println!("tank/data has nothing to back up yet"),
    }

    Ok(())
}
