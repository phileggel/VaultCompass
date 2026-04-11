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

# Run frontend tests
test:
    npm test

# Run backend tests
test-rust:
    cd src-tauri && cargo test

# Run all tests
test-all: test test-rust

# Generate TypeScript bindings from Rust
generate-types:
    ./scripts/generate-types.sh

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
