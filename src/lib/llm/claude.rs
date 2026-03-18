use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::debug;

use super::error::TowlLlmError;
use super::types::LlmUsage;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug)]
pub struct ClaudeProvider {
    http: reqwest::Client,
    model: String,
    max_tokens: u32,
}

impl ClaudeProvider {
    /// # Errors
    /// Returns `TowlLlmError::ApiError` if the HTTP client fails to build.
    pub fn new(model: &str, max_tokens: u32) -> Result<Self, TowlLlmError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| TowlLlmError::ApiError {
                message: format!("Failed to build HTTP client: {e}"),
                status: None,
            })?;

        Ok(Self {
            http,
            model: model.to_string(), // clone: owned String for struct field
            max_tokens,
        })
    }

    #[must_use]
    pub fn build_request_body(&self, user_content: &str, system_prompt: &str) -> String {
        serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_content}]
        })
        .to_string()
    }

    /// # Errors
    /// Returns `TowlLlmError::AuthError` if the API key header is invalid.
    /// Returns `TowlLlmError::ApiError` on network or non-200 responses.
    /// Returns `TowlLlmError::ParseError` if the response JSON is malformed.
    pub async fn call_raw(
        &self,
        user_content: &str,
        system_prompt: &str,
        api_key: &SecretString,
    ) -> Result<(String, LlmUsage), TowlLlmError> {
        let body = self.build_request_body(user_content, system_prompt);

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key.expose_secret()).map_err(|_| TowlLlmError::AuthError)?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http
            .post(ANTHROPIC_API_URL)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| TowlLlmError::ApiError {
                message: format!("Request failed: {e}"),
                status: e.status().map(|s| s.as_u16()),
            })?;

        let status = response.status().as_u16();
        if status != 200 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read body".to_string());

            return Err(TowlLlmError::classify_http_error(
                status,
                &body_text,
                retry_after,
            ));
        }

        let json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| TowlLlmError::ParseError {
                    message: format!("Failed to parse response JSON: {e}"),
                })?;

        let text = json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .ok_or_else(|| TowlLlmError::ParseError {
                message: "Missing content[0].text in response".to_string(),
            })?
            .to_string(); // clone: &str -> owned String for return

        let usage = LlmUsage {
            input_tokens: json["usage"]["input_tokens"].as_u64().unwrap_or(0),
            output_tokens: json["usage"]["output_tokens"].as_u64().unwrap_or(0),
        };

        debug!(
            model = %self.model,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            "Claude API call complete"
        );

        Ok((text, usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_request_body_structure() {
        let provider = ClaudeProvider::new("claude-opus-4-6", 4096).unwrap();
        let body = provider.build_request_body("user content", "system prompt");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["model"], "claude-opus-4-6");
        assert_eq!(json["system"], "system prompt");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "user content");
        assert_eq!(json["max_tokens"], 4096);
    }
}
