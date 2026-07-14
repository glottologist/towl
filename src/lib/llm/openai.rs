use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use tracing::debug;

use super::error::TowlLlmError;
use super::types::LlmUsage;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug)]
pub struct OpenAiProvider {
    http: reqwest::Client,
    model: String,
    max_tokens: u32,
    base_url: String,
}

impl OpenAiProvider {
    /// # Errors
    /// Returns `TowlLlmError::ApiError` if the HTTP client fails to build.
    pub fn new(model: &str, max_tokens: u32, base_url: Option<&str>) -> Result<Self, TowlLlmError> {
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
            base_url: {
                let raw = base_url.unwrap_or(DEFAULT_BASE_URL).trim_end_matches('/');
                format!("{raw}/") // clone: owned String with trailing slash for Url::join
            },
        })
    }

    #[must_use]
    pub fn build_request_body(&self, user_content: &str, system_prompt: &str) -> String {
        serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_content}
            ]
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
        let url = reqwest::Url::parse(&self.base_url)
            .and_then(|u| u.join("chat/completions"))
            .map_err(|e| TowlLlmError::ApiError {
                message: format!("Invalid base URL: {e}"),
                status: None,
            })?;

        let bearer = format!("Bearer {}", api_key.expose_secret());
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&bearer).map_err(|_| TowlLlmError::AuthError)?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| TowlLlmError::ApiError {
                message: format!("Request failed: {e}"),
                status: e.status().map(|s| s.as_u16()),
            })?;

        if response.status().as_u16() != 200 {
            return Err(TowlLlmError::from_response(response).await);
        }

        let json: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| TowlLlmError::ParseError {
                    message: format!("Failed to parse response JSON: {e}"),
                })?;

        let text = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or_else(|| TowlLlmError::ParseError {
                message: "Missing choices[0].message.content in response".to_string(),
            })?
            .to_string();

        let usage = LlmUsage {
            input_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
            output_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        };

        debug!(
            model = %self.model,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            "OpenAI API call complete"
        );

        Ok((text, usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_openai_request_body_structure() {
        let provider = OpenAiProvider::new("gpt-4o", 4096, None).unwrap();
        let body = provider.build_request_body("user content", "system prompt");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][0]["content"], "system prompt");
        assert_eq!(json["messages"][1]["role"], "user");
        assert_eq!(json["messages"][1]["content"], "user content");
        assert_eq!(json["max_tokens"], 4096);
    }

    #[rstest]
    #[case(Some("http://localhost:11434/v1"), "http://localhost:11434/v1/")]
    #[case(None, &format!("{DEFAULT_BASE_URL}/"))]
    #[case(Some("http://localhost:11434/v1/"), "http://localhost:11434/v1/")]
    fn test_openai_base_url_normalization(#[case] input: Option<&str>, #[case] expected: &str) {
        let provider = OpenAiProvider::new("gpt-4o", 4096, input).unwrap();
        assert_eq!(provider.base_url, expected);
    }
}
