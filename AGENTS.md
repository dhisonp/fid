# Agent guidance

## Architecture

- `worker/src/lib.rs` owns routing and scheduled orchestration; `collect.rs` discovers and archives sources; `editorial.rs` drafts, validates, audits, and rechecks evidence before handing publication to `store.rs`.
- Keep model instructions in `editorial.rs`, separate from untrusted source text. README and agent guidance are not runtime prompt inputs.
- Keep `worker/src/rules.rs` independent of Cargo dependencies and other Worker modules: offline checks compile it directly with `rustc`.
- `sql/` queries are embedded by `store.rs` and executed directly by `tests/test_storage.py`. Parameter changes must update both callers; storage tests must exercise production SQL.
- Coordinate public response changes across `worker/src/types.rs`, `worker/src/lib.rs`, `web/src/api.ts`, and `tests/api.test.ts`; frontend types and validation are maintained separately.
- Storage tests replace SQLite's `unixepoch()` with a controlled clock. Preserve database-clock assertions for leases and deadlines.

## Code conventions

- Keep dependencies and public interfaces small; avoid speculative abstractions.
- Parse untrusted data into typed values at boundaries; propagate errors rather than silently substituting defaults.
- Keep resource acquisition and side effects explicit. Lease expiry releases jobs without fallible cleanup.
- Comments explain invariants and reasons rather than restating code.
- Keep unrelated cleanup separate from functional changes.

## Git

- Use simple sentence commit messages, without conventional-commit prefixes or Claude authorship.
