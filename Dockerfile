# Build Stage
FROM rust:1.78-alpine AS builder
RUN apk add --no-cache musl-dev git
WORKDIR /app
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/crates ./crates
# Build release binary
RUN cargo build --release -p kcode-cli
RUN strip target/release/kcode

# Runtime Stage
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/kcode /usr/local/bin/kcode

# Runtime Config
ENV RUST_LOG=info
EXPOSE 3000

# Volumes for persistence
VOLUME ["/root/.kcode"]

ENTRYPOINT ["kcode", "bridge"]
