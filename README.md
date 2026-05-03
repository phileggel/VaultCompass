# VaultCompass

[![coverage](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?label=coverage)](https://codecov.io/gh/phileggel/VaultCompass)
[![frontend](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?flag=frontend&label=frontend)](https://codecov.io/gh/phileggel/VaultCompass?flags=frontend)
[![backend](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?flag=backend&label=backend)](https://codecov.io/gh/phileggel/VaultCompass?flags=backend)

Personal portfolio manager for desktop — track your financial assets across multiple accounts, record purchases, monitor positions, and follow your cost basis over time. All data is stored locally on your device.

Built with Tauri 2, React 19, and Rust.

## Quick Start

### Prerequisites

- **Node.js**: https://nodejs.org
- **Rust**: https://rustup.rs

(See `package.json` and `src-tauri/Cargo.toml` for version requirements)

### Setup

```bash
git clone <repository-url>
cd VaultCompass
npm install
```

### Development

```bash
./scripts/start-app.sh
```

App opens automatically with hot reload.

### Build

```bash
./scripts/build.sh
```

Output: `src-tauri/target/release/bundle/`

## Code Quality

```bash
python3 scripts/check.py   # Full check (tests, lint, build, types)
npm run test               # Frontend tests only
cd src-tauri && cargo test # Backend tests only
```

## Documentation

- [Architecture](ARCHITECTURE.md) — system design, bounded contexts, data flow
- [Frontend Rules](docs/frontend-rules.md) — React/TS conventions
- [Backend Rules](docs/backend-rules.md) — Rust/DDD conventions

## License

MIT License — Copyright (c) 2026 Philippe Eggel. See [LICENSE](LICENSE) for details.
