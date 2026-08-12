# Balance Roadmap

This roadmap describes what Balance is — honestly, from reading its own code —
and where it should end up. It follows the design system in
[DESIGN.md](DESIGN.md) and the conventions in the repo. Nothing here
is called "done" on intent alone; the CI pipeline
(`.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy -D warnings`,
`trunk build`, design-token enforcement, `cargo-deny`) is the gate for every
phase's acceptance.

> **What Balance is.** A *reconciliation dashboard* for Thai hospital drug
> inventory: it sits next to two systems of record — HOSxP (the hospital
> information system, MySQL) and INVS (the Ministry of Public Health
> inventory system, SQL Server) — and compares what a hospital *dispensed*
> (HOSxP quantities) against what it *purchased* (INVS values) so the
> pharmacy can spot mismatches, over-stocking, missing procurement, and
> unexplained consumption. One window, two panels, one fiscal year at a
> time, all in Thai.
>
> **What Balance is not.** Not an EHR, not an ERP, not an inventory
> management system, not a billing tool, not a patient-facing app, and
> not a replacement for the pharmacist. HOSxP and INVS remain the systems
> of record — Balance connects to them *read-only* and never writes a row
> into either. Features that break that line are listed under "Out of
> Scope" on purpose.

---

## Design Principles

Every feature in Balance should reinforce one or more of these principles.
When a new feature is proposed, ask: "which principle does it serve, and
does it violate any other?"

1. **Source systems stay authoritative.** Balance reads from HOSxP and
   INVS; it never writes to them. If a number in Balance disagrees with
   the source database, Balance is wrong, and the bug must be fixed — the
   numbers must be verifiable back to the source query.
2. **Reconciliation honesty over visual comfort.** When two values cannot
   honestly be compared (different units, an unmapped drug, missing data),
   the UI must say so — not render a number that looks comparable. "No
   data" is a legitimate, visible state.
3. **Deterministic, testable math.** Fiscal-year boundaries, month
   alignment, formatting, and discrepancy calculations are pure functions
   with unit tests. Same inputs always produce the same outputs.
4. **Offline-first.** Balance is a desktop tool in a hospital. It must
   boot and run with no network at all — and today that is violated (see
   Gap 4). Nothing in the critical path may depend on a CDN or the
   internet.
5. **Fast on real hospital data.** A regional hospital's `opitemrece` has
   millions of rows and `drugitems` has tens of thousands. Every query
   must be written, indexed, and measured against that scale — not
   against a demo database.
6. **Thai-first UX.** The user is a Thai hospital pharmacist. All UI copy
   is Thai, numbers are Thai-formatted, the fiscal year is the Thai fiscal
   year — but strings are structured so they can be managed, not scattered
   across views.

---

## Engineering Goals

- Backend commands stay thin adapters: they parse arguments, call a
  domain module, serialize the result. Business logic (year math,
  matching, discrepancy math) lives in pure, testable modules.
- The HOSxP and INVS connections are guaranteed read-only by construction
  (read-only DB accounts + code that only ever issues `SELECT`).
- All app-owned data (drug mappings, watchlists, settings schema) lives in
  a local, versioned store with migrations — never in source databases.
- Every calculation is unit-tested and deterministically reproducible.
- Supply chain is auditable — `cargo-deny`, pinned Actions, `clippy -D
  warnings`.

---

## Current State (verified against the repo, not assumed)

- **Stack**: Tauri 2 (Rust) shell + Leptos 0.8 CSR frontend compiled to
  `wasm32-unknown-unknown` and bundled by Trunk (no Node toolchain at all;
  the Tauri API module is loaded at runtime from `esm.sh`). Version
  `0.1.0`. Two workspaces: `balance` (src-tauri, host target) and
  `balance-frontend` (src/, wasm target).
- **Backend** (`src-tauri`): 13 Tauri commands — 2 settings, 5 HOSxP, 6
  INVS. HOSxP: `sqlx` MySQL pool (12 max / 3 min connections, global
  `OnceLock<RwLock<Option<MySqlPool>>>`). INVS: a single `tiberius`
  client in `Arc<Mutex<Option<...>>>` managed state — one connection,
  held for the app's lifetime, serialised behind a mutex.
