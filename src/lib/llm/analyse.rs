use std::path::Path;

use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, error, info, warn};

use crate::comment::todo::TodoComment;
use crate::config::LlmConfig;

use super::build_provider;
use super::error::TowlLlmError;
use super::prompt::{build_user_content, SYSTEM_PROMPT};
use super::types::{parse_analysis_result, AnalysisSummary, Validity};
use super::LlmProvider;

const EXPANDED_CONTEXT_RADIUS: usize = 15;
const BRACE_DELIMITED_EXTENSIONS: &[&str] = &[
    "rs", "c", "cpp", "h", "hpp", "java", "js", "ts", "jsx", "tsx", "go", "cs", "swift", "kt",
    "scala", "json",
];

/// Reads expanded context from a source file for LLM analysis.
///
/// Returns (surrounding_lines, optional_function_body).
/// Reads ~30 lines around `line_number` (15 above, 15 below).
/// If `function_name` is provided and the file uses brace-delimited blocks,
/// searches for the function definition and extracts its full body.
///
/// # Errors
/// Returns `TowlLlmError::IoError` if the file cannot be read.
pub async fn gather_expanded_context(
    path: &Path,
    line_number: usize,
    function_name: Option<&str>,
) -> Result<(Vec<String>, Option<String>), TowlLlmError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| TowlLlmError::IoError {
            message: format!("Failed to read {}: {e}", path.display()),
        })?;

    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let start = line_number.saturating_sub(EXPANDED_CONTEXT_RADIUS + 1);
    let end = (line_number + EXPANDED_CONTEXT_RADIUS).min(total);
    let expanded: Vec<String> = lines[start..end]
        .iter()
        .map(|l| (*l).to_string()) // clone: &str -> owned String for Vec
        .collect();

    let is_brace_lang = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| BRACE_DELIMITED_EXTENSIONS.contains(&ext));

    let func_body = if is_brace_lang {
        function_name.and_then(|name| extract_function_body(&lines, name, line_number))
    } else {
        None
    };

    Ok((expanded, func_body))
}

fn extract_function_body(lines: &[&str], function_name: &str, todo_line: usize) -> Option<String> {
    let search_start = todo_line.saturating_sub(1);
    let search_end = search_start.saturating_sub(50);

    let mut func_start = None;
    for i in (search_end..=search_start).rev() {
        if i < lines.len() && lines[i].contains(function_name) {
            func_start = Some(i);
            break;
        }
    }

    let start = func_start?;
    let mut depth: i32 = 0;
    let mut found_open = false;
    let mut func_end = start;

    for (i, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                found_open = true;
            }
            if ch == '}' {
                depth -= 1;
            }
        }
        func_end = i;
        if found_open && depth <= 0 {
            break;
        }
        if i > start + 200 {
            break;
        }
    }

    let body: String = lines[start..=func_end].join("\n");
    Some(body)
}

async fn analyse_single_todo(
    todo: &mut TodoComment,
    provider: &LlmProvider,
    api_key: &SecretString,
) -> Result<Validity, TowlLlmError> {
    let file_path_str = todo.file_path.display().to_string(); // clone: Display -> owned for logging

    let (expanded_context, function_body) = gather_expanded_context(
        &todo.file_path,
        todo.line_number,
        todo.function_context.as_deref(),
    )
    .await
    .unwrap_or_else(|e| {
        warn!("Failed to gather context for {}: {e}", file_path_str);
        (todo.context_lines.clone(), None) // clone: fallback to existing context
    });

    let user_content = build_user_content(
        &todo.description,
        &file_path_str,
        todo.line_number,
        &expanded_context,
        function_body.as_deref(),
    );

    let (response_text, _usage) = provider
        .call_raw(&user_content, SYSTEM_PROMPT, api_key)
        .await?;
    let mut result = parse_analysis_result(&response_text)?;
    result.confidence = result.confidence.clamp(0.0, 1.0);

    let validity = result.validity;
    debug!(
        file = %file_path_str,
        validity = %validity,
        confidence = format!("{:.0}%", result.confidence * 100.0),
        "TODO analysis complete"
    );
    todo.analysis = Some(result);
    Ok(validity)
}

