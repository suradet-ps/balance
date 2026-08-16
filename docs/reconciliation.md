# Reconciliation & Discrepancy Engine (Phase 2)

How Balance compares what the hospital **dispensed** (HOSxP `opitemrece`
quantities) against what it **purchased** (INVS `MS_IVO_C` quantities and
values) for one mapped drug and one fiscal year, and which rules flag a
discrepancy.  All math lives in `src-tauri/src/reconcile/mod.rs` — pure,
deterministic, unit-tested; the command layer (`reconcile/commands.rs`) is
only the adapter that fetches data and calls the engine.

## Why the comparison is year-first, not month-first

A hospital buys a drug once or twice a year; that stock then covers months
of dispensing.  A month-by-month "purchase vs dispensing" comparison is
therefore structurally noisy — a purchase month without dispensing (or vice
versa) is **normal stock behavior**, not an anomaly.  Flagging it would
produce a false alarm on nearly every bulk-bought drug.

So the engine answers the questions that are actually answerable:

- **Year level** — the headline: total dispensed vs total purchased, the
  yearly unit price, and whether the *stock curve* (cumulative
  purchased − dispensed) ends the year with a material imbalance.
- **Month level** — reported as *data*, never flagged: dispensing per
  month, purchase events, the cumulative stock curve, and the purchase
  price on purchase months.
- **Unit price** — per-month prices are *purchase* prices (value ÷ qty on
  purchase months only).  Comparing purchase value against the same
  month's dispensing is meaningless when the stock was bought earlier.

## Inputs

Both monthly series are 12-element vectors in **Thai fiscal order**
(index 0 = ต.ค. … 11 = ก.ย.) — the same axis both charts plot (see
`src-tauri/src/fiscal.rs`):

| Series | Source | Meaning |
|--------|--------|---------|
| `dispensed_qty` | HOSxP `opitemrece.qty`, grouped by month | จ่าย |
| `purchased_qty` | INVS `MS_IVO_C.QTY_ORDER` | ซื้อ (จำนวน) |
| `purchased_value` | INVS `MS_IVO_C.VALUE` | ซื้อ (บาท) |

## Figures

### Year level

- **`unit_price_year`** = Σ purchase value ÷ Σ dispensed qty — the cost per
  dispensed unit.  `None` when nothing was dispensed (rendered as
  "no data", never `∞`).
- **`coverage_ratio`** = Σ dispensed ÷ Σ purchased qty — how much of what
  was bought actually left the pharmacy this year.  ~100% = balanced;
  < 100% = bought more than used (stock carried forward); > 100% = used
  more than bought (stock from previous years / data problem).
- **`cv_dispensed_qty` / `cv_purchased_value`** — month-to-month variation
  (coefficient of variation; `None` on zero-mean series).
- **The stock curve's end point** (below) drives the `year-end-stock-gap`
  flag.

### Month level (data, fiscal order)

- **`monthly_deltas`** — purchased − dispensed per month.
- **`cumulative_deltas`** — the running sum of the deltas: the implied
  stock-on-hand curve.  A bulk purchase in ต.ค. shows stock piling up,
  then gradually running down to ~0 by ก.ย. — normal.
- **`purchase_price_month`** — value ÷ qty on purchase months only
  (`None` where nothing was bought).

## Discrepancy flags

| Flag | Rule | month |
|------|------|-------|
| `zero-use-full-purchase` | Σ dispensed = 0 and Σ value > 0 | whole year |
| `dispensed-without-purchase` | Σ dispensed > 0 and Σ value = 0 | whole year |
| `unit-price-spike` | purchase month's price > `unit_price_spike_factor` (default 3) × median monthly purchase price; median 0 with a positive price counts as a spike | the purchase month |
| `year-end-stock-gap` | stock curve at year end deviates from 0 by more than `year_end_stock_gap_ratio` (default 0.25) × Σ dispensed | whole year (`gap`: `overstock` / `overuse`) |

Every flag carries **the two numbers that produced it** (dispensed qty,
purchased qty, purchased value, month) so the pharmacist can verify against
the source databases — traceability is a requirement, not a nicety.

Thresholds are parameters with defaults; Phase 8 exposes them in settings
without touching the rules.

## Worked examples

1. **Steady year** — 10 units dispensed and 10 purchased every month at
   ฿10/unit: no flags; yearly unit price ฿10; coverage 100%; stock curve
   flat at 0.
2. **Bulk purchase (the common real-world shape)** — 120 units bought once
   in ต.ค., 10 units dispensed every month: coverage 100%, the curve rises
   to +110 then returns to 0 by ก.ย., **no flags** — this is healthy stock
   behavior, not a discrepancy.
3. **Overstock at year end** — 200 units bought in ต.ค., 10/mo dispensed
   (120/year): the curve ends at +80 (> 25% of 120) → `year-end-stock-gap`
   with `gap: overstock` ("เหลือสต็อกปลายปี 80 หน่วย").
4. **Over-use** — 50 units bought in ต.ค., 10/mo dispensed (120/year): the
   curve ends at −70 → `year-end-stock-gap` with `gap: overuse`
   ("จ่ายเกินซื้อ 70 หน่วย — ใช้สต็อกจากปีก่อนหรือข้อมูลผิดปกติ").
5. **Unit-price spike** — 10 units at ฿10/month except month ก.พ.: 10
   units for ฿400 (price ฿40 vs median ฿10, > 3×) → `unit-price-spike`.
   A single purchase all year can never spike (the median *is* that
   price) — spike detection needs ≥ 2 purchase events.

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
`Reconciliation` (series echoed + computed figures + flags), so the
frontend's discrepancy view needs no further round-trips.

## Testing

`reconcile::tests` covers: division-by-zero guard, a clean year producing
no flags, the bulk-purchase shape being *normal* (no flags), both
whole-year flags, spike firing/not firing around the threshold + the
single-purchase-no-spike case, overstock/overuse and the small-residual
boundary, all-zero years, and coefficient of variation — all against
synthetic 12-month fixtures, run in CI (`cargo test -p balance`).
