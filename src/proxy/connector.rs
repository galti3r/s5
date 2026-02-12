use super::dns_cache::DnsCache;
use super::ip_guard;
use crate::metrics::MetricsRegistry;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Resolve hostname and check all addresses against ip_guard.
/// Returns only safe addresses (H-6: prevents port scanning oracle).
pub async fn resolve_and_check(
    host: &str,
    port: u16,
    timeout_secs: u64,
    ip_guard_enabled: bool,
) -> Result<Vec<SocketAddr>> {
    let addr_str = if host.contains(':') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };

    let dns_timeout = std::time::Duration::from_secs(timeout_secs.min(30));
    let addrs: Vec<SocketAddr> =
        tokio::time::timeout(dns_timeout, tokio::net::lookup_host(&addr_str))
            .await
            .context("DNS lookup timeout")?
            .with_context(|| format!("DNS lookup failed for {}", addr_str))?
            .collect();

    if addrs.is_empty() {
        anyhow::bail!("no addresses found for {}", addr_str);
    }

    if !ip_guard_enabled {
        return Ok(addrs);
    }

    let safe_addrs: Vec<SocketAddr> = addrs
        .into_iter()
        .filter(|addr| {
            if let Some(range_name) = ip_guard::classify_dangerous_ip(&addr.ip()) {
                warn!(
                    target_host = %host,
                    resolved_ip = %addr.ip(),
                    range = %range_name,
                    "Blocked connection to {} IP (anti-SSRF)", range_name
                );
                false
            } else {
                true
            }
        })
        .collect();

    if safe_addrs.is_empty() {
        anyhow::bail!(
            "all resolved addresses for {} are blocked by ip_guard",
            host
        );
    }

    Ok(safe_addrs)
}

/// DNS resolve + TCP connect with timeout.
/// Blocks connections to private/reserved IPs (anti-SSRF) when ip_guard_enabled is true.
pub async fn connect(
    host: &str,
    port: u16,
    timeout_secs: u64,
    ip_guard_enabled: bool,
) -> Result<(TcpStream, SocketAddr)> {
    // M-9: Reject port 0
    if port == 0 {
        anyhow::bail!("port 0 is not allowed");
    }

    let addrs = resolve_and_check(host, port, timeout_secs, ip_guard_enabled).await?;

    debug!(target_host = %host, resolved = ?addrs, "Resolved target (ip_guard filtered)");

    // Try to connect to each resolved address
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let mut last_err = None;

    for addr in &addrs {
        match tokio::time::timeout(timeout_duration, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                debug!(target_addr = %addr, "TCP connected");
                configure_tcp_socket(&stream);
                return Ok((stream, *addr));
            }
            Ok(Err(e)) => {
                debug!(target_addr = %addr, error = %e, "TCP connect failed");
                last_err = Some(e);
            }
            Err(_) => {
                debug!(target_addr = %addr, "TCP connect timeout");
                last_err = Some(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "connection timeout",
                ));
            }
        }
    }

    Err(last_err
        .map(|e| anyhow::anyhow!(e))
        .unwrap_or_else(|| anyhow::anyhow!("failed to connect to {}:{}", host, port)))
}

/// P3-3: DNS resolve + TCP connect with DNS cache support.
pub async fn connect_with_cache(
    host: &str,
    port: u16,
    timeout_secs: u64,
    ip_guard_enabled: bool,
    dns_cache: &DnsCache,
    metrics: Option<&MetricsRegistry>,
) -> Result<(TcpStream, SocketAddr)> {
    if port == 0 {
        anyhow::bail!("port 0 is not allowed");
    }

    // Build cache key on the stack to avoid heap allocation in hot path
    let mut cache_key = String::with_capacity(host.len() + 6);
    cache_key.push_str(host);
    cache_key.push(':');
    {
        use std::fmt::Write;
        let _ = write!(cache_key, "{}", port);
    }

    // Check cache first
    if let Some(cached_addrs) = dns_cache.get(&cache_key, ip_guard_enabled) {
        debug!(target_host = %host, cached_addrs = ?cached_addrs, "DNS cache hit");
        if let Some(m) = metrics {
            m.dns_cache_hits_total.inc();
        }
        return connect_to_addrs(&cached_addrs, timeout_secs, host, port).await;
    }

    // Cache miss — resolve normally
    if let Some(m) = metrics {
        m.dns_cache_misses_total.inc();
    }
    let addrs = resolve_and_check(host, port, timeout_secs, ip_guard_enabled).await?;

    debug!(target_host = %host, resolved = ?addrs, "Resolved target (ip_guard filtered)");

    // Store in cache (use default TTL since we don't have native TTL from tokio::net::lookup_host)
    dns_cache.insert(&cache_key, addrs.clone(), None);

    connect_to_addrs(&addrs, timeout_secs, host, port).await
}

/// Connect to a list of already-resolved addresses.
async fn connect_to_addrs(
    addrs: &[SocketAddr],
    timeout_secs: u64,
    host: &str,
    port: u16,
) -> Result<(TcpStream, SocketAddr)> {
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let mut last_err = None;

    for addr in addrs {
        match tokio::time::timeout(timeout_duration, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                debug!(target_addr = %addr, "TCP connected");
                configure_tcp_socket(&stream);
                return Ok((stream, *addr));
            }
            Ok(Err(e)) => {
                debug!(target_addr = %addr, error = %e, "TCP connect failed");
                last_err = Some(e);
            }
            Err(_) => {
                debug!(target_addr = %addr, "TCP connect timeout");
                last_err = Some(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "connection timeout",
                ));
            }
        }
    }

    Err(last_err
        .map(|e| anyhow::anyhow!(e))
        .unwrap_or_else(|| anyhow::anyhow!("failed to connect to {}:{}", host, port)))
}

/// Set TCP keepalive and nodelay on a connected stream.
fn configure_tcp_socket(stream: &TcpStream) {
    use socket2::SockRef;
    let sock = SockRef::from(stream);
    let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(15));
    let _ = sock.set_tcp_keepalive(&ka);
    let _ = stream.set_nodelay(true);
}