/// Analyses TODOs using an LLM, attaching results to each `TodoComment`.
///
/// Respects `config.max_analyse_count` (hard cap). TODOs beyond the cap are skipped.
/// Calls `on_progress(completed, total)` after each TODO is analysed.
///
/// # Errors
/// Returns `TowlLlmError::NotConfigured` if the API key is empty for API providers.
/// Returns `TowlLlmError::UnsupportedProvider` if the provider is unknown.
pub async fn analyse_todos(
    todos: &mut [TodoComment],
    config: &LlmConfig,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<AnalysisSummary, TowlLlmError> {
    let provider = build_provider(config)?;

    if !provider.is_cli_provider() && config.api_key.expose_secret().is_empty() {
        return Err(TowlLlmError::NotConfigured);
    }
    let api_key = config.api_key.clone(); // clone: SecretString for analysis calls

    let count = todos.len().min(config.max_analyse_count);
    if todos.len() > config.max_analyse_count {
        warn!(
            "TODO count ({}) exceeds analysis limit ({}), analysing first {} only",
            todos.len(),
            config.max_analyse_count,
            count
        );
    }

    info!(
        "Analysing {} TODOs with {} ({})",
        count, config.provider, config.model
    );

    let mut summary = AnalysisSummary::default();

    for (i, todo) in todos.iter_mut().take(count).enumerate() {
        match analyse_single_todo(todo, &provider, &api_key).await {
            Ok(validity) => match validity {
                Validity::Valid => summary.valid_count += 1,
                Validity::Invalid => summary.invalid_count += 1,
                Validity::Uncertain => summary.uncertain_count += 1,
            },
            Err(e) => {
                error!(
                    file = %todo.file_path.display(),
                    "LLM analysis failed: {e}"
                );
                summary.error_count += 1;
            }
        }
        on_progress(i + 1, count);
    }

    info!(
        "Analysis complete: {} valid, {} invalid, {} uncertain, {} errors",
        summary.valid_count, summary.invalid_count, summary.uncertain_count, summary.error_count
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_gather_expanded_context_window() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.rs");
        let content: String = (0..50).map(|i| format!("line {i}\n")).collect();
        std::fs::write(&file, &content).unwrap();

        let (lines, func_body) = gather_expanded_context(&file, 25, None).await.unwrap();
        assert!(lines.len() >= 20);
        assert!(lines.iter().any(|l| l.contains("line 24")));
        assert!(func_body.is_none());
    }

    #[tokio::test]
    async fn test_gather_expanded_context_with_function() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.rs");
        let content =
            "fn before() {}\n\nfn target() {\n    let x = 1;\n    // TODO: fix\n    let y = 2;\n}\n\nfn after() {}\n";
        std::fs::write(&file, content).unwrap();

        let (_, func_body) = gather_expanded_context(&file, 5, Some("target"))
            .await
            .unwrap();
        let body = func_body.expect("should find function body");
        assert!(body.contains("fn target()"));
        assert!(body.contains("let y = 2;"));
    }

    #[tokio::test]
    async fn test_gather_expanded_context_skips_python() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.py");
        let content = "def target():\n    x = 1\n    # TODO: fix\n    y = 2\n";
        std::fs::write(&file, content).unwrap();

        let (lines, func_body) = gather_expanded_context(&file, 3, Some("target"))
            .await
            .unwrap();
        assert!(!lines.is_empty());
        assert!(
            func_body.is_none(),
            "Should skip function body for Python files"
        );
    }

    #[tokio::test]
    async fn test_gather_expanded_context_small_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("small.rs");
        std::fs::write(&file, "// TODO: fix\nfn main() {}\n").unwrap();

        let (lines, _) = gather_expanded_context(&file, 1, None).await.unwrap();
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("TODO")));
    }

    #[tokio::test]
    async fn test_gather_expanded_context_nonexistent_file() {
        let result = gather_expanded_context(Path::new("/nonexistent/file.rs"), 1, None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, TowlLlmError::IoError { .. }),
            "Expected IoError, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_analyse_todos_not_configured() {
        let config = LlmConfig::default();
        let mut todos = vec![];
        let result = analyse_todos(&mut todos, &config, |_, _| {}).await;
        assert!(matches!(result, Err(TowlLlmError::NotConfigured)));
    }

    #[tokio::test]
    async fn test_analyse_todos_cli_fallback_no_key_fails() {
        let config = LlmConfig {
            provider: "claude-code".to_string(),
            command: Some("nonexistent_binary_xyz".to_string()),
            ..Default::default()
        };
        let mut todos = vec![];
        let result = analyse_todos(&mut todos, &config, |_, _| {}).await;
        assert!(
            matches!(result, Err(TowlLlmError::NotConfigured)),
            "CLI fallback to API with no key should fail as NotConfigured"
        );
    }

    #[tokio::test]
    async fn test_gather_expanded_context_function_not_found() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.rs");
        let content = "fn other() {\n    let x = 1;\n}\n";
        std::fs::write(&file, content).unwrap();

        let (_, func_body) = gather_expanded_context(&file, 2, Some("nonexistent"))
            .await
            .unwrap();
        assert!(func_body.is_none(), "Should not find nonexistent function");
    }
}
