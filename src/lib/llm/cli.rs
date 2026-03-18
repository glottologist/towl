use std::process::Stdio;
use std::time::Duration;

use secrecy::SecretString;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::debug;

use super::error::TowlLlmError;
use super::types::LlmUsage;

const CLI_TIMEOUT_SECS: u64 = 120;

/// Checks if a command exists on PATH.
#[must_use]
pub fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

#[must_use]
fn build_cli_prompt(user_content: &str, system_prompt: &str) -> String {
    format!("{system_prompt}\n\n---\n\n{user_content}")
}

/// Spawns a CLI agent, pipes the prompt via stdin, and captures stdout.
///
/// # Errors
/// Returns `TowlLlmError::ApiError` if the process fails to spawn, exits non-zero,
/// or times out after 120 seconds.
async fn invoke_cli(command: &str, args: &[String], prompt: &str) -> Result<String, TowlLlmError> {
    if command.contains("..") || command.contains('/') && !command.starts_with('/') {
        return Err(TowlLlmError::ApiError {
            message: format!("Rejecting relative path in CLI command: {command}"),
            status: None,
        });
    }

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| TowlLlmError::ApiError {
            message: format!("Failed to run {command}: {e}"),
            status: None,
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| TowlLlmError::ApiError {
                message: format!("Failed to write to {command} stdin: {e}"),
                status: None,
            })?;
        drop(stdin);
    }

    let output = tokio::time::timeout(
        Duration::from_secs(CLI_TIMEOUT_SECS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| TowlLlmError::ApiError {
        message: format!("{command} timed out after {CLI_TIMEOUT_SECS}s"),
        status: None,
    })?
    .map_err(|e| TowlLlmError::ApiError {
        message: format!("Failed to wait for {command}: {e}"),
        status: None,
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TowlLlmError::ApiError {
            message: format!("{command} exited with {}: {}", output.status, stderr.trim()),
            status: output.status.code().and_then(|c| u16::try_from(c).ok()),
        });
    }

    String::from_utf8(output.stdout).map_err(|e| TowlLlmError::ParseError {
        message: format!("Non-UTF-8 output from {command}: {e}"),
    })
}

const CLAUDE_CODE_COMMAND: &str = "claude";
const CLAUDE_CODE_DEFAULT_ARGS: &[&str] = &["-p", "--output-format", "json"];

#[derive(Debug)]
pub struct ClaudeCodeProvider {
    command: String,
    args: Vec<String>,
}

impl ClaudeCodeProvider {
    #[must_use]
    pub fn new(command: Option<&str>, args: Option<&[String]>) -> Self {
        Self {
            command: command.unwrap_or(CLAUDE_CODE_COMMAND).to_string(), // clone: &str -> owned String for struct field
            args: args.map_or_else(
                || {
                    CLAUDE_CODE_DEFAULT_ARGS
                        .iter()
                        .map(|s| (*s).to_string()) // clone: &str -> owned String for Vec
                        .collect()
                },
                <[String]>::to_vec,
            ),
        }
    }

    /// Invokes Claude Code CLI with the combined prompt via stdin.
    ///
    /// # Errors
    /// Returns `TowlLlmError::ApiError` if the CLI fails to run or returns non-zero.
    pub async fn call_raw(
        &self,
        user_content: &str,
        system_prompt: &str,
        _api_key: &SecretString,
    ) -> Result<(String, LlmUsage), TowlLlmError> {
        let prompt = build_cli_prompt(user_content, system_prompt);

        debug!(command = %self.command, "Invoking Claude Code CLI");
        let raw_output = invoke_cli(&self.command, &self.args, &prompt).await?;

        let text = extract_claude_code_result(&raw_output);
        Ok((text, LlmUsage::default()))
    }
}

/// Extracts analysis text from Claude Code's JSON output.
/// `--output-format json` returns `{"type":"result","result":"..."}`.
/// Falls back to raw output if the format doesn't match.
fn extract_claude_code_result(output: &str) -> String {
    let trimmed = output.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
            return result.to_string(); // clone: &str -> owned String for return
        }
    }
    trimmed.to_string() // clone: &str -> owned String for return
}

