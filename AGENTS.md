# Agent guidance

## Architecture

- `pipeline/src/main.rs` runs daily collection and generation as a native Rust executable in GitHub Actions. `collect.rs` discovers and archives sources; `editorial.rs` selects, validates, audits, and rechecks stories; `store.rs` publishes atomically through SQLite and saves immutable edition revisions and originating evidence on the separate `fid-data` Git branch.
- `export.rs` exports only committed reader data into `web/dist/data`; Cloudflare serves static assets without Worker code, D1, R2, or cron. Preserve the React reader UI and edition/revision permalinks. Recompute reader freshness locally so failed schedules cannot leave an old edition marked current indefinitely.
- Collection and generation run once daily with no fixed local-time requirement. Use a post-collection cutoff, the preceding 24 hours, and a New York edition date. Skip collection and model calls if that date is already published. Keep concurrent publication serialized and persist history before deployment.
- Daily editions require nine distinct stories ranked for technology/engineering leads: at most three AI stories and at least three non-AI tooling, infrastructure, studies, or substantive articles combined. Remaining slots are flexible; the reader has no category controls.
- Favor stories that keep engineers current with the tech scene and help them grow. Legal/regulatory coverage is exceptional and requires evidenced consequences across the broader technology industry; exclude narrowly local or niche-sector compliance, employee disputes and sensational news without engineering value.
- Keep editorial instructions in `editorial.rs`, separate from untrusted evidence and history. AI-specific stories count toward the AI cap regardless of their other consequences. README and agent guidance are not runtime prompt inputs.
- Keep `pipeline/src/rules.rs` independent of Cargo dependencies and other modules; offline checks compile it directly with `rustc`.
- `store.rs` embeds production queries from `sql/`; `tests/test_storage.py` executes those same files. Update both callers when parameters change, and preserve controlled database-clock tests for leases and publication deadlines.
- Coordinate reader data changes across `pipeline/src/types.rs`, `pipeline/src/export.rs`, `web/src/api.ts`, and `tests/api.test.ts`. Keep internal categories, claims, scoring, and research inspection out of reader stories; preserve stored editions and original evidence dates.
- Hosting must work on free Cloudflare static hosting and standard GitHub runners within free quotas. Do not add billable hosting resources. xAI calls still use paid provider credit; checks and deployment-only runs must not call models.

## Code conventions

- Prefer idiomatic Rust, small interfaces, and existing dependencies; avoid speculative abstractions.
- Parse untrusted input into typed values at boundaries and propagate errors. Do not silently replace invalid data with defaults or weaken publication checks to fill the feed.
- Keep resource acquisition and side effects explicit. Bound source and provider requests. Reject corrupt or unsupported saved state rather than losing history.
- Comments explain reasons and invariants. Keep unrelated cleanup separate from functional changes.
- Run `npm run check`, `npm test`, `npm run build`, `npm run verify:runtime`, and `npm run audit` before committing. Distinguish builds and local runtime checks from live editorial and deployed daily verification.

## Git

- Use simple sentence commit messages, without conventional-commit prefixes or Claude authorship. Keep secrets, local state, and generated artifacts out of commits to the application branch. Published snapshots belong only on the data branch.
