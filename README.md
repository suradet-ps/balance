# Balance

```
██████╗  █████╗ ██╗      █████╗ ███╗   ██╗ ██████╗███████╗
██╔══██╗██╔══██╗██║     ██╔══██╗████╗  ██║██╔════╝██╔════╝
██████╔╝███████║██║     ███████║██╔██╗ ██║██║     █████╗
██╔══██╗██╔══██║██║     ██╔══██║██║╚██╗██║██║     ██╔══╝
██████╔╝██║  ██║███████╗██║  ██║██║ ╚████║╚██████╗███████╗
╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝╚══════╝
```

---

## ◆ PULSE

Two systems of record, one quiet question: **does what we dispensed match
what we bought?** Balance sits beside HOSxP (the hospital information
system, MySQL) and INVS (the Ministry of Public Health inventory system,
SQL Server) and compares dispensed quantities against purchased values -
on the same Thai fiscal-year axis, one year at a time, all in Thai.
The pharmacy spots mismatches, over-stocking, and unexplained consumption
before they become a shortage or a write-off.

| P1 ▣ | P2 ▣ | P3 ▢ | P4-P10 ☐ |
|---|---|---|---|

*Drug mapping and the reconciliation engine are sealed. Offline-first is
half-forged: the app boots without the internet and pings both sources,
but TLS verification and the CI smoke tests still wait. Phases 4-10 stand
open.*

> Built with Tauri 2 + Leptos 0.8, read from HOSxP by `sqlx` and from INVS
> by `tiberius` - never a write into either system of record.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One target, two tools, one launch.

```
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked
⟫ cargo tauri dev       # desktop app; trunk serves itself
```

The release artifact is forged with:

```
⟫ cargo tauri build
```

Frontend only? `⟫ trunk serve --config src/Trunk.toml`

<details>
<summary>Prerequisites</summary>

- [Rust](https://rustup.rs/) (stable) with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/) - `cargo install trunk --locked`
- [Tauri CLI](https://tauri.app/start/) - `cargo install tauri-cli --locked`

</details>

---

## ◆ ANATOMY

Three parts, one rule that never bends: the source systems stay
authoritative. If a number in Balance disagrees with HOSxP or INVS,
Balance is wrong.

- **Maps** - a local SQLite store binds HOSxP `icode` to INVS
  `working_code`: auto-suggested candidates scored with Thai-aware name
  matching, batch auto-match, bulk CSV import, and a match-status chip on
  both panels. Unmapped drugs stay visibly unmapped - never silently
  compared.
- **Reconciles** - year-first comparison for mapped drugs: unit price,
  coverage ratio, the cumulative stock curve, and rule-based discrepancy
  flags, all computed by pure functions with unit tests. Same inputs,
  same verdict, always.
- **Shows** - a hand-rolled `<canvas>` renderer draws 12 fiscal-month bars
  with a 3-month moving average - no charting library, no ECharts, no
  Node toolchain anywhere in the build.
- **Connects** - `sqlx` pools HOSxP MySQL and a single `tiberius` client
  holds INVS SQL Server, both read-only, both behind encrypted settings
  (AES-256-GCM, master key in the OS keychain). When a connection dies,
  the banner says so and the app keeps its composure.
- **Boots** - offline-first is a law: no CDN in the critical path, the
  Tauri API module loaded locally, the app opening on a hospital LAN with
  no internet at all.

---

## ◆ RITUALS

**The core ceremony** - one fiscal year at a time:

1. Configure both connections once; the keychain keeps them sealed.
2. Search by HOSxP `icode` or INVS `working_code` - the panels answer
   independently, then together.
3. Map the drugs that matter: let the scorer suggest, auto-match the
   batch, or import the CSV. Watch both panels light up with status.
4. Read the reconciliation: unit price, coverage ratio, the cumulative
   stock curve, and the flags. The KPI bar tells the day's story.

**The ceremony of honesty** - when two values cannot honestly be compared
(different units, an unmapped drug, missing data), the UI says so. "No
data" is a legitimate, visible state, not a blank that looks comparable.

**The ceremony of silence** - Balance reads from HOSxP and INVS and
never writes a row into either. The systems of record stay the systems
of record.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P1   ▸ drug mapping engine ────────────────────────────────────── ▸ sealed
P2   ▸ reconciliation & discrepancy engine ────────────────────── ▸ sealed
P3   ▸ offline boot + connection health done; TLS, CI smoke ▸ ▸ ▸ forging
P4   ▸ search & query performance on millions of opitemrece rows  ▸ ahead
P5   ▸ export & reporting ──────────────────────────────────────── ▸ ahead
P6   ▸ testing & CI hardening (WASM test runner) ───────────────── ▸ ahead
P7   ▸ top-drugs analytics & multi-year comparison ─────────────── ▸ ahead
P8-P10 ▸ watchlist, localization structure, v1.0 distribution gate ▸ ahead
```

**Raising the artifact** - the honest path is in `docs/ROADMAP.md`; the
IPC surface and data flow in `docs/architecture.md`; matching heuristics
in `docs/mapping.md`; discrepancy rules with worked examples in
`docs/reconciliation.md`. Open an issue first to discuss a change.

**Status** - CI gates every change with four jobs: format + clippy +
backend unit tests, the Trunk build, design-token enforcement, and
`cargo-deny`. [Watch the gates](.github/workflows).

---

```
  ─────────────────────────────────────────
   Dispensed is not purchased.
   The gap between them is the truth worth finding.
  ─────────────────────────────────────────
```

Balance is distributed under the [MIT License](LICENSE).