- **Data sources (read-only)**:
  - HOSxP MySQL: `opitemrece` (dispensed quantities), `drugitems`
    (drug catalog). Calendar months.
  - INVS SQL Server: `MS_IVO` + `MS_IVO_C` (purchase invoices/lines),
    `DRUG_GN` (drug names). Thai fiscal months, `RECEIVE_DATE` stored as
    `YYYYMMDD` integers. TLS is disabled (`EncryptionLevel::NotSupported`
    + `trust_cert`).
- **Frontend** (`src/src`, ~3.8K lines Rust): two contexts
  (`DashboardContext`, `DbConfigContext`), six components (header, two
  search panels, two canvas charts, settings drawer, KPI bar). Charts are
  a hand-rolled `<canvas>` renderer (12 bars + 3-month moving average +
  HTML tooltip) replacing ECharts. Thai fiscal-year helpers and number
  formatting live in `models.rs` with `wasm_bindgen_test` unit tests.
- **Settings**: connection configs serialised to `settings.json`
  (app-data dir), AES-256-GCM encrypted with a master key kept in the OS
  keychain (`encryptman-keyring`). Boot auto-connect from saved settings.
- **CI** (4 jobs): Rust format + clippy, Trunk build, design-token
  enforcement (no raw hex outside `theme.css`), `cargo-deny`. **No test
  job runs** — the `wasm_bindgen_test`s are never executed in CI, and the
  backend has zero tests.
- **Known hard-coded context**: the header brand-sub title is
  `โรงพยาบาลสระโบสถ์` (a specific hospital) — a placeholder that needs
  to become configurable.

### Gaps found while reading the repo (these shape the phases below)

1. **There is no drug mapping between the two systems.** HOSxP
   identifies drugs by `icode`, INVS by `working_code` — two different
   catalogs with no link. Today the two panels are independent: the
   pharmacist must search both sides by hand and keep the correspondence
   in their head. Every reconciliation feature in the rest of this
   roadmap hangs off a mapping table. **This is the single biggest gap.**
   (Phase 1.)

2. **The two panels are not even on the same calendar.** The HOSxP chart
   plots calendar months (ม.ค. = January); the INVS chart plots fiscal
   months (ต.ค. = October) in the *same 12 columns*. A "side-by-side
   comparison" of month N on the left and month N on the right is
   actually comparing different months, three months out of phase. (Phase 2.)

3. **Nothing computes discrepancies.** The README's promise is "identify
   discrepancies and ensure accurate reporting", but no command computes a
   unit price (INVS value ÷ HOSxP quantity), a month-over-month variance,
   a quantity-vs-value ratio, or flags anything. The comparison is left
   entirely to eyeballing two charts. (Phase 2.)

4. **The app cannot boot offline.** `index.html` imports
   `https://esm.sh/@tauri-apps/api@2` at runtime and everything else
   depends on `window.__TAURI__`. A hospital PC without internet — or
   behind a proxy that blocks `esm.sh` — gets a dead app. The CSP was
   already widened to allow it. This violates the offline-first principle
   at the most basic level. (Phase 3.)

5. **The INVS connection is a single serialised client with no TLS.**
   Every INVS command locks one `Mutex<Client>` for the whole query; two
   panels refreshing concurrently block each other. `EncryptionLevel::NotSupported`
   + `trust_cert` means passwords and data travel unencrypted unless the
   hospital network is trusted. The `top drugs` command also runs one
   extra query per drug (N+1). (Phase 3/4.)

6. **Two backend commands are dead code.** `hosxp_get_top_drugs` and
   `invs_get_top_drugs_by_value` are registered in `lib.rs` but have no
   frontend wrapper and no UI. They were presumably part of a dropped
   "top drugs" view. Keep them honest: build the view (Phase 7) or remove
   them (do it in Phase 4 while touching the queries anyway).

7. **Search is a full scan with a wide-open LIKE.** Both sides run
   `LIKE '%query%'` over the entire catalog with no minimum length, no
   ranking (prefix matches should rank first), no index, and no
   pagination — HOSxP `LIMIT 50`, INVS `TOP 30`. On a hospital catalog
   this is slow and noisy. (Phase 4.)

8. **No local persistence layer for app data.** The only thing stored is
   the settings file. There is no local database, no migration mechanism,
   and nowhere to keep drug mappings, watchlists, or derived data.
   Phase 1 cannot land without one. (Phase 1.)

9. **Tests exist but never run.** `models.rs` has `wasm_bindgen_test`
   unit tests; the backend has none. CI runs no test job at all. A
   regression in fiscal-year math, formatting, or the chart renderer
   would ship silently. (Phase 6.)

