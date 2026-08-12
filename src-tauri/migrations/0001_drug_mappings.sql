-- Migration 0001: the drug mapping table.
--
-- Links a HOSxP `icode` to an INVS `working_code`.  `match_method` records
-- how the link was established: `auto` (machine-suggested, score above
-- threshold), `manual` (pharmacist forced it), `approved` (pharmacist
-- confirmed a suggested candidate).  One icode may carry several working_code
-- links, but the same pair is unique — re-confirming a link is an upsert.

CREATE TABLE IF NOT EXISTS drug_mappings (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    icode            TEXT    NOT NULL,
    working_code     TEXT    NOT NULL,
    drug_name_hosxp  TEXT    NOT NULL DEFAULT '',
    drug_name_invs   TEXT    NOT NULL DEFAULT '',
    match_method     TEXT    NOT NULL
                     CHECK (match_method IN ('auto', 'manual', 'approved')),
    match_score      REAL,
    created_at       TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (icode, working_code)
);

CREATE INDEX IF NOT EXISTS idx_drug_mappings_icode
    ON drug_mappings (icode);

CREATE INDEX IF NOT EXISTS idx_drug_mappings_working_code
    ON drug_mappings (working_code);
