# Fid

Daily technical briefing: Rust on Cloudflare Workers, React/TypeScript/Vite reader, D1 publication state, private R2 evidence archives.

Real-source publication, Workers runtime packaging, desktop/mobile browser usability, and deployment remain unverified.

## Setup

Requires Node 24+, Python 3.11+, current stable Rust, and network access to package registries, source sites, and xAI.

```sh
npm install
python3 -m pip install -r requirements-dev.txt
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
cargo install worker-build --locked
cargo install cargo-audit --locked
cp .dev.vars.example .dev.vars
```

Set `GROK_API_KEY` and `ADMIN_TOKEN` (at least 24 characters) in `.dev.vars`. Configure the model in `wrangler.toml`; it must support JSON output and Responses API `web_search`. The Grok key serves both generation and search. Blank token prices mean unknown cost; search tool charges are unpriced. Retain both lockfiles.

## Local development

```sh
npm run db:local
npm run dev
```

Wrangler builds and serves the Worker and reader; state persists in `.wrangler/`. Optional `npm run dev:web` enables frontend hot reload and proxies `/api` to port 8787.

For real-source verification outside the publication window, restart with `npm run dev:verify`. In another shell, export the same `ADMIN_TOKEN`, then run `npm run verify:live`. Set `FID_ORIGIN` if the Worker uses a different local address.

Verification requires localhost, marks the API edition with `verification: true`, and writes `artifacts/live-verification.json`. Source or audit failures can prevent publication. A second publication for the same date returns `409`; use a separate local persistence directory to repeat. Keep `LOCAL_VERIFICATION=false` in deployment.

Local verification currently requests three stories in each of Security, Industry & Policy, and Development & Infrastructure, using `medium` reasoning. The other four categories have no stories in this edition. Verification collection polls feeds in those three categories and runs only their three supplementary searches. Scheduled publication still requires 4–7 stories in all seven categories with `high` reasoning; evidence validation, factual audit, and source rechecks remain required.

## Checks

```sh
npm run format          # apply formatting
npm run check           # formatting, lint, types; required before committing
npm test                # Rust, SQLite, frontend parser
npm run build           # frontend and Wasm Worker
npm run verify:runtime  # Wrangler deployment dry run
npm run audit           # Cargo and production npm dependencies
```

Without downloaded dependencies, `npm run test:offline` checks standalone Rust rules, SQL against SQLite, the frontend parser, and rustfmt. Host tests and builds do not verify Workers bindings, live editorial quality, or browser usability.

## Runtime

- A quarter-hour cron polls sources hourly and observes HN. Generation starts at 06:30 `America/New_York`, retries until 07:30, and accounts for DST. Reader requests never trigger generation.
- Publication requires 4–7 supported entries in each of seven categories. Insufficient evidence leaves the previous edition available with stale status; before the first publication, the reader shows no edition.
- D1 atomically commits publication; retries cannot overwrite an edition. Corrections create retained revisions. R2 stays private; only evidence referenced by committed editions is publicly exposed. Research requires inspectable full HTML; PDF extraction and artifact execution are unsupported.
- Public API: `/api/latest`, `/api/editions`, `/api/editions/YYYY-MM-DD[/revisions/N]`, `/api/evidence/ID`, `/api/sources`. Reader permalinks: `/editions/YYYY-MM-DD[/revisions/N]`.
- `/api/admin/*` requires `Authorization: Bearer <ADMIN_TOKEN>`. Collect, generate, verify, usage, and correction handlers are in [worker/src/lib.rs](worker/src/lib.rs).

## Deployment

Complete local runtime, real-source, and desktop/mobile verification first. A Workers paid plan is required for the configured CPU and subrequest budgets.

```sh
npx wrangler login
npx wrangler d1 create fid
npx wrangler r2 bucket create fid-evidence
```

Replace the placeholder D1 database ID in `wrangler.toml`, configure the model/prices, and leave local verification disabled.

```sh
npx wrangler secret put GROK_API_KEY
npx wrangler secret put ADMIN_TOKEN
npx wrangler d1 migrations apply fid --remote
npm run deploy
```

Deployment includes assets, Worker, bindings, and cron. Verify the public URL, scheduled execution, first published edition, coverage, and usage. CI checks without deploying.
