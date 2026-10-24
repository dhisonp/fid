# Agent guidance

## Architecture

- `worker/src/lib.rs` owns routing and scheduled orchestration; `collect.rs` discovers and archives sources; `editorial.rs` selects, validates, audits, and rechecks stories; `store.rs` publishes atomically to D1 with evidence in R2.
- Scheduled and local verification editions require nine distinct stories ranked for technology/engineering leads: at most three AI stories and at least three non-AI tooling, infrastructure, studies, or substantive articles combined. Remaining slots are flexible; the reader has no category controls.
- Keep editorial instructions in `editorial.rs`, separate from untrusted evidence and history. AI-specific stories count toward the AI cap regardless of their other consequences. README and agent guidance are not runtime prompt inputs.
- Keep `worker/src/rules.rs` independent of Cargo dependencies and other Worker modules; offline checks compile it directly with `rustc`.
- `store.rs` embeds production queries from `sql/`; `tests/test_storage.py` executes those same files. Update both callers when parameters change, and preserve controlled database-clock tests for leases and publication deadlines.
- Coordinate reader response changes across `worker/src/types.rs`, `worker/src/lib.rs`, `web/src/api.ts`, and `tests/api.test.ts`. Keep internal categories, claims, scoring, and research inspection out of reader stories; preserve stored editions and original evidence dates.

## Code conventions

- Prefer idiomatic Rust, small interfaces, and existing dependencies; avoid speculative abstractions.
- Parse untrusted input into typed values at boundaries and propagate errors. Do not silently replace invalid data with defaults or weaken publication checks to fill the feed.
- Keep resource acquisition and side effects explicit. Bound collection and provider requests; lease expiry releases jobs without fallible cleanup.
- Comments explain reasons and invariants. Keep unrelated cleanup separate from functional changes.
- Run `npm run check`, `npm test`, `npm run build`, `npm run verify:runtime`, and `npm run audit` before committing. Distinguish builds and local runtime checks from live editorial and deployed scheduled verification.

## Git

- Use simple sentence commit messages, without conventional-commit prefixes or Claude authorship. Keep secrets, local state, and generated artifacts out of commits.
