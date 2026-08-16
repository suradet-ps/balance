# Balance Local Database

The local SQLite store holds every piece of app-owned data — drug mappings,
exclusions, and later watchlists and settings schema.  It sits in the
app-data directory next to `settings.json` as `balance.db`, is opened
read-write **locally only**, and is versioned through embedded migrations.

## Where it lives

| Thing | Location |
|-------|----------|
| Database file | `<app-data>/balance.db` (next to `settings.json`) |
| Migrations | `src-tauri/migrations/NNNN_name.sql` (embedded at compile time) |
| Migration runner | `src-tauri/src/store.rs` (`migrate()`) |
| Connection | One `rusqlite::Connection` behind a mutex, managed Tauri state |

The store is opened in `lib.rs` `.setup()`, **before the UI mounts**, so the
mapping view never sees an unmigrated schema.  The connection is a plain
`std::sync::Mutex<Connection>` — every statement is short and synchronous.

## Migration mechanics

- `schema_migrations(version TEXT PRIMARY KEY, applied_at TEXT)` records
  what has run.
- `migrate()` applies each entry of the compile-time `MIGRATIONS` list that
  is not yet recorded, in order, each inside a transaction.
- **Rule: never edit an applied migration — append a new one.**  The file
  stem is the version name.
- CI proves migrations against a fresh DB: `cargo test -p balance` runs
  `migrate()` on an in-memory database (`store::tests`).

To add a migration: write `src-tauri/migrations/0003_your_name.sql`, then
add `("0003_your_name", include_str!("../migrations/0003_your_name.sql"))`
to `MIGRATIONS` in `src-tauri/src/store.rs`.

## Schema (current version: 2)

### `drug_mappings` — the icode ↔ working_code link (migration 0001)

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | autoincrement |
| `icode` | TEXT | HOSxP code |
| `working_code` | TEXT | INVS code |
| `drug_name_hosxp` | TEXT | name snapshot at match time |
| `drug_name_invs` | TEXT | name snapshot at match time |
| `match_method` | TEXT | `auto` \| `manual` \| `approved` (CHECK) |
| `match_score` | REAL | similarity score for `auto`/`approved` |
| `created_at` / `updated_at` | TEXT | UTC (`datetime('now')`) |

- `UNIQUE (icode, working_code)` — re-confirming a link is an upsert.
- Indexes on `icode` and `working_code` (both panels look up by code).
- **One icode holds at most one active link**: `repo::upsert` deletes any
  previous link for the icode before inserting, so remapping replaces rather
  than stacks rows.  The UI reads only the latest link everywhere, so a
  hidden stale link (which would silently survive "ยกเลิกการแมป") must not
  exist.  A working_code may still map to several icodes.

### `mapping_exclusions` — "no INVS equivalent" markers (migration 0002)

| Column | Type | Notes |
|--------|------|-------|
| `icode` | TEXT PK | the HOSxP drug |
| `reason` | TEXT | pharmacist-recorded reason (e.g. เลิกจัดซื้อแล้ว) |
| `created_at` / `updated_at` | TEXT | UTC |

Marking an exclusion and creating a mapping are **mutually exclusive**: both
`repo::upsert` and `repo::set_no_invs` delete the other side's row in the
same transaction.

## Query patterns

- All data access goes through `src-tauri/src/mapping/repo.rs` — no SQL in
  the command layer.
- Two-step joins (HOSxP rows × mapping state): `mapping_list_rows` fetches
  the MySQL rows, then `repo::links_for_icodes` + `repo::excluded_map`
  enrich them from the local store (a real cross-DB `JOIN` is impossible).
- Reads are snapshot-consistent per call; the mutex serialises writes.

## Testing

`store::tests` and `mapping::repo::tests` run against in-memory databases:
fresh migration, idempotence, the method CHECK, upsert semantics,
mutual-exclusion, and the stats breakdown.
