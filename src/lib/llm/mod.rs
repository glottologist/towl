//! LLM-powered TODO validation and analysis.
//!
//! Supports Claude (Anthropic API), any OpenAI-compatible endpoint,
//! and local CLI agents (Claude Code, Codex).
//! Uses enum-based dispatch matching towl's existing `FormatterImpl`/`WriterImpl` pattern.

pub mod analyse;
pub mod claude;
pub mod cli;
pub mod error;
pub mod openai;
pub mod prompt;
pub mod types;

pub use types::{AnalysisResult, AnalysisSummary, Validity};

use crate::config::LlmConfig;
use secrecy::SecretString;

const CODEX_FALLBACK_MODEL: &str = "gpt-4o";

/// LLM provider for TODO analysis, dispatched by enum variant.
#[derive(Debug)]
pub enum LlmProvider {
    Claude(claude::ClaudeProvider),
    OpenAi(openai::OpenAiProvider),
    ClaudeCode(cli::ClaudeCodeProvider),
    Codex(cli::CodexProvider),
}

impl LlmProvider {
    pub async fn call_raw(
        &self,
        user_content: &str,
        system_prompt: &str,
        api_key: &SecretString,
    ) -> Result<(String, types::LlmUsage), error::TowlLlmError> {
        match self {
            Self::Claude(c) => c.call_raw(user_content, system_prompt, api_key).await,
            Self::OpenAi(o) => o.call_raw(user_content, system_prompt, api_key).await,
            Self::ClaudeCode(c) => c.call_raw(user_content, system_prompt, api_key).await,
            Self::Codex(c) => c.call_raw(user_content, system_prompt, api_key).await,
        }
    }

    pub const fn is_cli_provider(&self) -> bool {
        matches!(self, Self::ClaudeCode(_) | Self::Codex(_))
    }
}

/// Builds an LLM provider from configuration.
///
/// For `claude-code` and `codex`, auto-falls back to the API provider
/// if the CLI binary is not found on PATH.
///
/// # Errors
/// Returns `TowlLlmError::UnsupportedProvider` if the provider name is unknown.
/// Returns `TowlLlmError::ApiError` if the HTTP client fails to build (API providers).
pub fn build_provider(config: &LlmConfig) -> Result<LlmProvider, error::TowlLlmError> {
    match config.provider.as_str() {
        "claude" => Ok(LlmProvider::Claude(claude::ClaudeProvider::new(
            &config.model,
            config.max_tokens,
        )?)),
        "openai" => Ok(LlmProvider::OpenAi(openai::OpenAiProvider::new(
            &config.model,
            config.max_tokens,
            config.base_url.as_deref(),
        )?)),
        "claude-code" => {
            let cmd = config.command.as_deref().unwrap_or("claude");
            if cli::command_exists(cmd) {
                Ok(LlmProvider::ClaudeCode(cli::ClaudeCodeProvider::new(
                    config.command.as_deref(),
                    config.args.as_deref(),
                )))
            } else {
                tracing::warn!(
                    command = cmd,
                    "CLI not found on PATH, falling back to Claude API"
                );
                Ok(LlmProvider::Claude(claude::ClaudeProvider::new(
                    &config.model,
                    config.max_tokens,
                )?))
            }
        }
        "codex" => {
            let cmd = config.command.as_deref().unwrap_or("codex");
            if cli::command_exists(cmd) {
                Ok(LlmProvider::Codex(cli::CodexProvider::new(
                    config.command.as_deref(),
                    config.args.as_deref(),
                )))
            } else {
                let default_model = crate::config::defaults::default_llm_model();
                let model = if config.model == default_model {
                    CODEX_FALLBACK_MODEL
                } else {
                    &config.model
                };
                tracing::warn!(
                    command = cmd,
                    fallback_model = model,
                    "CLI not found on PATH, falling back to OpenAI API"
                );
                Ok(LlmProvider::OpenAi(openai::OpenAiProvider::new(
                    model,
                    config.max_tokens,
                    config.base_url.as_deref(),
                )?))
            }
        }
        other => Err(error::TowlLlmError::UnsupportedProvider {
            provider: other.to_string(), // clone: &str -> owned String for error
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("claude", "claude-opus-4-6", None)]
    #[case("openai", "gpt-4o", None)]
    #[case("openai", "llama3", Some("http://localhost:11434/v1".to_string()))]
    fn test_build_provider(
        #[case] provider: &str,
        #[case] model: &str,
        #[case] base_url: Option<String>,
    ) {
        let config = LlmConfig {
            provider: provider.to_string(),
            model: model.to_string(),
            base_url,
            ..Default::default()
        };
        assert!(build_provider(&config).is_ok());
    }

    #[test]
    fn test_build_provider_unsupported() {
        let config = LlmConfig {
            provider: "gemini".to_string(),
            ..Default::default()
        };
        let err = build_provider(&config).unwrap_err();
        assert!(matches!(
            err,
            error::TowlLlmError::UnsupportedProvider { .. }
        ));
    }

    #[rstest]
    #[case("claude-code", "nonexistent_claude_test_binary_xyz")]
    #[case("codex", "nonexistent_codex_test_binary_xyz")]
    fn test_build_provider_cli_fallback_to_api(#[case] provider_name: &str, #[case] command: &str) {
        let config = LlmConfig {
            provider: provider_name.to_string(),
            command: Some(command.to_string()),
            ..Default::default()
        };
        let provider = build_provider(&config).unwrap();
        assert!(!provider.is_cli_provider());
    }

    #[test]
    fn test_is_cli_provider() {
        let claude_code = LlmProvider::ClaudeCode(cli::ClaudeCodeProvider::new(None, None));
        assert!(claude_code.is_cli_provider());

        let codex = LlmProvider::Codex(cli::CodexProvider::new(None, None));
        assert!(codex.is_cli_provider());

        let config = LlmConfig {
            provider: "claude".to_string(),
            ..Default::default()
        };
        let claude_api = build_provider(&config).unwrap();
        assert!(!claude_api.is_cli_provider());
    }
}
