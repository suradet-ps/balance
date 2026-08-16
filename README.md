# Balance

<p align="center">
  <img src="icon-master.svg" width="120" alt="Balance Logo">
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/suradet-ps/balance" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs Welcome">
</p>

---

**Balance** is a desktop application that unifies [HOSxP](https://hosxp.org/) and [INVS](https://inventory.moph.go.th/) into a single, side-by-side dashboard. Compare drug quantities from HOSxP with drug values from INVS to identify discrepancies and ensure accurate reporting.

## Features

- Side-by-side comparison of HOSxP (quantity) and INVS (value), on the same
  Thai fiscal-year axis (FY N = 1 Oct N−1 … 30 Sep N)
- Drug mapping between HOSxP `icode` and INVS `working_code` (local SQLite
  store): auto-suggest candidates with Thai-aware name scoring, batch
  auto-match, bulk CSV import, match status on both panels
- Year-first reconciliation for mapped drugs: unit price, coverage ratio,
  the cumulative stock curve and rule-based discrepancy flags
- Independent drug search using HOSxP `icode` or INVS `working_code`
- Interactive bar + line trend charts rendered on canvas (custom Rust renderer)
- Encrypted connection settings stored in the OS keyring
- Cross-platform support: Windows, macOS, and Linux

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable) with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/) — `cargo install trunk --locked`
- [Tauri CLI](https://tauri.app/start/) — `cargo install tauri-cli --locked`

### Installation

```bash
git clone https://github.com/suradet-ps/balance.git
cd balance
rustup target add wasm32-unknown-unknown
```

### Development

```bash
cargo tauri dev
```

(`trunk serve` runs automatically via `beforeDevCommand` in `src-tauri/tauri.conf.json`.)

### Production Build

```bash
cargo tauri build
```

### Frontend Only

```bash
trunk serve --config src/Trunk.toml
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri 2](https://tauri.app/) |
| Frontend | [Leptos 0.8](https://leptos.dev/) (Rust CSR, compiled to wasm32) |
| Frontend build | [Trunk](https://trunkrs.dev/) |
| Backend | Rust — [sqlx](https://github.com/launchbadge/sqlx) (MySQL/HOSxP), [tiberius](https://github.com/prisma/tiberius) (SQL Server/INVS) |
| Settings | [encryptman-keyring](https://github.com/suradet-ps/encryptman-keyring) (encrypted, OS keychain-backed) |

## Documentation

- [ROADMAP.md](docs/ROADMAP.md) — where the product is going, phase by phase
- [DESIGN.md](docs/DESIGN.md) — design system, tokens, and UI conventions
- [architecture.md](docs/architecture.md) — modules, IPC surface, data flow
- [database.md](docs/database.md) — local store schema and migrations
- [mapping.md](docs/mapping.md) — matching heuristics, scoring, import format
- [reconciliation.md](docs/reconciliation.md) — discrepancy rules and worked examples

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
