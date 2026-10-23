# Edition request revision

The minimum useful story is an accurate headline, a brief explanation of what changed and why it matters, and links to originating sources. The revised prompt asks for an array containing one paragraph, normally 45–80 words, with extra space allowed for consequential facts and qualifications. Readers follow the sources for details.

## Scope and request shape

Request changes are in `worker/src/editorial.rs` and `worker/src/model.rs`; `worker/src/lib.rs` also accepts an optional supplied draft on the existing local verification endpoint. The story/API schema, frontend, source collection, production category counts, model, reasoning effort, output ceiling, local provenance validation, independent audit, source rechecks, deadline and atomic publication remain intact. No benchmark infrastructure or dependency was added.

Generation already batches all candidates into one draft request and only the selected documents into one audit request. Retain those two calls: combining them would remove the independent review. Keep full originating documents in both stages; quote-only auditing and arbitrary source truncation could hide contradictory context or limitations. Remove storage metadata from model evidence and repeated verification ledgers from history. Retain all history event identities, last edition, category, label, headline, brief, delta, uncertainty and research inspection for continuity and duplicate review.

Both requests keep this provider envelope:

```json
{
  "model": "grok-4.7",
  "store": false,
  "response_format": { "type": "json_object" },
  "max_completion_tokens": 24000,
  "reasoning_effort": "medium",
  "messages": [
    { "role": "system", "content": "stage instructions from editorial.rs" },
    { "role": "user", "content": "serialized JSON data" }
  ]
}
```

`medium` is the existing local verification setting; scheduled generation keeps `high`. The draft data contains edition date, window start, cutoff, full evidence and compact previous events. The audit data contains the same window, selected full evidence, complete draft and compact history. Previously the audit had no window/cutoff and inherited the entire drafting contract, including a conflicting stories output format. Its revised instructions specify only the audit response. Source text remains separate from instructions.

Exact comparison requests and full responses are in the local ignored directory [artifacts/edition-request-comparison](../artifacts/edition-request-comparison/): `before-draft-request.json`, `revised-draft-request.json`, the corresponding audit requests, `*-response.json`, and `*-measurement.json`. They contain no API credentials. The short comparison runner is a disposable `.cache` file, not a new repository tool.

## Payload fields

| Input                                                                                            | Purpose and token decision                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `edition_date`, `window_start`, `cutoff`                                                         | Keep the date and two timestamps; needed for freshness and post-cutoff exclusion. Also send them to the audit.                                                                                                                                                                                         |
| Evidence `id`, `url`, `title`                                                                    | Keep identity, originating link and document context. URLs are supplied once, then referenced by ID.                                                                                                                                                                                                   |
| Evidence `published`, `updated`, `text`                                                          | Keep original date precision and complete inspectable text. This is the dominant useful input; no snippets or arbitrary truncation.                                                                                                                                                                    |
| Evidence `source_id`, `first_seen`, `retrieved_at`, `content_hash`                               | Omit from model data: feed routing, snapshot admission and fingerprints are enforced in code/storage. Full records stay archived for local checks and publication. Removing these six documents' bookkeeping saves 923 serialized data characters per draft.                                           |
| History event ID, last edition, category, label, headline, paragraphs, delta, uncertainty, paper | Keep all events and consequential prior coverage for continuity, material updates and semantic duplicates. Do not filter history by event ID alone.                                                                                                                                                    |
| History fingerprint, claim quotes, evidence IDs, score, rationale                                | Omit from model history: local continuity still uses the original fingerprint; old quote ledgers and selection explanations repeat information unnecessary for comparing prior coverage. History is empty in this sample, so savings and quality effects here are expectations, not measured evidence. |
| Audit stories and evidence                                                                       | Keep the complete candidate draft, including verification fields, and full selected documents together in one request. Approval IDs and issues remain the entire audit output.                                                                                                                         |

## Requested fields

