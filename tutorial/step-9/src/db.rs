// Sottoinsieme di ../step-7/src/db.rs, già spiegato in dettaglio lì.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS fs_src (pool TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS snap_src (parent_guid TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY, creation TEXT NOT NULL, snapshot TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS snap_dst (parent_guid TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY, creation TEXT NOT NULL, snapshot TEXT NOT NULL);
"#;

pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .context("Failed to create schema")
}

pub fn insert_fs(conn: &Connection, pool: &str, guid: &str, name: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO fs_src (pool, guid, name) VALUES (?, ?, ?)",
        params![pool, guid, name],
    )?;
    Ok(())
}

pub fn insert_snapshot(
    conn: &Connection,
    table: &str,
    parent_guid: &str,
    guid: &str,
    creation: &str,
    snapshot: &str,
) -> Result<()> {
    let query =
        format!("INSERT INTO {table} (parent_guid, guid, creation, snapshot) VALUES (?, ?, ?, ?)");
    conn.execute(&query, params![parent_guid, guid, creation, snapshot])?;
    Ok(())
}

pub fn get_filesystems(conn: &Connection, pool: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM fs_src WHERE pool = ? ORDER BY name")?;
    let names: Result<Vec<String>, rusqlite::Error> =
        stmt.query_map(params![pool], |row| row.get(0))?.collect();
    Ok(names?)
}

pub fn get_common_snapshot(conn: &Connection, fs_guid: &str) -> Result<Option<String>> {
    let query = r#"
    SELECT '@' || s1.snapshot FROM snap_src AS s1
    JOIN snap_dst AS s2 ON s1.guid = s2.guid
    WHERE s1.parent_guid = ? ORDER BY s1.creation DESC LIMIT 1
    "#;
    conn.query_row(query, params![fs_guid], |row| row.get(0))
        .optional()
        .context("Failed to query common snapshot")
}

pub fn get_newest_snapshot(conn: &Connection, fs_guid: &str) -> Result<Option<String>> {
    let query =
        "SELECT '@' || snapshot FROM snap_src WHERE parent_guid = ? ORDER BY creation DESC LIMIT 1";
    conn.query_row(query, params![fs_guid], |row| row.get(0))
        .optional()
        .context("Failed to query newest snapshot")
}

pub fn get_fs_guid(conn: &Connection, name: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT guid FROM fs_src WHERE name = ?",
        params![name],
        |row| row.get(0),
    )
    .optional()
    .context("Failed to query filesystem guid")
}
