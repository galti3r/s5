# s5 System Installation

## Systemd

### Create user and directories

```bash
sudo useradd -r -s /usr/sbin/nologin s5
sudo mkdir -p /etc/s5 /var/lib/s5 /var/log/s5
sudo chown s5:s5 /var/lib/s5 /var/log/s5
```

### Install binary and config

```bash
sudo cp target/release/s5 /usr/local/bin/s5
sudo chmod 755 /usr/local/bin/s5
sudo cp config.example.toml /etc/s5/config.toml
sudo chown root:s5 /etc/s5/config.toml
sudo chmod 640 /etc/s5/config.toml
```

### Install and enable service

```bash
sudo cp contrib/s5.service /etc/systemd/system/s5.service
sudo systemctl daemon-reload
sudo systemctl enable s5
sudo systemctl start s5
```

### Check status

```bash
sudo systemctl status s5
sudo journalctl -u s5 -f
```

### Reload configuration

```bash
sudo systemctl reload s5
```
