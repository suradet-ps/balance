//! Local SQLite app store.
//!
//! Balance-owned data (drug mappings, exclusions) lives in a local SQLite
//! database in the app-data dir, next to `settings.json` — never in the
//! source databases.  The store is opened read-write *locally only* and is
//! versioned through embedded migrations applied at startup, before the UI
//! mounts (see [`open_store`] and [`migrate`]).
//!
//! Migration rule: never edit an already-applied migration file — append a
//! new `NNNN_name.sql` to [`MIGRATIONS`].  Migrations are runnable against a
//! fresh database in CI (unit-tested below), which is what makes them the
//! audit trail for the app-owned schema.

use rusqlite::{Connection, params};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::Manager;

/// Managed state: the single local SQLite connection, serialised behind a
/// mutex.  Every statement here is short and synchronous, so a plain mutex
/// is enough (matching the codebase's "one client behind a lock" style).
pub struct StoreState(pub Arc<Mutex<Connection>>);

impl StoreState {
    /// Lock the connection, tolerating a poisoned mutex (a panic in a
    /// statement would otherwise brick every later command).
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.0
            .lock()
            .map_err(|e| format!("local store lock poisoned: {e}"))
    }
}

/// The ordered migration list.  Each entry is a version name (also the file
/// stem) and its SQL, embedded at compile time so the app ships self-
/// contained — no runtime file discovery.
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_drug_mappings",
        include_str!("../migrations/0001_drug_mappings.sql"),
    ),
    (
        "0002_mapping_exclusions",
        include_str!("../migrations/0002_mapping_exclusions.sql"),
    ),
];

fn sql_err(e: rusqlite::Error) -> String {
    format!("local store error: {e}")
}

/// Resolve the store path (app-data dir + `balance.db`), creating the dir.
fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create app data dir: {e}"))?;
    dir.push("balance.db");
    Ok(dir)
}

/// Open (creating if needed) the local store and migrate it to the latest
/// schema version.  Called once in `lib.rs::run` via `.setup()`, before the
/// UI mounts.
pub fn open_store(app: &tauri::AppHandle) -> Result<StoreState, String> {
    let path = db_path(app)?;
    let mut conn = Connection::open(&path).map_err(|e| format!("cannot open local store: {e}"))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    migrate(&mut conn)?;
    Ok(StoreState(Arc::new(Mutex::new(conn))))
}

/// Apply every migration not yet recorded in `schema_migrations`, in order,
/// each inside a transaction.  Idempotent: running twice is a no-op.
pub fn migrate(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(sql_err)?;

    let applied = applied_versions(conn)?;
    let tx = conn.transaction().map_err(sql_err)?;
    for (version, sql) in MIGRATIONS {
        if applied.contains(*version) {
            continue;
        }
        tx.execute_batch(sql)
            .map_err(|e| format!("migration {version} failed: {e}"))?;
        tx.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            params![version],
        )
        .map_err(sql_err)?;
    }
    tx.commit().map_err(sql_err)?;
    Ok(())
}

fn applied_versions(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT version FROM schema_migrations")
        .map_err(sql_err)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sql_err)?;
    let mut versions = HashSet::new();
    for row in rows {
        versions.insert(row.map_err(sql_err)?);
    }
    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Connection {
        Connection::open_in_memory().expect("in-memory store opens")
    }

    fn table_names(conn: &Connection) -> HashSet<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .expect("sqlite_master query prepares");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("sqlite_master query runs");
        rows.map(|r| r.expect("name column reads"))
            .filter(|n| !n.starts_with("sqlite_"))
            .collect()
    }

    #[test]
    fn migrate_creates_the_schema_on_a_fresh_db() {
        let mut conn = fresh_db();
        migrate(&mut conn).expect("fresh migration succeeds");

        let tables = table_names(&conn);
        assert!(tables.contains("schema_migrations"), "{tables:?}");
        assert!(tables.contains("drug_mappings"), "{tables:?}");
        assert!(tables.contains("mapping_exclusions"), "{tables:?}");
    }

    #[test]
    fn migrate_is_idempotent_and_records_versions() {
        let mut conn = fresh_db();
        migrate(&mut conn).expect("first migration succeeds");
        migrate(&mut conn).expect("second migration is a no-op");

        let versions = applied_versions(&conn).expect("versions read");
        let expected: HashSet<String> = MIGRATIONS.iter().map(|(v, _)| v.to_string()).collect();
        assert_eq!(versions, expected);
    }

    #[test]
    fn drug_mappings_schema_enforces_method_enum() {
        let mut conn = fresh_db();
        migrate(&mut conn).expect("migration succeeds");
        let err = conn
            .execute(
                "INSERT INTO drug_mappings (icode, working_code, match_method)
                 VALUES ('A', 'B', 'bogus')",
                [],
            )
            .expect_err("bogus method must be rejected");
        assert!(err.to_string().contains("CHECK"), "{err}");
    }
}
