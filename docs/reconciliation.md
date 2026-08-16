# Reconciliation & Discrepancy Engine (Phase 2)

How Balance compares what the hospital **dispensed** (HOSxP `opitemrece`
quantities) against what it **purchased** (INVS `MS_IVO_C` quantities and
values) for one mapped drug and one fiscal year, and which rules flag a
discrepancy.  All math lives in `src-tauri/src/reconcile/mod.rs` — pure,
deterministic, unit-tested; the command layer (`reconcile/commands.rs`) is
only the adapter that fetches data and calls the engine.

## Inputs

Both monthly series are 12-element vectors in **Thai fiscal order**
(index 0 = ต.ค. … 11 = ก.ย.) — the same axis both charts plot (see
`src-tauri/src/fiscal.rs`):

| Series | Source | Meaning |
|--------|--------|---------|
| `dispensed_qty` | HOSxP `opitemrece.qty`, grouped by month | จ่าย |
| `purchased_qty` | INVS `MS_IVO_C.QTY_ORDER` | ซื้อ (จำนวน) |
| `purchased_value` | INVS `MS_IVO_C.VALUE` | ซื้อ (บาท) |

## Rules

### Unit price

```
unit_price = INVS value ÷ HOSxP quantity
```

- Computed per month **and** per year.
- **Division by zero is guarded**: a month with zero dispensed quantity
  yields `None` and is rendered as "ไม่มีข้อมูลการจ่าย" — never `∞`, never
  a number that looks comparable when it is not.

### Monthly deltas

```
delta_month = purchased_qty − dispensed_qty
```

A **one-sided month** (purchase with no dispensing, or dispensing with no
purchase) is flagged individually — it is **not averaged away**.  A month
counts as "purchased" only when its INVS value is above
`Thresholds::min_purchase_value` (default 0), so ฿0 invoice rows do not
create phantom one-sided months.

### Discrepancy flags

| Flag | Rule | month |
|------|------|-------|
| `zero-use-full-purchase` | Σ dispensed = 0 and Σ value > 0 | whole year |
| `dispensed-without-purchase` | Σ dispensed > 0 and Σ value = 0 | whole year |
| `unit-price-spike` | month price > `unit_price_spike_factor` (default 3) × yearly median; median 0 with a positive price counts as a spike | the month |
| `seasonal-flip` | peak dispensed month ≠ peak purchased month (peaks = first maximum of the positive series) | the dispensed peak month |
| `one-sided-month` | purchase without dispensing (or vice versa) | the month |

Every flag carries **the two numbers that produced it** (dispensed qty,
purchased qty, purchased value, month) so the pharmacist can verify against
the source databases — traceability is a requirement, not a nicety.

Thresholds are parameters with defaults; Phase 8 exposes them in settings
without touching the rules.

## Worked examples

1. **Steady year** — 10 units dispensed and 15 purchased every month at
   ฿10/unit: no flags; yearly unit price ฿10; every delta +5.
2. **Zero use, full purchase** — nothing dispensed, ฿100/month bought: one
   `zero-use-full-purchase` flag (whole year) + twelve `one-sided-month`
   (`only-purchased`) flags.
3. **Unit-price spike** — 10 units at ฿10/month, except month 5: 1 unit for
   ฿200: median ฿10, 200 > 3×10 → one `unit-price-spike` flag on month 5
   (fiscal index 4 → ก.พ.).

## Command

```
reconcile_drug(store, invs, year: i32, icode: String) -> ReconcileReport
```

1. Resolves `icode → working_code` in the **local store** (never in a
   source DB); errors with Thai copy when the drug is unmapped.
2. Fetches the HOSxP dispensing series and the INVS purchase series (both
   in fiscal order, reusing `hosxp::fetch_monthly_qty` /
   `invs::fetch_monthly_value`).
3. Runs `reconcile()` with the default thresholds.

`ReconcileReport` returns both codes, both drug names and the full
`Reconciliation` (series echoed + computed numbers + flags), so the
frontend's discrepancy view needs no further round-trips.

## Testing

`reconcile::tests` covers: division-by-zero guard, a clean year producing
no flags, both whole-year flags, spike firing/not firing around the
threshold, seasonal flip with offset/aligned peaks, all-zero years,
coefficient of variation, and first-max peak selection — all against
synthetic 12-month fixtures, run in CI (`cargo test -p balance`).
