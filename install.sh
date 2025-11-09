#!/usr/bin/env bash
set -euo pipefail

SPAN_VERSION="${SPAN_VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/opt/span}"
DATA_DIR="${DATA_DIR:-/var/lib/span}"

print_banner() {
    cat << "EOF"
   _____ ____  ___    _   __
  / ___// __ \/   |  / | / /
  \__ \/ /_/ / /| | /  |/ / 
 ___/ / ____/ ___ |/ /|  /  
/____/_/   /_/  |_/_/ |_/_   
                             
Distributed Personal Cloud
EOF
}

check_requirements() {
    echo "Checking system requirements..."
    if [[ ! -f /etc/os-release ]]; then
        echo "Error: /etc/os-release not found. Unsupported OS."
        exit 1
    fi
    . /etc/os-release
    echo "✓ Detected: $PRETTY_NAME"
    if [[ $EUID -ne 0 ]]; then
        echo "Error: This script must be run as root"
        exit 1
    fi
    echo "✓ Running as root"
}

install_docker() {
    if command -v docker &>/dev/null; then
        echo "✓ Docker already installed"; return
    fi
    echo "Installing Docker..."
    curl -fsSL https://get.docker.com | sh
    systemctl enable docker
    systemctl start docker
    echo "✓ Docker installed"
}

install_docker_compose() {
    if command -v docker-compose &>/dev/null; then
        echo "✓ docker-compose present"; return
    fi
    if docker compose version &>/dev/null; then
        echo "✓ Docker Compose plugin available"
        cat >/usr/local/bin/docker-compose <<'EOC'
#!/bin/sh
exec docker compose "$@"
EOC
        chmod +x /usr/local/bin/docker-compose
        return
    fi
    echo "Installing docker-compose standalone..."
    COMPOSE_VERSION=$(curl -s https://api.github.com/repos/docker/compose/releases/latest | grep 'tag_name' | sed -E 's/.*"([^"]+)".*/\1/')
    curl -L "https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
    chmod +x /usr/local/bin/docker-compose
}

install_containerd() {
    if command -v containerd &>/dev/null; then
        echo "✓ containerd already installed"; return
    fi
    echo "Installing containerd..."
    apt-get update && apt-get install -y containerd
    systemctl enable containerd
    systemctl start containerd
}

download_span() {
    echo "Installing Span into ${INSTALL_DIR}..."
    mkdir -p "${INSTALL_DIR}" "${DATA_DIR}"
    cp -r "$(pwd)/deploy" "${INSTALL_DIR}/" || true
    cp -r "$(pwd)/dashboard" "${INSTALL_DIR}/" || true

    if [[ -f "$(pwd)/target/release/span" ]]; then
        install "$(pwd)/target/release/span" /usr/local/bin/span
        echo "✓ Installed local CLI binary"
        return
    fi

    echo "Downloading release artifacts..."
    if [[ "${SPAN_VERSION}" == "latest" ]]; then
        VERSION=$(curl -s https://api.github.com/repos/KAJdev/span/releases/latest | grep 'tag_name' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        VERSION="${SPAN_VERSION}"
    fi
    ARCH=$(uname -m)
    [[ "$ARCH" == "aarch64" ]] && ARCH=arm64
    [[ "$ARCH" == "x86_64" ]] && ARCH=amd64
    curl -L "https://github.com/KAJdev/span/releases/download/${VERSION}/span-${VERSION}-linux-${ARCH}.tar.gz" -o /tmp/span.tar.gz || {
        echo "Failed to download release tarball"; exit 1; }
    tar -xzf /tmp/span.tar.gz -C "${INSTALL_DIR}"
    rm -f /tmp/span.tar.gz
    install "${INSTALL_DIR}/bin/span" /usr/local/bin/span
    chmod +x /usr/local/bin/span
    echo "✓ Span CLI installed"
}

generate_config() {
    echo "Generating .env configuration..."
    if [[ ! -f "${INSTALL_DIR}/.env" ]]; then
        cp "${INSTALL_DIR}/deploy/.env.example" "${INSTALL_DIR}/.env"
        POSTGRES_PASSWORD=$(openssl rand -hex 16)
        MINIO_PASSWORD=$(openssl rand -hex 16)
        JWT_SECRET=$(openssl rand -hex 32)
        MASTER_KEY=$(openssl rand -hex 32)
        NEXTAUTH_SECRET=$(openssl rand -hex 16)
        NODE_ID=$(cat /proc/sys/kernel/random/uuid)
        cat >>"${INSTALL_DIR}/.env" <<EOF

NODE_ID=${NODE_ID}
NODE_NAME=$(hostname)
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
MINIO_ROOT_PASSWORD=${MINIO_PASSWORD}
JWT_SECRET=${JWT_SECRET}
SPAN_MASTER_KEY=${MASTER_KEY}
NEXTAUTH_SECRET=${NEXTAUTH_SECRET}
EOF
        echo "✓ .env created"
    else
        echo "✓ Using existing ${INSTALL_DIR}/.env"
    fi
}

install_systemd_service() {
    echo "Installing systemd service..."
    cat >/etc/systemd/system/span.service <<EOF
[Unit]
Description=Span Distributed Cloud
Requires=docker.service
After=docker.service network-online.target
Wants=network-online.target

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=${INSTALL_DIR}
EnvironmentFile=${INSTALL_DIR}/.env
ExecStart=/usr/local/bin/docker-compose -f ${INSTALL_DIR}/deploy/docker-compose.yml up -d
ExecStop=/usr/local/bin/docker-compose -f ${INSTALL_DIR}/deploy/docker-compose.yml down
Restart=on-failure
RestartSec=10s
TimeoutStartSec=300

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
    echo "✓ Service unit installed"
}

main() {
    print_banner
    check_requirements
    install_docker
    install_docker_compose
    install_containerd
    download_span
    generate_config
    install_systemd_service

    echo "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✓ Span installed successfully!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
    echo "Next steps:\n"
    echo "  1. Initialize a new cluster:"
    echo "     span init"
    echo "\n  OR join an existing cluster:"
    echo "     span init --join <node-ip-or-hostname>\n"
    echo "  2. Start services:"
    echo "     systemctl start span"
    echo "     systemctl enable span\n"
    echo "  3. Check status:"
    echo "     span status"
    echo "     systemctl status span\n"
    echo "  4. Access dashboard:"
    IP=$(hostname -I | awk '{print $1}')
    echo "     http://${IP}:3000\n"
}

main
