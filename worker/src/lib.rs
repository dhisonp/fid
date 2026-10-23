mod collect;
mod editorial;
mod http;
mod model;
mod rules;
mod sources;
mod store;
mod time;
mod types;

use crate::{
    store::{error, Store},
    types::{Draft, PublicEdition},
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use worker::*;

fn owner() -> String {
    format!("{}-{}", js_sys::Date::now(), js_sys::Math::random())
}

async fn deadline<T>(
    future: impl std::future::Future<Output = Result<T>>,
    seconds: u64,
) -> Result<T> {
    let timeout = Delay::from(std::time::Duration::from_secs(seconds));
    futures_util::pin_mut!(future, timeout);
    match futures_util::future::select(future, timeout).await {
        futures_util::future::Either::Left((result, _)) => result,
        futures_util::future::Either::Right(_) => Err(error(
            "Execution budget exhausted; publication remains incomplete",
        )),
    }
}

async fn generate(env: &Env, store: &Store) -> Result<()> {
    let now = time::now();
    let window = time::today(now)?;
    rules::generation_window(window.previous, window.cutoff, window.deadline, now)
        .map_err(error)?;
    if store.edition(Some(&window.date), None).await?.is_some() {
        return Ok(());
    }
    let run = owner();
    if !store.acquire("generation", &run).await? {
        return Ok(());
    }
    store
        .start_run(&run, &window.date, window.cutoff, now)
        .await?;
    let result = deadline(editorial::generate(env, store, &window, &run, None), 780).await;
    if let Err(ref failure) = result {
        // Failure reporting is explicit; expiry, rather than fallible cleanup, releases the lease.
        store.fail_run(&run, &failure.to_string()).await?;
    }
    result
}

#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let result: Result<()> = async {
        let store = Store::new(&env)?;
        let now = time::now();
        store.execute("UPDATE runs SET status='incomplete',finished_at=?,detail='Generation was interrupted or exceeded its execution lease' WHERE status='running' AND started_at<?",&[store::integer(now),store::integer(now-840)]).await?;
        let window = time::today(now)?;
        // Generation gets the full budget at cutoff. Other polls collect without reader traffic.
        if now >= window.cutoff && now < window.deadline && store.edition(Some(&window.date),None).await?.is_none() {
            generate(&env, &store).await
        } else {
            let invocation = owner();
            if !store.acquire("collection", &invocation).await? {
                return Ok(());
            }
            deadline(
                collect::tick(&env, &store, (event.schedule() / 1000.0) as i64),
                720,
            )
            .await
        }
    }
    .await;
    if let Err(failure) = result {
        console_error!("Scheduled Fid job failed: {}", failure);
        // A rejected scheduled promise is observable in Cloudflare, not a false successful job.
        wasm_bindgen::throw_str(&failure.to_string());
    }
}

fn authorized(request: &Request, env: &Env) -> Result<bool> {
    let token = match env.secret("ADMIN_TOKEN") {
        Ok(token) => token.to_string(),
        Err(_) => return Ok(false),
    };
    if token.len() < 24 {
        return Ok(false);
    }
    let supplied = request.headers().get("Authorization")?.unwrap_or_default();
    if supplied.len() > 1024 {
        return Ok(false);
    }
    let expected = store::hash(&format!("Bearer {token}"));
    let actual = store::hash(&supplied);
    Ok(expected
        .bytes()
        .zip(actual.bytes())
        .fold(0u8, |value, (a, b)| value | (a ^ b))
        == 0)
}

async fn reading(store: &Store, date: Option<&str>, revision: Option<u32>) -> Result<Response> {
    let now = time::now();
    let expected = time::expected(now)?;
    let edition = store.edition(date, revision).await?;
    if date.is_some() && edition.is_none() {
        return Response::error("Edition not found", 404);
    }
    let run = store.latest_run(&expected.date).await?;
    let stale = edition.as_ref().is_none_or(|e| e.date < expected.date);
    let status = if edition.is_none() {
        "unavailable"
    } else if stale && date.is_none() {
        "stale"
    } else {
        "available"
    };
    Response::from_json(
        &json!({"status":status,"expected_date":expected.date,"checked_at":now,
        "edition":edition.as_ref().map(PublicEdition::from),"run":run}),
    )
}

