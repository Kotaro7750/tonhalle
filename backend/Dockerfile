# syntax=docker/dockerfile:1

FROM rust:1.99-bookworm AS builder
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY openapi_generated ./openapi_generated
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release \
    && cp target/release/tonhalle-backend /tonhalle-backend

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder --chown=nonroot:nonroot /tonhalle-backend /tonhalle-backend

ENV TONHALLE_LISTEN_ADDRESS=0.0.0.0 \
    TONHALLE_LISTEN_PORT=3000
EXPOSE 3000
ENTRYPOINT ["/tonhalle-backend"]
