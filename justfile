# Portfolio Manager — Command Runner
# Install just: https://github.com/casey/just

import "common.just"

# List all available commands
default:
    @just --list

# Install all dependencies
install:
    npm install

# Start the application with hot reload
dev *ARGS:
    ./scripts/start-app.sh {{ARGS}}

# Start the application using Wayland backend (use when window doesn't appear with just dev)
dev-wayland *ARGS:
    GDK_BACKEND=wayland ./scripts/start-app.sh {{ARGS}}

# Run frontend tests
test:
    npm test

# Run backend tests
test-rust:
    cd src-tauri && cargo test

# Run frontend tests with lcov coverage (output: coverage/lcov.info)
test-coverage:
    npm run test:coverage

# Run backend tests with coverage (output: tarpaulin-report.json); requires: cargo install cargo-tarpaulin
test-rust-coverage:
    cd src-tauri && cargo tarpaulin --out Json --output-dir ../coverage --tests --exclude-files "build.rs"

# Run all tests
test-all: test test-rust

# Collect logs for debugging
collect-logs:
    ./scripts/collect-logs.sh

# Take a screenshot of the app
screenshot:
    ./scripts/screenshot.sh

# Run linters only
lint:
    npm run lint
    cd src-tauri && cargo clippy -- -D warnings

# Clean build artifacts
clean:
    rm -rf dist src-tauri/target

# ⚠️  Destructive: resets database and restarts app in dev mode
reset-db:
    ./scripts/start-app.sh --reset-db
