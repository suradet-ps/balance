//! Discrepancy view (Phase 2) — the reconciliation strip under the panels.
//!
//! When the drug selected on the HOSxP panel is mapped, this strip shows
//! its comparison against the INVS counterpart: the yearly unit price and
//! variance, a 12-fiscal-month table (จ่าย / ซื้อ / ส่วนต่าง / ราคาต่อหน่วย)
//! and every discrepancy flag with the exact numbers and months that
//! produced it.  All math runs in the backend's pure engine
//! (`reconcile_drug`); this component only renders.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, MappingContext};
use crate::models::{
  format_baht, format_qty, DiscrepancyFlag, ReconcileReport, FISCAL_MONTHS_SHORT,
};
use crate::services::commands;

/// Props for [`DiscrepancyView`].
#[component]
pub fn DiscrepancyView() -> impl IntoView {
  let dash = expect_context::<DashboardContext>();
  let mapping = expect_context::<MappingContext>();

  let report = RwSignal::new(None::<ReconcileReport>);
  let loading = RwSignal::new(false);
  let error = RwSignal::new(None::<String>);
  let gen = RwSignal::new(0u64);

  // Re-fetch whenever the selected HOSxP drug, the year, or its mapping
  // status changes; clear when the drug is gone or unmapped.
  Effect::new(move |_| {
    let icode = dash.hosxp_selected_icode.get();
    let year = dash.selected_year.get();
    let mapped = mapping
      .hosxp_link
      .get()
      .is_some_and(|s| s.status == "mapped");
    gen.update(|g| *g += 1);
    let g = gen.get_untracked();
    let Some(icode) = icode else {
      report.set(None);
      loading.set(false);
      error.set(None);
      return;
    };
    if !mapped {
      report.set(None);
      loading.set(false);
      error.set(None);
      return;
    }
    loading.set(true);
    error.set(None);
    spawn_local(async move {
      match commands::reconcile_drug(year, &icode).await {
        Ok(r) => {
          if gen.get_untracked() == g {
            report.set(Some(r));
          }
        }
        Err(e) => {
          if gen.get_untracked() == g {
            error.set(Some(e.message));
          }
        }
      }
      if gen.get_untracked() == g {
        loading.set(false);
      }
    });
  });

  view! {
      <Show when=move || { report.get().is_some() || loading.get() || error.get().is_some() }>
          <section class="discrepancy-view card">
              <div class="discrepancy-head">
                  <span class="discrepancy-title">
                      <Icon kind=IconKind::BarChart2 size=15 />
                      "การเปรียบเทียบ HOSxP ↔ INVS"
                      <Show when=move || report.get().is_some()>
                          <span class="discrepancy-drug">
                              {move || report.get().map_or(String::new(), |r| format!(
                                  "{} {} ↔ {} {}",
                                  r.icode,
                                  r.drug_name_hosxp,
                                  r.working_code,
                                  r.drug_name_invs,
                              ))}
                          </span>
                      </Show>
                  </span>
                  <Show when=move || !loading.get() && error.get().is_none()>
                      <ReconcileSummary report=report />
                  </Show>
              </div>

              <Show when=move || loading.get()>
                  <div class="discrepancy-empty">
                      <span class="animate-pulse">"กำลังคำนวณการเปรียบเทียบ…"</span>
                  </div>
              </Show>

              <Show when=move || !loading.get() && error.get().is_some()>
                  <div class="discrepancy-empty discrepancy-error">
                      <Icon kind=IconKind::AlertTriangle size=13 />
                      {move || error.get().unwrap_or_default()}
                  </div>
              </Show>

              <Show when=move || !loading.get() && report.get().is_some()>
                  <ReconcileTable report=report />
              </Show>
          </section>
      </Show>
  }
}

