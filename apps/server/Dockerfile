FROM rust:1.93-slim AS builder

WORKDIR /app

# Install build dependencies for OpenSSL and curl for swagger-ui download
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

# Fly builds from the repository root so the server and CLI share one lockfile.
COPY Cargo.toml Cargo.lock ./
COPY apps/server/Cargo.toml ./apps/server/Cargo.toml
COPY apps/cli/Cargo.toml ./apps/cli/Cargo.toml

# Create minimal workspace targets to cache dependencies.
RUN mkdir -p apps/server/src apps/cli/src \
    && echo "fn main() {}" > apps/server/src/main.rs \
    && echo "fn main() {}" > apps/cli/src/main.rs
RUN cargo build --release -p server

# Copy actual source and rebuild
COPY apps/server/src ./apps/server/src
RUN touch apps/server/src/main.rs && cargo build --release -p server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/server

ENV PORT=3000
EXPOSE 3000

CMD ["server"]
