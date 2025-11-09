#!/usr/bin/env bash
set -euo pipefail
INSTALL_DIR=${INSTALL_DIR:-/opt/span}

read -p "This will stop Span and remove containers/volumes. Continue? [y/N] " ans
[[ "${ans:-N}" == "y" || "${ans:-N}" == "Y" ]] || { echo "Aborted"; exit 1; }

systemctl stop span || true
systemctl disable span || true
rm -f /etc/systemd/system/span.service
systemctl daemon-reload || true

/usr/local/bin/docker-compose -f ${INSTALL_DIR}/deploy/docker-compose.yml down -v || true

rm -rf ${INSTALL_DIR}
rm -rf /var/lib/span

echo "✓ Uninstalled Span"