/// The headline numbers: yearly unit price, variation, flag count.
#[component]
fn ReconcileSummary(report: RwSignal<Option<ReconcileReport>>) -> impl IntoView {
  let recon = move || report.get().map(|r| r.reconciliation.clone());
  let unit_price = move || recon().and_then(|r| r.unit_price_year);
  let cv_qty = move || recon().and_then(|r| r.cv_dispensed_qty);
  let cv_value = move || recon().and_then(|r| r.cv_purchased_value);
  let flag_count = move || recon().map_or(0, |r| r.flags.len());
  let has_flags = move || flag_count() > 0;
  let year_dispensed = move || recon().map_or(0.0, |r| r.dispensed_qty.iter().sum::<f64>());
  let year_value = move || recon().map_or(0.0, |r| r.purchased_value.iter().sum::<f64>());

  view! {
      <span class="discrepancy-summary">
          <Show when=move || unit_price().is_some()>
              <span class="pill pill-primary">
                  "ราคาต่อหน่วยทั้งปี: "
                  {move || {
                      unit_price().map_or_else(String::new, |p| format!("{} บาท/หน่วย", format_qty(p)))
                  }}
              </span>
          </Show>
          <span class="pill">
              {move || format!("จ่ายรวม {} หน่วย", format_qty(year_dispensed()))}
          </span>
          <span class="pill">
              {move || format!("ซื้อรวม {}", format_baht(year_value(), 0))}
          </span>
          <Show when=move || cv_qty().is_some()>
              <span class="pill">
                  {move || {
                      cv_qty().map_or_else(String::new, |cv| format!("จ่ายผันผวน {:.0}%", cv * 100.0))
                  }}
              </span>
          </Show>
          <Show when=move || cv_value().is_some()>
              <span class="pill">
                  {move || {
                      cv_value().map_or_else(
                          String::new,
                          |cv| format!("ซื้อผันผวน {:.0}%", cv * 100.0),
                      )
                  }}
              </span>
          </Show>
          <Show when=has_flags>
              <span class="pill pill-warn">
                  {move || format!("พบ {} รายการที่ต้องตรวจ", flag_count())}
              </span>
          </Show>
      </span>
  }
}

/// The flags + the 12-month table.
#[component]
fn ReconcileTable(report: RwSignal<Option<ReconcileReport>>) -> impl IntoView {
  let recon = move || report.get().map(|r| r.reconciliation.clone());
  let flags = move || recon().map_or(Vec::new(), |r| r.flags);
  let months = move || (0..12).collect::<Vec<usize>>();

  view! {
      <div class="discrepancy-body">
          <Show when=move || !flags().is_empty()>
              <div class="flag-list">
                  <For each=move || flags() key=flag_key let:flag>
                      <FlagRow flag=flag />
                  </For>
              </div>
          </Show>

          <Show when=move || { flags().is_empty() && recon().is_some() }>
              <div class="discrepancy-clean">
                  <Icon kind=IconKind::Check size=13 />
                  "ไม่พบความผิดปกติ — การจ่ายและการซื้อสอดคล้องกันในปีนี้"
              </div>
          </Show>

          <div class="month-table">
              <div class="month-row month-row--head">
                  <span class="mt-month">"เดือน"</span>
                  <span class="mt-num">"จ่าย (HOSxP)"</span>
                  <span class="mt-num">"ซื้อ (INVS)"</span>
                  <span class="mt-num">"ส่วนต่าง"</span>
                  <span class="mt-num">"ราคา/หน่วย"</span>
              </div>
              <For each=months key=|mi| *mi let:mi>
                  <MonthRow index=mi report=report />
              </For>
          </div>
      </div>
  }
}

/// One fiscal month's row; a zero-dispensing month shows "ไม่มีข้อมูลการจ่าย"
/// in the price column — never a comparable-looking number.
#[component]
fn MonthRow(index: usize, report: RwSignal<Option<ReconcileReport>>) -> impl IntoView {
  let dispensed = move || {
    report
      .get()
      .and_then(|r| r.reconciliation.dispensed_qty.get(index).copied())
      .unwrap_or(0.0)
  };
  let purchased_qty = move || {
    report
      .get()
      .and_then(|r| r.reconciliation.purchased_qty.get(index).copied())
      .unwrap_or(0.0)
  };
  let delta = move || {
    report
      .get()
      .and_then(|r| r.reconciliation.monthly_deltas.get(index).copied())
      .unwrap_or(0.0)
  };
  let price = move || {
    report
      .get()
      .and_then(|r| r.reconciliation.unit_price_month.get(index).copied())
      .flatten()
  };

  view! {
      <div class="month-row">
          <span class="mt-month">{move || FISCAL_MONTHS_SHORT[index]}</span>
          <span class="mt-num">{move || format_qty(dispensed())}</span>
          <span class="mt-num">{move || format_qty(purchased_qty())}</span>
          <span class="mt-num mt-delta">{move || delta_text(delta())}</span>
          <span class="mt-num">
              {move || {
                  price().map_or_else(
                      || "ไม่มีข้อมูลการจ่าย".to_owned(),
                      |p| format!("{} บาท", format_qty(p)),
                  )
              }}
          </span>
      </div>
  }
}

