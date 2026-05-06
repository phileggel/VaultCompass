# VaultCompass

[![coverage](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?label=coverage)](https://codecov.io/gh/phileggel/VaultCompass)
[![frontend](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?flag=frontend&label=frontend)](https://codecov.io/gh/phileggel/VaultCompass?flags=frontend)
[![backend](https://img.shields.io/codecov/c/github/phileggel/VaultCompass?flag=backend&label=backend)](https://codecov.io/gh/phileggel/VaultCompass?flags=backend)

Personal portfolio manager for desktop — track your financial assets across multiple accounts, record purchases, monitor positions, and follow your cost basis over time. All data is stored locally on your device.

Built with Tauri 2, React 19, and Rust.

## Quick Start

### Setup (Linux)

The steps below bootstrap a fresh machine end-to-end. Run them in order.

**1. System libraries (Tauri Linux build deps — requires sudo)**

```bash
sudo apt update && sudo apt install -y pkgconf libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev libssl-dev libayatana-appindicator3-dev libxdo-dev
```

(On Ubuntu 22.04 and earlier, replace `pkgconf` with `pkg-config`.)

**2. Rust toolchain (user-local)**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --component clippy rustfmt
. "$HOME/.cargo/env"
```

**3. `just` task runner + `sqlx-cli`**

```bash
cargo install just
cargo install sqlx-cli --no-default-features --features sqlite
```

**4. Node.js via nvm (user-local)**

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
. "$HOME/.nvm/nvm.sh"
nvm install --lts
nvm use --lts
```

**5. Clone and install project deps**

```bash
git clone <repository-url>
cd VaultCompass
npm install
```

**6. Verify**

```bash
just check-full
```

This runs lint + format + typecheck + tests for both backend and frontend. If it goes green, the environment is ready.

(Version requirements: `package.json` and `src-tauri/Cargo.toml`. macOS/Windows: install Rust via https://rustup.rs and Node via https://nodejs.org; system-library step is Linux-only.)

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
