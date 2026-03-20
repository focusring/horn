FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/horn /usr/local/bin/horn

ENTRYPOINT ["horn"]
CMD ["--help"]
