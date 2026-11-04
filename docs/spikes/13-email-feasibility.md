# Spike 13: Email Feasibility & Architecture Evaluation

## Objective
Evaluate architectural options for integrating email notifications, unread summaries, and quick access into the Nevermore sidebar (`crates/nevermore-ui/src/plugins/email/mod.rs`), adhering to the strict Nevermore constraints:
- **Zero background idle CPU %** when the tab or sidebar is hidden.
- **Minimal RAM footprint** (< 10 MB addition to the UI process).
- **Zero credential liability** (no plaintext credentials on disk, use Freedesktop Secret Service if needed).
- **Fast, resilient failure modes** with graceful degradation when no local account is configured.

---

## Architectural Decision: Official Gmail REST API v1 with OAuth2 Loopback
1. **Unread Summary & List:**
   - The plugin communicates directly with official Google Gmail REST API v1 (`https://gmail.googleapis.com/gmail/v1/users/me/messages?q=is:unread+in:inbox&maxResults=15`) passing auth headers exclusively over `curl -K -` stdin.
   - Renders unread emails with Sender, Subject (prominently displayed), and Snippet, with one-click browser thread opener and mark-as-read actions.
2. **OAuth2 Loopback Authentication:**
   - One-click authorization in Settings page binds an ephemeral `127.0.0.1:0` listener, opens browser consent, exchanges tokens over stdin, and stores `gmail_refresh_token` in Secret Service.
3. **Lifecycle Respect:**
   - Polling runs on a lifecycle timer (`ctx.every`) that is completely paused when the tab or sidebar is hidden, guaranteeing 0.0% idle CPU.
