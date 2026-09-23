FROM rust:1.98-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

ENV SQLX_OFFLINE=true

COPY . .

RUN cargo build --release -p tilcayo-api

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/tilcayo-api /app/tilcayo-api

EXPOSE 3000

CMD ["./tilcayo-api"]
