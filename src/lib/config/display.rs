use std::fmt;

use secrecy::ExposeSecret;

use super::types::TowlConfig;

fn fmt_list_section(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    items: &[String],
    is_last: bool,
) -> fmt::Result {
    let branch = if is_last {
        "│  └─"
    } else {
        "│  ├─"
    };
    writeln!(f, "{branch} {label}:")?;
    let (mid, end) = if is_last {
        ("│     ├─", "│     └─")
    } else {
        ("│  │  ├─", "│  │  └─")
    };
    for (i, item) in items.iter().enumerate() {
        let prefix = if i == items.len() - 1 { end } else { mid };
        writeln!(f, "{prefix} {item}")?;
    }
    Ok(())
}

impl fmt::Display for TowlConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Towl Configuration")?;
        writeln!(f, "┌─ Parsing")?;
        let mut sorted_extensions: Vec<_> = self.parsing.file_extensions.iter().collect();
        sorted_extensions.sort();
        writeln!(
            f,
            "│  ├─ File Extensions: {}",
            sorted_extensions
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(
            f,
            "│  ├─ Exclude Patterns: {}",
            self.parsing.exclude_patterns.join(", ")
        )?;
        writeln!(
            f,
            "│  ├─ Context Lines: {}",
            self.parsing.include_context_lines
        )?;
        fmt_list_section(f, "Comment Prefixes", &self.parsing.comment_prefixes, false)?;
        fmt_list_section(f, "TODO Patterns", &self.parsing.todo_patterns, false)?;
        fmt_list_section(
            f,
            "Function Patterns",
            &self.parsing.function_patterns,
            true,
        )?;
        writeln!(f, "├─ GitHub")?;
        writeln!(f, "│  ├─ Owner: {}", self.github.owner)?;
        writeln!(f, "│  ├─ Repo: {}", self.github.repo)?;
        writeln!(
            f,
            "│  ├─ Token: {}",
            if self.github.token.expose_secret().is_empty() {
                "not set"
            } else {
                "configured"
            }
        )?;
        writeln!(
            f,
            "│  └─ Rate Limit Delay: {}ms",
            self.github.rate_limit_delay_ms
        )?;
        writeln!(f, "└─ LLM")?;
        writeln!(f, "   ├─ Provider: {}", self.llm.provider)?;
        writeln!(f, "   ├─ Model: {}", self.llm.model)?;
        writeln!(
            f,
            "   ├─ Base URL: {}",
            self.llm.base_url.as_deref().unwrap_or("provider default")
        )?;
        writeln!(
            f,
            "   ├─ API Key: {}",
            if self.llm.api_key.expose_secret().is_empty() {
                "not set"
            } else {
                "configured"
            }
        )?;
        writeln!(
            f,
            "   ├─ Max Concurrent Analyses: {}",
            self.llm.max_concurrent_analyses
        )?;
        writeln!(f, "   ├─ Max Analyse Count: {}", self.llm.max_analyse_count)?;
        writeln!(f, "   ├─ Max Tokens: {}", self.llm.max_tokens)?;
        writeln!(f, "   ├─ Max Retries: {}", self.llm.max_retries)?;
        write!(
            f,
            "   └─ Command: {}",
            self.llm.command.as_deref().unwrap_or("none")
        )
    }
}
