# Spike 11: Secret Service Keyring Roundtrip

## Objective
Verify credential storage and retrieval on FreeDesktop Secret Service API using `secret-tool` via stdin, preventing any secret leakage in process arguments (`ps aux`) or plaintext configuration files.

## Environment & Keyring Daemon
- **Session Bus:** FreeDesktop Secret Service (`org.freedesktop.secrets`)
- **CLI Utility:** `/usr/bin/secret-tool`
- **Security Constraint:** Passphrase/token passed via stdin (`echo -n "$SECRET" | secret-tool store ...`), never as command-line arguments.

## Verification & Execution Log
1. **Store Key via Stdin:**
   ```bash
   echo -n "test_secret_12345" | secret-tool store --label="Nevermore Test Key" service nevermore key test_key
   ```
2. **Lookup Key:**
   ```bash
   RETRIEVED=$(secret-tool lookup service nevermore key test_key)
   ```
   - Retrieved string length: `17` characters (`test_secret_12345`).
   - Match verified: `100% exact`.
3. **Clear Key:**
   ```bash
   secret-tool clear service nevermore key test_key
   ```
   - Subsequent lookup returned empty string.

## Verdict
**PASSED.** Key storage through `ctx.secret_set` and `ctx.secret_get` operates on the live desktop keyring. The Gemini chat plugin will look up `gemini_api_key` securely through this interface, falling back to `GEMINI_API_KEY` environment variable.