The provider returns request totals, not billed field totals. A separate call to its [tokenizer](https://docs.x.ai/developers/rest-api-reference/inference/other) measured serialized groups of values for each field. These grouped text counts describe field cost; they exclude hidden reasoning and differ from actual output formatting/overhead. They are not additive billing components.

| Field                                                 | Purpose and cost                                                                             | Revision                                                                                                                                                                                           |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `event_id`                                            | Verification: continuity and duplicate identity; small repeated identifier.                  | Retain stable IDs and semantic duplicate review.                                                                                                                                                   |
| `category`                                            | Reader grouping and count safeguard; small enum.                                             | Retain; replace the long category table and repeated routing rules with one compact list per request.                                                                                              |
| `headline`                                            | Reader's first useful fact; ledger must support it.                                          | Retain a sharp, accurate headline.                                                                                                                                                                 |
| `paragraphs`                                          | Reader explanation; previously unbounded dense paragraphs also multiplied ledger/audit work. | One normally 45–80 word paragraph; preserve affected scope, versions, mitigation, attribution and qualifications.                                                                                  |
| `claims[].text`                                       | Verification: explicit material assertion; necessarily overlaps public prose.                | Retain coverage of every material headline/prose assertion; the auditor also checks delta, uncertainty and research panels. Keep this overlap because exact quotes alone cannot establish support. |
| `claims[].kind`                                       | Verification: confirmed, attributed, inference or uncertain.                                 | Retain; brevity cannot change epistemic status.                                                                                                                                                    |
| `claims[].evidence_id`                                | Verification: binds assertion to originating document.                                       | Retain; validate membership locally.                                                                                                                                                               |
| `claims[].quote`                                      | Verification and inspectable support; often the largest repeated source text.                | Shortest sufficient exact contiguous quote, still at least 24 characters and semantically supporting the assertion. Keep full document context for the auditor.                                    |
| `evidence_ids`                                        | Reader links and local provenance/fingerprints; overlaps claim references.                   | Retain the existing schema and unique originating documents actually used. No model-generated URLs.                                                                                                |
| `label`                                               | Reader freshness and continuity safeguard.                                                   | Retain all five labels, original dates and date precision.                                                                                                                                         |
| `delta`                                               | Reader's consequential difference from earlier coverage; can duplicate prose.                | Short, updates only; treat it and the paragraph as one brief.                                                                                                                                      |
| `uncertainty`                                         | Reader qualification; can duplicate prose.                                                   | Specific and short when needed; empty otherwise; never invent uncertainty.                                                                                                                         |
| `score`                                               | Internal selection threshold and ranking.                                                    | Retain 70–100 and selection criteria.                                                                                                                                                              |
| `rationale`                                           | Internal selection explanation; previously an unbounded second significance summary.         | One short phrase, without repeating the brief.                                                                                                                                                     |
| `paper.version`, `paper.status`                       | Verification of inspected version and review status.                                         | Retain evidenced version and preprint/peer-reviewed/lecture status.                                                                                                                                |
| `paper.methods`, `paper.results`, `paper.limitations` | Inspection record; duplicates details if expanded into an article.                           | One concise sentence each; inspect full paper and preserve consequential limitations in the brief.                                                                                                 |
| `paper.artifacts`, `paper.executed`                   | Verification of identified original artifact URLs and no execution.                          | Retain original URLs, local URL checks and `false`; non-research remains `null`.                                                                                                                   |
| Audit `approved_events`, `issues`                     | Publication gate: exact approval set and all failures.                                       | Retain the gate; successful audit need not rewrite prose or repeat evidence.                                                                                                                       |

| Serialized field group | Baseline tokens | Revised tokens |
| ---------------------- | --------------: | -------------: |
| `event_id`             |              93 |             78 |
| `category`             |              25 |             25 |
| `headline`             |             133 |             96 |
| `paragraphs`           |           2,815 |            712 |
| `claims`               |          11,302 |          5,498 |
| `claims[].text`        |           2,994 |          1,122 |
| `claims[].kind`        |             306 |            152 |
| `claims[].evidence_id` |           3,842 |          2,219 |
| `claims[].quote`       |           2,683 |          1,158 |
| `evidence_ids`         |             244 |            243 |
| `label`                |              25 |             21 |
| `delta`                |              10 |             10 |
| `uncertainty`          |             414 |            171 |
| `score`                |              17 |             17 |
| `rationale`            |             312 |            105 |
| `paper`                |              17 |             17 |

`claims` already includes its subfields; do not add those rows twice. All six `paper` values are null, so research subfield costs and quality are unmeasured. Long evidence identifiers still account for 2,219 tokens in the grouped revised claim references; retaining the existing identifier scheme avoids a new remapping interface.

Tokenized draft instructions fell from 1,120 to 1,037 tokens; audit instructions fell from 1,319 to 604. These text-only instruction counts exclude provider message framing.

Repeated instructions removed: the audit's drafting schema, the category table scaffolding, secondary-tag advice with no corresponding output field, the repeated category-boundary footer, and the redundant off-schedule explanation already provided by verification selection. Instructions preventing injection, unsupported claims, filler, mistaken dates, false corroboration and abstract-only research remain explicit.

## Measurement and factual review

The local D1 database contained one canceled generation run and seven search-usage rows, but no draft/audit usage or response archives. Search records have no reasoning breakdown or per-request timing. They cannot establish an edition-generation baseline. The comparison therefore sends fresh requests using the archived originating documents, without recollecting or changing the evidence.

Both variants start from the same six complete documents, in the same order, with empty history, `grok-4.7`, `medium` reasoning, the same 24,000-token ceiling and the same fixed window (2026-10-22 21:34:30 UTC through 2026-10-23 21:34:30 UTC). The comparison selects two stories in each of Security, Industry & Policy, and Development & Infrastructure. That smaller selection replaces only the count/scope sentence in both prompts; production quotas are unchanged. No publication is attempted. Audits are diagnostic even when local checks would already withhold the draft. Evidence text and metadata are retained in `evidence.json` (SHA-256 `ecba75df06a1e4afa1789d4d83d64e5c687e05fc0f91abc3fc61fa492a3c8b98`).

| Originating document                                                                                                                           | Essential facts checked                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [OpenSourceMalware: Python packages and Node2Shell](https://opensourcemalware.com/blog/node2shell-affects-python)                              | Affected Python packages and fixed versions; shared decoder exposure; scanner blind spot; attribution and the distinction between testing performed and memory corruption/code execution not pursued. |
| [Cloudflare full transaction WAF](https://blog.cloudflare.com/full-transaction-waf/)                                                           | Request/response correlation, availability including Free, response blocking cannot undo origin actions, response-body rules unavailable.                                                             |
| [EFF: Florida and Georgia age gates](https://www.eff.org/deeplinks/2026/10/two-states-can-now-restrict-social-media-access-heres-what-do-next) | Lifted injunctions; Florida under-14 ban and 14–15 consent; Georgia under-16 consent; age-check/privacy effects and continuing litigation; all attributed to EFF.                                     |
| [EFF: Bill C-22](https://www.eff.org/deeplinks/2026/10/canadians-are-running-out-time-oppose-dangerous-c-22-surveillance-bill)                 | House-passed bill before Senate, not enacted law; proposed surveillance/backdoor authority and condition; up-to-one-year retention; a U.S. data-sharing agreement is separately under negotiation.    |
| [Cloudflare Origin Range Requests](https://blog.cloudflare.com/origin-range-requests/)                                                         | Self-service/all plans; chunk-aligned fetches; approximate R2 p99 and traffic improvements attributed to Cloudflare; origin byte-range support and unchanged cache eligibility.                       |
| [Rust 1.99.1](https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/)                                                                              | Original October 15 date and older-news label; all three regressions: UEFI/wcslen, x86_64 FFI bool, macOS linker.                                                                                     |

The first broader rewrite is retained as `after-*` exploratory artifacts, not the final proposal. Its draft reduced completion tokens from 15,586 to 6,782 but increased reasoning from 23,255 to 31,180 and latency from 337.387 to 356.110 seconds. It selected five rather than six stories, omitting WAF. Exact quotes all matched, yet its “Python memory corruption was not reproduced” overstates a source that says it did not pursue that testing, and its C-22 paragraph blurred the separately negotiated U.S. agreement into the bill. These results motivated retaining more of the original editorial contract in the narrower final revision.

The baseline selected all six stories but misassigned the WAF evidence ID to all 25 Python-vulnerability claims. Local quote validation would reject those references. Its diagnostic audit approved four events, rejecting that citation failure and unsupported/inadequately labeled WAF claims. No duplicate event IDs or underlying events were found. Because source selection follows returned IDs, the baseline audit received five distinct documents and the final audit receives six; this affects audit-stage input comparisons. Both draft requests use the exact same six-document evidence set.

The narrower revised draft retains all six stories and all 58 claim quotes match their cited documents (including the 24-character floor). It contains 477 paragraph words, versus 1,976 and 100 claims in the baseline. It keeps all three Rust regressions, the two states' differing age/consent rules, the C-22 bill/Senate status and separate agreement negotiation, WAF's inability to undo origin actions, source attribution, and the vulnerability's fixed versions and testing caveat. Its source-linked brief omits root-cause walkthroughs, API field catalogs, illustrative transactions and advocacy contact details. A manual review flags remaining qualification risks, including Florida's compressed platform-coverage description and a range-cache uncertainty statement about measurement windows despite the source's week-over-week traffic figure. Exact quote validity alone does not settle these issues.

API comparison testing was stopped at the user's request after the revised draft completed. The pending revised audit was canceled; its token use and duration are unknown, and no end-to-end improvement or final audit approval is claimed.

| Stage / measure                                 |              Baseline | Final revised draft |     Change |
| ----------------------------------------------- | --------------------: | ------------------: | ---------: |
| Draft input tokens                              |                13,979 |              13,475 |      −3.6% |
| Draft completion tokens (excludes reasoning)    |                15,586 |               7,172 |     −54.0% |
| Draft reasoning tokens                          |                23,255 |              27,375 |     +17.7% |
| Draft total tokens                              |                52,820 |              48,022 |      −9.1% |
| Draft elapsed seconds                           |               337.387 |             297.891 |     −11.7% |
| Diagnostic audit input / completion / reasoning | 26,116 / 275 / 22,880 |   Canceled; unknown | Unmeasured |
| Diagnostic audit elapsed seconds                |               262.854 |   Canceled; unknown | Unmeasured |
| Sum of both request durations                   |       600.241 seconds |         Unavailable | Unmeasured |

The two completed drafts returned the same provider model, system fingerprint and default service tier. Each saw 1,152 cached prompt tokens. These are single runs with overlapping request schedules, not a latency distribution or a controlled provider-load experiment. Timings include the provider request/response, not collection, R2/D1 operations, source rechecks or publication. The later local generation is operational verification, not another controlled comparison.

Measured: substantially shorter reader prose and completion output, modest input reduction and a faster revised draft, with increased reasoning. Expected but unproven: shorter audit processing from its smaller instructions and draft, savings on nonempty history, and lower total edition latency. Risks: compression can narrow scope or overstate certainty; unknown-date, research, carryover and changed-event history behavior are not exercised by this small evidence set. Keep the independent audit and publication safeguards. This comparison alone does not establish publication readiness.

## Runtime capture and validation

Requests are archived before sending so cancellation leaves the actual payload. Responses now retain the entire provider JSON and elapsed request milliseconds, including usage details, before typed parsing and publication checks. Missing reasoning counts stay unknown. The existing D1 usage row stores completion and reasoning breakdowns and elapsed time in `detail`; no schema migration is needed.

The previous output accounting used only `completion_tokens`. xAI's [Chat Completions example](https://docs.x.ai/developers/rest-api-reference/inference/chat-completions) reports prompt 32, completion 9, reasoning 94 and total 135: reasoning is additional to the completion field. Output accounting and configured token cost now use `total_tokens - prompt_tokens`, with the provider's completion and reasoning counts retained separately. Historical rows are not rewritten. Configured prices are blank here, so dollar savings remain unknown.

Rust tests cover preserving full evidence and date precision, keeping history's consequential content while removing repeated quote ledgers, rejecting malformed stored history, and distinguishing reasoning/completion/total usage. Existing tests still exercise provenance, dates, counts, continuity, SQL publication, deadlines, leases and API parsing.

## Requested local verification

The user requested three stories in each of the three active categories (nine total), a database review, a commit, and shutdown of local instances. Comparison API calls were stopped. The implemented Worker was run against an isolated copy of local storage at `.cache/edition-verification-state`, using nine relevant complete originating documents; the original `.wrangler/state` was preserved. No model-powered search or collection calls were made.

The first operational draft used 22,051 input tokens and 38,160 generated tokens (9,439 completion plus 28,721 reasoning), taking 351.506 seconds. It returned eight stories: three Security, two Industry & Policy, and three Development & Infrastructure. The existing count check rejected it before any audit call. The interoperability opinion did not qualify; no threshold was lowered to fill the missing row.

A concrete EFF account of the October 20 OPM/DOGE summary-judgment ruling was found in the already archived feed and fetched directly. Extraction used the production `collect::extract` function; original source dates, full text and raw HTML were archived in the isolated R2 bucket. It replaced the opinion for the requested retry. The completed failed run remains recorded. The local fixture's lease was reset only after verifying that no run was active and no edition existed; production lease code and database-clock safeguards were unchanged.

The retry used 21,349 input tokens and 41,177 generated tokens (7,352 completion and 33,825 reasoning), taking 415.159 seconds. It returned seven stories: three Security, two Industry & Policy and two Development & Infrastructure. The count safeguard again prevented an audit/publication. It selected the OPM ruling but omitted C-22 and Kubernetes.

Together the retained drafts contain nine distinct supported event candidates. To avoid another expensive generation call, the existing localhost-only, authenticated, `LOCAL_VERIFICATION=true` endpoint now optionally accepts a typed `Draft` JSON body (maximum 500 KB). An empty body preserves generation behavior. Scheduled generation cannot accept supplied drafts. A supplied draft is archived and passes through the same count/date/quote/provenance checks, independent audit, source rechecks, deadline and atomic publication. It does not bypass any publication gate. This is an option on the existing local verification route, with no new framework, model, production workflow or public response format.

For the requested local edition, the two drafts were combined, speculative questions about unreported details were removed, the brotlicffi claim was limited to the checked 1.2.0.x line, and the Python test wording was corrected to “did not pursue” rather than implying a negative reproduction result. The range-cache traffic claim retains R2 and similar workloads and the week-over-week period. The combined nine-story draft is being audited without another draft-generation request.

The combined draft passed all local validation. Its first independent audit used 31,138 input tokens, 168 completion tokens and 25,121 reasoning tokens (249.198 seconds), approving eight stories and rejecting OPM wording that implied agents left after the ruling rather than since filing. It also required the source's “millions of records” scope. The OPM paragraph and claim ledger were corrected to retain both facts, then submitted for another required audit without regenerating the draft.

The audit caught further scope/quotation issues during review (appeal status, the qualification on origin range support, measurement attribution, a C-22 claim/quote mismatch, the exact brotlicffi versions, and conditional LLVM behavior). Each failure remained unpublished. The supplied draft and ledger were corrected against the full documents; no additional draft generation was performed.

The final audit approved all nine event IDs with no issues. All nine source rechecks passed and the Worker atomically published verification edition `2026-10-23`, revision 1. The successful supplied-draft request took 267.596 seconds, including audit, rechecks and publication. Its audit used 30,836 input, 132 completion and 25,715 reasoning tokens in 265.942 seconds. These are operational measurements, not a controlled comparison against the six-document baseline.

Database review confirmed exactly three rows in each active category, nine unique events and history records, nine committed source references, one complete paragraph per story, all 78 quotes matching their cited full documents, and HTTPS originating links. The public response matches the stored story order and excludes claims, research-inspection/selection metadata and full evidence text. Paragraphs span 60–111 words; consequential qualifications remain in the prose rather than speculative questions. The other four categories remain WIP.

Review artifacts: [nine stored briefs and checks](../artifacts/local-database-review.json), [final audit](../artifacts/local-final-audit.json), and [all operational attempts and usage](../artifacts/local-generation-usage.json). The database remains at `.cache/edition-verification-state/v3/d1/miniflare-D1DatabaseObject/9ba2b04bf514d9facfd57ed57d849e77241a7adc99d1c1545d06688b43d84248.sqlite`. The original local database was preserved.

Across the two operational draft calls and five required audits, measured usage was 198,130 input, 17,756 completion and 198,625 reasoning tokens. These retries are a real cost of the verification exercise; they are not counted as optimization savings. Controlled comparison requests and the canceled comparison audit are separate. The canceled request's usage remains unknown.

Validation: 17 Rust tests, Clippy with warnings denied, Wasm Worker build, seven SQL/storage tests and five API-parser tests passed. Rust formatting, report formatting and diff checks passed. No frontend source edits are part of this change.
