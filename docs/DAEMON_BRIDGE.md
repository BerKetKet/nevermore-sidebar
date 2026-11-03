# Nevermore Manager AI Bridge Daemon & Subagent Architecture

## 1. Problem & Root Cause
In Chrome v120+, establishing a **new** DevTools Protocol (CDP) WebSocket connection to `--remote-debugging-port` triggers an interactive user prompt: *"Allow remote debugging connection?"*.
Previously, ad-hoc agent scripts initialized a fresh connection per message or status check, causing Chrome to display repeated authorization dialogs and blocking execution when the user stepped away.

---

## 2. Solution & Permanent Architecture

```mermaid
flowchart LR
    A["Worker AI / Subagents"] -->|"Unix Domain Socket (/tmp/nevermore_bridge.sock)"| B["nevermore-manager-bridge.service"]
    B -->|"Persistent Single WebSocket (Holding connection open)"| C["Google Chrome (Gemini Conversation)"]
```

### Components:
1. **Persistent Daemon (`nevermore-manager-bridge.service`):**
   - Managed as a `systemd --user` service (`~/.config/systemd/user/nevermore-manager-bridge.service`).
   - Starts automatically on boot, restarts on crash (`Restart=always`).
   - Connects **ONCE** to Chrome DevTools WebSocket and holds the session permanently open.
   - Binds a secure local Unix domain socket at `/tmp/nevermore_bridge.sock` (`mode 0700`, user `bgg`).
2. **Chrome Enterprise Managed Policy:**
   - Installed at `/etc/opt/chrome/policies/managed/nevermore_devtools.json`:
     ```json
     {
       "RemoteDebuggingAllowed": true,
       "DeveloperToolsAvailability": 1,
       "CommandLineFlagSecurityWarningsEnabled": false
     }
     ```
3. **Dedicated Subagent (`browser-bridge`):**
   - Defined in `.agents/agents/browser-bridge.md` and registered in Antigravity via `define_subagent`.
   - Dedicated role for interacting with Manager AI in Chrome.
   - Strictly routes all actions through `scripts/manager-cli` or `/tmp/nevermore_bridge.sock`.
   - Never touches port 9222 directly.
4. **CLI Wrapper (`scripts/manager-cli`):**
   - High-speed CLI interface to `/tmp/nevermore_bridge.sock`:
     - `scripts/manager-cli --ping`: Immediate daemon health check.
     - `scripts/manager-cli --status`: Checks attachment, editor element, and quota limit state.
     - `scripts/manager-cli --send-file <path>`: Transmits report file to Manager AI and waits for streaming response.
     - `scripts/manager-cli --send "<msg>"`: Transmits text string to Manager AI.
     - `scripts/manager-cli --get-latest`: Reads the latest response turn without typing.
     - `scripts/manager-cli --eval "<js>"`: Evaluates JavaScript in the Gemini page.

---

## 3. Verification & Evidence
- **Daemon Service Status:** Active and running (`PID 472046`), attached to sessionId `5EB72A395CF9B0D5B1D8A813D334487D`.
- **Latency:** Ping / Status response latency over Unix domain socket is `< 15 ms`.
- **Chrome Prompts:** Zero popups or interactive prompts triggered after initial attachment.
