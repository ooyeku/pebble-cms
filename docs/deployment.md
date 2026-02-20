# Production Deployment Guide

This guide covers deploying Pebble CMS on a Linux server with HTTPS, a reverse proxy, and automated backups.

## Prerequisites

- Linux server (Ubuntu 22.04+, Debian 12+, or similar)
- Ports 80 and 443 available
- A domain name pointing to your server

## 1. Install Pebble

### From source

```bash
cargo install --path .
```

### From release binary

Download the latest release and place it in your PATH:

```bash
curl -L https://github.com/your-org/pebble/releases/latest/download/pebble-linux-amd64 -o /usr/local/bin/pebble
chmod +x /usr/local/bin/pebble
```

## 2. Initial setup

```bash
# Create a dedicated user
sudo useradd -r -s /bin/false -m -d /opt/pebble pebble

# Initialize the site
sudo -u pebble pebble init /opt/pebble/site --name "My Blog"
cd /opt/pebble/site

# Run migrations
sudo -u pebble pebble migrate

# Create an admin user
sudo -u pebble pebble user add --username admin --email admin@example.com --role admin

# Verify the setup
sudo -u pebble pebble doctor
```

## 3. systemd Service

Create `/etc/systemd/system/pebble.service`:

```ini
[Unit]
Description=Pebble CMS
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=pebble
Group=pebble
WorkingDirectory=/opt/pebble/site
ExecStart=/usr/local/bin/pebble deploy --host 127.0.0.1 --port 8080
Restart=on-failure
RestartSec=5s

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/pebble/site
PrivateTmp=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Limits
LimitNOFILE=65536
MemoryMax=512M

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now pebble
sudo systemctl status pebble
```

## 4. Reverse Proxy

### nginx

Create `/etc/nginx/sites-available/pebble`:

```nginx
server {
    listen 80;
    server_name blog.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name blog.example.com;

    ssl_certificate /etc/letsencrypt/live/blog.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/blog.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Cache static assets
    location /media/ {
        proxy_pass http://127.0.0.1:8080;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }

    location /static/ {
        proxy_pass http://127.0.0.1:8080;
        expires 7d;
        add_header Cache-Control "public";
    }
}
```

Enable the site:

```bash
sudo ln -s /etc/nginx/sites-available/pebble /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

### Caddy

Create `/etc/caddy/Caddyfile`:

```
blog.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

Caddy handles TLS certificates automatically.

## 5. TLS Certificates

### With nginx (Let's Encrypt)

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d blog.example.com
```

### With Caddy

Caddy provisions certificates automatically. No additional steps needed.

## 6. Firewall

```bash
sudo ufw allow OpenSSH
sudo ufw allow 'Nginx Full'   # or: sudo ufw allow 80,443/tcp
sudo ufw enable
```

## 7. Backups

Configure automatic backups in `pebble.toml`:

```toml
[backup]
auto_enabled = true
interval_hours = 24
retention_count = 7
directory = "/opt/pebble/backups"
```

The backup scheduler runs automatically when using `pebble deploy`. Manual backups:

```bash
pebble backup create --output /opt/pebble/backups
pebble backup list --dir /opt/pebble/backups
```

## 8. Monitoring

Pebble exposes a `/health` endpoint for uptime monitoring:

```bash
curl -f http://127.0.0.1:8080/health
```

Use with any monitoring tool (UptimeRobot, Healthchecks.io, cron):

```bash
# Add to crontab for simple monitoring
*/5 * * * * curl -fsS http://127.0.0.1:8080/health > /dev/null || echo "Pebble is down" | mail -s "Alert" admin@example.com
```

Run a full system health check:

```bash
pebble doctor
```

## 9. Updating

```bash
# Download or build the new binary
cargo install --path .  # or download new release

# Restart the service
sudo systemctl restart pebble

# Verify
pebble --version
pebble doctor
```

Pebble applies pending database migrations automatically on startup.

## 10. Docker Deployment

See the included `Dockerfile` and `docker-compose.yml` for container-based deployment:

```bash
docker compose up -d
```

Refer to the Docker section in the main README for details.
