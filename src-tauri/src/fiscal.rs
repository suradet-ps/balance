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
}
