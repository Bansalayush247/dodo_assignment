FROM rust:1.78 as builder
WORKDIR /usr/src/app

# Copy manifest only, fetch dependencies inside container to produce a compatible Cargo.lock
COPY Cargo.toml ./
RUN mkdir -p src
RUN echo "fn main() {println!(\"dummy\");}" > src/main.rs
RUN cargo fetch

# Now copy source and build release
COPY src ./src
RUN mkdir -p /usr/src/app/target
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libpq5 postgresql-client && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/app/target/release/transaction-service /app/transaction-service
COPY ./migrations /app/migrations
COPY ./entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh
ENV RUST_LOG=info
EXPOSE 3000
CMD ["/app/entrypoint.sh"]
