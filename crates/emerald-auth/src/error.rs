use std::time::Duration;

/// Errors that can occur during the Microsoft authentication flow.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json decode error: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("oauth error: {error} - {description}")]
    OAuth { error: String, description: String },

    #[error("csrf state mismatch: expected {expected}, got {actual}")]
    StateMismatch { expected: String, actual: String },

    #[error("missing query parameter: {0}")]
    MissingParam(String),

    #[error("xbox live authentication failed: {0}")]
    XboxLive(String),

    #[error("xsts authentication failed (xerr {xerr}): {message}")]
    Xsts { xerr: u64, message: String },

    #[error("minecraft authentication failed: {0}")]
    Minecraft(String),

    #[error("account does not own Minecraft")]
    NoGameOwnership,

    #[error("authentication timed out after {0:?}")]
    Timeout(Duration),

    #[error("failed to open browser: {0}")]
    BrowserOpen(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;
