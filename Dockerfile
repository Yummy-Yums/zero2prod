# Builder stage
FROM rust:1.87.0 AS builder

WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

# Runtime stage
FROM rust:1.87.0 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configurations configurations
ENV APP_ENVIRONMENT production

ENTRYPOINT ["./zero2prod"]