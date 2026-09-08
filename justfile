# List available commands
default:
    @just --list

alias ci := check

# Run all dev servers (MongoDB + extension + server)
dev: db
    #!/usr/bin/env bash
    trap 'kill 0' EXIT SIGINT SIGTERM
    (cd apps/extension && bun run dev) &
    (cd apps/server && cargo watch -x run) &
    wait

# Stop all dev processes
stop:
    -pkill -f "target/debug/server"
    -pkill -f "wxt"
    -lsof -ti:3000 | xargs -r kill -9
    -lsof -ti:5173 | xargs -r kill -9

# Build everything
build:
    cd apps/extension && bun run build
    cargo build --workspace --release
    rm -rf /mnt/c/order-wizard-ext
    cp -r apps/extension/.output/chrome-mv3 /mnt/c/order-wizard-ext

# Run all checks (typecheck + lint + test + clippy)
check:
    cd apps/extension && bun run typecheck
    cd apps/extension && bun run lint
    cd apps/extension && bun run test
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Run extension unit tests
test:
    cd apps/extension && bun run test

# TypeScript type check
typecheck:
    cd apps/extension && bun run typecheck

# Lint extension code
lint:
    cd apps/extension && bun run lint

# Lint and auto-fix
lint-fix:
    cd apps/extension && bun run lint:fix

# Format code
format:
    cd apps/extension && bun run format
    cargo fmt --all

# Start MongoDB
db:
    docker-compose up -d mongodb

# Stop MongoDB
db-stop:
    docker-compose down

# Clean all build artifacts
clean:
    cd apps/extension && bun run clean
    cargo clean

# Install dependencies
install:
    cd apps/extension && bun install
    cargo fetch

# Bump the synchronized product version and refresh Cargo.lock
bump version:
    bun scripts/set-version.ts "{{version}}"

# Setup git hooks
setup:
    git config core.hooksPath .githooks
    @echo "Git hooks configured"
