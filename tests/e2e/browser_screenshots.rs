#[allow(dead_code, unused_imports)]
mod helpers;

use futures::StreamExt;
use helpers::*;
use std::process::Command;
use std::time::Duration;
use tokio::sync::OnceCell;

// ---------------------------------------------------------------------------
// Shared Chrome container — one per test binary execution
// ---------------------------------------------------------------------------

struct ChromeState {
    _container_id: String,
    cdp_port: u16,
}

static CHROME: OnceCell<ChromeState> = OnceCell::const_new();

async fn ensure_chrome() -> u16 {
    CHROME
        .get_or_init(|| async {
            let _ = Command::new("sh")
                .args([
                    "-c",
                    "podman ps -aq --filter name=s5-chrome | xargs -r podman rm -f 2>/dev/null",
                ])
                .output();

            let cdp_port = free_port().await;
            let container_name = format!("s5-chrome-screenshots-{}", std::process::id());

            let output = Command::new("podman")
                .args([
                    "run",
                    "-d",
                    "--rm",
                    "--network=host",
                    "--name",
                    &container_name,
                    "docker.io/chromedp/headless-shell:latest",
                    "--remote-debugging-address=0.0.0.0",
                    &format!("--remote-debugging-port={}", cdp_port),
                    "--no-sandbox",
                    "--disable-gpu",
                    "--disable-dev-shm-usage",
                ])
                .output()
                .expect("podman run");

            assert!(
                output.status.success(),
                "podman run failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

            let client = reqwest::Client::new();
            let url = format!("http://127.0.0.1:{}/json/version", cdp_port);
            for i in 0..20 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if client.get(&url).send().await.is_ok() {
                    eprintln!("Chrome CDP ready on port {} (attempt {})", cdp_port, i + 1);
                    return ChromeState {
                        _container_id: container_id,
                        cdp_port,
                    };
                }
            }
            panic!("Chrome DevTools not ready at {} after 10s", url);
        })
        .await
        .cdp_port
}

fn podman_available() -> bool {
    Command::new("podman")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn connect_browser(cdp_port: u16) -> (chromiumoxide::Browser, tokio::task::JoinHandle<()>) {
    let url = format!("http://127.0.0.1:{}", cdp_port);
    let (browser, mut handler) = chromiumoxide::Browser::connect(&url)
        .await
        .expect("connect to Chrome CDP");
    let handle = tokio::spawn(async move { while handler.next().await.is_some() {} });
    (browser, handle)
}

async fn wait_api_ready(api_port: u16, token: &str) {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/status", api_port);
    for _ in 0..20 {
        if let Ok(resp) = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            if resp.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("API server not ready on port {} after 2s", api_port);
}

fn screenshot_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("SCREENSHOT_DIR").unwrap_or_else(|_| "/tmp/s5-screenshots".to_string()),
    )
}

/// Build an enriched config with 2 users, quotas, and a group for realistic screenshots.
fn screenshot_config(
    api_port: u16,
    token: &str,
    password_hash: &str,
) -> s5::config::types::AppConfig {
    let toml_str = format!(
        r##"
[server]
ssh_listen = "127.0.0.1:0"
host_key_path = "/tmp/s5-screenshot-key"

[api]
enabled = true
listen = "127.0.0.1:{api_port}"
token = "{token}"

[security]
ban_enabled = true
ip_guard_enabled = false

[logging]
level = "debug"

[logging.audit]
enabled = true

[[groups]]
name = "developers"
max_bandwidth_kbps = 10240
allow_forwarding = true
allow_shell = true

[[users]]
username = "alice"
password_hash = "{password_hash}"
allow_forwarding = true
allow_shell = true
group = "developers"

[users.quotas]
daily_bandwidth_bytes = 1073741824
monthly_connection_limit = 1000

[[users]]
username = "bob"
password_hash = "{password_hash}"
allow_forwarding = true
allow_shell = false
group = "developers"

[users.quotas]
daily_bandwidth_bytes = 536870912
monthly_connection_limit = 500
"##
    );
    toml::from_str(&toml_str).unwrap()
}

async fn open_dashboard(
    browser: &chromiumoxide::Browser,
    api_port: u16,
    token: &str,
) -> chromiumoxide::Page {
    let page = browser
        .new_page(format!(
            "http://127.0.0.1:{}/dashboard?token={}",
            api_port, token
        ))
        .await
        .expect("open new page");

    // Set viewport to 1280x800 for deterministic screenshots via CDP
    let set_metrics =
        chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
            .width(1280)
            .height(800)
            .device_scale_factor(2.0)
            .mobile(false)
            .build()
            .unwrap();
    page.execute(set_metrics).await.expect("set viewport");

    // Wait for page DOM + initial scripts to load
    tokio::time::sleep(Duration::from_millis(1500)).await;
    page
}

