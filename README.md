# Span

Distributed personal cloud. Every node is equal.

## One-line installation

```
curl -sSL https://get.span.sh | sudo sh
```

This installs Span to /opt/span, prepares Docker and systemd, and sets up the unified stack.

## Initialize

- Bootstrap a new cluster on the first node:

```
sudo span init
sudo systemctl start span
sudo systemctl enable span
```

- Join an existing cluster from another node:

```
sudo span init --join <node-ip>
sudo systemctl start span
sudo systemctl enable span
```

## Status

```
span status
systemctl status span
```

## Access the Dashboard

Open http://<node-ip>:3000 in your browser.

## Development

- Ensure Rust is installed.
- Build locally:

```
cargo build --release
```

## Repository structure

- crates/ — Rust crates (control-plane, agent, gateway, cli)
- deploy/ — Docker Compose stack and Dockerfiles
- dashboard/ — Minimal dashboard server
- install.sh — Unified installer
- uninstall.sh — Cleanup script

## WireGuard mesh

Span configures a private WireGuard mesh across nodes. See docs/wireguard-debug.md for troubleshooting and manual commands.

## Health check

```
curl http://localhost:8080/health
```
