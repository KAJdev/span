#!/usr/bin/env bash
set -euo pipefail
if command -v containerd >/dev/null 2>&1; then echo "containerd present"; exit 0; fi
apt-get update && apt-get install -y containerd
systemctl enable containerd && systemctl start containerd
