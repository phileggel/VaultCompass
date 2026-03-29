# Portfolio Manager — Command Runner
# Install just: https://github.com/casey/just

# List all available commands
default:
    @just --list

# Install all dependencies
install:
    npm install

# Start the application with hot reload
dev *ARGS:
    ./scripts/start-app.sh {{ARGS}}

# Run full quality check (tests + linters)
check:
    ./scripts/check.sh

# Run quality check with verbose output
check-verbose:
    ./scripts/check.sh --verbose

# Run fast quality check (lint/format only)
check-fast:
    ./scripts/check.sh --fast

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

# Release new version (interactive)
release *ARGS:
    python3 scripts/release.py {{ARGS}}

# Collect logs for debugging
collect-logs:
    ./scripts/collect-logs.sh

# Run pending database migrations
# Prerequisites: sqlx must be on $PATH and DATABASE_URL must be set
migrate:
    cd src-tauri && sqlx migrate run

# Take a screenshot of the app
screenshot:
    ./scripts/screenshot.sh

# Run linters only
lint:
    npm run lint
    cd src-tauri && cargo clippy -- -D warnings

# Auto-fix formatting and linting
format:
    cd src-tauri && cargo fmt
    cd src-tauri && cargo clippy --fix --allow-dirty
    npm run format:fix

# Clean build artifacts
clean:
    rm -rf dist src-tauri/target

# ⚠️  Destructive: deletes local database and recreates schema
# Prerequisites: sqlx must be on $PATH
clean-db:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf src-tauri/.local/*
    cd src-tauri && sqlx database setup

# ⚠️  Destructive: resets database and restarts app in dev mode
reset-db:
    ./scripts/start-app.sh --reset-db

# ⚠️  Destructive: removes stale remote-tracking branches (force delete)
clean-branches:
    git fetch --prune
    git branch -vv | grep ': gone]' | awk '{print $1}' | xargs git branch -D || true
