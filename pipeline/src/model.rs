use crate::{config::Config, Result};
use crate::{
    http,
    store::{error, hash, Store},
    time,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

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
    total_tokens: u32,
    completion_tokens_details: Option<CompletionTokenDetails>,
}
impl Usage {
    fn output_tokens(&self) -> Result<u32> {
        // xAI counts reasoning separately from completion_tokens, within total_tokens.
        self.total_tokens
            .checked_sub(self.prompt_tokens)
            .ok_or_else(|| error("Model total tokens are less than input tokens"))
    }
}
#[derive(Deserialize)]
struct CompletionTokenDetails {
    reasoning_tokens: Option<u32>,
}

pub async fn generate<T: DeserializeOwned>(
    config: &Config,
    store: &Store,
    run: &str,
    stage: &str,
    instruction: &str,
    data: &Value,
) -> Result<T> {
    let endpoint = &config.endpoint;
    let model = &config.model;
    let effort = "high";
    let input = serde_json::to_string(data).map_err(error)?;
    if input.len() > 900_000 {
        return Err(error(
            "Evidence exceeds model context budget; publication withheld",
        ));
    }
    let request_body = json!({
        "model":model,"store":false,"response_format":{"type":"json_object"},
        "max_completion_tokens":24000,
        "reasoning_effort":effort,
        "messages":[{"role":"system","content":instruction},{"role":"user","content":input}]
    });
    store.artifact(run, &format!("{stage}-request"), &request_body)?;
    let started = std::time::Instant::now();
    let response = http::post_json(endpoint, &config.api_key, &request_body).await?;
    if response.status != 200 {
        // Provider error bodies can echo private data. Persist only the status.
        store.execute(
            "INSERT INTO usage(id,run_id,measured_at,kind,model,detail) VALUES(?,?,?,?,?,?)",
            &[
                hash(&format!("{run}:{stage}:failed")).into(),
                run.into(),
                crate::store::integer(time::now()),
                stage.into(),
                model.clone().into(),
                format!("Model HTTP {}; token usage unknown", response.status).into(),
            ],
        )?;
        return Err(error(format!(
            "Model HTTP {}; see server configuration",
            response.status
        )));
    }
    let provider_response = response
        .body
        .ok_or_else(|| error("Model returned no response"))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    store
        .artifact(
            run,
            stage,
            &json!({"request":request_body,"provider_response":provider_response,"elapsed_ms":elapsed_ms}),
        )?;
    let completion: Completion = serde_json::from_value(provider_response).map_err(error)?;
    let reasoning_tokens = completion
        .usage
        .completion_tokens_details
        .as_ref()
        .and_then(|details| details.reasoning_tokens);
    let output_tokens = completion.usage.output_tokens()?;
    let cost = config
        .input_price
        .zip(config.output_price)
        .map(|(input, output)| {
            (input * f64::from(completion.usage.prompt_tokens) + output * f64::from(output_tokens))
                / 1_000_000.0
        });
    store.execute("INSERT INTO usage(id,run_id,measured_at,kind,model,input_tokens,output_tokens,cost_usd,detail) VALUES(?,?,?,?,?,?,?,?,?)",&[
        hash(&format!("{run}:{stage}:{}",completion.id)).into(),run.into(),crate::store::integer(time::now()),stage.into(),model.clone().into(),
        completion.usage.prompt_tokens.into(),output_tokens.into(),crate::store::nullable(cost),
        json!({"id":completion.id,"reasoning_effort":effort,"completion_tokens":completion.usage.completion_tokens,"reasoning_tokens":reasoning_tokens,"elapsed_ms":elapsed_ms}).to_string().into()
    ])?;
    let choice = completion
        .choices
        .first()
        .ok_or_else(|| error("Model returned no choice"))?;
    let content = choice
        .message
        .content
        .as_deref()
        .ok_or_else(|| error("Model refused or returned no content"))?;
    if choice.finish_reason != "stop" {
        return Err(error("Model output is incomplete"));
    }
    serde_json::from_str(content).map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_usage_is_optional_and_separate_from_completion_total() {
        let mut response = json!({
            "id":"completion","choices":[{"finish_reason":"stop","message":{"content":"{}"}}],
            "usage":{"prompt_tokens":100,"completion_tokens":80,"total_tokens":230,
                "completion_tokens_details":{"reasoning_tokens":50}},
        });
        let parsed: Completion = serde_json::from_value(response.clone()).unwrap();
        assert_eq!(parsed.usage.completion_tokens, 80);
        assert_eq!(parsed.usage.output_tokens().unwrap(), 130);
        assert_eq!(
            parsed
                .usage
                .completion_tokens_details
                .unwrap()
                .reasoning_tokens,
            Some(50)
        );
        response["usage"]
            .as_object_mut()
            .unwrap()
            .remove("completion_tokens_details");
        let parsed: Completion = serde_json::from_value(response).unwrap();
        assert!(parsed.usage.completion_tokens_details.is_none());
        assert_eq!(parsed.usage.output_tokens().unwrap(), 130);
        let invalid: Usage = serde_json::from_value(json!({
            "prompt_tokens":100,"completion_tokens":0,"total_tokens":90,
        }))
        .unwrap();
        assert!(invalid.output_tokens().is_err());
    }
}
