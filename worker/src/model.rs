use crate::{
    http,
    store::{error, hash, Store},
    time,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use worker::{Env, Fetch, Method, Request, RequestInit, Result};

#[derive(Deserialize)]
struct Completion {
    id: String,
    choices: Vec<Choice>,
    usage: Usage,
}
#[derive(Deserialize)]
struct Choice {
    finish_reason: String,
    message: Message,
}
#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}
#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

pub fn api_key(env: &Env) -> Result<String> {
    let key = env.secret("GROK_API_KEY")?.to_string();
    if key.trim().is_empty() {
        return Err(error("GROK_API_KEY is not configured"));
    }
    Ok(key)
}

pub async fn generate<T: DeserializeOwned>(
    env: &Env,
    store: &Store,
    run: &str,
    stage: &str,
    instruction: &str,
    data: &Value,
) -> Result<T> {
    let endpoint = env.var("MODEL_API_URL")?.to_string();
    http::public_url(&endpoint)?;
    let model = env.var("MODEL_NAME")?.to_string();
    if model.trim().is_empty() {
        return Err(error("MODEL_NAME is not configured"));
    }
    let key = api_key(env)?;
    let price = |name: &str| -> Result<Option<f64>> {
        let value = env.var(name)?.to_string();
        if value.is_empty() {
            return Ok(None);
        }
        let price = value.parse::<f64>().map_err(error)?;
        if !price.is_finite() || price < 0.0 {
            return Err(error("Model token price is invalid"));
        }
        Ok(Some(price))
    };
    let input_price = price("MODEL_INPUT_USD_PER_MILLION")?;
    let output_price = price("MODEL_OUTPUT_USD_PER_MILLION")?;
    let input = serde_json::to_string(data).map_err(error)?;
    if input.len() > 900_000 {
        return Err(error(
            "Evidence exceeds model context budget; publication withheld",
        ));
    }
    let request_body = json!({
        "model":model,"store":false,"response_format":{"type":"json_object"},
        "max_completion_tokens":24000,
        "messages":[{"role":"system","content":instruction},{"role":"user","content":input}]
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_redirect(worker::RequestRedirect::Manual)
        .with_body(Some(
            serde_json::to_string(&request_body).map_err(error)?.into(),
        ));
    let mut request = Request::new_with_init(&endpoint, &init)?;
    request
        .headers_mut()?
        .set("Content-Type", "application/json")?;
    request
        .headers_mut()?
        .set("Authorization", &format!("Bearer {key}"))?;
    let fetch = Fetch::Request(request);
    let mut response = fetch.send().await?;
    if response.status_code() != 200 {
        // Provider error bodies can echo private data. Persist only the status.
        store
            .execute(
                "INSERT INTO usage(id,run_id,measured_at,kind,model,detail) VALUES(?,?,?,?,?,?)",
                &[
                    hash(&format!("{run}:{stage}:failed")).into(),
                    run.into(),
                    crate::store::integer(time::now()),
                    stage.into(),
                    model.into(),
                    format!("Model HTTP {}; token usage unknown", response.status_code()).into(),
                ],
            )
            .await?;
        return Err(error(format!(
            "Model HTTP {}; see server configuration",
            response.status_code()
        )));
    }
    let completion: Completion = response.json().await?;
    let cost = input_price.zip(output_price).map(|(input, output)| {
        (input * f64::from(completion.usage.prompt_tokens)
            + output * f64::from(completion.usage.completion_tokens))
            / 1_000_000.0
    });
    store.execute("INSERT INTO usage(id,run_id,measured_at,kind,model,input_tokens,output_tokens,cost_usd,detail) VALUES(?,?,?,?,?,?,?,?,?)",&[
        hash(&format!("{run}:{stage}:{}",completion.id)).into(),run.into(),crate::store::integer(time::now()),stage.into(),model.into(),
        completion.usage.prompt_tokens.into(),completion.usage.completion_tokens.into(),crate::store::nullable(cost),completion.id.into()
    ]).await?;
    let choice = completion
        .choices
        .first()
        .ok_or_else(|| error("Model returned no choice"))?;
    let content = choice
        .message
        .content
        .as_deref()
        .ok_or_else(|| error("Model refused or returned no content"))?;
    store.artifact(run,stage,&json!({"request":request_body,"response":content,"finish_reason":choice.finish_reason})).await?;
    if choice.finish_reason != "stop" {
        return Err(error("Model output is incomplete"));
    }
    serde_json::from_str(content).map_err(error)
}
