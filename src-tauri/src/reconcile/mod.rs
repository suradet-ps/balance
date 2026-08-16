//! Reconciliation & discrepancy engine (Phase 2) — pure, deterministic math.
//!
//! This module has **no I/O**: it takes the two monthly series — HOSxP
//! dispensed quantities and the mapped INVS purchase quantities/values,
//! both already in Thai fiscal order (index 0 = ต.ค.) — and produces the
//! numbers and rule-based flags the pharmacy director actually wants.
//!
//! **Why the comparison is year-first, not month-first.** A hospital buys a
//! drug once or twice a year (stock covering 12 months of dispensing), so a
//! month-by-month "purchase vs dispensing" comparison is structurally
//! noisy — a purchase month without dispensing is normal, not an anomaly.
//! The engine therefore answers the questions that are answerable:
//!
//! - **year level**: total dispensed vs total purchased (coverage ratio),
//!   the yearly unit price, and whether the *stock curve* (cumulative
//!   purchased − dispensed) still shows a material imbalance at year end;
//! - **month level**: purchase events and the cumulative stock curve are
//!   reported as *data* (the frontend table), never flagged as anomalies;
//! - **unit price**: per-month *purchase* prices (value ÷ qty on purchase
//!   months only — comparing purchase value against that month's dispensing
//!   is meaningless when the stock was bought earlier).
//!
//! Every rule is a pure function with unit tests and synthetic fixtures;
//! thresholds are parameters with defaults, so Phase 8 can expose them in
//! settings without touching the rules.  Zero-quantity months yield `None`
//! prices — never `∞`, never a number that looks comparable when it is not.

pub mod commands;

use serde::{Deserialize, Serialize};

/// One kind of discrepancy flag, serialised as a stable kebab-case string
/// so the frontend can render Thai copy without re-implementing the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlagKind {
    /// Bought all year (value > 0) but never dispensed anything.
    ZeroUseFullPurchase,
    /// Dispensed all year but never purchased (legacy stock / data problem).
    DispensedWithoutPurchase,
    /// One purchase month's price exceeds `unit_price_spike_factor` × the
    /// median monthly purchase price.
    UnitPriceSpike,
    /// The stock curve still shows a material imbalance at year end:
    /// leftover stock (bought more than dispensed) or over-use (dispensed
    /// more than bought — stock from previous years / data problem).
    YearEndStockGap,
}

/// Which direction a [`FlagKind::YearEndStockGap`] flag points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StockGapKind {
    /// Bought more than dispensed — stock carried past the year end
    /// (expiry / over-procurement risk).
    Overstock,
    /// Dispensed more than bought — consumed stock from previous years,
    /// or a data problem.
    Overuse,
}

/// A single discrepancy flag, carrying the two numbers that produced it so
/// the pharmacist can verify against the source systems.  `month` is the
/// fiscal-month index (0 = ต.ค., `None` = whole fiscal year).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyFlag {
    pub kind: FlagKind,
    pub month: Option<usize>,
    pub gap: Option<StockGapKind>,
    /// HOSxP dispensed quantity for the month / the whole year.
    pub dispensed_qty: f64,
    /// INVS purchase quantity for the month / the whole year.
    pub purchased_qty: f64,
    /// INVS purchase value (THB) for the month / the whole year.
    pub purchased_value: f64,
}

/// Rule thresholds.  Defaults match the roadmap; Phase 8 makes them
/// user-configurable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// `unit_price_spike` fires when a purchase month's price is above
    /// `factor` × the median monthly purchase price.
    pub unit_price_spike_factor: f64,
    /// A "purchase" counts only when the month's INVS value is above this
    /// (avoids flagging ฿0 invoice rows as purchases).
    pub min_purchase_value: f64,
    /// `year_end_stock_gap` fires when the cumulative stock curve at year
    /// end deviates from zero by more than this fraction of the yearly
    /// dispensed quantity.
    pub year_end_stock_gap_ratio: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            unit_price_spike_factor: 3.0,
            min_purchase_value: 0.0,
            year_end_stock_gap_ratio: 0.25,
        }
    }
}

/// Both monthly series, fiscal order (index 0 = ต.ค., 11 = ก.ย.).
#[derive(Debug, Clone)]
pub struct ReconcileInput {
    /// HOSxP dispensed quantities per fiscal month.
    pub dispensed_qty: Vec<f64>,
    /// INVS `QTY_ORDER` per fiscal month.
    pub purchased_qty: Vec<f64>,
    /// INVS purchase value (THB) per fiscal month.
    pub purchased_value: Vec<f64>,
}

