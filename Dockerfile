# Stage 1: Build
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Build the actual application
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false -m -d /data pebble

COPY --from=builder /app/target/release/pebble /usr/local/bin/pebble

# Create data directory
RUN mkdir -p /data && chown pebble:pebble /data

USER pebble
WORKDIR /data

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD pebble doctor || exit 1

ENTRYPOINT ["pebble"]
CMD ["deploy", "--host", "0.0.0.0", "--port", "8080"]
