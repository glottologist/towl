use thiserror::Error;

/// Errors from LLM API interactions and analysis.
#[derive(Error, Debug)]
pub enum TowlLlmError {
    #[error("LLM API error: {message}")]
    ApiError {
        message: String,
        status: Option<u16>,
    },
    #[error("LLM authentication failed: check TOWL_LLM_API_KEY")]
    AuthError,
    #[error("LLM rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("Failed to parse LLM response: {message}")]
    ParseError { message: String },
    #[error("LLM not configured: set TOWL_LLM_API_KEY environment variable")]
    NotConfigured,
    #[error("Unsupported LLM provider: {provider}")]
    UnsupportedProvider { provider: String },
    #[error("File I/O error: {message}")]
    IoError { message: String },
}

impl TowlLlmError {
    /// Whether this error is transient and the operation should be retried.
    ///
    /// `ParseError` is retryable because LLM output is nondeterministic; a
    /// reprompt frequently yields well-formed JSON.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. }
                | Self::ParseError { .. }
                | Self::ApiError {
                    status: Some(500..) | None,
                    ..
                }
        )
    }

    /// Builds an error from a non-success HTTP response, reading the
    /// `retry-after` header and the body text.
    pub(crate) async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unable to read body".to_string());

        Self::classify_http_error(status, &body_text, retry_after)
    }

    /// Classifies an HTTP status code into the appropriate error variant.
    ///
    /// Shared between Claude and `OpenAI` providers.
    pub(crate) fn classify_http_error(
        status: u16,
        message: &str,
        retry_after: Option<u64>,
    ) -> Self {
        match status {
            401 => Self::AuthError,
            429 => Self::RateLimited {
                retry_after_secs: retry_after.unwrap_or(60),
            },
            _ => Self::ApiError {
                message: message.to_string(), // clone: &str -> owned String for error
                status: Some(status),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_classify_401_always_auth_error(
            msg in ".*",
            retry_after in proptest::option::of(0u64..3600),
        ) {
            let err = TowlLlmError::classify_http_error(401, &msg, retry_after);
            prop_assert!(matches!(err, TowlLlmError::AuthError));
        }

        #[test]
        fn prop_classify_429_always_rate_limited(
            msg in ".*",
            retry_after in proptest::option::of(0u64..3600),
        ) {
            let err = TowlLlmError::classify_http_error(429, &msg, retry_after);
            match err {
                TowlLlmError::RateLimited { retry_after_secs } => {
                    prop_assert_eq!(retry_after_secs, retry_after.unwrap_or(60));
                }
                other => prop_assert!(false, "Expected RateLimited, got: {other:?}"),
            }
        }

        #[test]
        fn prop_classify_other_status_produces_api_error(
            status in (0u16..=u16::MAX).prop_filter("not 401 or 429", |s| *s != 401 && *s != 429),
            msg in "[a-zA-Z0-9 ]{0,100}",
        ) {
            let err = TowlLlmError::classify_http_error(status, &msg, None);
            match err {
                TowlLlmError::ApiError { message, status: s } => {
                    prop_assert_eq!(message, msg);
                    prop_assert_eq!(s, Some(status));
                }
                other => prop_assert!(false, "Expected ApiError, got: {other:?}"),
            }
        }
    }
}