/// +/- delta with a sign so a surplus purchase is visually distinct.
fn delta_text(delta: f64) -> String {
  if delta > 0.0 {
    format!("+{}", format_qty(delta))
  } else {
    format_qty(delta)
  }
}

/// Stable key: (kind, month) — unique within one report (one-sided flags
/// share kind/month only when they carry different one_sided kinds; those
/// differ in kind name, so the pair stays unique).
fn flag_key(f: &DiscrepancyFlag) -> (String, usize) {
  (f.kind.clone(), f.month.unwrap_or(usize::MAX))
}

/// One discrepancy flag with its underlying numbers and month.
#[component]
fn FlagRow(flag: DiscrepancyFlag) -> impl IntoView {
  let flag_state = StoredValue::new(flag);
  let month_label = move || {
    flag_state.get_value().month.map_or_else(
      || "ทั้งปี".to_owned(),
      |m| {
        FISCAL_MONTHS_SHORT
          .get(m)
          .copied()
          .unwrap_or("?")
          .to_owned()
      },
    )
  };
  view! {
      <div class="flag-row">
          <Icon kind=IconKind::AlertTriangle size=13 />
          <span class="flag-text">
              {move || flag_message(&flag_state.get_value(), &month_label())}
          </span>
          <span class="flag-meta">
              {move || {
                  let f = &flag_state.get_value();
                  format!(
                      "จ่าย {} · ซื้อ {} · มูลค่า {}",
                      format_qty(f.dispensed_qty),
                      format_qty(f.purchased_qty),
                      format_baht(f.purchased_value, 0),
                  )
              }}
          </span>
      </div>
  }
}

/// Thai copy for a flag kind (the rules live on the backend; the copy lives
/// here so the UI strings stay in the frontend).
fn flag_message(flag: &DiscrepancyFlag, month_label: &str) -> String {
  match flag.kind.as_str() {
    "zero-use-full-purchase" => format!(
      "ซื้อทั้งปี ({} บาท) แต่ไม่มีการจ่ายยาเลย — ยาที่ซื้อมาไม่ได้ใช้",
      format_qty(flag.purchased_value),
    ),
    "dispensed-without-purchase" => format!(
      "จ่ายยาไปทั้งปี ({} หน่วย) แต่ไม่มีการซื้อเลย — สต็อกเก่าหรือข้อมูลผิดปกติ",
      format_qty(flag.dispensed_qty),
    ),
    "unit-price-spike" => format!(
      "ราคาต่อหน่วยเดือน {} สูงผิดปกติ ({} บาท/หน่วย)",
      month_label,
      format_qty(if flag.dispensed_qty > 0.0 {
        flag.purchased_value / flag.dispensed_qty
      } else {
        0.0
      }),
    ),
    "seasonal-flip" => format!(
      "เดือนที่จ่ายสูงสุด ({}) ไม่ตรงกับเดือนที่ซื้อสูงสุด — อาจมีสต็อกสะสมหรือซื้อล่วงหน้า",
      month_label,
    ),
    "one-sided-month" => match flag.one_sided.as_deref() {
      Some("only-purchased") => format!(
        "เดือน {} ซื้อ ({} หน่วย) แต่ไม่มีการจ่ายเลย",
        month_label,
        format_qty(flag.purchased_qty),
      ),
      _ => format!(
        "เดือน {} จ่าย ({} หน่วย) แต่ไม่มีการซื้อเลย",
        month_label,
        format_qty(flag.dispensed_qty),
      ),
    },
    _ => "พบความผิดปกติ".to_owned(),
  }
}
