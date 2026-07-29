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

- Side-by-side comparison of HOSxP (quantity) and INVS (value)
- Independent drug search using HOSxP `icode` or INVS `working_code`
- Thai fiscal year selection with automatic date range calculation
- Interactive bar + line trend charts powered by Apache ECharts
- Cross-platform support: Windows, macOS, and Linux

## Getting Started

### Prerequisites

- [Bun](https://bun.sh/) >= 1.0
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI](https://tauri.app/start/)

### Installation

```bash
git clone https://github.com/suradet-ps/balance.git
cd balance
bun install
```

### Development

```bash
bun run tauri dev
```

### Production Build

```bash
bun run tauri build
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Tauri 2](https://tauri.app/) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| Build | [Vite 6](https://vitejs.dev/) |
| Backend | [Rust](https://www.rust-lang.org/) |

## Roadmap

- [ ] Drug name mapping between `icode` <-> `working_code`
- [ ] Export to Excel / PDF
- [ ] Multi-year trend comparison
- [ ] Alert on significant discrepancies

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
