-- Migration 0002: HOSxP drugs with no INVS equivalent.
--
-- A pharmacist can mark a HOSxP drug as "no INVS equivalent" (e.g. no longer
-- procured) so it stops appearing as an unmapped gap.  The reason is recorded
-- for the audit trail.  A row here is mutually exclusive with a mapping: both
-- `mapping_set` and `mapping_mark_no_invs` remove the other side's row.

CREATE TABLE IF NOT EXISTS mapping_exclusions (
    icode      TEXT PRIMARY KEY,
    reason     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
