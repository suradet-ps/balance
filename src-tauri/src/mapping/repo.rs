//! Data-access layer for the local store (drug mappings + exclusions).
//!
//! Plain functions over `rusqlite::Connection` — no Tauri types, no I/O
//! beyond the store itself, so the SQL is unit-testable in isolation.

use rusqlite::{Connection, params};
use std::collections::HashMap;

fn sql_err(e: rusqlite::Error) -> String {
    format!("local store error: {e}")
}

/// A full mapping row: `(icode, working_code, drug_name_hosxp, drug_name_invs,
/// match_method, match_score)`.
pub type LinkRow = (String, String, String, String, String, Option<f64>);

/// The latest mapping for a HOSxP `icode`, if any.
pub fn link_by_icode(conn: &Connection, icode: &str) -> Result<Option<LinkRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT icode, working_code, drug_name_hosxp, drug_name_invs,
                    match_method, match_score
             FROM drug_mappings
             WHERE icode = ?1
             ORDER BY id DESC
             LIMIT 1",
        )
        .map_err(sql_err)?;
    let mut rows = stmt
        .query_map(params![icode], row_to_link)
        .map_err(sql_err)?;
    rows.next().transpose().map_err(sql_err)
}

/// The latest mapping for an INVS `working_code`, if any.
pub fn link_by_working_code(
    conn: &Connection,
    working_code: &str,
) -> Result<Option<LinkRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT icode, working_code, drug_name_hosxp, drug_name_invs,
                    match_method, match_score
             FROM drug_mappings
             WHERE working_code = ?1
             ORDER BY id DESC
             LIMIT 1",
        )
        .map_err(sql_err)?;
    let mut rows = stmt
        .query_map(params![working_code], row_to_link)
        .map_err(sql_err)?;
    rows.next().transpose().map_err(sql_err)
}

fn row_to_link(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

/// Insert or update a mapping (and clear any exclusion for the same icode —
/// the two states are mutually exclusive).  Inside a transaction.
///
/// A HOSxP drug has **one** active link at a time: remapping an icode to a
/// different working code replaces the previous link instead of stacking a
/// second row.  Otherwise the UI (which reads only the latest link) would
/// silently keep the old mapping alive after "ยกเลิกการแมป".
#[allow(clippy::too_many_arguments)]
pub fn upsert(
    conn: &mut Connection,
    icode: &str,
    working_code: &str,
    drug_name_hosxp: &str,
    drug_name_invs: &str,
    method: &str,
    score: Option<f64>,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(sql_err)?;
    tx.execute(
        "DELETE FROM mapping_exclusions WHERE icode = ?1",
        params![icode],
    )
    .map_err(sql_err)?;
    tx.execute("DELETE FROM drug_mappings WHERE icode = ?1", params![icode])
        .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO drug_mappings
            (icode, working_code, drug_name_hosxp, drug_name_invs, match_method, match_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            icode,
            working_code,
            drug_name_hosxp,
            drug_name_invs,
            method,
            score
        ],
    )
    .map_err(sql_err)?;
    tx.commit().map_err(sql_err)?;
    Ok(())
}

/// Break a specific link.
pub fn remove(conn: &Connection, icode: &str, working_code: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM drug_mappings WHERE icode = ?1 AND working_code = ?2",
        params![icode, working_code],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Mark a HOSxP drug as having no INVS equivalent, dropping any existing
/// links (mutually exclusive states).
pub fn set_no_invs(conn: &mut Connection, icode: &str, reason: &str) -> Result<(), String> {
    let tx = conn.transaction().map_err(sql_err)?;
    tx.execute("DELETE FROM drug_mappings WHERE icode = ?1", params![icode])
        .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO mapping_exclusions (icode, reason) VALUES (?1, ?2)
         ON CONFLICT (icode) DO UPDATE SET
            reason     = excluded.reason,
            updated_at = datetime('now')",
        params![icode, reason],
    )
    .map_err(sql_err)?;
    tx.commit().map_err(sql_err)?;
    Ok(())
}

/// Remove the "no INVS equivalent" mark.
pub fn unset_no_invs(conn: &Connection, icode: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM mapping_exclusions WHERE icode = ?1",
        params![icode],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// `icode → reason` for every exclusion.
pub fn excluded_map(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT icode, reason FROM mapping_exclusions")
        .map_err(sql_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_err)?;
    let mut map = HashMap::new();
    for row in rows {
        let (icode, reason) = row.map_err(sql_err)?;
        map.insert(icode, reason);
    }
    Ok(map)
}

/// Latest `(working_code, match_method)` per icode for the given icodes
/// (only icodes that have at least one mapping appear in the map).
pub fn links_for_icodes(
    conn: &Connection,
    icodes: &[&str],
) -> Result<HashMap<String, (String, String)>, String> {
    let mut map = HashMap::new();
    if icodes.is_empty() {
        return Ok(map);
    }
    let placeholders = icodes.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT icode, working_code, match_method
         FROM drug_mappings
         WHERE icode IN ({placeholders})
         ORDER BY id DESC"
    );
    let mut stmt = conn.prepare(&sql).map_err(sql_err)?;
    let mut params = Vec::with_capacity(icodes.len());
    for i in icodes {
        params.push(*i);
    }
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(sql_err)?;
    for row in rows {
        let (icode, working_code, method) = row.map_err(sql_err)?;
        map.entry(icode).or_insert((working_code, method));
    }
    Ok(map)
}

