# Builder stage
FROM rust:1.68-buster AS builder

WORKDIR /app

COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN cargo build --release

# Runtime stage
FROM debian:buster-slim

WORKDIR /app

COPY --from=builder /app/target/release/mpc-manager .

CMD ["./mpc-manager"]