/// The full reconciliation result for one mapped drug and one fiscal year.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reconciliation {
    /// HOSxP dispensed quantity per fiscal month (echoed input — makes the
    /// report self-contained for the table view).
    pub dispensed_qty: Vec<f64>,
    /// INVS purchased quantity per fiscal month (echoed input).
    pub purchased_qty: Vec<f64>,
    /// INVS purchase value (THB) per fiscal month (echoed input).
    pub purchased_value: Vec<f64>,
    /// Yearly unit price = Σ purchase value ÷ Σ dispensed quantity — the
    /// cost per dispensed unit (`None` when nothing dispensed).
    pub unit_price_year: Option<f64>,
    /// Monthly *purchase* price = value ÷ qty, on purchase months only
    /// (`None` where nothing was bought — that month has no price event).
    /// Comparing purchase value against the same month's dispensing is
    /// meaningless with stock bought in bulk, so the monthly price is a
    /// purchase-price, not a dispensing-price.
    pub purchase_price_month: Vec<Option<f64>>,
    /// Per-month purchased − dispensed quantity delta (fiscal order).
    pub monthly_deltas: Vec<f64>,
    /// The cumulative stock curve: running sum of the deltas, i.e. the
    /// implied stock-on-hand across the year (index 11 = year end).
    pub cumulative_deltas: Vec<f64>,
    /// Σ dispensed ÷ Σ purchased (None when nothing was purchased) — how
    /// much of what was bought actually left the pharmacy this year.
    pub coverage_ratio: Option<f64>,
    /// Coefficient of variation of the dispensed quantities (`None` when
    /// the mean is 0 — variance is meaningless on an all-zero series).
    pub cv_dispensed_qty: Option<f64>,
    /// Coefficient of variation of the purchase values.
    pub cv_purchased_value: Option<f64>,
    pub flags: Vec<DiscrepancyFlag>,
}

/// Unit price = value ÷ quantity.  `None` when nothing was bought /
/// dispensed — a zero-quantity month renders "no data".
#[must_use]
pub fn unit_price(value: f64, qty: f64) -> Option<f64> {
    if qty > 0.0 { Some(value / qty) } else { None }
}

/// Coefficient of variation (σ/μ) of a series; `None` for empty or zero-mean
/// series.
#[must_use]
pub fn coefficient_of_variation(values: &[f64]) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let mean: f64 = values.iter().sum::<f64>() / n as f64;
    if mean == 0.0 {
        return None;
    }
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    Some(variance.sqrt() / mean)
}

/// Median of the non-`None` values; `None` when there are none.
fn median_of(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut vals: Vec<f64> = values.collect();
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = vals.len() / 2;
    if vals.len().is_multiple_of(2) {
        Some((vals[mid - 1] + vals[mid]) / 2.0)
    } else {
        Some(vals[mid])
    }
}

