# Fid

A daily briefing for technology and engineering leads. Fid publishes one feed of nine stories, ranked by relevance, with short summaries and links to originating sources.

## Run locally

After completing the first-time setup below, run these commands from the repository root:

```sh
npm run db:local
npm run dev:verify
```

Keep that terminal running and open **http://localhost:8787**. Wrangler builds and serves both the reader and backend; no separate build or frontend server is needed.

A fresh database shows **“No edition is available yet.”** Starting the server does not generate stories, and local cron jobs do not run automatically. To collect sources and generate your first edition, open a second terminal in the repository and run:

```sh
export ADMIN_TOKEN='the same ADMIN_TOKEN you set in .dev.vars'
npm run verify:live
```

Generation takes several minutes and makes paid xAI requests. The page refreshes its data automatically every five minutes; reload to see the result immediately. Failed source or audit checks leave the edition unpublished; details appear in the command output.

Local state persists in `.wrangler/`. If an existing edition is stored in another directory, start the server with `npm run dev -- --persist-to <state-directory>` to read it. Use the same persistence directory for migrations, collection, and reading. Stop the server with **Ctrl+C**.

For frontend hot reload, keep the Worker running and run `npm run dev:web` in another terminal; open the Vite URL it prints.

## First-time setup

Requires Node 24+, Python 3.11+, stable Rust, and network access to package registries, originating sources, and xAI.

```sh
npm ci
python3 -m pip install -r requirements-dev.txt
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
cargo install worker-build --locked
cargo install cargo-audit --locked
cp .dev.vars.example .dev.vars
```

Set `GROK_API_KEY` and an `ADMIN_TOKEN` of at least 24 characters in `.dev.vars`. Never commit real secrets. `wrangler.toml` configures the model, endpoint, and optional token prices; the model must support JSON chat responses and Responses API `web_search`. Blank prices mean unknown cost, and search tool charges remain unpriced.

## Editorial behavior

- At most three stories are AI-specific, including AI-focused tooling, infrastructure, research, policy, and acquisitions.
- At least three non-AI stories cover developer tooling, infrastructure, substantive engineering articles, or studies combined. Selection favors a mix within that minimum.
- The remaining slots are flexible. Acquisitions, emerging technologies, licensing, regulation, and platform pricing or subscription changes qualify through concrete industry impact or decision value. Unused AI slots are flexible too.
- Internal topics support selection and diversity checks. The reader has no category navigation, and reader stories omit internal classifications, claims, scores, and research inspection records.

Every story needs a score of at least 70, fetched originating evidence, and supported claims. Generation produces one draft followed by an independent factual audit; selected sources are rechecked before publication. Research requires full inspectable methods, results, and limitations, rather than an abstract alone. PDF extraction and research artifact execution are unsupported.

Stories retain original publication dates and distinguish new reporting, material updates, carryovers, late discoveries, and older technical work. Insufficient evidence or failed checks withhold the edition; readers keep the previous edition with a delay notice, or see an empty state before the first publication.

## Architecture

The Rust Worker runs on Cloudflare Workers, using D1 for publication state and private R2 storage for archived sources. React, TypeScript, and Vite provide the reader.

| Location                                          | Responsibility                                                                  |
| ------------------------------------------------- | ------------------------------------------------------------------------------- |
| `worker/src/lib.rs`, `time.rs`                    | API routes, scheduled jobs, leases, and New York publication windows            |
| `worker/src/collect.rs`, `sources.rs`, `http.rs`  | Feed/search discovery, bounded source fetching, and evidence extraction         |
| `worker/src/editorial.rs`, `rules.rs`, `model.rs` | Selection, validation, model requests, factual audit, and source rechecks       |
| `worker/src/store.rs`, `sql/`, `migrations/`      | Evidence archives, database queries, atomic publication, and retained revisions |
| `web/src/`                                        | Reader rendering, response validation, and edition permalinks                   |

A quarter-hour cron distributes feed checks across the hour and observes Hacker News. Supplementary searches discover originating documents; search results themselves are not factual evidence. Generation starts at 06:30 `America/New_York` and retries until 07:30, accounting for daylight saving time. Reader traffic never starts collection or generation.

D1 checks generation leases and deadlines against its own clock. Publication commits the edition, revision, history, and evidence references atomically. Corrections retain earlier revisions and preserve event identities. Only evidence referenced by a committed edition is available through the evidence API.

## Local publication details

`npm run dev:verify` enables publication outside the scheduled window. Verification requires localhost, uses the preceding 24 hours, and marks the edition with `verification: true`. It uses the same nine-story rules and full collection scope as scheduled publication, with `medium` rather than `high` reasoning.

`npm run verify:live` collects originating sources, generates and audits a draft, rechecks evidence, and writes `artifacts/live-verification.json`. Set `FID_ORIGIN` if the Worker uses a different address. A second edition for the same date returns `409`; use a fresh `--persist-to` directory for both migrations and the dev server to repeat.

The local verification endpoint also accepts a JSON draft containing `stories`. Supplied drafts still pass validation, independent audit, source rechecks, and atomic publication against locally archived evidence.

## Checks

```sh
npm run check           # formatting, lint, and TypeScript
npm test                # Rust rules/editorial logic, production SQL, and API parsing
npm run build           # reader and release Wasm Worker
npm run verify:runtime  # deployment packaging dry run; does not deploy
npm run audit           # Rust and production npm dependency advisories
```

`npm run format` applies formatting. `npm run test:offline` checks standalone Rust rules, storage SQL, API parsing, and rustfmt without downloading dependencies. CI runs the full checks, builds, packaging dry run, and dependency audits.

For runtime verification, exercise the built Worker with local D1/R2 state and check the reader at mobile and desktop widths. Builds, stored-edition reads, and browser checks do not establish fresh model output quality or successful deployed scheduled execution.

## API

| Public route                             | Response                                                                 |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| `/api/latest`                            | Latest edition, availability/stale status, and expected publication date |
| `/api/editions`                          | Recent edition dates and revision metadata                               |
| `/api/editions/YYYY-MM-DD[/revisions/N]` | Edition or retained revision                                             |
| `/api/evidence/ID`                       | Archived source referenced by a published edition                        |
| `/api/sources`                           | Configured source list                                                   |

Reader permalinks use `/editions/YYYY-MM-DD[/revisions/N]`. Operator routes under `/api/admin/` require `Authorization: Bearer <ADMIN_TOKEN>` and provide collection, scheduled generation, local verification, usage, and corrections; their request contracts are in `worker/src/lib.rs`.

## Deployment

Configure a Cloudflare Workers paid plan for the CPU budget in `wrangler.toml`, then create the storage resources:

```sh
npx wrangler login
npx wrangler d1 create fid
npx wrangler r2 bucket create fid-evidence
```

Replace the placeholder D1 database ID, configure the model and token prices, and keep `LOCAL_VERIFICATION=false` in deployment. After local runtime and real-source verification:

```sh
npx wrangler secret put GROK_API_KEY
npx wrangler secret put ADMIN_TOKEN
npx wrangler d1 migrations apply fid --remote
npm run deploy
```

Deployment publishes assets, Worker code, bindings, and cron configuration. Check the public reader, scheduled execution, first complete edition, coverage, and usage after deploying.
