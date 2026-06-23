/**
 * Fiscal year utilities for Balance dashboard.
 *
 * Thai fiscal year (ปีงบประมาณ):
 *   FY N = 1 Oct (N-1)  to  30 Sep N
 *   e.g. FY 68 = 1 Oct 67 to 30 Sep 68  (CE: 2024-10-01 to 2025-09-30)
 */

/** Return the current Thai fiscal year (CE). */
export function currentFiscalYear(): number {
  const now = new Date()
  const month = now.getMonth() + 1
  const year = now.getFullYear()
  return month >= 10 ? year + 1 : year
}

/** Convert a calendar month (1–12) to 0-based fiscal index (Oct=0, …, Sep=11). */
export function calMonthToFiscalIdx(calMonth: number): number {
  return calMonth >= 10 ? calMonth - 10 : calMonth + 2
}

/** Convert a 0-based fiscal index back to calendar month (1–12). */
export function fiscalIdxToCalMonth(fIdx: number): number {
  return fIdx < 3 ? fIdx + 10 : fIdx - 2
}

// ─── Calendar month arrays (index 0 = January) ───────────────────────────────

export const THAI_MONTHS_SHORT = [
  'ม.ค.', 'ก.พ.', 'มี.ค.', 'เม.ย.', 'พ.ค.', 'มิ.ย.',
  'ก.ค.', 'ส.ค.', 'ก.ย.', 'ต.ค.', 'พ.ย.', 'ธ.ค.',
]

export const THAI_MONTHS_FULL = [
  'มกราคม', 'กุมภาพันธ์', 'มีนาคม', 'เมษายน',
  'พฤษภาคม', 'มิถุนายน', 'กรกฎาคม', 'สิงหาคม',
  'กันยายน', 'ตุลาคม', 'พฤศจิกายน', 'ธันวาคม',
]

// ─── Fiscal month arrays (index 0 = fiscal month 1 = ต.ค.) ─────────────────

export const FISCAL_MONTHS_SHORT = [
  'ต.ค.', 'พ.ย.', 'ธ.ค.',
  'ม.ค.', 'ก.พ.', 'มี.ค.', 'เม.ย.',
  'พ.ค.', 'มิ.ย.', 'ก.ค.', 'ส.ค.', 'ก.ย.',
]

export const FISCAL_MONTHS_FULL = [
  'ตุลาคม', 'พฤศจิกายน', 'ธันวาคม',
  'มกราคม', 'กุมภาพันธ์', 'มีนาคม', 'เมษายน',
  'พฤษภาคม', 'มิถุนายน', 'กรกฎาคม', 'สิงหาคม', 'กันยายน',
]

/** Format a number as Thai Baht with thousands separator. */
export function formatBaht(value: number, decimals = 0): string {
  return '฿' + value.toLocaleString('th-TH', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })
}

/** Format a number with Thai thousands separator. */
export function formatQty(qty: number): string {
  return qty.toLocaleString('th-TH', { maximumFractionDigits: 2 })
}

/** Convert CE year to Buddhist Era. */
export function ceToBe(year: number): number {
  return year + 543
}
