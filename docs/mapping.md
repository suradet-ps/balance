# Drug Mapping — Matching Heuristics, Scoring, Import Format

How Balance links a HOSxP drug (`icode`) to an INVS drug (`working_code`).
All matching math is **pure Rust** in `src-tauri/src/mapping/normalizer.rs`,
unit-tested against real drug-name fixtures; the databases only ever supply
candidate names.

## The workflow

1. **List** — the pharmacist searches the HOSxP catalog in the mapping
   drawer (ตารางรายการ).  Each row shows its state: `mapped` (with the
   linked INVS code), `no_invs` (marked as having no INVS equivalent), or
   `unmapped`.
2. **Suggest** — "ดูคำแนะนำ" on an unmapped row fetches the top INVS
   candidates scored by name similarity, best first, with the score shown
   as a percentage.
3. **Match** — the pharmacist clicks "แมป" on a candidate (recorded as
   `approved`: suggested by machine, confirmed by a person), or force-links
   an arbitrary INVS drug via the manual search (recorded as `manual`).
4. **Batch** — "แมปอัตโนมัติ" runs the same scorer over the current list and
   previews every match at or above the threshold (`auto`); the pharmacist
   confirms before anything is written.  CSV import (tab นำเข้า CSV) does
   the same for the hospital's own `icode ↔ working_code` list.
5. **Exclusions** — "ไม่มีใน INVS" marks a drug as having no INVS equivalent
   (e.g. no longer procured), with the reason recorded.  A mapping and an
   exclusion for the same icode are mutually exclusive.

## The normalizer

Applied identically to both sides (so HOSxP names and INVS names are
comparable):

1. lowercase;
2. parenthesized *dose* content is dropped — `(500 mg)` goes away, while
   `(พาราเซตามอล)` is kept because it is a translation, not a strength;
3. Thai spelling variants unified: `รร` → `ร`, `รา` → `ร` (when followed by
   a consonant or end-of-word — so `ธารา`/`ธาร` and `การันต์`/`กรนต์`
   collide, but `พาราเซตามอล` keeps its `รา`), `รึ` → `ริ`;
4. tokens are split on non-alphanumerics, digit boundaries and script
   boundaries (Latin ↔ Thai) — `amoxicillin500mg` and
   `Paracetamol(พาราเซตามอล)` both tokenize cleanly;
5. pure numbers and dose/unit/dosage-form tokens are dropped (`mg`, `มก.`,
   `tablet`, `เม็ด`, `แคปซูล`, … — see `DOSE_TOKENS` in the source);
6. the surviving tokens are sorted and deduped: the normalized output is a
   canonical token set, so `Paracetamol (พาราเซตามอล)` and
   `พาราเซตามอล paracetamol` are equal.

**Known, deliberate limitation:** strengths are stripped, so
`Aspirin 81 mg` and `Aspirin 325 mg` normalize identically and score 1.0.
Strength-aware matching is out of scope for Phase 1 — the auto-match
threshold and the pharmacist review are the guardrails.

## The scorer

`similarity(a, b)` in `[0.0, 1.0]`, a cascade:

1. equal normalized strings → **1.0**;
2. token overlap (Jaccard) > 0 → **0.5 + 0.5 · Jaccard** (any shared token
   beats a pure edit-distance guess);
3. otherwise → **1 − Levenshtein / max_len** on the normalized strings.

Nothing is ever fabricated: if either side normalizes to an empty set the
score is 0.0.

| Pair | Score |
|------|-------|
| `Amoxicillin 500 mg` vs `Amoxicillin 500mg` | 1.0 |
| `พาราเซตามอล 500 มก.` vs `พาราเซตามอล 500 มิลลิกรัม` | 1.0 |
| `Paracetamol (พาราเซตามอล)` vs `พาราเซตามอล paracetamol` | 1.0 |
| `Amoxicillin` vs `Amoxicillin + Clavulanate` | 0.75 |
| `Ciprofloxacin 500 mg` vs `Cefixime 100 mg` | < 0.5 |

### Auto-match threshold

`AUTO_MATCH_THRESHOLD = 0.95` (`normalizer.rs`).  Candidates at or above it
may be applied automatically (`match_method = 'auto'`), always through the
preview-then-confirm flow.  Below it they are only suggestions.

## Candidate generation

`mapping_suggest` queries `DRUG_GN` loosely — exact `WORKING_CODE`, code
prefix, or name substring of the first search word (second word joined in
when the first is under 3 chars; capped at 16 chars) — then scores every
hit in Rust and returns the top N (default 10).

## Bulk CSV format

Pasted into the นำเข้า CSV tab.  Header optional (auto-detected), the two
name columns optional:

```csv
icode,working_code,drug_name_hosxp,drug_name_invs
041234,WA001,Amoxicillin 500 mg,Amoxicillin (แคปซูล)
041235,WA002,พาราเซตามอล 500 มก.,พาราเซตามอล (ยาเม็ด)
```

Rules:

- rows with an empty `icode` or `working_code` are **skipped** (counted);
- rows whose icode is already mapped to a **different** working_code are
  **conflicts** — never overwritten silently; the dry-run preview lists
  them for manual resolution in the list view;
- everything else is applied as `match_method = 'approved'` (the hospital's
  own list is treated as human-confirmed), names recorded when present;
- unparsable lines are reported with their line numbers, never dropped
  silently.

## Where the rules are tested

- `mapping/normalizer.rs` — fixtures with real drug names (strength strip,
  parens, Thai spelling variants, score cascade, threshold placement).
- `mapping/bulk.rs` — CSV parsing: header detection, quoting, line errors.
- `mapping/repo.rs` + `store.rs` — upsert/uniqueness, exclusion
  mutual-exclusion, migration idempotence.
