# Balance

<p align="center">
  <img src="icon-master.svg" width="120" alt="Balance Logo">
</p>

<h3 align="center">HOSxP & INVS Unified Drug Dashboard</h3>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri" alt="Tauri">
  <img src="https://img.shields.io/badge/Vue-3.5-42b883?logo=vue.js" alt="Vue.js">
  <img src="https://img.shields.io/badge/Rust-2024-000000?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/TypeScript-5.9-3178c6?logo=typescript" alt="TypeScript">
  <img src="https://img.shields.io/badge/Vite-6.4-646cff?logo=vite" alt="Vite">
  <img src="https://img.shields.io/badge/ECharts-5.6-aa354d?logo=apache" alt="ECharts">
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/suradet-ps/balance" alt="License">
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs Welcome">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey" alt="Platform">
</p>

---

**Balance** is a desktop application that unifies [HOSxP](https://hosxp.org/) (hospital pharmacy system) and [INVS](https://inventory.moph.go.th/) (Thai MOPH inventory system) into a single, side-by-side dashboard. Compare drug quantities from HOSxP with drug values from INVS to identify discrepancies and ensure accurate reporting.

## Features

- **Side-by-side comparison** — HOSxP (quantity) and INVS (value) displayed in a unified two-panel layout
- **Drug search per side** — Independent search using HOSxP `icode` or INVS `working_code`
- **Fiscal year selection** — Thai fiscal year (Oct–Sep) with automatic date range calculation
- **Interactive charts** — Bar + line trend charts powered by Apache ECharts
- **Connection management** — Tabbed drawer for MySQL (HOSxP) and SQL Server (INVS) configuration
- **Kraken theme** — Custom dark UI design system with IBM Plex Sans/Mono typography
- **Cross-platform** — Windows, macOS, and Linux via Tauri 2

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Tauri 2](https://tauri.app/) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| Build | [Vite 6](https://vitejs.dev/) |
| State | [Pinia](https://pinia.vuejs.org/) |
| Charts | [Apache ECharts](https://echarts.apache.org/) via [vue-echarts](https://github.com/ecomfe/vue-echarts) |
| Icons | [Lucide Vue](https://lucide.dev/) |
| Backend | [Rust](https://www.rust-lang.org/) (2024 edition) |
| MySQL | [sqlx](https://github.com/launchbadge/sqlx) (async, compile-time checked) |
| SQL Server | [tiberius](https://github.com/prisma/tiberius) (async, pure Rust) |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI](https://tauri.app/start/)

```bash
npm install -g @tauri-apps/cli
```

### Installation

```bash
# Clone
git clone https://github.com/suradet-ps/balance.git
cd balance

# Install dependencies
npm install
```

### Development

```bash
# Start dev server
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

### Generate Icons

```bash
# Requires icon-master.svg in project root
npm run gen-icons
```

## Project Structure

```
balance/
├── src/                        # Vue frontend
│   ├── components/
│   │   ├── AppHeader.vue       # Header with year selector & DB badges
│   │   ├── ConnectionSettings.vue  # MySQL/MSSQL connection drawer
│   │   ├── DrugSearchPanel.vue # Per-side drug search with autocomplete
│   │   ├── DrugTrendChart.vue  # ECharts bar+line trend chart
│   │   └── SummaryKpiBar.vue   # KPI summary bar
│   ├── composables/
│   │   ├── useHosxpData.ts     # HOSxP IPC wrappers
│   │   └── useInvsData.ts      # INVS IPC wrappers
│   ├── stores/
│   │   ├── dashboard.ts        # Drug data & selection state
│   │   └── dbConfig.ts         # Database connection state
│   ├── styles/
│   │   └── kraken-theme.css    # Kraken Purple CSS design system
│   ├── utils/
│   │   └── dateUtils.ts        # Fiscal year helpers, formatters
│   └── App.vue                 # Main layout
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── hosxp/              # HOSxP MySQL module
│   │   │   ├── db.rs           # Connection pool (OnceLock/RwLock)
│   │   │   └── commands.rs     # Tauri commands
│   │   ├── invs/               # INVS SQL Server module
│   │   │   ├── db.rs           # Tiberius client management
│   │   │   └── commands.rs     # Tauri commands
│   │   └── lib.rs              # Command registration
│   ├── capabilities/
│   │   └── default.json        # Tauri v2 capability permissions
│   └── tauri.conf.json         # App config (1440×900, bundle settings)
├── scripts/
│   └── gen-icons.cjs           # Icon generator from SVG source
├── DESIGN.md                   # Kraken design system reference
└── icon-master.svg             # Source logo (balance/scale)
```

## Configuration

### HOSxP (MySQL)

| Field | Example |
|-------|---------|
| Host | `127.0.0.1` |
| Port | `3306` |
| User | `root` |
| Password | `••••` |
| Database | `hosxp` |

### INVS (SQL Server)

| Field | Example |
|-------|---------|
| Host | `10.0.0.5` |
| Port | `1433` |
| User | `sa` |
| Password | `••••` |
| Database | `INVS` |

## Database Schema

### HOSxP

| Table | Key Column | Description |
|-------|------------|-------------|
| `opitemrece` | `icode` | Drug dispensing records |
| `drugitems` | `icode` | Drug master data |

### INVS

| Table | Key Column | Description |
|-------|------------|-------------|
| `MS_IVO` | `working_code` | Receiving header |
| `MS_IVO_C` | `working_code` | Receiving detail |
| `DRUG_GN` | `working_code` | Drug master data |

## Roadmap

- [ ] Drug name mapping between `icode` ↔ `working_code`
- [ ] Export to Excel / PDF
- [ ] Multi-year trend comparison
- [ ] Alert on significant discrepancies
- [ ] Dark / light theme toggle

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

[MIT](LICENSE)

## Acknowledgments

- [Tauri](https://tauri.app/) — Build smaller, faster, more secure desktop applications
- [HOSxP](https://hosxp.org/) — Open-source hospital information system
- [MOPH INVS](https://inventory.moph.go.th/) — Thai Ministry of Public Health inventory system
- [Apache ECharts](https://echarts.apache.org/) — Powerful charting and visualization library
- [IBM Plex](https://github.com/IBM/plex) — Typography family used in Kraken theme