10. **No export, no reporting, no alerts.** Nothing can be exported to
    Excel/CSV/PDF (a stated roadmap item), there is no multi-year
    comparison, and the KPI bar is INVS-heavy (INVS totals + drug count +
    a HOSxP connection status) with no HOSxP quantity totals. (Phase 5/7/8.)

11. **No connection health monitoring.** The MySQL pool never
    `test_before_acquire`s; if the database restarts, queries fail with
    raw errors until the user re-opens settings. No auto-reconnect, no
    "connection lost" UX. (Phase 3.)

12. **Hard-coded hospital name and scattered Thai strings.** The header
    shows a specific hospital, and Thai copy is inline in every view with
    no string structure. Fine for one site; fragile for a product.
    (Phase 9.)

---

## Phase 1: Drug Mapping Engine (the foundation)

Nothing in this roadmap matters until a drug in HOSxP can be linked to the
same drug in INVS. This phase builds the local store and the matching
workflow. *This phase has no acceptance shortcuts: mapping is the product.*

### Local app store

- [x] **Local SQLite database in the backend** (`rusqlite`, bundled), stored
  under the app-data dir next to `settings.json` as `balance.db`, opened
  read-write *locally only*. Schema and patterns in `docs/database.md`.
- [x] **Versioned migrations.** A `migrations/` directory and a `migrate()`
  call at startup (before the UI mounts). Schema version recorded; CI runs
  the migrations against a fresh DB (`cargo test -p balance`).
- [x] **`drug_mappings` table**: `{ id, icode (HOSxP), working_code (INVS),
  drug_name_hosxp, drug_name_invs, match_method (auto|manual|approved),
  match_score REAL, created_at, updated_at }` with unique
  `(icode, working_code)` and indexes on both codes. A second migration
  adds `mapping_exclusions` for the no-INVS marker.

### Matching workflow

- [x] **Side-by-side comparison view.** The mapping drawer (แมปยา in the
  header) lists HOSxP drugs with their mapping state; per row the
  pharmacist opens scored INVS candidates (code, name, similarity %) and
  gets match / skip actions. Keyboard accessible (Enter-driven search,
  focusable row actions), batch-confirmable.
- [x] **Auto-suggest candidates.** A pure-Rust normalizer (lowercase, strip
  parens/dose-strength suffixes, unify Thai `รร/รา/รึ` spellings) +
  similarity scoring (normalized equality → token overlap → Levenshtein),
  unit-tested against a fixture of real drug names
  (`mapping/normalizer.rs`). Heuristics documented in `docs/mapping.md`.
- [x] **Manual match + unmatch.** Pharmacist can force a link (manual INVS
  search), break a wrong link, and mark a HOSxP drug as "no INVS
  equivalent" (e.g. no longer procured) — with the reason recorded in
  `mapping_exclusions`; mappings and exclusions are mutually exclusive.
- [x] **Bulk import.** Paste an `icode ↔ working_code` CSV with a dry-run
  preview: "N will be added, M will conflict — review before applying";
  conflicts are never overwritten silently.
- [x] **Match status on both panels.** Once mapped, the HOSxP panel shows
  "แมปแล้ว ↔ INVS: <working_code>" and vice versa (plus ยังไม่แมป /
  ไม่มีใน INVS states) — refreshed on every selection.

**Acceptance:** a fresh install can migrate the local DB (unit-tested in
CI); the pharmacist can map 100+ drugs in one session via auto-suggest +
bulk CSV; mappings survive app restart (SQLite persistence); unmapped drugs
are visible as unmapped, never silently treated as equivalent.

**Status:** DONE (Phase 1)

---

## Phase 2: Reconciliation & Discrepancy Engine (the core value)

With mappings in place, Balance can finally do what it claims: compare.

### Aligned axes first

- [ ] **Unify both charts on the fiscal calendar.** The HOSxP side
  switches from calendar-month to fiscal-month alignment (a pure function
  `calendar_to_fiscal_index(month) -> usize` — already exists on the INVS
  side as `cal_to_fiscal_idx`; extract and reuse). The month label row
  becomes identical on both panels: ต.ค. … ก.ย. for the same fiscal year.
  This is a small change with a big honesty payoff — a precondition for
  every comparison below.
- [ ] **Test the alignment.** Unit tests for fiscal-year boundary months
  (September → October flips, January stays in-year, 12-entry arrays).