/// `(total, by_method, exclusions)` counts for the mapping header.
pub fn stats(conn: &Connection) -> Result<(i64, HashMap<String, i64>, i64), String> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM drug_mappings", [], |row| row.get(0))
        .map_err(sql_err)?;
    let mut by_method = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT match_method, COUNT(*) FROM drug_mappings GROUP BY match_method")
            .map_err(sql_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (method, count) = row.map_err(sql_err)?;
            by_method.insert(method, count);
        }
    }
    let exclusions: i64 = conn
        .query_row("SELECT COUNT(*) FROM mapping_exclusions", [], |row| {
            row.get(0)
        })
        .map_err(sql_err)?;
    Ok((total, by_method, exclusions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::migrate;

    fn repo_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory store opens");
        migrate(&mut conn).expect("migrations apply");
        conn
    }

    #[test]
    fn upsert_roundtrip_and_uniqueness() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "ยา A", "Drug A", "manual", None).expect("first insert");
        upsert(
            &mut conn,
            "A1",
            "W1",
            "ยา A ใหม่",
            "Drug A ใหม่",
            "approved",
            Some(0.99),
        )
        .expect("same-pair upsert updates");
        let link = link_by_icode(&conn, "A1").expect("link reads");
        let (icode, wc, name_h, name_i, method, score) = link.expect("link exists");
        assert_eq!((icode.as_str(), wc.as_str()), ("A1", "W1"));
        assert_eq!(name_h, "ยา A ใหม่");
        assert_eq!(name_i, "Drug A ใหม่");
        assert_eq!(method, "approved");
        assert_eq!(score, Some(0.99));
    }

    #[test]
    fn two_codes_can_share_one_working_code_but_lookup_is_latest() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "", "", "manual", None).expect("A1→W1");
        upsert(&mut conn, "A2", "W1", "", "", "manual", None).expect("A2→W1");
        let by_ic = link_by_working_code(&conn, "W1").expect("read");
        let (icode, ..) = by_ic.expect("exists");
        assert_eq!(icode, "A2");
        let links = links_for_icodes(&conn, &["A1", "A2", "A3"]).expect("batch read");
        assert!(links.contains_key("A1"));
        assert!(links.get("A2").map(|(w, _)| w.as_str()) == Some("W1"));
        assert!(!links.contains_key("A3"));
    }

    #[test]
    fn remapping_an_icode_replaces_the_previous_link() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "", "", "manual", None).expect("A1→W1");
        upsert(&mut conn, "A1", "W2", "", "", "manual", None).expect("A1→W2");

        // Exactly one row per icode: the old link is gone, not hidden.
        let link = link_by_icode(&conn, "A1").expect("read");
        let (_, wc, ..) = link.expect("new link exists");
        assert_eq!(wc, "W2");
        let links = links_for_icodes(&conn, &["A1"]).expect("batch read");
        assert_eq!(links.get("A1").map(|(w, _)| w.as_str()), Some("W2"));

        // Breaking the visible link really unmaps the drug — no stale
        // earlier mapping resurfaces.
        remove(&conn, "A1", "W2").expect("remove");
        assert!(link_by_icode(&conn, "A1").expect("read").is_none());
        assert!(
            links_for_icodes(&conn, &["A1"])
                .expect("batch read")
                .is_empty()
        );
    }

    #[test]
    fn re_confirming_the_same_pair_is_still_an_upsert() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "ยา A", "Drug A", "auto", Some(0.9)).expect("first");
        upsert(
            &mut conn,
            "A1",
            "W1",
            "ยา A ใหม่",
            "Drug A ใหม่",
            "approved",
            Some(0.99),
        )
        .expect("same pair again");
        let (_, wc, name_h, name_i, method, score) = link_by_icode(&conn, "A1")
            .expect("read")
            .expect("still linked");
        assert_eq!(wc, "W1");
        assert_eq!(name_h, "ยา A ใหม่");
        assert_eq!(name_i, "Drug A ใหม่");
        assert_eq!(method, "approved");
        assert_eq!(score, Some(0.99));
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM drug_mappings", [], |r| r.get(0))
            .expect("count");
        assert_eq!(total, 1);
    }

    #[test]
    fn remove_breaks_only_the_given_link() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "", "", "auto", Some(1.0)).expect("insert");
        remove(&conn, "A1", "W1").expect("remove");
        assert!(link_by_icode(&conn, "A1").expect("read").is_none());
    }

    #[test]
    fn no_invs_marker_is_mutually_exclusive_with_mappings() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "", "", "manual", None).expect("insert");
        set_no_invs(&mut conn, "A1", "ไม่จัดซื้อแล้ว").expect("mark");
        assert!(link_by_icode(&conn, "A1").expect("read").is_none());
        let ex = excluded_map(&conn).expect("exclusions read");
        assert_eq!(ex.get("A1").map(String::as_str), Some("ไม่จัดซื้อแล้ว"));

        upsert(&mut conn, "A1", "W2", "", "", "manual", None).expect("rematch");
        assert!(excluded_map(&conn).expect("exclusions read").is_empty());

        unset_no_invs(&conn, "A1").expect("unmark");
        assert!(excluded_map(&conn).expect("exclusions read").is_empty());
    }

    #[test]
    fn stats_break_down_by_method() {
        let mut conn = repo_db();
        upsert(&mut conn, "A1", "W1", "", "", "auto", Some(0.97)).expect("auto");
        upsert(&mut conn, "A2", "W2", "", "", "manual", None).expect("manual");
        set_no_invs(&mut conn, "A3", "").expect("exclusion");
        let (total, by_method, exclusions) = stats(&conn).expect("stats");
        assert_eq!(total, 2);
        assert_eq!(by_method.get("auto"), Some(&1));
        assert_eq!(by_method.get("manual"), Some(&1));
        assert_eq!(exclusions, 1);
    }
}
