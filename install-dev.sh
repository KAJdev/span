#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

if [[ ! -f Cargo.toml || ! -d crates ]]; then
  echo "Error: run this from the repository root" >&2
  exit 1
fi

if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
  SUDO="sudo"
else
  SUDO=""
fi

require() {
  command -v "$1" >/dev/null 2>&1 || return 1
}

apt_install() {
  $SUDO apt-get update -y
  DEBIAN_FRONTEND=noninteractive $SUDO apt-get install -y \
    build-essential pkg-config libssl-dev protobuf-compiler cmake \
    curl git ca-certificates openssl \
    docker.io docker-compose-plugin containerd uuid-runtime
}

ensure_docker() {
  if ! require docker; then
    apt_install
  fi
  $SUDO systemctl enable --now containerd || true
  $SUDO systemctl enable --now docker || true
}

ensure_rust() {
  if ! require cargo; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
  if require rustup && [[ -f rust-toolchain.toml ]]; then
    TOOLCHAIN=$(grep -E '^channel\s*=\s*"' rust-toolchain.toml | sed -E 's/.*"([^"]+)".*/\1/')
    if [[ -n "${TOOLCHAIN:-}" ]]; then
      rustup toolchain install "$TOOLCHAIN" -c rustfmt -c clippy
      rustup default "$TOOLCHAIN"
    fi
  fi
}

build_cli() {
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo build --release -p span-cli
  $SUDO install -Dm755 target/release/span /usr/local/bin/span
}

random_hex() { openssl rand -hex "$1"; }
random_uuid() { cat /proc/sys/kernel/random/uuid; }

ensure_env() {
  ENV_FILE="${REPO_ROOT}/deploy/.env"
  if [[ -f "$ENV_FILE" ]]; then
    echo "✓ Using existing deploy/.env"
    return
  fi
  POSTGRES_PASSWORD=$(random_hex 16)
  MINIO_ROOT_PASSWORD=$(random_hex 16)
  JWT_SECRET=$(random_hex 32)
  SPAN_MASTER_KEY=$(random_hex 32)
  NEXTAUTH_SECRET=$(random_hex 16)
  NODE_ID=$(random_uuid)
  NODE_NAME=$(hostname)
  cat >"$ENV_FILE" <<EOF
NODE_ID=${NODE_ID}
NODE_NAME=${NODE_NAME}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
MINIO_ROOT_PASSWORD=${MINIO_ROOT_PASSWORD}
JWT_SECRET=${JWT_SECRET}
SPAN_MASTER_KEY=${SPAN_MASTER_KEY}
NEXTAUTH_SECRET=${NEXTAUTH_SECRET}
EOF
  echo "✓ Wrote deploy/.env"
}

build_images() {
  ensure_docker
  docker compose -f deploy/docker-compose.yml build
}

main() {
  echo "Installing development dependencies and building from source..."
  apt_install
  ensure_rust
  build_cli
  ensure_env
  build_images
  echo
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "✓ Development setup complete"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo
  echo "Next steps:"
  echo "  1) Initialize a new cluster:"
  echo "     sudo span init"
  echo "  2) Start the stack locally:"
  echo "     docker compose -f deploy/docker-compose.yml up -d"
  echo "  3) Access the dashboard:"
  IP=$(hostname -I | awk '{print $1}') || true
  echo "     http://${IP:-localhost}:3000"
}

main
