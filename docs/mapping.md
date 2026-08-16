# Drug Mapping — Matching Heuristics, Scoring, Import Format

How Balance links a HOSxP drug (`icode`) to an INVS drug (`working_code`).
All matching math is **pure Rust** in `src-tauri/src/mapping/normalizer.rs`,
unit-tested against real drug-name fixtures; the databases only ever supply
candidate names.

## The workflow

The mapping view is a full-screen **master–detail** layout (no drawer, no
tabs):

1. **List (left pane)** — the pharmacist searches the HOSxP catalog
   (debounced), filters by status chip (ทั้งหมด / ยังไม่แมป / แมปแล้ว /
   ไม่มีใน INVS) and clicks a row.  Each row shows its state: `mapped`
   (with the linked INVS code), `no_invs` (marked as having no INVS
   equivalent), or `unmapped`.
2. **Detail (right pane)** — the selected drug's state and its scored INVS
   candidates, best first, with the score shown as a percentage.
3. **Match** — the pharmacist clicks "แมป" on a candidate (recorded as
   `approved`: suggested by machine, confirmed by a person), or force-links
   an arbitrary INVS drug via the manual search in the same pane (recorded
   as `manual`).  After any change the selection **auto-advances** to the
   next unmapped row, so a 100+ drug session never returns to the list by
   hand.
4. **Batch** — the header's "แมปอัตโนมัติ" runs the same scorer over the
   current search results and previews every match at or above the
   threshold (`auto`) in a confirm dialog; nothing is written before the
   pharmacist confirms.  "นำเข้า CSV" opens the same preview-then-confirm
   flow for the hospital's own `icode ↔ working_code` list.
5. **Exclusions** — "ยานี้ไม่มีใน INVS" in the detail pane marks a drug as
   having no INVS equivalent (e.g. no longer procured), with the reason
   recorded.  A mapping and an exclusion for the same icode are mutually
   exclusive.

## The normalizer

Applied identically to both sides (so HOSxP names and INVS names are
comparable):

1. lowercase;
2. parenthesized *dose* content is dropped — `(500 mg)` goes away, while
   `(พาราเซตามอล)` is kept because it is a translation, not a strength;
   a paren that mixes both — `(แอมม็อกซิซิลลิน 500 มก.)` — keeps the
   translation and drops the dose tokens;
3. Thai spelling variants unified: `รร` → `ร`, `รา` → `ร` (when followed by
   a consonant or end-of-word — so `ธารา`/`ธาร` and `การันต์`/`กรนต์`
   collide, but `พาราเซตามอล` keeps its `รา`), `รึ` → `ริ`;
4. tokens are split on non-alphanumerics, digit boundaries and script
   boundaries (Latin ↔ Thai) — `amoxicillin500mg` and
   `Paracetamol(พาราเซตามอล)` both tokenize cleanly;
5. pure numbers and dose/unit/dosage-form tokens are dropped (`mg`, `มก.`,
   `tablet`, `เม็ด`, `แคปซูล`, … — see `DOSE_TOKENS` in the source); Thai
   numerals (`๐`–`๙`) count as digits here, so `500 มก.` and `๕๐๐ มก.`
   normalize identically;
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

`mapping_suggest` queries `DRUG_GN` — exact `WORKING_CODE`, code prefix, or
name substring of the first search word (second word joined in when the
first is under 3 chars; capped at 16 chars), ordered exact → prefix →
substring so the `TOP` cut is relevance-first.  Wildcards are escaped
(`ESCAPE '~'`), a blank name returns no candidates (never `LIKE '%%'`),
and hits are deduped by `WORKING_CODE` keeping the best-scoring name —
then scored in Rust, best N (default 10) returned.

## Bulk CSV format

Pasted into the นำเข้า CSV dialog (header button in the mapping view).
Header optional (auto-detected — only when **both** leading fields are
column names, so a drug literally named "Code" is never eaten), the two
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
  them for manual resolution in the mapping view;
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