### Discrepancy math (pure Rust module)

- [ ] **Unit price.** For a mapped drug: `unit_price = INVS value ÷ HOSxP
  quantity` (per month and per year). Guard against division by zero; a
  zero-quantity month renders "no dispensing data" instead of ∞.
- [ ] **Month-over-month variance** on both quantity and value, and a
  per-month `purchased minus dispensed` delta. Months with purchases but
  no dispensing (or vice versa) are flagged, not averaged away.
- [ ] **Discrepancy flags** (rule-based, deterministic, unit-tested):
  - *Zero use, full purchase* — purchased but nothing dispensed all year.
  - *Dispensed without purchase* — dispensed but never purchased (legacy
    stock? data problem?).
  - *Unit-price spike* — a month's unit price > N× the yearly median.
  - *Seasonal flip* — dispensing peaks in a month with no purchase peak.
  - Thresholds are configurable in settings (Phase 8 adds the alert UI).
- [ ] **Discrepancy view.** A per-drug detail strip under (or beside) the
  charts listing the flags with the underlying numbers and the exact
  months, so the pharmacist can verify against the source systems.
  Every flag must be traceable to the two numbers that produced it.

**Acceptance:** both charts plot the same 12 fiscal months; a mapped drug
shows unit price and per-month deltas; all flag rules are pure functions
with unit tests and fixtures; a zero-data month is displayed as "no data",
never as a comparable number.

**Status:** PENDING

---

## Phase 3: Reliability & Offline-First

Balance must boot and work on a hospital PC with no internet. Today it
cannot (Gap 4).

- [ ] **Vendor the Tauri API.** Replace the runtime `esm.sh` import with a
  checked-in copy of the `@tauri-apps/api` module (or the few exports the
  app actually needs — `invoke` and friends) served from `dist/` as a
  static asset. The CSP can then drop `https://esm.sh` again.
- [ ] **Prove it offline.** CI smoke: build the bundle and check the HTML
  references only local assets; manual acceptance: launch with
  networking disabled and confirm the dashboard still reaches "ยังไม่ได้
  เชื่อมต่อฐานข้อมูล" state and the settings drawer works.
- [ ] **INVS connection health.** A `invs_ping` command (cheap round-trip
  — e.g. `SELECT 1`). The frontend polls on an interval; on failure the
  header badge flips to disconnected and a banner offers "เชื่อมต่อใหม่".
- [ ] **MySQL pool health.** Enable `test_before_acquire` on the pool and
  add `hosxp_ping`; same polling UX. Auto-reconnect on next refresh when
  the pool was lost (the pool is rebuilt on connect).
- [ ] **Optional INVS TLS.** Make encryption level a settings choice
  (`off` default for legacy servers / `required` with a warning in the
  drawer when off). Never downgrade silently; show a small "ไม่มีการเข้ารหัส"
  hint next to the MSSQL badge when TLS is off.
- [ ] **Graceful DB-down states.** Per-side "connection lost" overlay on
  the chart area instead of a global error banner and empty charts.
  Keep the last-loaded data visible with a "ข้อมูลล้าสมัย" watermark.

**Acceptance:** the app runs fully offline once installed; connection loss
is detected within ~15 s and recoverable without restarting; INVS
encryption level is user-visible and configurable; no error path shows a
raw backend string in English.

**Status:** PENDING

---

## Phase 4: Search & Query Performance

The pharmacy searches by Thai name and code every day; it must be fast on
the real catalog (Gap 7), and the INVS query path must stop blocking
itself (Gap 5).

- [ ] **Minimum query length** (2 chars) with a hint in the dropdown;
  keep the 300 ms debounce and the generation-counter race protection
  already in `drug_search_panel.rs`.
- [ ] **Ranked results.** Front-load exact `icode`/`working_code` matches,
  then prefix matches, then substring — a pure ranking function in the
  backend, not SQL gymnastics. HOSxP: `icode = ? OR icode LIKE 'q%' OR
  name LIKE 'q%' OR name LIKE '%q%'`, each tier bounded.
- [ ] **Indexed search where possible.** For MySQL add (or verify) an
  index on `drugitems.name` for the prefix tier; measure. For SQL Server
  verify the `DRUG_GN` lookups used by the value query have covering
  indexes (`WORKING_CODE`).
- [ ] **INVS query concurrency.** Replace the single serialised client
  with a small pool of `tiberius` clients (e.g. 2–4) behind a semaphore so
  the two panels don't block each other; keep the state managed.
