# Fid

A daily briefing for engineers and technology leads. Fid publishes one feed of nine stories, ranked by relevance, with short summaries and links to originating sources.

The React reader runs on Cloudflare's free static hosting. A native Rust program collects sources and generates one edition per day in GitHub Actions. Page visits never start collection, generation, or model calls. No Workers Paid subscription, D1 database, or R2 bucket is required.

**Hosting can stay free; xAI is still a paid API.** The existing Grok search, generation, and independent audit require your `GROK_API_KEY` and available xAI credit. This repository does not provide a free model service. Running checks, building, and deploying without generation make no model calls.

## Deploy

Requires Node 24+, Python 3.11+, stable Rust, a GitHub repository, and a Cloudflare account. Use a private repository if the archived sources and internal editorial history should remain private: the `fid-data` branch stores both. Only the exported reader files are deployed to Cloudflare.

Install and verify locally:

```sh
npm ci
python3 -m pip install -r requirements-dev.txt
rustup component add rustfmt clippy
cargo install cargo-audit --locked
npm run check
npm test
npm run build
npm run verify:runtime
npm run audit
```

No WebAssembly target or `worker-build` installation is needed. The initial build exports an honest empty feed until a complete edition is generated.

Push this repository to GitHub. If it has no remote yet, the GitHub CLI can create a private repository:

```sh
gh auth login
gh repo create fid --private --source=. --remote=origin --push
```

If a repository already exists, configure its remote and push your default branch instead. GitHub runs scheduled workflows from the default branch.

Create a Cloudflare API token with **Account → Workers Scripts → Edit**, scoped to the account hosting Fid. Copy your account ID from the Cloudflare dashboard. Configure these in the GitHub repository:

```sh
gh secret set CLOUDFLARE_API_TOKEN
gh secret set GROK_API_KEY
gh variable set CLOUDFLARE_ACCOUNT_ID --body 'YOUR_CLOUDFLARE_ACCOUNT_ID'
```

The secret commands prompt for values; do not paste keys into source files or frontend environment variables. The Worker name defaults to `fid` in `wrangler.toml`; change it if that name is already used in your account. If your Cloudflare account has no `workers.dev` subdomain yet, initialize it with the local deployment command below before using CI.

To deploy the reader immediately without making model calls:

```sh
gh workflow run publish.yml -f generate=false
gh run list --workflow publish.yml --limit 1
gh run watch
```

To collect sources and publish the first edition now:

```sh
gh workflow run publish.yml -f generate=true
gh run watch
```

The workflow prints the deployed `https://fid.<your-subdomain>.workers.dev` address. It also runs automatically once a day at **11:37 UTC**, with no strict start-time guarantee. This is 07:37 in New York during daylight saving time and 06:37 during standard time. The edition date follows `America/New_York`. Re-running generation for a date that is already published skips collection and model calls, then can retry deployment.

For a direct local deployment:

```sh
npx wrangler login
npm run deploy
```

This uses editions in local `.fid/`, so use the GitHub workflow for production after it has accumulated history. To restore production state before a local deployment, run `python3 scripts/publication_state.py prepare .fid` from a clean checkout where `.fid` does not exist.

The static site itself has no Worker compute, database, object-storage, or cron billing. Daily Actions jobs have a 30-minute timeout (at most 930 runner minutes in a 31-day month for scheduled runs). GitHub Free includes 2,000 standard runner minutes per month for private repositories, shared with your other workflows and repositories; standard public-repository runners are free. CI and manual runs also consume private-repository minutes. Keep paid Actions usage disabled or set a zero spending budget in GitHub billing settings if you need a hard spending boundary. Public repository schedules can be disabled by GitHub after 60 days without activity; re-enable the workflow if needed. No deployment command upgrades either account to a paid plan.

## Run locally

```sh
npm run dev
```

Open **http://localhost:8787**. This builds the reader, exports saved editions, and serves the same static assets used in production. Stop with **Ctrl+C**. For frontend hot reload, run `npm run dev:web` in another terminal after building; keep `npm run dev` running so Vite can proxy edition data to the local static server.

To generate a local edition, configure the runner environment:

```sh
cp .env.example .env
```

Fill in `GROK_API_KEY` in `.env`, then load it into your shell and run:

```sh
set -a
. ./.env
set +a
npm run edition:daily
npm run build
```

Collection and generation take several minutes and make paid xAI requests. There is no local-verification edition or fixed publication hour: a successful run publishes today's daily edition. A fresh directory can be used for isolated runs:

```sh
cargo run --release --locked -- daily --state /tmp/fid-local
cargo run --locked -- export --state /tmp/fid-local --out web/dist
```

`npm run build` uses `.fid` by default. Never point the deployment at the runner's state directory; only `web/dist` is public.

