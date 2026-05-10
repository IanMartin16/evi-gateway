FROM rust:1.88 as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/evi-gate /app/evi-gate

ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["./evi-gate"]