- [ ] **Kill the N+1 in `invs_get_top_drugs_by_value`** — compute peak
  month in one query (windowed function / `ROW_NUMBER()`), not one query
  per drug.
- [ ] **Decide the dead commands.** If Phase 7 lands, build the view on
  the fixed queries; otherwise remove both commands, their models, and
  their registrations. No dead IPC surface.
- [ ] **Baseline measurement.** A short `docs/perf-baseline.md`: search
  latency, year-summary latency, chart fetch latency on a representative
  hospital dataset (even a synthetic 500K-row `opitemrece`). Regression
  budget: search < 300 ms, chart fetch < 1 s.

**Acceptance:** search on a 100K-row catalog returns ranked results in
< 300 ms; the two panels refresh concurrently without blocking each other;
`top_drugs` (if kept) issues 2 queries total, not N+1; perf baseline
documented in `docs/perf-baseline.md`.

**Status:** PENDING

---

## Phase 5: Export & Reporting

Pharmacists report to the hospital board and the MOPH. Balance must hand
them the numbers, not just show them.

- [ ] **CSV export everywhere.** Every chart and every discrepancy view
  gets an export button: the 12-month series, the mapping table, the
  discrepancy flags. UTF-8 with BOM so Excel opens Thai text correctly.
  Written by the backend to a user-chosen path (`tauri-plugin-dialog` +
  `fs` write; capability `shell:allow-open` already exists).
- [ ] **Excel export** (`.xlsx`, not CSV-in-disguise): a workbook with
  sheets per export (series, deltas, unit prices, mappings) — via a pure
  Rust xlsx writer on the backend.
- [ ] **Per-drug annual report (PDF).** A one-page printable summary:
  drug names both sides, codes, 12-month table (qty / value / unit
  price), flags, totals. Thai font embedded in the PDF. Print from a
  generated HTML in a new window, or a Rust PDF builder — whichever
  renders Thai correctly (test against Thai glyphs in CI if feasible).
- [ ] **Yearly reconciliation report.** All mapped drugs for a fiscal
  year: qty, value, unit price, flags — the thing the pharmacy director
  actually wants. Same PDF/Excel/CSV trio.
- [ ] **Export naming convention** (e.g. `balance-รายงาน-2568.xlsx`) and a
  "export complete" toast with the saved path.

**Acceptance:** every export opens correctly in Excel and Thai renders
correctly; the yearly reconciliation report includes all mapped drugs
with flags; exports complete from a fresh app state with no database
round-trips beyond the existing data.

**Status:** PENDING

---

## Phase 6: Testing & CI Hardening

The math must be proven and CI must run it (Gap 9).

- [ ] **Backend unit tests.** Fiscal-year helpers, DSN encoding
  (`urlencoding_simple`), mapping normalizer/scorer, discrepancy math —
  all pure functions with fixtures, no I/O.
- [ ] **Backend integration tests against real schemas.** `sqlx` tests
  against a disposable MySQL (testcontainers or a CI MySQL service) with
  a minimal `opitemrece`/`drugitems` fixture; verify the queries return
  the expected shapes (including the empty-row and null-name paths the
  `col_*` helpers exist for).
- [ ] **INVS query tests.** Replay `tiberius` row fixtures through the
  row helpers (`get_str`/`get_f64`) and the fiscal-month mapping — at
  minimum a SQL Server CI service for the live queries, or a recorded
  fixture harness if infra isn't available.
- [ ] **Run the frontend tests in CI.** `wasm_bindgen_test` needs a
  runner (wasmtime or headless Chrome); add a job so `models.rs` tests
  (fiscal year, formatting) actually execute on every push.
- [ ] **Chart renderer tests.** Extract the pure parts of the canvas
  renderer (axis step `nice_step`, label formatting `fmt_y`, moving
  average) into testable functions and cover them. The canvas itself is
  visually verified in Phase 6's e2e step below if feasible.
- [ ] **E2E smoke (optional, valuable).** `tauri-driver` + WebDriver:
  boot → settings → test-connection failure path → no-connection banner
  shows Thai copy. Keep it to the must-not-break paths only.
- [ ] **Add a `cargo test` job to CI** for the workspace, alongside the
  existing fmt/clippy/build/deny jobs.

**Acceptance:** every pure function has tests and they run in CI; the
backend has at least one integration test per query family; a wrong
fiscal-year boundary or a formatting regression fails CI, not a
pharmacist's report.

