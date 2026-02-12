# s5 - SSH + SOCKS5 Proxy Server

[![CI](https://github.com/galti3r/s5/actions/workflows/ci.yml/badge.svg)](https://github.com/galti3r/s5/actions/workflows/ci.yml)

Lightweight SSH server in Rust serving as a SOCKS5 proxy with shell emulation, multi-user auth, and ACL.

## Features

### Core
- **SSH Server**: Password, public key, and certificate authentication
- **Dynamic forwarding** (`ssh -D`): SOCKS5 proxy via SSH
- **Local port forwarding** (`ssh -L`): TCP forwarding via SSH
- **Standalone SOCKS5**: Dedicated SOCKS5 listener with username/password auth
- **TLS SOCKS5**: Optional TLS encryption on standalone SOCKS5 listener
- **Shell emulation**: Virtual filesystem, basic commands (ls, cd, cat, whoami, etc.)
- **Container-ready**: Podman/Docker support with multi-stage builds

### Authentication & Authorization
- **Argon2id passwords**: Secure password hashing
- **SSH public keys**: Ed25519, RSA key-based auth
- **SSH certificates**: Trusted CA-based certificate auth
- **TOTP 2FA**: Google Authenticator / Authy compatible
- **Auth method chaining**: Require multiple auth methods (e.g. pubkey + password)
- **Per-user ACL**: Allow/deny rules for destination hosts/ports (CIDR, wildcards)
- **Global ACL**: Server-wide rules inherited by all users
- **User groups**: Hierarchical config inheritance (global -> group -> user)
- **User roles**: `user` (default) and `admin` roles with different permissions

### Security
- **Auto-ban**: Fail2ban-style IP banning after auth failures
- **IP reputation scoring**: Dynamic scoring with exponential decay
- **IP whitelist**: Global and per-user source IP restrictions
- **GeoIP filtering**: Country-based allow/deny with MaxMind GeoLite2
- **Anti-SSRF guard**: Blocks RFC 1918, link-local, loopback, multicast
- **Pre-auth rate limiting**: Per-IP connection rate limit
- **SOCKS5 password zeroization**: Secure memory handling
- **SFTP/SCP/reverse forwarding blocked**: Prevents unintended access

### User Management
- **User groups**: Shared config for bandwidth, ACL, quotas, permissions
- **Quotas**: Daily/monthly/hourly bandwidth and connection limits with runtime enforcement
- **Time-based access**: Restrict login hours and days (per timezone)
- **Account expiration**: ISO 8601 timestamp, auto-deny after expiry
- **Bandwidth throttling**: Per-connection and per-user aggregate caps
- **Idle warning**: Notify users before idle timeout disconnect
- **Shell permissions**: Fine-grained control over shell commands per user/group

### Shell
- **MOTD (Message of the Day)**: Template engine with 13 variables, ANSI colors
- **Extended commands**: `show status`, `show connections`, `show bandwidth`, `show acl`, `show fingerprint`, `show history`
- **Network tools**: `test host:port`, `ping host`, `resolve hostname`
- **Bookmarks**: Save/list/delete host:port bookmarks
- **Aliases**: Create command shortcuts (persistent per-user config)
- **Command history**: Arrow up/down to recall previous commands (in-session)
- **Left/right arrows**: Cursor movement within the command line
- **Tab completion**: Autocomplete for commands and arguments
- **ANSI colors**: Configurable per-user/group color support

### Networking
- **Connection pooling**: LIFO per-host TCP pool with idle timeout
- **Smart retry**: Exponential backoff on connection failures (capped at 10s)
- **DNS cache**: Configurable TTL (native, custom, or disabled)
- **Upstream proxy**: Global or per-user SOCKS5 upstream proxy
- **PROXY protocol**: HAProxy v1/v2 support

### Observability
- **Prometheus metrics**: `/metrics` endpoint with cardinality protection (includes quota usage metrics)
- **Structured audit logging**: JSON audit log with rotation
- **Connection flow logs**: Detailed per-step timing logs
- **Webhooks**: HTTP webhooks with retry and HMAC signatures
- **Alerting**: Rules on bandwidth, connections, and auth failures
- **Real-time dashboard**: Web UI with SSE live updates, quota usage panel, per-user bandwidth/connection tracking

### Management
- **REST API**: Bearer token auth, user/connection/ban management
- **Broadcast messages**: Send messages to all connected users
- **Kick users**: Disconnect specific users via API
- **Hot reload**: Reload config without restart
- **Maintenance mode**: Toggle maintenance with custom message
- **Maintenance windows**: Scheduled maintenance with cron-like syntax
- **SSH config generator**: API endpoint for `.ssh/config` snippets
- **SSE ticket auth**: HMAC-based short-lived tickets for browser SSE

## Quick Start

### Fastest way (zero config)

```bash
# Auto-generated password
cargo run -- quick-start

# With a chosen password
cargo run -- quick-start --password demo

# With SOCKS5 standalone listener
cargo run -- quick-start --password demo --socks5-listen 0.0.0.0:1080

# Save the generated config for later use
cargo run -- quick-start --password demo --save-config config.toml
```

### Generate a config file

```bash
# Interactive (prompts for password)
cargo run -- init

# Non-interactive
cargo run -- init --username alice --password secret --output config.toml
```

### Manual setup

```bash
make build
cp config.example.toml config.toml
# Generate a password hash
cargo run -- hash-password --password "your-password"
# Edit config.toml with the generated hash
make run
```

### Connect

```bash
# SSH shell
ssh -o StrictHostKeyChecking=no alice@localhost -p 2222

# Dynamic forwarding (SOCKS5 via SSH)
ssh -D 1080 -N alice@localhost -p 2222
curl --socks5 localhost:1080 http://example.com

# Local port forwarding
ssh -L 8080:httpbin.org:80 -N alice@localhost -p 2222
curl http://localhost:8080/ip

# Standalone SOCKS5
curl --socks5 alice:password@localhost:1080 http://example.com
```

## Configuration

See [config.example.toml](config.example.toml) for a complete configuration reference.

### Key sections

| Section | Required | Description |
|---------|:--------:|-------------|
| `[server]` | Yes | SSH/SOCKS5 listen addresses, host key, TLS, DNS cache, retry |
| `[[users]]` | Yes | User accounts (at least one required) |
| `[shell]` | No | Hostname, prompt, colors, autocomplete |
| `[limits]` | No | Connection limits, timeouts, idle warning |
| `[security]` | No | Auto-ban, IP guard, TOTP, IP reputation, CA keys |
| `[logging]` | No | Log level, format, audit log, flow logs |
| `[metrics]` | No | Prometheus endpoint |
| `[api]` | No | REST API listen address, bearer token |
| `[geoip]` | No | GeoIP country filtering |
| `[motd]` | No | Message of the Day template and colors |
| `[acl]` | No | Global ACL rules (inherited by all users) |
| `[[groups]]` | No | User groups for config inheritance |
| `[[webhooks]]` | No | HTTP webhooks with retry |
| `[alerting]` | No | Alert rules on bandwidth/connections/auth |
| `[[maintenance_windows]]` | No | Scheduled maintenance windows |
| `[connection_pool]` | No | TCP connection pooling |
| `[upstream_proxy]` | No | Upstream SOCKS5 proxy |

### User groups

Groups allow shared configuration inheritance. Users reference groups by name:

```toml
[[groups]]
name = "developers"
max_bandwidth_kbps = 10240  # 10 Mbps
allow_forwarding = true
allow_shell = true

[groups.acl]
default_policy = "allow"
deny = ["10.0.0.0/8:*"]

[[users]]
username = "alice"
password_hash = "$argon2id$..."
group = "developers"  # Inherits group settings
```

Inheritance order: **user > group > global defaults**. User fields override group, group overrides global.

### MOTD templates

The MOTD supports variable substitution and ANSI colors:

```toml
[motd]
enabled = true
colors = true
template = """
Welcome {user}! Connected from {source_ip}
Role: {role} | Group: {group}
Bandwidth: {bandwidth_used}/{bandwidth_limit}
Server uptime: {uptime}
"""
```

Available variables: `{user}`, `{auth_method}`, `{source_ip}`, `{connections}`, `{acl_policy}`, `{denied}`, `{expires_at}`, `{bandwidth_used}`, `{bandwidth_limit}`, `{last_login}`, `{uptime}`, `{version}`, `{group}`, `{role}`.

Per-user/group MOTD overrides the global one.

### Shell commands

| Command | Description | Permission |
|---------|-------------|------------|
| `ls` / `cd` / `pwd` / `cat` | Virtual filesystem navigation | Always |
| `whoami` / `id` / `hostname` | Identity commands | Always |
| `echo` / `env` / `uname` | Basic utilities | Always |
| `help` / `exit` / `clear` | Shell control | Always |
| `show connections` | Live active proxy connections count | `show_connections` |
| `show bandwidth` | Live bandwidth usage (daily/monthly/hourly/rate) | `show_bandwidth` |
| `show acl` | ACL rules for current user | `show_acl` |
| `show status` | Session info with live connection count | `show_status` |
| `show history` | Connection summary (today/this month) | `show_history` |
| `show fingerprint` | SSH key fingerprint (from auth) | `show_fingerprint` |
| `test host:port` | TCP connectivity test | `test_command` |
| `ping host` | Simulated ICMP ping | `ping_command` |
| `resolve hostname` | DNS lookup | `resolve_command` |
| `bookmark add/list/del` | Manage host:port bookmarks | `bookmark_command` |
| `alias add/list/del` | Command aliases | `alias_command` |

Permissions are configured via `shell_permissions` at user, group, or global level.

### Quotas, rate limiting, and time-based access

```toml
[[users]]
username = "contractor"
password_hash = "$argon2id$..."
group = "external"
expires_at = "2026-06-30T23:59:59Z"
max_connections = 5                   # Max concurrent connections

[users.rate_limits]
connections_per_second = 2
connections_per_minute = 30
connections_per_hour = 200

[users.quotas]
daily_bandwidth_bytes = 1073741824    # 1 GB/day
monthly_connection_limit = 1000
bandwidth_per_hour_bytes = 536870912  # 512 MB/hour rolling

[users.time_access]
access_hours = "08:00-18:00"
access_days = ["mon", "tue", "wed", "thu", "fri"]
timezone = "Europe/Paris"
```

Server-level rate limiting in `[limits]`:

```toml
[limits]
max_bandwidth_mbps = 100              # Server-wide bandwidth cap
max_new_connections_per_second = 50   # Server-level rate limit
max_new_connections_per_minute = 500
```

### Alerting

```toml
[alerting]
enabled = true

[[alerting.rules]]
name = "high_bandwidth"
condition = "bandwidth_exceeded"
threshold = 1073741824       # 1 GB
window_secs = 3600           # per hour
webhook_url = "https://hooks.example.com/alert"

[[alerting.rules]]
name = "brute_force"
condition = "auth_failures"
threshold = 50
window_secs = 300
users = []                   # all users
```

### Connection pooling and smart retry

```toml
[connection_pool]
enabled = true
max_idle_per_host = 10
idle_timeout_secs = 60

[server]
connect_retry = 3            # retry 3 times on failure
connect_retry_delay_ms = 500 # initial delay 500ms, doubles each retry, max 10s
```

## Environment Variables

s5 supports full configuration via environment variables for Docker/Kubernetes deployments. See `docker-compose.yml` for complete examples.

| Variable | Description | Default |
|----------|-------------|---------|
| `S5_CONFIG` | Path to config file | -- |
| `S5_SSH_LISTEN` | SSH listen address | `0.0.0.0:2222` |
| `S5_SOCKS5_LISTEN` | Standalone SOCKS5 listen address | (disabled) |
| `S5_HOST_KEY_PATH` | SSH host key path | `host_key` |
| `S5_LOG_LEVEL` | Log level (trace/debug/info/warn/error) | `info` |
| `S5_LOG_FORMAT` | Log format (pretty/json) | `pretty` |
| `S5_SHUTDOWN_TIMEOUT` | Graceful shutdown timeout (seconds) | `30` |
| `S5_DNS_CACHE_TTL` | DNS cache TTL (-1=native, 0=off, N=seconds) | `-1` |
| `S5_METRICS_ENABLED` | Enable Prometheus metrics | `false` |
| `S5_API_ENABLED` | Enable management API | `false` |
| `S5_API_TOKEN` | API bearer token | -- |
| `S5_BAN_ENABLED` | Enable auto-ban | `true` |

**Multi-user mode** (indexed):

| Variable | Description |
|----------|-------------|
| `S5_USER_<N>_USERNAME` | Username (N = 0, 1, 2, ...) |
| `S5_USER_<N>_PASSWORD_HASH` | Argon2id password hash |
| `S5_USER_<N>_ALLOW_FORWARDING` | Allow forwarding (true/false) |
| `S5_USER_<N>_ALLOW_SHELL` | Allow shell access (true/false) |
| `S5_USER_<N>_TOTP_ENABLED` | Enable TOTP 2FA |
| `S5_USER_<N>_TOTP_SECRET` | Base32-encoded TOTP secret |
| `S5_USER_<N>_MAX_BANDWIDTH_KBPS` | Per-connection bandwidth cap |
| `S5_USER_<N>_MAX_AGGREGATE_BANDWIDTH_KBPS` | Total bandwidth cap |

**Docker/K8s secrets** (`_FILE` convention): append `_FILE` to read from a file path instead of the env var value directly.

| Variable | Supports `_FILE` |
|----------|:-:|
| `S5_PASSWORD_HASH` / `S5_USER_<N>_PASSWORD_HASH` | Yes |
| `S5_API_TOKEN` | Yes |
| `S5_TOTP_SECRET` / `S5_USER_<N>_TOTP_SECRET` | Yes |

## Health Check

```bash
s5 health-check --addr 127.0.0.1:2222 --timeout 5
```

Returns exit code 0 if the SSH listener is accepting connections, 1 otherwise. Used in Docker healthchecks and monitoring.

## Container

### Build

```bash
make docker-build
```

### Run

```bash
make docker-run
```

### Compose

```bash
make compose-up
```

### Multi-Architecture Builds

Build for both `linux/amd64` and `linux/arm64` (Apple Silicon, AWS Graviton, etc.).

#### Cross-compilation (fast, recommended)

```bash
make docker-build-cross
```

#### QEMU emulation (slower, simpler)

```bash
make docker-build-multiarch
```

#### Push to registry

```bash
PUSH=true IMAGE_NAME=ghcr.io/user/s5 IMAGE_TAG=v1.0.0 make docker-build-cross
```

#### CI/CD

Multi-arch images are automatically built and pushed to GHCR on tag pushes via CI.

CI pipelines are provided for three platforms:

| Platform | Config | Pipeline |
|----------|--------|----------|
| **GitHub Actions** | `.github/workflows/ci.yml` | lint + test + coverage + security + SARIF + Docker |
| **GitLab CI** | `.gitlab-ci.yml` | lint + test + coverage + security + Docker + Pages |
| **Forgejo Actions** | `.forgejo/workflows/ci.yml` | lint + test + security + build + Docker |

All pipelines include:
- `cargo fmt --check` + `cargo clippy -D warnings` + `hadolint`
- `cargo test --all-targets`
- `cargo audit` + `cargo deny check`
- Multi-arch Docker build + push to registry
- Container vulnerability scanning (Trivy)

## Testing

```bash
# All tests (unit + E2E)
make test

# Unit tests only
make test-unit

# E2E tests only
make test-e2e

# E2E tests including ignored (IPv6, perf)
make test-e2e-all

# Performance benchmarks (throughput, latency, concurrency)
make test-perf

# Security scan (clippy + cargo-audit + cargo-deny)
make security-scan

# Full suite (tests + security)
make test-all

# E2E in Podman containers (isolated network)
make test-e2e-podman
```

### Test coverage

| Category | Tests | Description |
|----------|------:|-------------|
| Unit (lib) | 93 | Config, ACL, auth, shell, SOCKS5 protocol, proxy, security, show commands, session |
| Unit (standalone) | 202 | Auth service, CLI, config validation, connectors, geoip, webhooks, pre-auth, source IP, audit, protocol fuzz, SSH keys, pubkey, DNS cache, metrics, SSE ticket, IP rate limiter, SOCKS5 TLS, webhook retry, API users, SOCKS5 timeout, proxy details |
| Unit (new features) | 48 | Connection pool, smart retry, shell commands, IP reputation, MOTD, time access, alerting, groups, roles |
| Unit (quota) | 22 | QuotaTracker, rolling windows, rate limiting, throttle, daily/monthly quotas, reset, cleanup |
| E2E - Auth | 5 | Password success/failure, unknown user, shell prompt, retry |
| E2E - Shell | 16 | exec commands, dangerous commands blocked, interactive shell |
| E2E - Shell commands | 18 | show status/bandwidth/connections, help, echo, alias, unknown commands |
| E2E - ACL | 12 | FQDN, subnet/CIDR, combined rules, wildcard, IPv6 |
| E2E - Forwarding | 3 | Local forward, large data, denied user |
| E2E - Rejection | 8 | SFTP, reverse forward, bash/sh/nc/rsync blocked |
| E2E - SOCKS5 | 11 | Auth, forwarding, concurrency, anti-SSRF, standalone |
| E2E - API | 12 | Health, users, connections, bans, maintenance, auth, dashboard, SSE |
| E2E - Quota API | 10 | List quotas, user detail, reset, auth, enforcement, rate limiting, Prometheus metrics |
| E2E - Reload | 3 | Valid/invalid config reload, auth required |
| E2E - Status | 3 | Health, Prometheus metrics, maintenance mode |
| E2E - Autoban | 3 | Trigger after threshold, reject banned IP, no false positive |
| E2E - Audit | 2 | Auth success/failure audit events |
| E2E - Performance | 5 | Throughput, latency, concurrent connections |
| **Total** | **~738** | |

## API

The HTTP API is protected by Bearer token (configured in `api.token`).

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check (no auth) |
| GET | `/api/status` | Server status (uptime, connections, users) |
| GET | `/api/users` | List users (no password hashes) |
| GET | `/api/users?details=true` | Extended user info with connection stats |
| GET | `/api/connections` | Active connections per user |
| GET | `/api/bans` | Banned IPs |
| DELETE | `/api/bans/:ip` | Unban an IP |
| POST | `/api/maintenance` | Toggle maintenance mode |
| POST | `/api/reload` | Hot-reload config from disk |
| POST | `/api/sse-ticket` | Generate short-lived HMAC ticket for SSE auth |
| POST | `/api/kick/:username` | Disconnect all sessions for a user |
| POST | `/api/broadcast` | Broadcast message to all connected users |
| GET | `/api/quotas` | List all users' quota usage |
| GET | `/api/quotas/:username` | Quota usage detail for a specific user |
| POST | `/api/quotas/:username/reset` | Reset quota counters for a user |
| GET | `/api/ssh-config` | Generate SSH config snippet (`?user=&host=`) |
| GET | `/api/events` | SSE stream (token via `?token=` or `?ticket=`) |
| GET | `/dashboard` | Web dashboard (no auth required) |

## Security

### Hardening features

- Argon2id password hashing, SSH public key auth, SSH certificate auth
- TOTP 2FA (Google Authenticator / Authy compatible)
- Auth method chaining (e.g. require pubkey + password)
- Per-user ACL (CIDR, FQDN wildcard, port ranges)
- Auto-ban (fail2ban-style), global/per-user IP whitelist
- IP reputation scoring (dynamic, exponential decay)
- Per-IP pre-auth rate limiting (`max_new_connections_per_ip_per_minute`)
- Multi-window rate limiting (per-second/minute/hour, per-user and server-level)
- GeoIP country filtering
- Anti-SSRF IP guard (blocks RFC 1918, link-local, loopback, multicast)
- Webhook SSRF protection (private IP blocking + DNS rebinding guard)
- SOCKS5 password zeroization (secure memory handling)
- Pre-auth ban check (reject before auth attempt)
- Metrics cardinality protection (label cap prevents high-cardinality explosion)
- SSE ticket auth (HMAC-based short-lived tickets, no API token in URL)
- Virtual filesystem (zero real file exposure)
- SFTP/SCP/reverse forwarding blocked
- Time-based access control (per-user login hours/days)
- Account expiration with automatic denial

### Security scanning

```bash
# Run all security checks
make security-scan

# Individual tools
cargo clippy --all-targets -- -D warnings
cargo audit
cargo deny check
```

Configuration: [`deny.toml`](deny.toml) (licenses, advisories, sources), [`.hadolint.yaml`](.hadolint.yaml) (Dockerfile lint).

### Audit report

See [docs/SECURITY-AUDIT.md](docs/SECURITY-AUDIT.md) for the full code security audit.

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT
