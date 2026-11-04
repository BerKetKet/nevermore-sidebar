# Nevermore VPN Setup Guide

This guide describes how to configure and manage VPN connections in Nevermore Sidebar using Cloudflare WARP and NetworkManager WireGuard profiles.

---

## 1. Cloudflare WARP Integration

Nevermore integrates directly with `warp-cli`. When installed, WARP status and controls appear in the VPN tab:

- **Connect:** Toggle the shield icon in the WARP card or run `warp-cli connect`.
- **Disconnect:** Toggle off or run `warp-cli disconnect`.
- **Mode Verification:** Check current mode via `warp-cli settings`. The sidebar respects your configured mode (`Warp`, `DoH`, etc.) and never alters network policies.

---

## 2. Importing WireGuard Profiles (`.conf`) into NetworkManager

NetworkManager natively supports WireGuard tunnels. To add your custom WireGuard profiles so they appear in Nevermore:

```bash
# Import a standard WireGuard configuration file
nmcli connection import type wireguard file /path/to/wg0.conf

# Verify the imported connection
nmcli connection show wg0

# Test manual activation
nmcli connection up wg0

# Test manual deactivation
nmcli connection down wg0
```

Once imported, Nevermore auto-discovers the profile on its next 5-second polling cycle and renders a control card with a one-click connection toggle.
