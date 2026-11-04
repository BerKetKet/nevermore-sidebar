# Spike 12: WARP CLI & WireGuard/NetworkManager Detection

## Objective
Detect and control Cloudflare WARP and WireGuard/VPN connections using `warp-cli` and `nmcli` on the user's host machine.

## System Configuration
- **WARP CLI Version:** `/usr/bin/warp-cli` (Cloudflare WARP)
- **Current WARP Mode:** `Warp` (protocol: `MASQUE` HTTP/3 with HTTP/2 fallback)
- **Live Status:** `Connected`, Network: `healthy`
- **NetworkManager (`nmcli`):** `/usr/bin/nmcli`
  - Active TUN: `CloudflareWARP` (`tun` interface)
  - Active WiFi: `VB` (`wlan0`)

## Command Signatures & Integration Strategy
1. **Cloudflare WARP Status:**
   - Command: `warp-cli status`
   - Output Parsing: Lines beginning with `Status update: Connected` / `Disconnected` / `Connecting`.
   - Control:
     - Connect: `warp-cli connect`
     - Disconnect: `warp-cli disconnect`
     - Rule: Never alter configured mode (`Mode: Warp`).

2. **NetworkManager WireGuard Profiles:**
   - Discovery command: `nmcli -t -f NAME,UUID,TYPE,DEVICE,STATE connection show`
   - Filter: `TYPE == "wireguard"` or `TYPE == "vpn"`.
   - Control:
     - Activate: `nmcli connection up <name>`
     - Deactivate: `nmcli connection down <name>`
   - Fallback: If no custom WireGuard profiles are imported, render `nm_empty_state("vpn", "No WireGuard profiles configured")`.

## Verdict
**PASSED.** Live integration verified against active CloudflareWARP daemon and NetworkManager interfaces.
