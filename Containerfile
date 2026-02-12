# Multi-stage build: builder + runtime
# Compatible with both Podman and Docker
FROM rust:1.88-slim-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY assets/ assets/

RUN cargo build --release && strip target/release/s5

# Runtime stage
FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="s5" \
      org.opencontainers.image.description="Lightweight SSH server with SOCKS5 proxy, shell emulation, and ACL" \
      org.opencontainers.image.source="https://github.com/galti3r/s5" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd -r s5 && useradd -r -g s5 -d /etc/s5 -s /usr/sbin/nologin s5 && \
    mkdir -p /etc/s5 /var/log/s5 && \
    chown -R s5:s5 /etc/s5 /var/log/s5

COPY --from=builder /build/target/release/s5 /usr/local/bin/s5

USER s5
WORKDIR /etc/s5

# SSH, SOCKS5, metrics/API
EXPOSE 2222 1080 9090 9091

VOLUME ["/etc/s5"]

STOPSIGNAL SIGTERM

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD ["s5", "health-check", "--addr", "127.0.0.1:2222", "--timeout", "3"]

ENTRYPOINT ["s5"]
CMD ["--config", "/etc/s5/config.toml"]
