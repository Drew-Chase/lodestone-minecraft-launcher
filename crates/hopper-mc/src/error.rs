//! Error type for all provider operations.

use crate::platform::{ContentType, Platform};

/// Errors that can occur while fetching or decoding content from a provider.
#[derive(thiserror::Error, Debug)]
pub enum ContentError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("rate limited")]
    RateLimited(Option<std::time::Duration>),

    #[error("{platform:?} does not support {kind:?}")]
    UnsupportedContentType {
        platform: Platform,
        kind: ContentType,
    },

    #[error("provider not yet implemented: {0:?}")]
    NotImplemented(Platform),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unexpected response: {0}")]
    Unexpected(String),
}

/// Convenience alias used by trait and dispatch APIs.
pub type Result<T> = std::result::Result<T, ContentError>;
