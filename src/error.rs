use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Arxiv2MdError {
    #[error("invalid arXiv identifier or URL: {0}")]
    InvalidInput(String),
    #[error("unsupported host: {0}")]
    UnsupportedHost(String),
    #[error("paper not found: {0}")]
    NotFound(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response from {0}: {1}")]
    UnexpectedResponse(String, String),
    #[error("cache error for {0}: {1}")]
    Cache(String, String),
    #[error("failed to parse metadata: {0}")]
    Metadata(String),
    #[error("failed to parse HTML: {0}")]
    Html(String),
    #[error("failed to render markdown: {0}")]
    Markdown(String),
    #[error("latex conversion failed: {0}")]
    Latex(String),
    #[error("pdf extraction failed: {0}")]
    Pdf(String),
    #[error("failed to write output to {0}: {1}")]
    Output(PathBuf, String),
}

pub type Result<T> = std::result::Result<T, Arxiv2MdError>;
