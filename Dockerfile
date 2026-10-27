FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p pm-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/pm-server /usr/local/bin/pm-server
ENV DATABASE_URL=postgres://quant:quant@postgres:5432/quantdb
CMD ["pm-server"]