use crate::{http, store::error, Result};

pub struct Config {
    pub endpoint: url::Url,
    pub model: String,
    pub api_key: String,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::parse(|key| match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err(error(format!("{key} is not valid Unicode")))
            }
        })
    }

    fn parse(get: impl Fn(&str) -> Result<Option<String>>) -> Result<Self> {
        let required = |key| {
            get(key)?
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| error(format!("{key} is not configured")))
        };
        let price = |key| -> Result<Option<f64>> {
            get(key)?
                .filter(|v| !v.is_empty())
                .map(|value| {
                    let value = value.parse::<f64>().map_err(error)?;
                    if !value.is_finite() || value < 0.0 {
                        return Err(error(format!("{key} must be a nonnegative finite price")));
                    }
                    Ok(value)
                })
                .transpose()
        };
        let model = get("MODEL_NAME")?.unwrap_or_else(|| "grok-4.7".into());
        if model.trim().is_empty() {
            return Err(error("MODEL_NAME must not be blank"));
        }
        Ok(Self {
            endpoint: http::public_url(
                &get("MODEL_API_URL")?
                    .unwrap_or_else(|| "https://api.x.ai/v1/chat/completions".into()),
            )?,
            model,
            api_key: required("GROK_API_KEY")?,
            input_price: price("MODEL_INPUT_USD_PER_MILLION")?,
            output_price: price("MODEL_OUTPUT_USD_PER_MILLION")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_rejects_missing_secrets_and_invalid_prices() {
        assert!(Config::parse(|_| Ok(None)).is_err());
        for value in ["NaN", "-1", "infinity", "unknown"] {
            assert!(Config::parse(|key| Ok(match key {
                "GROK_API_KEY" => Some("test-key".into()),
                "MODEL_INPUT_USD_PER_MILLION" => Some(value.into()),
                _ => None,
            }))
            .is_err());
        }
        let config =
            Config::parse(|key| Ok((key == "GROK_API_KEY").then(|| "test-key".into()))).unwrap();
        assert!(config.input_price.is_none());
    }
}