## Daily publication

The workflow checks out the application and restores the separate `fid-data` branch. That branch contains a versioned state manifest, immutable edition revisions, and archived originating documents. SQLite rebuilds its index from those files in each fresh runner. History survives workflow-artifact expiration and cache eviction. Failed reads, corrupt archives, and unsupported state versions stop the run rather than reset history.

Once a day, the runner polls all configured feeds, observes the first ten Hacker News top stories and six new stories, and performs supplementary web and X searches. Feed collection inspects up to twelve entries per feed. Collection is a daily snapshot, not continuous monitoring; coverage records show failed sources without claiming hourly coverage. The evidence cutoff is captured after collection, and the freshness window is the preceding 24 hours.

The same Rust editorial rules select and validate nine stories, request an independent factual audit, and recheck the selected sources. Only a complete validated edition can commit. A failed attempt leaves the previous edition available, or an honest empty state before the first publication. The frontend recalculates stale status from New York's current date even if scheduled execution fails completely.

GitHub concurrency serializes publication jobs. SQLite transactions enforce publication ownership, deadlines, and revision consistency. The workflow saves publication history before deploying, so a deployment failure can be retried without regenerating an edition. Git pushes never force-overwrite the state branch. A scheduled job that fails generation still deploys the previous saved edition and reports failure in Actions. Corrective revisions already in saved state remain available at their original permalinks.

A static deployment contains the React app, `data/latest.json`, dated editions and revisions, source metadata, and evidence referenced by committed editions. Internal story categories, claims, scoring, model requests, and unpublished sources are excluded from reader stories. Build and runtime checks make no model calls and do not establish live editorial output quality.

## Editorial behavior

- Exactly nine distinct stories, ranked for engineering usefulness, technical depth, novelty, evidence, and lasting significance.
- At most three AI-specific stories, regardless of their other consequences.
- At least three non-AI stories covering developer tooling, infrastructure, substantive engineering articles, or studies combined. Remaining slots are flexible.
- Hacker News and X discovery receive a modest preference when engineering value and evidence quality are comparable. Discussions and search results are never factual evidence.
- Legal and regulatory coverage is exceptional and requires evidenced consequences across the broader technology industry. Exclude narrowly local compliance, employee disputes, gossip, and sensational news without engineering value.
- Every story needs a score of at least 70, fetched originating evidence, supported claims, and original publication dates. Research needs inspectable methods, results, and limitations; abstracts alone are insufficient.
- Keep original evidence dates and distinguish new reporting, material updates, carryovers, late discoveries, and older technical work. Never lower publication checks to fill the feed.

## Architecture

| Location                                                        | Responsibility                                                      |
| --------------------------------------------------------------- | ------------------------------------------------------------------- |
| `pipeline/src/main.rs`, `config.rs`, `time.rs`                  | Native daily runner, validated configuration, daily windows         |
| `pipeline/src/collect.rs`, `sources.rs`, `http.rs`              | Bounded feed/search discovery, safe HTTP, source extraction         |
| `pipeline/src/editorial.rs`, `rules.rs`, `model.rs`             | Selection, validation, model calls, independent audit, rechecks     |
| `pipeline/src/store.rs`, `sql/`, `migrations/`                  | SQLite transactions, immutable edition snapshots, evidence archives |
| `pipeline/src/export.rs`                                        | Public edition and evidence exports                                 |
| `.github/workflows/publish.yml`, `scripts/publication_state.py` | Daily schedule, durable Git history, static deployment              |
| `web/src/`, `wrangler.toml`                                     | Unchanged reader UI and free static hosting                         |

The reader retains its current layout, five-minute refresh, original dates, and edition/revision permalinks. Its data adapter maps reader requests to static JSON. The hosted app has no admin API or live database. Runner configuration belongs in environment variables or GitHub secrets and variables, never the frontend.

Optional repository variables `MODEL_NAME`, `MODEL_INPUT_USD_PER_MILLION`, and `MODEL_OUTPUT_USD_PER_MILLION` configure the model and token cost estimates. The model must support JSON chat responses and Responses API `web_search` and `x_search`. Blank prices mean unknown cost; search tool charges remain unpriced.

## Checks

```sh
npm run check           # formatting, lint, and TypeScript
npm test                # Rust, native publication, production SQL, Git state, reader data
npm run build           # reader and static edition exports; no model calls
npm run build:pipeline  # optimized native Rust executable
npm run verify:runtime  # static deployment packaging dry run; does not deploy
npm run audit           # Rust and production npm dependency advisories
```

`npm run format` applies formatting. `npm run test:offline` checks standalone Rust rules, storage SQL, Git publication state, reader parsing, and rustfmt without downloading dependencies. CI runs the full checks, build, packaging dry run, and dependency audits.
