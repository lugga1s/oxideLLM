FROM rust:1.87-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY src/ src/

RUN cargo build --release

FROM alpine:3.21

COPY --from=builder /app/target/release/oxidellm-mock /usr/local/bin/oxidellm-mock

EXPOSE 9000
CMD ["oxidellm-mock"]