/// Run every rule against the two series.  `input` series must be
/// 12 entries in fiscal order (callers debug-assert this).
#[must_use]
pub fn reconcile(input: &ReconcileInput, thresholds: Thresholds) -> Reconciliation {
    let ReconcileInput {
        dispensed_qty,
        purchased_qty,
        purchased_value,
    } = input;
    debug_assert_eq!(dispensed_qty.len(), 12);
    debug_assert_eq!(purchased_qty.len(), 12);
    debug_assert_eq!(purchased_value.len(), 12);

    // ── Year-level figures ─────────────────────────────────────────────
    let year_dispensed: f64 = dispensed_qty.iter().sum();
    let year_purchased_qty: f64 = purchased_qty.iter().sum();
    let year_value: f64 = purchased_value.iter().sum();
    let unit_price_year = unit_price(year_value, year_dispensed);
    let coverage_ratio = if year_purchased_qty > 0.0 {
        Some(year_dispensed / year_purchased_qty)
    } else {
        None
    };

    // ── Monthly data (prices, deltas, the stock curve) ─────────────────
    // Prices are *purchase* prices on purchase months only; deltas and the
    // cumulative curve are data for the table — not flags (bulk buying
    // makes month-level purchase↔dispensing mismatches normal).
    let purchase_price_month: Vec<Option<f64>> = (0..12)
        .map(|m| {
            if purchased_value[m] > thresholds.min_purchase_value {
                unit_price(purchased_value[m], purchased_qty[m])
            } else {
                None
            }
        })
        .collect();
    let mut monthly_deltas = Vec::with_capacity(12);
    let mut cumulative_deltas = Vec::with_capacity(12);
    let mut running = 0.0;
    for m in 0..12 {
        let delta = purchased_qty[m] - dispensed_qty[m];
        running += delta;
        monthly_deltas.push(delta);
        cumulative_deltas.push(running);
    }

    let mut flags: Vec<DiscrepancyFlag> = Vec::new();

    // ── Whole-year flags ───────────────────────────────────────────────
    if year_dispensed <= 0.0 && year_value > 0.0 {
        flags.push(DiscrepancyFlag {
            kind: FlagKind::ZeroUseFullPurchase,
            month: None,
            gap: None,
            dispensed_qty: year_dispensed,
            purchased_qty: year_purchased_qty,
            purchased_value: year_value,
        });
    }
    if year_dispensed > 0.0 && year_value <= 0.0 {
        flags.push(DiscrepancyFlag {
            kind: FlagKind::DispensedWithoutPurchase,
            month: None,
            gap: None,
            dispensed_qty: year_dispensed,
            purchased_qty: year_purchased_qty,
            purchased_value: year_value,
        });
    }

    // ── Unit-price spike (on purchase months only) ─────────────────────
    let median_price = median_of(purchase_price_month.iter().flatten().copied());
    if let Some(median) = median_price {
        for m in 0..12 {
            let Some(price) = purchase_price_month[m] else {
                continue;
            };
            let spike = if median > 0.0 {
                price > thresholds.unit_price_spike_factor * median
            } else {
                // Median 0 with a positive price: any positive price is a
                // spike away from an all-free year.
                price > 0.0
            };
            if spike {
                flags.push(DiscrepancyFlag {
                    kind: FlagKind::UnitPriceSpike,
                    month: Some(m),
                    gap: None,
                    dispensed_qty: dispensed_qty[m],
                    purchased_qty: purchased_qty[m],
                    purchased_value: purchased_value[m],
                });
            }
        }
    }

    // ── Year-end stock gap ─────────────────────────────────────────────
    // The stock curve's end point is Σpurchased − Σdispensed.  A material
    // residual in either direction is the real reconciliation question
    // (bought more than used → stock carried past year end; used more than
    // bought → stock from previous years / data problem).
    if year_dispensed > 0.0 {
        let gap = cumulative_deltas[11];
        let threshold = thresholds.year_end_stock_gap_ratio * year_dispensed;
        let kind = if gap > threshold {
            Some(StockGapKind::Overstock)
        } else if gap < -threshold {
            Some(StockGapKind::Overuse)
        } else {
            None
        };
        if let Some(gap_kind) = kind {
            flags.push(DiscrepancyFlag {
                kind: FlagKind::YearEndStockGap,
                month: None,
                gap: Some(gap_kind),
                dispensed_qty: year_dispensed,
                purchased_qty: year_purchased_qty,
                purchased_value: year_value,
            });
        }
    }

    flags.sort_by_key(|f| (f.kind as u8, f.month.unwrap_or(usize::MAX)));

    Reconciliation {
        dispensed_qty: dispensed_qty.clone(),
        purchased_qty: purchased_qty.clone(),
        purchased_value: purchased_value.clone(),
        unit_price_year,
        purchase_price_month,
        monthly_deltas,
        cumulative_deltas,
        coverage_ratio,
        cv_dispensed_qty: coefficient_of_variation(dispensed_qty),
        cv_purchased_value: coefficient_of_variation(purchased_value),
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        dispensed: [f64; 12],
        purchased_qty: [f64; 12],
        purchased_value: [f64; 12],
    ) -> ReconcileInput {
        ReconcileInput {
            dispensed_qty: dispensed.to_vec(),
            purchased_qty: purchased_qty.to_vec(),
            purchased_value: purchased_value.to_vec(),
        }
    }

    const ZERO: [f64; 12] = [0.0; 12];

    #[test]
    fn unit_price_guards_division_by_zero() {
        assert_eq!(unit_price(100.0, 0.0), None, "no dispensing → no price");
        assert_eq!(unit_price(100.0, 20.0), Some(5.0));
        assert_eq!(unit_price(0.0, 20.0), Some(0.0));
    }

    #[test]
    fn healthy_year_has_no_flags() {
        // Steady dispensing (10/mo) and matching purchases (10/mo, ฿10/unit):
        // deltas 0, stock curve flat at 0, no spikes, coverage 100%.
        let dispensed = [10.0; 12];
        let purchased_qty = [10.0; 12];
        let purchased_value = [100.0; 12];
        let r = reconcile(
            &input(dispensed, purchased_qty, purchased_value),
            Thresholds::default(),
        );
        assert!(r.flags.is_empty(), "{:?}", r.flags);
        assert_eq!(r.unit_price_year, Some(10.0));
        assert_eq!(r.monthly_deltas, vec![0.0; 12]);
        assert_eq!(r.cumulative_deltas, vec![0.0; 12]);
        assert_eq!(r.coverage_ratio, Some(1.0));
        assert!(r.cv_dispensed_qty.is_some());
        assert!(r.cv_purchased_value.is_some());
    }

    #[test]
    fn bulk_purchase_with_months_of_dispensing_is_normal_not_flagged() {
        // The user's real-world shape: bought ONCE (fiscal month 0, 120 units
        // covering the year), dispensed steadily 10/mo.  Month 0 has purchases
        // without dispensing... no wait, it has both.  Months 1..11 dispense
        // without purchases — normal stock behavior, must NOT be flagged.
        let dispensed = [10.0; 12];
        let mut purchased_qty = ZERO;
        let mut purchased_value = ZERO;
        purchased_qty[0] = 120.0;
        purchased_value[0] = 1200.0;
        let r = reconcile(
            &input(dispensed, purchased_qty, purchased_value),
            Thresholds::default(),
        );
        // Coverage 100% and the stock curve returns to zero by year end →
        // no year-end gap, no one-sided flags (by design).
        assert_eq!(r.coverage_ratio, Some(1.0));
        assert_eq!(
            r.cumulative_deltas[0], 110.0,
            "stock piles up after the buy"
        );
        assert_eq!(r.cumulative_deltas[11], 0.0, "stock fully consumed by Sep");
        assert!(r.flags.is_empty(), "{:?}", r.flags);
        // Monthly purchase price exists only on the purchase month.
        assert_eq!(r.purchase_price_month[0], Some(10.0));
        assert_eq!(r.purchase_price_month[1], None);
    }

    #[test]
    fn zero_use_full_purchase_is_flagged() {
        let r = reconcile(&input(ZERO, [10.0; 12], [100.0; 12]), Thresholds::default());
        assert!(
            r.flags
                .iter()
                .any(|f| f.kind == FlagKind::ZeroUseFullPurchase && f.month.is_none()),
            "{:?}",
            r.flags
        );
        assert_eq!(
            r.unit_price_year, None,
            "nothing dispensed → no yearly price"
        );
    }

    #[test]
    fn dispensed_without_purchase_is_flagged() {
        let mut dispensed = ZERO;
        dispensed[0] = 50.0;
        let r = reconcile(&input(dispensed, ZERO, ZERO), Thresholds::default());
        assert!(
            r.flags
                .iter()
                .any(|f| f.kind == FlagKind::DispensedWithoutPurchase && f.month.is_none()),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn unit_price_spike_fires_above_the_threshold() {
        // 10 units × ฿10/month, except month 5: same 10 units for ฿400
        // (purchase price ฿40 vs median ฿10 → > 3×).
        let mut value = [100.0; 12];
        value[5] = 400.0;
        let r = reconcile(&input([10.0; 12], [10.0; 12], value), Thresholds::default());
        let spike: Vec<&DiscrepancyFlag> = r
            .flags
            .iter()
            .filter(|f| f.kind == FlagKind::UnitPriceSpike)
            .collect();
        assert_eq!(spike.len(), 1, "{:?}", r.flags);
        assert_eq!(spike[0].month, Some(5));
        assert_eq!(spike[0].purchased_value, 400.0);
    }

    #[test]
    fn normal_price_noise_stays_below_the_threshold() {
        // Prices between 9 and 11 (median 10): 11 < 3×10 → no spike.
        let value: [f64; 12] = [
            90.0, 95.0, 100.0, 105.0, 110.0, 100.0, 90.0, 95.0, 105.0, 110.0, 95.0, 105.0,
        ];
        let r = reconcile(&input([10.0; 12], [10.0; 12], value), Thresholds::default());
        assert!(
            !r.flags.iter().any(|f| f.kind == FlagKind::UnitPriceSpike),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn single_purchase_never_spikes() {
        // One purchase all year → the median IS that price → no spike
        // (a single data point cannot detect a spike; that needs years).
        let mut purchased_qty = ZERO;
        let mut value = ZERO;
        purchased_qty[0] = 120.0;
        value[0] = 2400.0;
        let r = reconcile(
            &input([10.0; 12], purchased_qty, value),
            Thresholds::default(),
        );
        assert!(
            !r.flags.iter().any(|f| f.kind == FlagKind::UnitPriceSpike),
            "{:?}",
            r.flags
        );
        assert_eq!(r.unit_price_year, Some(20.0));
    }

    #[test]
    fn year_end_overstock_is_flagged_with_the_curve_numbers() {
        // Bought 200 in Oct, dispensed 10/mo (120/year) → stock curve ends
        // at +80, which is > 25% of 120 → Overstock.
        let mut purchased_qty = ZERO;
        let mut value = ZERO;
        purchased_qty[0] = 200.0;
        value[0] = 2000.0;
        let r = reconcile(
            &input([10.0; 12], purchased_qty, value),
            Thresholds::default(),
        );
        let gap: Vec<&DiscrepancyFlag> = r
            .flags
            .iter()
            .filter(|f| f.kind == FlagKind::YearEndStockGap)
            .collect();
        assert_eq!(gap.len(), 1, "{:?}", r.flags);
        assert_eq!(gap[0].gap, Some(StockGapKind::Overstock));
        assert_eq!(gap[0].dispensed_qty, 120.0);
        assert_eq!(gap[0].purchased_qty, 200.0);
        assert_eq!(
            r.cumulative_deltas[11], 80.0,
            "the flag's underlying curve point"
        );
    }

    #[test]
    fn year_end_overuse_is_flagged() {
        // Bought 50 in Oct, dispensed 10/mo (120/year) → curve ends at −70
        // (< −25% of 120) → Overuse (stock from before the year / data).
        let mut purchased_qty = ZERO;
        let mut value = ZERO;
        purchased_qty[0] = 50.0;
        value[0] = 500.0;
        let r = reconcile(
            &input([10.0; 12], purchased_qty, value),
            Thresholds::default(),
        );
        let gap: Vec<&DiscrepancyFlag> = r
            .flags
            .iter()
            .filter(|f| f.kind == FlagKind::YearEndStockGap)
            .collect();
        assert_eq!(gap.len(), 1, "{:?}", r.flags);
        assert_eq!(gap[0].gap, Some(StockGapKind::Overuse));
        assert_eq!(r.cumulative_deltas[11], -70.0);
    }

    #[test]
    fn small_year_end_residual_is_not_flagged() {
        // Bought 130, dispensed 120 → curve ends at +10 < 25% of 120 → clean.
        let mut purchased_qty = ZERO;
        let mut value = ZERO;
        purchased_qty[0] = 130.0;
        value[0] = 1300.0;
        let r = reconcile(
            &input([10.0; 12], purchased_qty, value),
            Thresholds::default(),
        );
        assert!(
            !r.flags.iter().any(|f| f.kind == FlagKind::YearEndStockGap),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn all_zero_year_produces_no_flags() {
        let r = reconcile(&input(ZERO, ZERO, ZERO), Thresholds::default());
        assert!(r.flags.is_empty());
        assert_eq!(r.unit_price_year, None);
        assert_eq!(r.coverage_ratio, None);
        assert!(r.cv_dispensed_qty.is_none());
        assert!(r.cv_purchased_value.is_none());
    }

    #[test]
    fn coefficient_of_variation_is_normalized() {
        assert_eq!(coefficient_of_variation(&[]), None);
        assert_eq!(coefficient_of_variation(&[0.0; 12]), None);
        assert_eq!(coefficient_of_variation(&[5.0, 5.0, 5.0]), Some(0.0));
        let cv = coefficient_of_variation(&[10.0, 30.0]).expect("cv");
        assert!((cv - 0.5).abs() < 1e-9, "{cv}");
    }
}
