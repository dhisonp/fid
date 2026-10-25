pub mod collect;
pub mod config;
pub mod editorial;
pub mod export;
pub mod http;
pub mod model;
pub mod rules;
pub mod sources;
pub mod store;
pub mod time;
pub mod types;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("Operation timed out")]
    Timeout(#[from] tokio::time::error::Elapsed),
}
pub type Result<T> = std::result::Result<T, Error>;
