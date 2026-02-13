# VHS demo tape for s5 — https://github.com/charmbracelet/vhs
#
# Usage:
#   vhs contrib/demo.tape
#
# Prerequisites:
#   - s5 binary in PATH or target/release/s5
#   - Podman or Docker for the container demo variant
#
Output contrib/demo.gif

Set FontFamily "JetBrains Mono"
Set FontSize 14
Set Width 1100
Set Height 550
Set Theme "Dracula"
Set Padding 20

# ── Quick Start ──────────────────────────────────────────────────────
Type "# Start s5 with a demo password"
Enter
Sleep 500ms
Type "s5 quick-start --password demo &"
Enter
Sleep 3s

# ── SSH Shell ────────────────────────────────────────────────────────
Type "# Connect via SSH and explore the shell"
Enter
Sleep 500ms
Type "ssh -o StrictHostKeyChecking=no -p 2222 user@localhost"
Enter
Sleep 2s
Type "demo"
Enter
Sleep 1s

Type "show status"
Enter
Sleep 1.5s

Type "show quota"
Enter
Sleep 1.5s

Type "show connections"
Enter
Sleep 1.5s

Type "show acl"
Enter
Sleep 1.5s

Type "exit"
Enter
Sleep 1s

# ── SOCKS5 Proxy ────────────────────────────────────────────────────
Type "# Use as a SOCKS5 proxy"
Enter
Sleep 500ms
Type "curl --socks5 user:demo@localhost:1080 https://httpbin.org/ip"
Enter
Sleep 3s

# ── Health Check ─────────────────────────────────────────────────────
Type "# Built-in health check"
Enter
Sleep 500ms
Type "s5 health-check --addr 127.0.0.1:2222 --timeout 5"
Enter
Sleep 2s

# ── Cleanup ──────────────────────────────────────────────────────────
Type "kill %1 2>/dev/null"
Enter
Sleep 500ms