**Status:** PENDING

---

## Phase 7: Top Drugs Analytics & Multi-Year Comparison

Activate the dormant ranking commands (Gap 6) and add the multi-year view
the README promised.

- [ ] **Top Drugs view.** A new panel/tab: "Top N by quantity" (HOSxP),
  "Top N by value" (INVS), and — once mapped — "Top N by unit-price
  change" and "Top N discrepancy flags". Clicking a row selects it in
  both panels simultaneously (the mapping finally makes this possible).
- [ ] **Multi-year comparison.** A 2–3 year overlay on the chart (pure
  series data already exists per year; add year selection chips) and a
  year-over-year table: qty, value, unit price, % change.
- [ ] **KPI bar parity.** Add HOSxP totals (yearly dispensed quantity and
  top-drug count) so the bottom bar is symmetric — currently only INVS
  has numbers (Gap 10).
- [ ] **Remove or keep the dead commands deliberately.** If the view
  lands, keep and fix them (Phase 4 already fixes the N+1); document in
  `docs/architecture.md` which commands back which views.

**Acceptance:** Top Drugs renders from the existing data path; multi-year
overlay works for both sides; every command in `lib.rs` is reachable from
a UI element or deleted.

**Status:** PENDING

---

## Phase 8: Watchlist & Alerts

The pharmacist shouldn't have to look at every drug to find the problem
ones.

- [ ] **Watchlist.** Star drugs in the search panel; a `watchlist` table
  in the local store (per-drug note optional). Pinned drugs show a badge
  in both panels.
- [ ] **Threshold configuration.** Unit-price spike factor, "zero use"
  months, dispensed-without-purchase tolerance — editable in a settings
  tab (stored locally).
- [ ] **Alert center.** A bell in the header with a badge count; the panel
  lists active flags for watchlist drugs across the selected year:
  drug, flag type, month(s), the two numbers behind it, and a jump-to
  action that selects the drug in both panels.
- [ ] **Year-boundary rollover.** Flags recompute per selected fiscal
  year; nothing persists "read" state across years.

**Acceptance:** a watchlist drug with a unit-price spike produces a
visible alert with traceable numbers; alert counts survive restarts;
thresholds are user-configurable and the config is validated (no NaN/∞).

**Status:** PENDING

---

## Phase 9: Localization Structure & Accessibility

Balance is Thai-only on purpose — but it must be Thai *by structure*, not
by accident (Gap 12).

- [ ] **String registry.** Move all inline Thai copy into one module
  (e.g. `ui_strings.rs` — a Rust `struct` of `&'static str`s) so text is
  findable, reviewable, and replaceable without hunting through views.
  No behavioral change, no i18n framework yet.
- [ ] **Configurable hospital name.** Replace the hard-coded
  `โรงพยาบาลสระโบสถ์` with a settings field (stored locally) used by the
  header and (later) the PDF report header.
- [ ] **Keyboard navigation audit.** All views (drawer, dropdowns, chart
  tooltips) keyboard-completable; focus rings visible (design tokens
  already exist — verify `:focus-visible` coverage).
- [ ] **Contrast & touch audit.** Badges, muted text, and disabled states
  meet WCAG AA where feasible; the header's translucent badges on the
  dark bar are checked against the white-drawer variants.
- [ ] **Screen-reader labels.** `aria-label`s for icon buttons
  (ตั้งค่า, ปิด, ล้างการค้นหา) and the connection badges.

**Acceptance:** zero user-facing strings outside the registry; hospital
name editable and persisted; keyboard-only walkthrough of settings →
search → year change succeeds; icon buttons have accessible labels.

**Status:** PENDING

---

## Phase 10: Distribution, Updates & the v1.0 Gate

The software is used by people who are not programmers; it must install,
update, and prove itself.

### Distribution

- [ ] **Signing.** macOS notarization and Windows code signing set up in
  CI (release workflow via `tauri-action`).
- [ ] **Auto-update.** `tauri-plugin-updater` with a release feed; the
  app checks on boot and the header shows "อัปเดตพร้อมแล้ว" with a
  restart action.
- [ ] **Installer polish.** App icons (`scripts/gen-icons.sh` regenerates
  every platform size from `icon-master.svg` via `cargo tauri icon`),
  app name in Thai-friendly casing, uninstall cleanliness
  (settings + local DB handled on uninstall for macOS/Windows).

