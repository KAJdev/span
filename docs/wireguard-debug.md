WireGuard mesh debugging guide

1) Check interface status
- ip link show wg0
- ip addr show wg0
- wg show

2) Verify keys
- cat ~/.config/span-agent/wg.pub (public key)
- cat ~/.config/span-agent/wg.key (private key)

3) Verify control plane config
- Ensure node has wg_ip and wg_pubkey in the database (nodes table)
- Ensure peers are healthy and have public_endpoint set

4) Test connectivity
- ping -c1 <peer-wg-ip>
- traceroute -i wg0 <peer-wg-ip>

5) Reset interface
- sudo ip link delete wg0
- Restart agent to reconfigure or run: wg set wg0 remove all-peers

6) Logs
- Agent logs should show periodic WireGuard config refreshes
- Control plane logs should show GetWireGuardConfig RPC calls

Notes
- Private keys must remain on the node and are never stored in the control plane.
- Default mesh subnet: 10.99.0.0/16; each node uses a /32 within this range.
- Default WireGuard port: 51820.
