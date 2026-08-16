//! Tauri command layer for the reconciliation engine (Phase 2).
//!
//! Wires the pure math in [`super`] to the real data: resolves a HOSxP
//! `icode` to its mapped INVS `working_code` (local store), fetches both
//! monthly series from the source databases, and returns the computed
//! reconciliation.  The command is a thin adapter — all rules live in the
//! pure module.

use crate::hosxp::db::with_pool;
use crate::hosxp::commands::fetch_monthly_qty;
use crate::invs::commands::fetch_monthly_value;
use crate::invs::db::InvsDbState;
use crate::mapping::repo;
use crate::reconcile::{ReconcileInput, Reconciliation, Thresholds};
use crate::store::StoreState;
use serde::{Deserialize, Serialize};

/// The reconciled view of one mapped drug for one fiscal year.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReconcileReport {
    pub icode: String,
    pub working_code: String,
    pub drug_name_hosxp: String,
    pub drug_name_invs: String,
    /// The pure engine's output (both series are embedded in it via the
    /// numbers each flag carries).
    pub reconciliation: Reconciliation,
}

/// Reconcile a mapped HOSxP drug against its INVS counterpart for `year`
/// (Thai fiscal year, CE).  Errors only when the drug is unmapped or a
/// database is unreachable — never for "no data" states (those are
/// legitimate `None`s in the report).
#[tauri::command]
pub async fn reconcile_drug(
    store: tauri::State<'_, StoreState>,
    invs: tauri::State<'_, InvsDbState>,
    year: i32,
    icode: String,
) -> Result<ReconcileReport, String> {
    // 1. Resolve the mapping locally (never in a source DB).
    let (working_code, drug_name_hosxp, drug_name_invs) = {
        let conn = store.lock()?;
        let Some((_i, wc, name_h, name_i, _method, _score)) = repo::link_by_icode(&conn, &icode)?
        else {
            return Err("ยานี้ยังไม่มีการแมปกับ INVS — ไปที่หน้าแมปยาก่อน".to_string());
        };
        (wc, name_h, name_i)
    };

    // 2. Fetch both monthly series in fiscal order.
    let icode_query = icode.clone();
    let dispensed = with_pool(move |pool| {
        Box::pin(async move { fetch_monthly_qty(pool, year, &icode_query).await })
    })
    .await?
    .into_iter()
    .find(|d| d.icode == icode)
    .map(|d| d.monthly_qty)
    .unwrap_or_else(|| vec![0.0; 12]);

    let purchased = fetch_monthly_value(invs.inner(), year as u16, &working_code).await?;

    // 3. Run the pure engine.
    let reconciliation = crate::reconcile::reconcile(
        &ReconcileInput {
            dispensed_qty: dispensed,
            purchased_qty: purchased.monthly_qty.to_vec(),
            purchased_value: purchased.monthly_value.to_vec(),
        },
        Thresholds::default(),
    );

    Ok(ReconcileReport {
        icode,
        working_code,
        drug_name_hosxp,
        drug_name_invs,
        reconciliation,
    })
}
