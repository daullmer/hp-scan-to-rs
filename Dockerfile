# ── Stage 1: build ────────────────────────────────────────────────────────────
# rust:alpine uses musl libc, so the binary is fully statically linked.
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY . .

RUN cargo build --release \
    && strip target/release/hp-scan-to

# ── Stage 2: minimal runtime image ────────────────────────────────────────────
# scratch has no OS at all — just our binary and CA certs for HTTPS (Resend).
FROM scratch

# CA certificates are required for TLS connections to the Resend API.
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /build/target/release/hp-scan-to /hp-scan-to

ENTRYPOINT ["/hp-scan-to"]
# Default: expect the config mounted at /config/config.toml
# Override with: docker run ... --config /other/path.toml
CMD ["--config", "/config/config.toml"]
