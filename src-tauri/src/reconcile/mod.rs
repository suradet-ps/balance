//! Reconciliation & discrepancy engine (Phase 2) — pure, deterministic math.
//!
//! This module has **no I/O**: it takes the two monthly series — HOSxP
//! dispensed quantities and the mapped INVS purchase quantities/values,
//! both already in Thai fiscal order (index 0 = ต.ค.) — and produces the
//! numbers and rule-based flags the pharmacy director actually wants.
//!
//! Every rule is a pure function with unit tests and synthetic fixtures;
//! thresholds are parameters with defaults, so Phase 8 can expose them in
//! settings without touching the rules.  A "no data" month (zero dispensed
//! quantity) yields `None` unit prices — never `∞`, never a number that
//! looks comparable when it is not.

use serde::Serialize;

/// One kind of discrepancy flag, serialised as a stable kebab-case string
/// so the frontend can render Thai copy without re-implementing the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlagKind {
    /// Bought all year (value > 0) but never dispensed anything.
    ZeroUseFullPurchase,
    /// Dispensed all year but never purchased (legacy stock / data problem).
    DispensedWithoutPurchase,
    /// One month's unit price exceeds `unit_price_spike_factor` × the
    /// yearly median.
    UnitPriceSpike,
    /// The dispensing peak month is not the purchase peak month.
    SeasonalFlip,
    /// A month with purchases but no dispensing, or vice versa.
    OneSidedMonth,
}

/// Which side of a [`FlagKind::OneSidedMonth`] flag has data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OneSidedKind {
    /// Dispensed (HOSxP) in this month, no purchase (INVS) at all.
    OnlyDispensed,
    /// Purchased (INVS) in this month, nothing dispensed (HOSxP).
    OnlyPurchased,
}

/// A single discrepancy flag, carrying the two numbers that produced it so
/// the pharmacist can verify against the source systems.  `month` is the
/// fiscal-month index (0 = ต.ค., `None` = whole fiscal year).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiscrepancyFlag {
    pub kind: FlagKind,
    pub month: Option<usize>,
    pub one_sided: Option<OneSidedKind>,
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
    /// `unit_price_spike` fires when a month's unit price is above
    /// `factor` × the yearly median unit price.
    pub unit_price_spike_factor: f64,
    /// A "purchase" counts only when the month's INVS value is above this
    /// (avoids flagging ฿0 invoice rows as purchases).
    pub min_purchase_value: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            unit_price_spike_factor: 3.0,
            min_purchase_value: 0.0,
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reconciliation {
    /// Yearly unit price = Σ value ÷ Σ qty (`None` when nothing dispensed).
    pub unit_price_year: Option<f64>,
    /// Monthly unit price (`None` in months with no dispensing — displayed
    /// as "no data", never as ∞).
    pub unit_price_month: Vec<Option<f64>>,
    /// Per-month purchased − dispensed quantity delta (fiscal order).
    pub monthly_deltas: Vec<f64>,
    /// Coefficient of variation of the dispensed quantities (`None` when
    /// the mean is 0 — variance is meaningless on an all-zero series).
    pub cv_dispensed_qty: Option<f64>,
    /// Coefficient of variation of the purchase values.
    pub cv_purchased_value: Option<f64>,
    pub flags: Vec<DiscrepancyFlag>,
}

/// Unit price = INVS value ÷ HOSxP quantity.  `None` when nothing was
/// dispensed — a zero-quantity month renders "no dispensing data".
#[must_use]
pub fn unit_price(value: f64, qty: f64) -> Option<f64> {
    if qty > 0.0 {
        Some(value / qty)
    } else {
        None
    }
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
    if vals.len() % 2 == 0 {
        Some((vals[mid - 1] + vals[mid]) / 2.0)
    } else {
        Some(vals[mid])
    }
}

