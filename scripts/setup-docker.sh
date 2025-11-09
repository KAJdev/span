#!/usr/bin/env bash
set -euo pipefail
if command -v docker >/dev/null 2>&1; then echo "docker present"; exit 0; fi
curl -fsSL https://get.docker.com | sh
systemctl enable docker && systemctl start docker
