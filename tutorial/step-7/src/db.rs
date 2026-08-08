// Sottoinsieme fedele di `../../../src/db.rs`. Lo schema è identico
// (4 tabelle: filesystem/snapshot per lato sorgente e destinazione),
// le query di "trova lo snapshot in comune" sono verbatim.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS fs_src (
  pool TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS fs_dst (
  pool TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS snap_src (
  parent_guid TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY,
  creation TEXT NOT NULL, name TEXT NOT NULL, snapshot TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS snap_dst (
  parent_guid TEXT NOT NULL, guid TEXT NOT NULL PRIMARY KEY,
  creation TEXT NOT NULL, name TEXT NOT NULL, snapshot TEXT NOT NULL
);
"#;

#[derive(Debug, Clone)]
pub struct Filesystem {
    pub pool: String,
    pub guid: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub parent_guid: String,
    pub guid: String,
    pub creation: String,
    pub name: String,
    pub snapshot: String,
}

/// `Connection` possiede il file descriptor sqlite sottostante. Non
/// c'è un metodo `.close()` da chiamare esplicitamente (anche se ne
/// esiste uno per gestire eventuali errori di chiusura): quando la
/// `Connection` esce di scope, il suo `Drop` chiude la connessione da
/// sola. Equivalente Python più vicino:
///
/// ```python
/// with sqlite3.connect(path) as conn:
///     ...
/// # chiusa automaticamente all'uscita dal blocco `with`
/// ```
///
/// ma qui non serve il blocco `with`: qualunque scope va bene, per la
/// stessa ragione vista per `TempDir` nello step 4.
pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .context("Failed to create schema")?;
    Ok(())
}

/// `params![...]` espande in un array di valori bindati per
/// posizione (i placeholder `?` nella query). È l'equivalente diretto
/// dei placeholder `?` del modulo `sqlite3` di Python:
///
/// ```python
/// conn.execute(
///     "INSERT INTO fs_src (pool, guid, name) VALUES (?, ?, ?)",
///     (fs.pool, fs.guid, fs.name),
/// )
/// ```
///
/// La regola di sicurezza è la stessa in entrambi i linguaggi: i
/// *valori* vanno SEMPRE passati come parametri bindati, mai
/// interpolati nella stringa SQL — altrimenti si apre la porta a SQL
/// injection. Qui il compilatore non può impedirti di sbagliare (una
/// query SQL resta una stringa qualunque), ma la API di `rusqlite`
/// rende il percorso sicuro (`params![]`) più comodo di quello
/// insicuro (`format!`).
pub fn insert_filesystem(conn: &Connection, table: &str, fs: &Filesystem) -> Result<()> {
    // Qui SÌ interpoliamo con `format!`, ma solo il NOME DELLA TABELLA
    // (`table`), mai un valore. SQL non permette di parametrizzare
    // identificatori (nomi di tabelle/colonne) con `?` — solo valori.
    // Per questo `table` deve provenire sempre da una lista fissa
    // decisa dal codice (qui: "fs_src" o "fs_dst"), MAI da input utente
    // o da rete: se lo fosse, questa sarebbe una SQL injection nel nome
    // tabella. È lo stesso identico compromesso che affronteresti in
    // Python con lo stesso pattern.
    let query = format!("INSERT INTO {table} (pool, guid, name) VALUES (?, ?, ?)");
    conn.execute(&query, params![fs.pool, fs.guid, fs.name])
        .context("Failed to insert filesystem")?;
    Ok(())
}

pub fn insert_snapshot(conn: &Connection, table: &str, snap: &Snapshot) -> Result<()> {
    let query = format!(
        "INSERT INTO {table} (parent_guid, guid, creation, name, snapshot) VALUES (?, ?, ?, ?, ?)"
    );
    conn.execute(
        &query,
        params![
            snap.parent_guid,
            snap.guid,
            snap.creation,
            snap.name,
            snap.snapshot
        ],
    )
    .context("Failed to insert snapshot")?;
    Ok(())
}

/// Trova lo snapshot più recente presente SIA sul lato sorgente SIA su
/// quello destinazione (stesso guid): il punto di partenza per un
/// backup incrementale. Restituisce `Option<String>`: `None` significa
/// "nessuno snapshot in comune", non un errore.
///
/// In Python, con il modulo `sqlite3`:
///
/// ```python
/// cur = conn.execute(query, (pool, fs_name))
/// row = cur.fetchone()
/// return row[0] if row is not None else None
/// ```
///
/// `.optional()` di rusqlite fa esattamente questo `fetchone() is None`
/// in un colpo solo, restituendo `Ok(None)` invece di un errore quando
/// la query non trova righe (senza `.optional()`, `query_row` restituirebbe
/// `Err(QueryReturnedNoRows)`, che qui NON è l'errore che vogliamo:
/// "nessuno snapshot in comune" è un esito normale, non un fallimento).
pub fn get_common_snapshot(conn: &Connection, pool: &str, fs_name: &str) -> Result<Option<String>> {
    let query = r#"
    SELECT '@' || s1.snapshot
    FROM fs_src AS f
    JOIN snap_src AS s1 ON f.guid = s1.parent_guid
    JOIN snap_dst AS s2 ON s1.guid = s2.guid
    WHERE f.pool = ? AND f.name = ?
    ORDER BY s1.creation DESC
    LIMIT 1
    "#;

    conn.query_row(query, params![pool, fs_name], |row| row.get::<_, String>(0))
        .optional()
        .context("Failed to query common snapshot")
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

    conn.query_row(query, params![pool, fs_name], |row| row.get::<_, String>(0))
        .optional()
        .context("Failed to query newest remote snapshot")
}

/// `query_map` restituisce un ITERATORE di `Result<String, rusqlite::Error>`,
/// una riga alla volta — non materializza subito tutto in memoria come
/// farebbe `cursor.fetchall()` in Python. `.collect::<Result<Vec<_>, _>>()`
/// consuma l'iteratore e si ferma al primo errore, se ce n'è uno,
/// altrimenti produce `Ok(Vec<String>)` — lo stesso idioma
/// `collect::<Result<...>>()` visto nello step 5, applicato qui a righe
/// di database invece che a righe di testo.
pub fn get_filesystems(conn: &Connection, table: &str, pool: &str) -> Result<Vec<String>> {
    let query = format!("SELECT name FROM {table} WHERE pool = ? ORDER BY name");
    let mut stmt = conn.prepare(&query).context("Failed to prepare query")?;

    let names: Result<Vec<String>, rusqlite::Error> = stmt
        .query_map(params![pool], |row| row.get::<_, String>(0))
        .context("Failed to query filesystems")?
        .collect();

    Ok(names?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();

        insert_filesystem(
            &conn,
            "fs_src",
            &Filesystem {
                pool: "tank".to_string(),
                guid: "guid-fs-1".to_string(),
                name: "tank/data".to_string(),
            },
        )
        .unwrap();

        // Uno snapshot vecchio, presente su entrambi i lati (in comune).
        insert_snapshot(
            &conn,
            "snap_src",
            &Snapshot {
                parent_guid: "guid-fs-1".to_string(),
                guid: "guid-snap-old".to_string(),
                creation: "2024-01-01".to_string(),
                name: "tank/data@daily-2024-01-01".to_string(),
                snapshot: "daily-2024-01-01".to_string(),
            },
        )
        .unwrap();
        insert_snapshot(
            &conn,
            "snap_dst",
            &Snapshot {
                parent_guid: "guid-fs-1".to_string(),
                guid: "guid-snap-old".to_string(), // stesso guid: è lo stesso identico snapshot ZFS
                creation: "2024-01-01".to_string(),
                name: "tank/data@daily-2024-01-01".to_string(),
                snapshot: "daily-2024-01-01".to_string(),
            },
        )
        .unwrap();

        // Uno snapshot più recente, solo sul lato sorgente (da inviare).
        insert_snapshot(
            &conn,
            "snap_src",
            &Snapshot {
                parent_guid: "guid-fs-1".to_string(),
                guid: "guid-snap-new".to_string(),
                creation: "2024-01-02".to_string(),
                name: "tank/data@daily-2024-01-02".to_string(),
                snapshot: "daily-2024-01-02".to_string(),
            },
        )
        .unwrap();

        conn
    }

    #[test]
    fn finds_common_snapshot() {
        let conn = seeded_db();
        let common = get_common_snapshot(&conn, "tank", "tank/data").unwrap();
        assert_eq!(common, Some("@daily-2024-01-01".to_string()));
    }

    #[test]
    fn finds_newest_remote_snapshot() {
        let conn = seeded_db();
        let newest = get_remote_newest_snapshot(&conn, "tank", "tank/data").unwrap();
        assert_eq!(newest, Some("@daily-2024-01-02".to_string()));
    }

    #[test]
    fn no_common_snapshot_returns_none_not_error() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();
        insert_filesystem(
            &conn,
            "fs_src",
            &Filesystem {
                pool: "tank".to_string(),
                guid: "guid-fs-2".to_string(),
                name: "tank/empty".to_string(),
            },
        )
        .unwrap();

        let common = get_common_snapshot(&conn, "tank", "tank/empty").unwrap();
        assert_eq!(common, None);
    }

    #[test]
    fn get_filesystems_lists_by_pool() {
        let conn = seeded_db();
        let names = get_filesystems(&conn, "fs_src", "tank").unwrap();
        assert_eq!(names, vec!["tank/data".to_string()]);
    }
}