const CODEX_COMMAND: &str = "codex";
const CODEX_DEFAULT_ARGS: &[&str] = &["-q"];

#[derive(Debug)]
pub struct CodexProvider {
    command: String,
    args: Vec<String>,
}

impl CodexProvider {
    #[must_use]
    pub fn new(command: Option<&str>, args: Option<&[String]>) -> Self {
        Self {
            command: command.unwrap_or(CODEX_COMMAND).to_string(), // clone: &str -> owned String for struct field
            args: args.map_or_else(
                || {
                    CODEX_DEFAULT_ARGS
                        .iter()
                        .map(|s| (*s).to_string()) // clone: &str -> owned String for Vec
                        .collect()
                },
                <[String]>::to_vec,
            ),
        }
    }

    /// Invokes Codex CLI with the combined prompt via stdin.
    ///
    /// # Errors
    /// Returns `TowlLlmError::ApiError` if the CLI fails to run or returns non-zero.
    pub async fn call_raw(
        &self,
        user_content: &str,
        system_prompt: &str,
        _api_key: &SecretString,
    ) -> Result<(String, LlmUsage), TowlLlmError> {
        let prompt = build_cli_prompt(user_content, system_prompt);

        debug!(command = %self.command, "Invoking Codex CLI");
        let text = invoke_cli(&self.command, &self.args, &prompt).await?;

        Ok((
            text.trim().to_string(), // clone: &str -> owned String for return
            LlmUsage::default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case(None, None, "claude", vec!["-p", "--output-format", "json"])]
    #[case(Some("/usr/local/bin/claude"), None, "/usr/local/bin/claude", vec!["-p", "--output-format", "json"])]
    fn test_claude_code_provider_construction(
        #[case] command: Option<&str>,
        #[case] args: Option<Vec<String>>,
        #[case] expected_command: &str,
        #[case] expected_args: Vec<&str>,
    ) {
        let provider = ClaudeCodeProvider::new(command, args.as_deref());
        assert_eq!(provider.command, expected_command);
        assert_eq!(provider.args, expected_args);
    }

    #[test]
    fn test_claude_code_provider_custom_args() {
        let custom_args = vec!["--json".to_string(), "-p".to_string()];
        let provider = ClaudeCodeProvider::new(None, Some(&custom_args));
        assert_eq!(provider.args, custom_args);
    }

    #[rstest]
    #[case(None, "codex", vec!["-q"])]
    #[case(Some("/opt/codex"), "/opt/codex", vec!["-q"])]
    fn test_codex_provider_construction(
        #[case] command: Option<&str>,
        #[case] expected_command: &str,
        #[case] expected_args: Vec<&str>,
    ) {
        let provider = CodexProvider::new(command, None);
        assert_eq!(provider.command, expected_command);
        assert_eq!(provider.args, expected_args);
    }

    #[rstest]
    #[case("sh", true)]
    #[case("definitely_not_a_real_command_12345", false)]
    fn test_command_exists(#[case] command: &str, #[case] expected: bool) {
        assert_eq!(command_exists(command), expected);
    }

    #[rstest]
    #[case(
        r#"{"type":"result","result":"```json\n{\"validity\":\"valid\"}\n```"}"#,
        true
    )]
    #[case("just plain text response", false)]
    fn test_extract_claude_code_result(#[case] input: &str, #[case] is_json: bool) {
        let result = extract_claude_code_result(input);
        if is_json {
            assert!(result.contains("validity"));
        } else {
            assert_eq!(result, "just plain text response");
        }
    }

    proptest! {
        #[test]
        fn prop_build_cli_prompt_structure(
            system in "[a-zA-Z0-9 ]{1,50}",
            user in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let prompt = build_cli_prompt(&user, &system);
            prop_assert!(prompt.starts_with(&system));
            prop_assert!(prompt.ends_with(&user));
            prop_assert!(prompt.contains("---"));
        }
    }
}
