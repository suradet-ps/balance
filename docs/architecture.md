# Balance Architecture

This document maps the codebase: modules, the IPC surface, and how data
flows between the systems of record and the UI.  It follows the current
state of the repo (updated after Phase 2).

## Stack

- **Shell**: Tauri 2 (Rust, `src-tauri/` — workspace member `balance`).
- **Frontend**: Leptos 0.8 CSR (`src/` — workspace member `balance-frontend`,
  compiled to `wasm32-unknown-unknown`, bundled by Trunk).  No Node toolchain.
- **Source systems (read-only)**: HOSxP (MySQL, `sqlx` pool) and INVS
  (SQL Server, single `tiberius` client behind a mutex).
- **App-owned data**: SQLite local store (`balance.db` in the app-data dir).

## Module map (backend, `src-tauri/src/`)

| Module | Responsibility |
|--------|----------------|
| `lib.rs` | Tauri wiring: manages state, opens+migrates the store in `.setup()`, registers every command. |
| `settings.rs` | Encrypted settings file (`settings.json`, AES-256-GCM, OS keychain master key). |
| `fiscal.rs` | **Pure** Thai fiscal-year helpers: `cal_to_fiscal_idx`, `reorder_calendar_to_fiscal`, `fiscal_year_range` / `fiscal_mysql_window` (the single boundary definition — FY N = 1 Oct N−1 … 30 Sep N). |
| `hosxp/db.rs` | MySQL pool lifecycle (`init_pool`, `with_pool`, DSN encoding). |
| `hosxp/commands.rs` | HOSxP queries (all fiscal-windowed): years, drug search, monthly quantities, top drugs, year summary. |
| `invs/db.rs` | SQL Server client lifecycle (`connect`). |
| `invs/commands.rs` | INVS queries: years, drug search, monthly value, top drugs, year summary. |
| `store.rs` | Local SQLite store: open + versioned migration at startup. |
| `mapping/normalizer.rs` | **Pure** Thai/Latin drug-name normalizer + similarity scorer (no I/O). |
| `mapping/repo.rs` | SQLite data access for mappings + exclusions (unit-tested). |
| `mapping/bulk.rs` | **Pure** bulk-CSV parser (unit-tested). |
| `mapping/commands.rs` | Mapping IPC surface (see below). |
| `reconcile/mod.rs` | **Pure** reconciliation & discrepancy engine (no I/O, unit-tested). |
| `reconcile/commands.rs` | `reconcile_drug` IPC adapter: resolve mapping → fetch both series → run the engine. |

Backend commands stay thin adapters: parse args → call domain logic →
serialize.  All matching math lives in `mapping/normalizer.rs`; all
reconciliation math in `reconcile/mod.rs`; all local store SQL lives in
`mapping/repo.rs`; the MySQL/SQL Server queries are isolated in their own
command modules.

## Module map (frontend, `src/src/`)

| Module | Responsibility |
|--------|----------------|
| `models.rs` | Wire types mirrored from the backend + pure helpers (fiscal year, number formatting). |
| `services/tauri.rs` | The one place that touches `window.__TAURI__` / `invoke`. |
| `services/commands.rs` | Typed wrappers for every Tauri command. |
| `contexts/dashboard.rs` | Dashboard state: year, per-side chart data, year summaries, loading, refreshes. |
| `contexts/db_config.rs` | Connection configs + connect/save actions. |
| `contexts/mapping.rs` | Mapping state: rows + status filter, detail session, auto-match preview, bulk import, panel chips, linked selection (follow the mapping across panels). |
| `components/*` | Presentational components: `mapping_panel.rs` is the full-screen master–detail mapping view, `discrepancy_view.rs` the reconciliation strip. |

## IPC surface (command ↔ view matrix)

| Command | Backs |
|---------|-------|
| `save_settings` / `load_settings` | Settings drawer (HOSxP + INVS tabs). |
| `hosxp_connect` | Settings drawer — HOSxP test; also boot auto-connect. |
| `hosxp_get_available_years` | Header year selector (**fiscal** years). |
| `hosxp_get_drug_list` | HOSxP search panel dropdown. |
| `hosxp_get_drug_monthly_qty` | HOSxP chart (fiscal-month buckets). |
| `hosxp_get_year_summary` | KPI bar (HOSxP ยอดจ่ายรวม). |
| `hosxp_get_top_drugs` | *No view yet* — Phase 7 decides (build Top Drugs or remove). |
| `invs_connect` | Settings drawer — INVS test; also boot auto-connect. |
| `invs_get_available_years` | Header year selector (fiscal years). |
| `invs_get_drug_list` | INVS search panel dropdown. |
| `invs_get_drug_monthly_value` | INVS chart. |
| `invs_get_year_summary` | KPI bar (INVS ยอดซื้อรวม). |
| `invs_get_top_drugs_by_value` | *No view yet* — Phase 7 decides. |
| `mapping_status_by_icode` / `mapping_status_by_working_code` | Panel match-status chips; linked selection across panels. |
| `mapping_list_rows` | Mapping view — HOSxP list with per-row state. |
| `mapping_stats` | Mapping view + KPI bar — headline counts. |
| `mapping_suggest` | Mapping view — scored INVS candidates for the detail pane. |
| `mapping_set` | Mapping view — manual/approved/auto link creation. |
| `mapping_remove` | Mapping view — break a link. |
| `mapping_mark_no_invs` / `mapping_unmark_no_invs` | Mapping view — "no INVS equivalent" marker. |
| `mapping_auto_match` | Mapping view — batch auto-match (dry-run preview then apply). |
| `mapping_bulk_import` | Mapping view — bulk CSV (dry-run preview then apply). |
| `reconcile_drug` | Discrepancy strip — the comparison for the selected mapped drug. |

## Data flow

```
HOSxP (MySQL, read-only)        INVS (SQL Server, read-only)
   │  │  │                          │  │  │
   │  │  └── drugitems ─────────────┼──┼──┼── DRUG_GN
   │  │                             │  │  └── MS_IVO / MS_IVO_C
   │  └── opitemrece                └──┘
   └── hosxp::commands             invs::commands
             │                          │
             ▼                          ▼
   ┌──────────────────────────────────────────────┐
   │            Tauri IPC (commands)              │
   │  ───────────────────────────────────────────  │
   │  mapping::commands ──mapping/repo.rs─────────►│
   │  reconcile::commands ──reconcile/mod.rs (pure)│
   └──────────────────────────────────────────────┘
             │                              │
             ▼                              ▼
      Leptos frontend                Local SQLite store
      (contexts / components)        (balance.db, migrations)
```

- Mapping scores are computed in **pure Rust** (`mapping/normalizer.rs`);
  the databases only supply candidate names.  The same scorer unit-tests
  in CI.
- Balance never writes to HOSxP or INVS — every query is a `SELECT`, and
  the local store is the only place Balance-owned rows land.
