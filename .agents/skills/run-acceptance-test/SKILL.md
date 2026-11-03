---
name: run-acceptance-test
description: Instructions for executing automated acceptance tests and logging verified proof in docs/EVIDENCE_LOG.md.
---

# Run Acceptance Test Skill

Use this skill to verify code quality, state machine transitions, and CLI operations, logging all evidence into `docs/EVIDENCE_LOG.md`.

## Protocol

### 1. Static Verification
Run static analysis commands in the workspace root:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### 2. Automated Test Suite
Execute the entire test suite with locked dependencies:
```bash
cargo test --workspace --locked --verbose
```

### 3. Integration & CLI Verification
For CLI tools:
- Verify subcommands `status`, `toggle`, `show`, `hide`, `pin` produce expected JSON or stdout.
- Verify exit codes are `0` on success and non-zero on failure.

### 4. Append to Evidence Log
Record every command, its exit code, timestamp, and stdout/stderr summary into `docs/EVIDENCE_LOG.md` adhering to the table format:
```markdown
| Date | Claim/Check | Command Run | Raw Output / Measurement / Screenshot Path | Status |
```
