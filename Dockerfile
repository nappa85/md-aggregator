FROM rust:bullseye AS builder

WORKDIR /app

COPY src /app/src
COPY * /app/

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/md-aggregator /usr/local/bin/

RUN chmod +x /usr/local/bin/md-aggregator

CMD md-aggregator
