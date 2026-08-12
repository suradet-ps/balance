# Balance Architecture

This document maps the codebase: modules, the IPC surface, and how data
flows between the systems of record and the UI.  It follows the current
state of the repo (updated in Phase 1).

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
| `hosxp/db.rs` | MySQL pool lifecycle (`init_pool`, `with_pool`, DSN encoding). |
| `hosxp/commands.rs` | HOSxP queries: years, drug search, monthly quantities, top drugs. |
| `invs/db.rs` | SQL Server client lifecycle (`connect`). |
| `invs/commands.rs` | INVS queries: years, drug search, monthly value, top drugs, year summary, fiscal-month helpers. |
| `store.rs` | Local SQLite store: open + versioned migration at startup. |
| `mapping/normalizer.rs` | **Pure** Thai/Latin drug-name normalizer + similarity scorer (no I/O). |
| `mapping/repo.rs` | SQLite data access for mappings + exclusions (unit-tested). |
| `mapping/bulk.rs` | **Pure** bulk-CSV parser (unit-tested). |
| `mapping/commands.rs` | Mapping IPC surface (see below). |

Backend commands stay thin adapters: parse args → call domain logic →
serialize.  All matching math lives in `mapping/normalizer.rs`; all local
store SQL lives in `mapping/repo.rs`; the MySQL/SQL Server queries are
isolated in their own command modules.

## Module map (frontend, `src/src/`)

| Module | Responsibility |
|--------|----------------|
| `models.rs` | Wire types mirrored from the backend + pure helpers (fiscal year, number formatting). |
| `services/tauri.rs` | The one place that touches `window.__TAURI__` / `invoke`. |
| `services/commands.rs` | Typed wrappers for every Tauri command. |
| `contexts/dashboard.rs` | Dashboard state: year, per-side chart data, loading, refreshes. |
| `contexts/db_config.rs` | Connection configs + connect/save actions. |
| `contexts/mapping.rs` | Mapping state: rows, suggestion session, auto-match preview, bulk import, panel chips. |
| `components/*` | Presentational components; `mapping_panel.rs` is the mapping drawer. |

## IPC surface (command ↔ view matrix)

| Command | Backs |
|---------|-------|
| `save_settings` / `load_settings` | Settings drawer (HOSxP + INVS tabs). |
| `hosxp_connect` | Settings drawer — HOSxP test; also boot auto-connect. |
| `hosxp_get_available_years` | Header year selector. |
| `hosxp_get_drug_list` | HOSxP search panel dropdown. |
| `hosxp_get_drug_monthly_qty` | HOSxP chart. |
| `hosxp_get_top_drugs` | *No view yet* — Phase 7 decides (build Top Drugs or remove). |
| `invs_connect` | Settings drawer — INVS test; also boot auto-connect. |
| `invs_get_available_years` | Header year selector. |
| `invs_get_drug_list` | INVS search panel dropdown. |
| `invs_get_drug_monthly_value` | INVS chart. |
| `invs_get_year_summary` | KPI bar (INVS totals). |
| `invs_get_top_drugs_by_value` | *No view yet* — Phase 7 decides. |
| `mapping_status_by_icode` / `mapping_status_by_working_code` | Panel match-status chips. |
| `mapping_list_rows` | Mapping drawer — HOSxP list with per-row state. |
| `mapping_stats` | Mapping drawer — headline counts. |
| `mapping_suggest` | Mapping drawer — scored INVS candidates for a row. |
| `mapping_set` | Mapping drawer — manual/approved/auto link creation. |
| `mapping_remove` | Mapping drawer — break a link. |
| `mapping_mark_no_invs` / `mapping_unmark_no_invs` | Mapping drawer — "no INVS equivalent" marker. |
| `mapping_auto_match` | Mapping drawer — batch auto-match (dry-run preview then apply). |
| `mapping_bulk_import` | Mapping drawer — bulk CSV (dry-run preview then apply). |

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
