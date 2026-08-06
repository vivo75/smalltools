use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use tracing::debug;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS fs_src (
  pool TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS fs_dst (
  pool TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS snap_src (
  parent_guid TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL,
  snapshot TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS snap_dst (
  parent_guid TEXT NOT NULL,
  guid TEXT NOT NULL PRIMARY KEY,
  createtxg TEXT NOT NULL,
  creation TEXT NOT NULL,
  type TEXT NOT NULL,
  used TEXT NOT NULL,
  name TEXT NOT NULL,
  snapshot TEXT NOT NULL
);
"#;

#[derive(Debug, Clone)]
pub struct Filesystem {
    pub pool: String,
    pub guid: String,
    pub createtxg: String,
    pub creation: String,
    pub fs_type: String,
    pub used: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub parent_guid: String,
    pub guid: String,
    pub createtxg: String,
    pub creation: String,
    pub snap_type: String,
    pub used: String,
    pub name: String,
    pub snapshot: String,
}

pub fn create_temp_db(pool: &str) -> Result<PathBuf> {
    let safe_pool = pool.replace("/", "--");
    
    // Create unique temp file
    let temp_file = tempfile::Builder::new()
        .prefix(&safe_pool)
        .suffix(".db")
        .tempfile()
        .context("Failed to create temporary database file")?;

    Ok(temp_file.path().to_path_buf())
}

pub fn initialize_db(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)
        .context("Failed to open database")?;

    for statement in SCHEMA.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            conn.execute(trimmed, [])
                .context("Failed to create table")?;
        }
    }

    debug!("Database initialized at: {:?}", db_path);
    Ok(())
}

pub fn get_connection(db_path: &Path) -> Result<Connection> {
    Connection::open(db_path).context("Failed to open database")
}

pub fn insert_filesystem(
    conn: &Connection,
    fs: &Filesystem,
) -> Result<()> {
    conn.execute(
        "INSERT INTO fs_src (pool, guid, createtxg, creation, type, used, name) 
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            &fs.pool, &fs.guid, &fs.createtxg, &fs.creation, &fs.fs_type, &fs.used, &fs.name
        ],
    )
    .context("Failed to insert filesystem")?;
    Ok(())
}

pub fn insert_snapshot(
    conn: &Connection,
    table: &str,
    snap: &Snapshot,
) -> Result<()> {
    let query = format!(
        "INSERT INTO {} (parent_guid, guid, createtxg, creation, type, used, name, snapshot) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        table
    );

    conn.execute(
        &query,
        params![
            &snap.parent_guid,
            &snap.guid,
            &snap.createtxg,
            &snap.creation,
            &snap.snap_type,
            &snap.used,
            &snap.name,
            &snap.snapshot
        ],
    )
    .context("Failed to insert snapshot")?;

    Ok(())
}

pub fn get_common_snapshot(
    conn: &Connection,
    pool: &str,
    fs_name: &str,
) -> Result<Option<String>> {
    let query = r#"
    SELECT '@' || s1.snapshot
    FROM fs_src AS f
    JOIN snap_src AS s1 ON f.guid = s1.parent_guid
    JOIN snap_dst AS s2 ON s1.guid = s2.guid
    WHERE f.pool = ? AND f.name = ?
    ORDER BY s1.creation DESC
    LIMIT 1
    "#;

    let result = conn
        .query_row(query, params![pool, fs_name], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .context("Failed to query common snapshot")?;

    Ok(result)
}

pub fn get_remote_newest_snapshot(
    conn: &Connection,
    pool: &str,
    fs_name: &str,
) -> Result<Option<String>> {
    let query = r#"
    SELECT '@' || s.snapshot
    FROM fs_src AS f
    JOIN snap_src AS s ON f.guid = s.parent_guid
    WHERE f.pool = ? AND f.name = ?
    ORDER BY s.creation DESC
    LIMIT 1
    "#;

    let result = conn
        .query_row(query, params![pool, fs_name], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .context("Failed to query newest remote snapshot")?;

    Ok(result)
}

pub fn get_fs_property(
    conn: &Connection,
    table: &str,
    field: &str,
    fs_name: &str,
) -> Result<Option<String>> {
    let query = format!(
        "SELECT {} FROM {} WHERE name = ? ORDER BY creation DESC LIMIT 1",
        field, table
    );

    let result = conn
        .query_row(&query, params![fs_name], |row| row.get::<_, String>(0))
        .optional()
        .context("Failed to query filesystem property")?;

    Ok(result)
}

pub fn get_filesystems(
    conn: &Connection,
    table: &str,
    pool: &str,
) -> Result<Vec<String>> {
    let query = format!(
        "SELECT name FROM {} WHERE pool = ? AND name != ? ORDER BY name",
        table
    );

    let mut stmt = conn
        .prepare(&query)
        .context("Failed to prepare query")?;

    let fs_iter = stmt
        .query_map(params![pool, pool], |row| row.get::<_, String>(0))
        .context("Failed to query filesystems")?;

    let mut filesystems = Vec::new();
    for fs in fs_iter {
        filesystems.push(fs.context("Failed to read filesystem")?);
    }

    Ok(filesystems)
}

pub fn clear_tables(conn: &Connection) -> Result<()> {
    for table in &["fs_src", "fs_dst", "snap_src", "snap_dst"] {
        conn.execute(&format!("DELETE FROM {}", table), [])
            .context("Failed to clear table")?;
    }
    Ok(())
}