fn valid_date(value: &str) -> bool {
    value.len() == 10 && NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionInput {
    expected_revision: u32,
    reason: String,
    stories: Vec<types::Story>,
}

async fn route(mut request: Request, env: Env) -> Result<Response> {
    let path = request.path();
    if !path.starts_with("/api/") {
        return env.service("ASSETS")?.fetch_request(request).await;
    }
    let store = Store::new(&env)?;
    let parts: Vec<_> = path.trim_matches('/').split('/').collect();
    if parts.get(1) == Some(&"admin") {
        if !authorized(&request, &env)? {
            return Response::error("Unauthorized", 401);
        }
        match (request.method(), parts.as_slice()) {
            (Method::Post, ["api", "admin", "verify"]) => {
                let local = matches!(
                    request.url()?.host_str(),
                    Some("localhost" | "127.0.0.1" | "[::1]")
                );
                let enabled = env
                    .var("LOCAL_VERIFICATION")
                    .is_ok_and(|v| v.to_string() == "true");
                if !local || !enabled {
                    return Response::error("Local verification is disabled", 403);
                }
                let bytes = request.bytes().await?;
                if bytes.len() > 500_000 {
                    return Response::error("Verification draft is too large", 413);
                }
                let supplied_draft = if bytes.is_empty() {
                    None
                } else {
                    Some(serde_json::from_slice::<Draft>(&bytes).map_err(error)?)
                };
                let now = time::now();
                let mut window = time::today(now)?;
                window.previous = now - 86400;
                window.cutoff = now;
                window.deadline = now + 3600;
                window.verification = true;
                if store.edition(Some(&window.date), None).await?.is_some() {
                    return Response::error("A local edition already exists for this date; use a fresh local persistence directory to repeat",409);
                }
                let run = owner();
                if !store.acquire("generation", &run).await? {
                    return Response::error("Generation already in progress", 409);
                }
                store
                    .start_run(&run, &window.date, window.cutoff, now)
                    .await?;
                let result = deadline(
                    editorial::generate(&env, &store, &window, &run, supplied_draft),
                    780,
                )
                .await;
                if let Err(ref failure) = result {
                    store.fail_run(&run, &failure.to_string()).await?;
                }
                result?;
                return reading(&store, Some(&window.date), None).await;
            }
            (Method::Post, ["api", "admin", "collect"]) => {
                let invocation = owner();
                if !store.acquire("collection", &invocation).await? {
                    return Response::error("Collection already in progress", 409);
                }
                deadline(collect::all(&env, &store), 720).await?;
                return Response::from_json(&json!({"status":"collected"}));
            }
            (Method::Post, ["api", "admin", "generate"]) => {
                generate(&env, &store).await?;
                return Response::from_json(&json!({"status":"finished-or-already-running"}));
            }
            (Method::Post, ["api", "admin", "corrections", date]) if valid_date(date) => {
                let bytes = request.bytes().await?;
                if bytes.len() > 500_000 {
                    return Response::error("Correction too large", 413);
                }
                let input: CorrectionInput = serde_json::from_slice(&bytes).map_err(error)?;
                let Some(edition) = store.edition(Some(date), None).await? else {
                    return Response::error("Edition not found", 404);
                };
                let invocation = owner();
                if !store.acquire("correction", &invocation).await? {
                    return Response::error("Correction already in progress", 409);
                }
                let corrected = deadline(
                    editorial::correction(
                        &env,
                        &store,
                        edition,
                        Draft {
                            stories: input.stories,
                        },
                        &input.reason,
                        input.expected_revision,
                        &invocation,
                    ),
                    240,
                )
                .await?;
                return Response::from_json(&PublicEdition::from(&corrected));
            }
            (Method::Get, ["api", "admin", "usage"]) => {
                #[derive(serde::Deserialize, serde::Serialize)]
                struct Usage {
                    kind: String,
                    requests: i64,
                    input_tokens: i64,
                    output_tokens: i64,
                    cost_usd: Option<f64>,
                    unpriced: i64,
                }
                let usage:Vec<Usage>=store.query("SELECT kind,COUNT(*) requests,SUM(input_tokens) input_tokens,SUM(output_tokens) output_tokens,SUM(cost_usd) cost_usd,SUM(cost_usd IS NULL) unpriced FROM usage WHERE measured_at>=? GROUP BY kind",&[store::integer(time::now()-30*86400)]).await?;
                return Response::from_json(&usage);
            }
            _ => return Response::error("Admin route not found", 404),
        }
    }
    if request.method() != Method::Get {
        return Response::error("Method not allowed", 405);
    }
    match parts.as_slice() {
        ["api", "latest"] => reading(&store, None, None).await,
        ["api", "editions", date] if valid_date(date) => reading(&store, Some(date), None).await,
        ["api", "editions", date, "revisions", revision] if valid_date(date) => {
            let revision = revision.parse::<u32>().map_err(error)?;
            reading(&store, Some(date), Some(revision)).await
        }
        ["api", "editions"] => {
            #[derive(serde::Deserialize, serde::Serialize)]
            struct Summary {
                edition_date: String,
                published_at: i64,
                current_revision: u32,
            }
            let editions:Vec<Summary>=store.query("SELECT edition_date,published_at,current_revision FROM editions ORDER BY edition_date DESC LIMIT 90",&[]).await?;
            Response::from_json(&editions)
        }
        ["api", "evidence", id] if id.len() == 64 && id.bytes().all(|c| c.is_ascii_hexdigit()) => {
            #[derive(serde::Deserialize)]
            struct Found {
                id: String,
            }
            let references: Vec<Found> = store
                .query(
                    "SELECT id FROM published_evidence WHERE id=? LIMIT 1",
                    &[(*id).into()],
                )
                .await?;
            if !references.iter().any(|r| r.id == *id) {
                return Response::error("Published evidence not found", 404);
            }
            Response::from_json(&store.load_evidence(id).await?)
        }
        ["api", "sources"] => Response::from_json(&sources::SOURCES),
        _ => Response::error("Not found", 404),
    }
}

#[event(fetch)]
pub async fn fetch(request: Request, env: Env, _ctx: Context) -> Result<Response> {
    let api = request.path().starts_with("/api/");
    let mut response = match route(request, env).await {
        Ok(response) => response,
        Err(failure) => {
            console_error!("Fid request failed: {}", failure);
            Response::from_json(&json!({"error":"The request could not be completed. The previous published edition has been preserved."}))?.with_status(503)
        }
    };
    if api {
        response.headers_mut().set("Cache-Control", "no-store")?;
        response
            .headers_mut()
            .set("X-Content-Type-Options", "nosniff")?;
        response.headers_mut().set(
            "Content-Security-Policy",
            "default-src 'none'; frame-ancestors 'none'",
        )?;
    }
    Ok(response)
}