async fn wait_for_text(page: &chromiumoxide::Page, expr: &str, expected: &str, timeout_secs: u64) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Ok(result) = page.evaluate(expr).await {
            if let Ok(val) = result.into_value::<String>() {
                if val == expected {
                    return;
                }
            }
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "Timeout ({}s) waiting for `{}` == {:?}",
                timeout_secs, expr, expected
            );
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn wait_for_text_ne(
    page: &chromiumoxide::Page,
    expr: &str,
    not_expected: &str,
    timeout_secs: u64,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Ok(result) = page.evaluate(expr).await {
            if let Ok(val) = result.into_value::<String>() {
                if val != not_expected {
                    return;
                }
            }
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "Timeout ({}s) waiting for `{}` != {:?}",
                timeout_secs, expr, not_expected
            );
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn save_screenshot(page: &chromiumoxide::Page, name: &str) {
    let dir = screenshot_dir();
    std::fs::create_dir_all(&dir).expect("create screenshot dir");
    let path = dir.join(name);

    let params = chromiumoxide::page::ScreenshotParams::builder()
        .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
        .build();
    let png_data = page.screenshot(params).await.expect("capture screenshot");

    std::fs::write(&path, png_data).expect("write screenshot");
    eprintln!("Screenshot saved: {}", path.display());
}

// ===========================================================================
// Tests
// ===========================================================================

const TOKEN: &str = "screenshot-test-token";

// ---------------------------------------------------------------------------
// Screenshot 1: Dashboard dark theme with live data
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore]
async fn screenshot_dashboard_dark() {
    if !podman_available() {
        eprintln!("SKIPPED: podman not available");
        return;
    }

    let cdp_port = ensure_chrome().await;
    let api_port = free_port().await;
    let hash = hash_pass("pass");
    let _server = start_api(screenshot_config(api_port, TOKEN, &hash)).await;
    wait_api_ready(api_port, TOKEN).await;

    let (browser, _handler) = connect_browser(cdp_port).await;
    let page = open_dashboard(&browser, api_port, TOKEN).await;

    // Wait for WebSocket to connect and data to populate
    wait_for_text(
        &page,
        "document.getElementById('connStatus').textContent",
        "Connected",
        10,
    )
    .await;
    wait_for_text_ne(
        &page,
        "document.getElementById('totalUsers').textContent",
        "-",
        10,
    )
    .await;

    // Ensure dark theme (default)
    page.evaluate("document.documentElement.classList.remove('light')")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    save_screenshot(&page, "dashboard-dark.png").await;
}

// ---------------------------------------------------------------------------
// Screenshot 2: Dashboard light theme
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore]
async fn screenshot_dashboard_light() {
    if !podman_available() {
        eprintln!("SKIPPED: podman not available");
        return;
    }

    let cdp_port = ensure_chrome().await;
    let api_port = free_port().await;
    let hash = hash_pass("pass");
    let _server = start_api(screenshot_config(api_port, TOKEN, &hash)).await;
    wait_api_ready(api_port, TOKEN).await;

    let (browser, _handler) = connect_browser(cdp_port).await;
    let page = open_dashboard(&browser, api_port, TOKEN).await;

    wait_for_text(
        &page,
        "document.getElementById('connStatus').textContent",
        "Connected",
        10,
    )
    .await;
    wait_for_text_ne(
        &page,
        "document.getElementById('totalUsers').textContent",
        "-",
        10,
    )
    .await;

    // Switch to light theme
    page.evaluate("document.documentElement.classList.add('light')")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    save_screenshot(&page, "dashboard-light.png").await;
}

// ---------------------------------------------------------------------------
// Screenshot 3: Quota panel
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore]
async fn screenshot_dashboard_quotas() {
    if !podman_available() {
        eprintln!("SKIPPED: podman not available");
        return;
    }

    let cdp_port = ensure_chrome().await;
    let api_port = free_port().await;
    let hash = hash_pass("pass");
    let _server = start_api(screenshot_config(api_port, TOKEN, &hash)).await;
    wait_api_ready(api_port, TOKEN).await;

    let (browser, _handler) = connect_browser(cdp_port).await;
    let page = open_dashboard(&browser, api_port, TOKEN).await;

    wait_for_text(
        &page,
        "document.getElementById('connStatus').textContent",
        "Connected",
        10,
    )
    .await;
    wait_for_text_ne(
        &page,
        "document.getElementById('totalUsers').textContent",
        "-",
        10,
    )
    .await;

    // Scroll to the quota section
    page.evaluate("document.querySelector('#quotaTable')?.scrollIntoView({behavior:'instant',block:'center'})")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    save_screenshot(&page, "dashboard-quotas.png").await;
}

// ---------------------------------------------------------------------------
// Screenshot 4: Audit log panel
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore]
async fn screenshot_dashboard_audit() {
    if !podman_available() {
        eprintln!("SKIPPED: podman not available");
        return;
    }

    let cdp_port = ensure_chrome().await;
    let api_port = free_port().await;
    let hash = hash_pass("pass");
    let _server = start_api(screenshot_config(api_port, TOKEN, &hash)).await;
    wait_api_ready(api_port, TOKEN).await;

    let (browser, _handler) = connect_browser(cdp_port).await;
    let page = open_dashboard(&browser, api_port, TOKEN).await;

    wait_for_text(
        &page,
        "document.getElementById('connStatus').textContent",
        "Connected",
        10,
    )
    .await;
    wait_for_text_ne(
        &page,
        "document.getElementById('totalUsers').textContent",
        "-",
        10,
    )
    .await;

    // Scroll to audit log area
    page.evaluate(
        "document.querySelector('#logArea')?.scrollIntoView({behavior:'instant',block:'center'})",
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    save_screenshot(&page, "dashboard-audit.png").await;
}
