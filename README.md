# span

Distributed personal cloud control plane.

## Install

- Ensure Rust 1.75+ is installed.
- Install binaries:

```
cargo install --path crates/cli
cargo install --path crates/control-plane
```

## Quick start

```
export DATABASE_URL="postgres://user:pass@localhost/span"
span init --run
```

## Health check

```
curl http://localhost:8080/health
# {"status":"ok","version":"0.1.0"}
```

## Repository structure
See crates/ for components. Control Plane lives in `crates/control-plane`.
