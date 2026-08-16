//! Thai fiscal-year helpers shared by both backends.
//!
//! The Thai fiscal year runs 1 Oct → 30 Sep (FY N starts 1 Oct of N−1).
//! All calendar↔fiscal mapping lives here so both the HOSxP and the INVS
//! sides (and the reconciliation engine) agree on the same axes — pure
//! functions, unit-tested in CI.

/// Map a calendar month (1 = January … 12 = December) to a fiscal-month
/// index: 0 = fiscal month 1 = ต.ค. … 11 = fiscal month 12 = ก.ย.
///
/// A caller that trusts `1..=12` for its input may also use this to convert
/// a `MONTH()` value from a query.
#[must_use]
pub fn cal_to_fiscal_idx(cal_month: i32) -> usize {
    if cal_month >= 10 {
        (cal_month - 10) as usize
    } else {
        (cal_month + 2) as usize
    }
}

/// Reorder a 12-element, calendar-ordered series (index 0 = January) into
/// fiscal order (index 0 = ต.ค.).  The input length must be 12; the output
/// is a fresh `Vec` so callers never alias their calendar-ordered data.
#[must_use]
pub fn reorder_calendar_to_fiscal(values: Vec<f64>) -> Vec<f64> {
    debug_assert_eq!(values.len(), 12, "monthly series must have 12 entries");
    let mut out = vec![0.0; values.len()];
    for (i, value) in values.iter().enumerate() {
        out[cal_to_fiscal_idx((i + 1) as i32)] = *value;
    }
    out
}

/// The fiscal-year window as `YYYYMMDD` integers (INVS `RECEIVE_DATE` shape):
/// FY `fy` = 1 Oct (`fy`−1) … 30 Sep `fy`.
///
/// This is the single definition of the boundary: a record on 30 Sep `fy`
/// belongs to FY `fy`; a record on 1 Oct `fy` belongs to FY `fy`+1.
#[must_use]
pub fn fiscal_year_range(fy: i32) -> (i32, i32) {
    ((fy - 1) * 10_000 + 1001, fy * 10_000 + 930)
}

/// The same window as `'YYYY-MM-DD'` strings with an **exclusive** upper
/// bound (`[start, end)`), for MySQL `DATE` columns: `end` = 1 Oct of the
/// label year, so 30 Sep is included and 1 Oct belongs to the next FY.
#[must_use]
pub fn fiscal_mysql_window(fy: i32) -> (String, String) {
    (format!("{:04}-10-01", fy - 1), format!("{:04}-10-01", fy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_month_maps_to_fiscal_index() {
        // Oct..Dec → fiscal months 1..3 (indices 0..2)
        assert_eq!(cal_to_fiscal_idx(10), 0);
        assert_eq!(cal_to_fiscal_idx(11), 1);
        assert_eq!(cal_to_fiscal_idx(12), 2);
        // Jan..Sep → fiscal months 4..12 (indices 3..11)
        assert_eq!(cal_to_fiscal_idx(1), 3);
        assert_eq!(cal_to_fiscal_idx(2), 4);
        assert_eq!(cal_to_fiscal_idx(9), 11);
    }

    #[test]
    fn reorder_puts_january_into_fiscal_slot_4() {
        let calendar = vec![
            10.0, // Jan
            20.0, // Feb
            30.0, // Mar
            40.0, // Apr
            50.0, // May
            60.0, // Jun
            70.0, // Jul
            80.0, // Aug
            90.0, // Sep
            1.0,  // Oct
            2.0,  // Nov
            3.0,  // Dec
        ];
        let fiscal = reorder_calendar_to_fiscal(calendar);
        assert_eq!(fiscal.len(), 12);
        assert_eq!(fiscal[0], 1.0, "Oct leads the fiscal year");
        assert_eq!(fiscal[2], 3.0, "Dec stays in the same fiscal year");
        assert_eq!(fiscal[3], 10.0, "Jan lands in fiscal month 4");
        assert_eq!(fiscal[11], 90.0, "Sep closes the fiscal year");
    }

    #[test]
    fn reorder_is_total_preserving() {
        let calendar: Vec<f64> = (1..=12).map(f64::from).collect();
        let fiscal = reorder_calendar_to_fiscal(calendar);
        let sum: f64 = fiscal.iter().sum();
        assert_eq!(sum, 78.0);
    }

    #[test]
    fn fiscal_year_range_covers_oct_to_sep() {
        // FY 2026 = 1 Oct 2025 … 30 Sep 2026 (standard Thai fiscal year).
        let (start, end) = fiscal_year_range(2026);
        assert_eq!(start, 20251001);
        assert_eq!(end, 20260930);
    }

    #[test]
    fn fiscal_boundary_months_belong_to_the_right_year() {
        // 30 Sep 2025 → FY 2025; 1 Oct 2025 → FY 2026.
        let (start25, end25) = fiscal_year_range(2025);
        assert_eq!((start25, end25), (20241001, 20250930));
        assert!(
            20250930 <= end25 && 20250930 >= start25,
            "30 Sep is in FY2025"
        );
        let (start26, _) = fiscal_year_range(2026);
        assert!(20251001 >= start26, "1 Oct starts FY2026");
        assert!(!(20250930 >= start26), "…and is NOT in FY2026");
    }

    #[test]
    fn mysql_window_is_inclusive_start_exclusive_end() {
        let (start, end) = fiscal_mysql_window(2026);
        assert_eq!(start, "2025-10-01");
        assert_eq!(
            end, "2026-10-01",
            "exclusive: 30 Sep 2026 is the last included day"
        );
    }
}