### The v1.0 gate (clinical/business validation)

- [ ] **Pilot at a real hospital.** Run the mapping engine against the
  real catalogs; the pharmacist maps the actual formulary.
- [ ] **Reconciliation audit.** Pick 10 mapped drugs; verify each flag
  and number against the source databases by hand. Zero unexplained
  numbers is the bar.
- [ ] **Performance sign-off on the hospital's PC** against the
  `docs/perf-baseline.md` budgets.
- [ ] **Offline acceptance.** The hospital network drops (or the IT
  department blocks `esm.sh`); the app still boots and reconciles.
- [ ] **User sign-off.** Two pharmacists use it daily for one fiscal
  quarter; written feedback logged in `docs/validation-report.md`; any
  blocking issue becomes a new phase's first item.
- [ ] **Tag `v1.0.0`** only when the above are documented, not on
  schedule.

**Acceptance:** signed, auto-updating builds in CI; a documented pilot
with hand-verified numbers; `v1.0.0` means "proven in a real pharmacy",
not "the code compiles".

**Status:** PENDING

---

## How the phases relate

```
Phase 1 (Drug Mapping)          -- foundation -- everything hangs off this
        |
        +---> Phase 2 (Reconciliation & Discrepancy) -- needs mappings
        |           |
        |           +---> Phase 5 (Export & Reporting)   -- needs the numbers
        |           +---> Phase 8 (Watchlist & Alerts)   -- flags the numbers
        |
Phase 3 (Reliability & Offline) -- independent, fixes a principle violation
Phase 4 (Search & Performance)  -- independent of Phase 1-2, do while queries
        |                             are still cheap to touch
        +---> Phase 7 (Top Drugs & Multi-Year) -- reuses fixed commands
        |
Phase 6 (Testing & CI)          -- parallel track, any time; gate for all
        |
Phase 9 (Localization & A11y)   -- independent polish, can start early
        |
Phase 10 (Distribution & v1.0)  -- last: ship a proven tool
```

Phase 1 comes first on purpose: without a mapping, the product cannot
compare anything. Phase 2 is the payoff and must not be diluted by
features. Phases 3 and 4 are debt payments that make the hospital
deployment actually usable. Phase 10 is the gate: **Balance ships v1.0
when a real pharmacy has verified the numbers by hand** — not when the
feature list is empty.

---

## Out of Scope (drawn on purpose)

Each of these is valuable *for a different product*. Balance stays a
reconciliation dashboard:

- **EHR / ERP replacement** — HOSxP and INVS stay the systems of record;
  Balance is read-only and remains so.
- **Writing to HOSxP or INVS** — no sync-back, no auto-adjusting stock,
  no write access ever. If a discrepancy needs fixing, a person fixes it
  in the source system.
- **Automated procurement** — suggesting purchase orders crosses into
  supply-chain management; Balance reports, the pharmacist decides.
- **AI/LLM analysis of drug data** — hallucinated numbers have no place
  in a reconciliation tool; rule-based, deterministic flags only.
- **Patient-facing anything** — no patient data display beyond drug
  codes/names, no PHI beyond what the source systems already expose to
  the pharmacy.
- **SaaS / multi-tenant hosting** — a hospital's inventory data stays in
  the hospital; Balance is a local desktop app.
- **Multi-language UI** — Thai-only today, but the string structure of
  Phase 9 keeps the door open without committing to it.
- **Mobile support** — the two-panel canvas dashboard is a desktop
  form-factor; a mobile app is a different product.
- **Blockchain/audit ledger** — the local DB + migration versioning is
  the audit trail; adding a ledger adds nothing verifiable for this use
  case.

---

## Documentation

The `docs/` directory should grow with the project:

| Document | Content | When |
|----------|---------|------|
| `DESIGN.md` | Design system, tokens, components | Exists (moved here) |
| `ROADMAP.md` | This document | Now |
| `architecture.md` | IPC surface, module map, data flow, command↔view matrix | Done (Phase 1) |
| `database.md` | Local store schema, migrations, query patterns | Done (Phase 1) |
| `mapping.md` | Matching heuristics, scoring, bulk-import format | Done (Phase 1) |
| `reconciliation.md` | Discrepancy rules, thresholds, worked examples | Phase 2 |
| `perf-baseline.md` | Latency budgets, measurement method, results | Phase 4 |
| `validation-report.md` | Pilot results, hand-verification log, sign-off | Phase 10 |
