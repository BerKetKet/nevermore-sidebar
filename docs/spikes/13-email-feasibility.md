# Spike 13: Email Feasibility & Architecture Evaluation

## Objective
Evaluate architectural options for integrating email notifications, unread summaries, and quick-client access into the Nevermore sidebar (`crates/nevermore-ui/src/plugins/email/mod.rs`), adhering to the strict Nevermore constraints:
- **Zero background idle CPU %** when the tab or sidebar is hidden.
- **Minimal RAM footprint** (< 10 MB addition to the UI process).
- **Zero credential liability** (no plaintext credentials on disk, use Freedesktop Secret Service if needed).
- **Fast, resilient failure modes** with graceful degradation when no local mail client or account is configured.

---

## Architectural Candidates

| Criteria | (A) Desktop Client Raise (`kmail` / `thunderbird`) | (B) CLI Sync (`himalaya` / `notmuch`) | (C) Pure Rust IMAP IDLE | (D) Akonadi KDE PIM DBus |
|---|---|---|---|---|
| **RAM Footprint** | 0 MB (delegated) | 5–15 MB on invocation | 15–30 MB persistent | 200–500 MB (Akonadi server + DB) |
| **Idle CPU %** | 0.0% | 0.0% | 0.1–0.5% (TLS keepalive) | 0.5–2.0% background indexing |
| **Setup Complexity** | Zero (uses system default) | Medium (CLI config + token) | High (OAuth2 / IMAP config) | High (KDE PIM account wizard) |
| **Credential Safety** | Handled by Desktop Client | Secret Service / Pass | Secret Service (`org.freedesktop.secrets`) | KWallet / Akonadi |
| **Crash Blast Radius** | Isolated to external client | Isolated subprocess | In-process thread | Akonadi daemon crashes |
| **Offline Resilience** | High | High (local cache) | Poor (network required) | High (local SQLite/MySQL) |

---

## Evaluation Details

### Option A: Desktop Client Window Raising & Launching
- **Mechanism:** Inspect installed desktop clients (`kmail`, `thunderbird`, `geary`, `neomutt`) or default `xdg-email` handler. Provide quick-action launcher buttons using `ctx.daemon` or `ctx.run` (`kmail`, `thunderbird`, or webmail).
- **Pros:** 100% stable, zero network code inside Nevermore, zero credential management required.
- **Cons:** Cannot show individual unread email subject lines without a secondary data source.

### Option B: `himalaya` / `notmuch` CLI Integration
- **Mechanism:** Asynchronous subprocess execution via `ctx.run(Cmd::new("himalaya").arg("envelope").arg("list"), ...)` reading JSON envelopes.
- **Pros:** Structured JSON parsing, standard Rust CLI tool (`himalaya`), clean fallback.
- **Cons:** If `himalaya` is not installed or configured, must cleanly fall back to empty state.

### Option C: In-Process Rust IMAP IDLE
- **Mechanism:** Native Rust IMAP client connection maintained via async thread.
- **Pros:** Real-time push updates.
- **Cons:** Heavy dependency footprint (`async-imap`, `native-tls`), OAuth2 token refresh complications for Gmail/Outlook, violates zero-context simplicity.

### Option D: Akonadi DBus
- **Mechanism:** Querying Akonadi via DBus on KDE Plasma.
- **Pros:** Already partially present on system (`/usr/bin/kmail`, `/usr/bin/akonadictl`).
- **Cons:** Akonadi daemon consumes excessive memory and CPU, frequent state corruption on Arch/CachyOS. Rejected.

---

## Provisional Decision & Implementation Architecture
**Hybrid Architecture (Option A + Option B Fallback):**
1. **Unread Summary & List:**
   - The plugin attempts to read recent envelope summaries from `himalaya envelope list --output json` or cached unread files in `ctx.data_dir/unread_cache.json`.
   - If configured, renders unread envelopes as `nm_list_row` cards with sender, subject, and time tag.
   - If unconfigured or empty, renders `nm_empty_state("email", "No unread messages or account configured")`.
2. **Quick Desktop Client Raise:**
   - Top action toolbar provides one-click launch/raise for detected mail clients (`kmail` or `thunderbird` via `ctx.daemon`).
3. **Lifecycle Respect:**
   - Polling runs on a 60-second timer (`ctx.every`) that is completely paused when the tab or sidebar is hidden, guaranteeing 0.0% idle CPU.