/// Index of the peak month (first max wins; `None` when the series is all
/// zero — an all-zero series has no meaningful "peak").
#[must_use]
pub fn peak_month(values: &[f64]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, v) in values.iter().copied().enumerate() {
        if v > 0.0 && best.is_none_or(|(_, bv)| v > bv) {
            best = Some((i, v));
        }
    }
    best.map(|(i, _)| i)
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

    // ── Unit prices ────────────────────────────────────────────────────
    let year_qty: f64 = dispensed_qty.iter().sum();
    let year_value: f64 = purchased_value.iter().sum();
    let unit_price_year = unit_price(year_value, year_qty);
    let unit_price_month: Vec<Option<f64>> = (0..12)
        .map(|m| unit_price(purchased_value[m], dispensed_qty[m]))
        .collect();

    // ── Per-month deltas + one-sided months ────────────────────────────
    let mut monthly_deltas = Vec::with_capacity(12);
    let mut flags: Vec<DiscrepancyFlag> = Vec::new();
    for m in 0..12 {
        monthly_deltas.push(purchased_qty[m] - dispensed_qty[m]);
        let dispensed = dispensed_qty[m] > 0.0;
        let purchased = purchased_value[m] > thresholds.min_purchase_value;
        if purchased && !dispensed {
            flags.push(DiscrepancyFlag {
                kind: FlagKind::OneSidedMonth,
                month: Some(m),
                one_sided: Some(OneSidedKind::OnlyPurchased),
                dispensed_qty: dispensed_qty[m],
                purchased_qty: purchased_qty[m],
                purchased_value: purchased_value[m],
            });
        } else if dispensed && !purchased {
            flags.push(DiscrepancyFlag {
                kind: FlagKind::OneSidedMonth,
                month: Some(m),
                one_sided: Some(OneSidedKind::OnlyDispensed),
                dispensed_qty: dispensed_qty[m],
                purchased_qty: purchased_qty[m],
                purchased_value: purchased_value[m],
            });
        }
    }

    // ── Whole-year flags ───────────────────────────────────────────────
    if year_qty <= 0.0 && year_value > 0.0 {
        flags.push(DiscrepancyFlag {
            kind: FlagKind::ZeroUseFullPurchase,
            month: None,
            one_sided: None,
            dispensed_qty: year_qty,
            purchased_qty: purchased_qty.iter().sum(),
            purchased_value: year_value,
        });
    }
    if year_qty > 0.0 && year_value <= 0.0 {
        flags.push(DiscrepancyFlag {
            kind: FlagKind::DispensedWithoutPurchase,
            month: None,
            one_sided: None,
            dispensed_qty: year_qty,
            purchased_qty: purchased_qty.iter().sum(),
            purchased_value: year_value,
        });
    }

    // ── Unit-price spike ───────────────────────────────────────────────
    let median_price = median_of(unit_price_month.iter().flatten().copied());
    if let Some(median) = median_price {
        for m in 0..12 {
            let Some(price) = unit_price_month[m] else { continue };
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
                    one_sided: None,
                    dispensed_qty: dispensed_qty[m],
                    purchased_qty: purchased_qty[m],
                    purchased_value: purchased_value[m],
                });
            }
        }
    }

    // ── Seasonal flip ──────────────────────────────────────────────────
    let peak_dispensed = peak_month(dispensed_qty);
    let peak_purchased = peak_month(purchased_qty);
    if let (Some(pd), Some(pp)) = (peak_dispensed, peak_purchased) {
        if pd != pp {
            flags.push(DiscrepancyFlag {
                kind: FlagKind::SeasonalFlip,
                month: Some(pd),
                one_sided: None,
                dispensed_qty: dispensed_qty[pd],
                purchased_qty: purchased_qty[pd],
                purchased_value: purchased_value[pd],
            });
        }
    }

    flags.sort_by_key(|f| (f.kind as u8, f.month.unwrap_or(usize::MAX)));

    Reconciliation {
        unit_price_year,
        unit_price_month,
        monthly_deltas,
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
        // Steady dispensing (10/mo) and steady purchases (15/mo, ฿10/unit) —
        // the deltas are all +5, no one-sided months, no spikes, peaks align.
        let dispensed = [10.0; 12];
        let purchased_qty = [15.0; 12];
        let purchased_value = [100.0; 12];
        let r = reconcile(&input(dispensed, purchased_qty, purchased_value), Thresholds::default());
        assert!(r.flags.is_empty(), "{:?}", r.flags);
        assert_eq!(r.unit_price_year, Some(10.0));
        assert_eq!(r.monthly_deltas, vec![5.0; 12]);
        assert!(r.cv_dispensed_qty.is_some());
        assert!(r.cv_purchased_value.is_some());
    }

    #[test]
    fn zero_use_full_purchase_is_flagged() {
        let r = reconcile(
            &input(ZERO, [10.0; 12], [100.0; 12]),
            Thresholds::default(),
        );
        assert!(
            r.flags
                .iter()
                .any(|f| f.kind == FlagKind::ZeroUseFullPurchase && f.month.is_none()),
            "{:?}",
            r.flags
        );
        // Every month is also a one-sided "OnlyPurchased" month.
        assert_eq!(
            r.flags
                .iter()
                .filter(|f| f.kind == FlagKind::OneSidedMonth)
                .count(),
            12
        );
        assert_eq!(r.unit_price_year, None, "nothing dispensed → no yearly price");
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
        assert!(
            r.flags
                .iter()
                .any(|f| f.kind == FlagKind::OneSidedMonth
                    && f.one_sided == Some(OneSidedKind::OnlyDispensed)
                    && f.month == Some(0)),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn unit_price_spike_fires_above_the_threshold() {
        // 10 units × ฿10/month except month 5: 1 unit for ฿200.
        let mut dispensed = [10.0; 12];
        let mut value = [100.0; 12];
        dispensed[5] = 1.0;
        value[5] = 200.0;
        let r = reconcile(
            &input(dispensed, [10.0; 12], value),
            Thresholds::default(),
        );
        let spike: Vec<&DiscrepancyFlag> = r
            .flags
            .iter()
            .filter(|f| f.kind == FlagKind::UnitPriceSpike)
            .collect();
        assert_eq!(spike.len(), 1, "{:?}", r.flags);
        assert_eq!(spike[0].month, Some(5));
        assert_eq!(spike[0].dispensed_qty, 1.0);
        assert_eq!(spike[0].purchased_value, 200.0);
    }

    #[test]
    fn normal_price_noise_stays_below_the_threshold() {
        // Prices between 9 and 11 (median 10): 11 < 3×10 → no spike.
        let value: [f64; 12] = [
            90.0, 95.0, 100.0, 105.0, 110.0, 100.0, 90.0, 95.0, 105.0, 110.0, 95.0, 105.0,
        ];
        let r = reconcile(
            &input([10.0; 12], [10.0; 12], value),
            Thresholds::default(),
        );
        assert!(
            !r.flags.iter().any(|f| f.kind == FlagKind::UnitPriceSpike),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn seasonal_flip_detects_offset_peaks() {
        // Dispensing peaks in fiscal month 0 (Oct), purchases in month 6 (Apr).
        let mut dispensed = [1.0; 12];
        let mut purchased_qty = [1.0; 12];
        dispensed[0] = 30.0;
        purchased_qty[6] = 30.0;
        let r = reconcile(
            &input(dispensed, purchased_qty, [10.0; 12]),
            Thresholds::default(),
        );
        assert!(
            r.flags
                .iter()
                .any(|f| f.kind == FlagKind::SeasonalFlip && f.month == Some(0)),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn aligned_peaks_do_not_flag_seasonal_flip() {
        let mut dispensed = [1.0; 12];
        let mut purchased_qty = [1.0; 12];
        dispensed[6] = 30.0;
        purchased_qty[6] = 30.0;
        let r = reconcile(
            &input(dispensed, purchased_qty, [10.0; 12]),
            Thresholds::default(),
        );
        assert!(
            !r.flags.iter().any(|f| f.kind == FlagKind::SeasonalFlip),
            "{:?}",
            r.flags
        );
    }

    #[test]
    fn all_zero_year_produces_no_flags() {
        let r = reconcile(&input(ZERO, ZERO, ZERO), Thresholds::default());
        assert!(r.flags.is_empty());
        assert_eq!(r.unit_price_year, None);
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

    #[test]
    fn peak_month_ignores_all_zero_series() {
        assert_eq!(peak_month(&ZERO), None);
        let mut v = [0.0; 12];
        v[4] = 5.0;
        v[7] = 5.0;
        assert_eq!(peak_month(&v), Some(4), "first max wins");
    }
}
