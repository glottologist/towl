use serde::{Deserialize, Serialize};
use std::fmt;

/// Whether a TODO is still valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Validity {
    Valid,
    Invalid,
    Uncertain,
}

impl Validity {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Valid => "Valid",
            Self::Invalid => "Invalid",
            Self::Uncertain => "Uncertain",
        }
    }
}

impl fmt::Display for Validity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// LLM analysis of a TODO comment's validity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub validity: Validity,
    pub reasoning: String,
    pub is_resolved: bool,
    pub is_relevant: bool,
    pub is_actionable: bool,
    pub confidence: f64,
    pub enrichment: String,
}

/// Summary counts from a batch analysis run.
#[derive(Debug, Clone, Default)]
pub struct AnalysisSummary {
    pub valid_count: usize,
    pub invalid_count: usize,
    pub uncertain_count: usize,
    pub error_count: usize,
}

/// Token usage from a single LLM call.
#[derive(Debug, Clone, Default)]
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn find_matching_brace(text: &str, open_pos: usize) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text[open_pos..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Extracts a JSON object from LLM response text.
///
/// Searches in priority order:
/// 1. ` ```json ... ``` ` code block
/// 2. ` ``` ... ``` ` generic code block
/// 3. Bare `{ ... }` JSON object
/// 4. Fallback: trimmed input
pub fn extract_json_block(text: &str) -> &str {
    if let Some(start) = text.find("```json") {
        let content_start = start + "```json".len();
        let remaining = &text[content_start..];
        if let Some(end) = remaining.find("```") {
            return remaining[..end].trim();
        }
    }
    if let Some(start) = text.find("```") {
        let content_start = start + "```".len();
        let remaining = &text[content_start..];
        if let Some(end) = remaining.find("```") {
            return remaining[..end].trim();
        }
    }
    if let Some(start) = text.find('{') {
        if let Some(end) = find_matching_brace(text, start) {
            return &text[start..=end];
        }
    }
    text.trim()
}

/// Parses an `AnalysisResult` from raw LLM response text.
///
/// # Errors
/// Returns `TowlLlmError::ParseError` if the extracted JSON cannot be deserialised.
pub fn parse_analysis_result(text: &str) -> Result<AnalysisResult, super::error::TowlLlmError> {
    let json_str = extract_json_block(text);
    serde_json::from_str(json_str).map_err(|e| super::error::TowlLlmError::ParseError {
        message: format!("Failed to parse analysis: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_extract_json_from_code_block(
            key in "[a-z_]{1,20}",
            value in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let json = format!("{{\"{key}\": \"{value}\"}}");
            let wrapped = format!("Some text\n```json\n{json}\n```\nMore text");
            let extracted = extract_json_block(&wrapped);
            prop_assert_eq!(extracted, json.as_str());
        }

        #[test]
        fn prop_extract_bare_json(
            key in "[a-z_]{1,20}",
            value in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let json = format!("{{\"{key}\": \"{value}\"}}");
            let wrapped = format!("Here is the result: {json} end");
            let extracted = extract_json_block(&wrapped);
            prop_assert_eq!(extracted, json.as_str());
        }

        #[test]
        fn prop_extract_generic_code_block(
            key in "[a-z_]{1,20}",
            value in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let json = format!("{{\"{key}\": \"{value}\"}}");
            let wrapped = format!("Result:\n```\n{json}\n```\nDone");
            let extracted = extract_json_block(&wrapped);
            prop_assert_eq!(extracted, json.as_str());
        }

        #[test]
        fn prop_find_matching_brace_balanced(
            inner in "[a-zA-Z0-9: ,]{0,100}",
        ) {
            let text = format!("{{{inner}}}");
            let result = find_matching_brace(&text, 0);
            prop_assert_eq!(result, Some(text.len() - 1));
        }
    }

    #[test]
    fn test_parse_analysis_result_valid_json() {
        let json = r#"```json
{
    "validity": "valid",
    "reasoning": "The cache is not implemented",
    "is_resolved": false,
    "is_relevant": true,
    "is_actionable": true,
    "confidence": 0.95,
    "enrichment": "Needs a caching layer"
}
```"#;
        let result = parse_analysis_result(json).unwrap();
        assert_eq!(result.validity, Validity::Valid);
        assert!(!result.is_resolved);
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_parse_analysis_result_invalid_json() {
        let result = parse_analysis_result("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_block_nested_braces() {
        let input = r#"Here is the result: {"outer": {"inner": "value"}} some trailing text"#;
        let extracted = extract_json_block(input);
        assert_eq!(extracted, r#"{"outer": {"inner": "value"}}"#);
    }

    #[test]
    fn test_extract_json_block_with_trailing_braces_in_commentary() {
        let input = r#"{"validity": "valid"} (note: use {} for config)"#;
        let extracted = extract_json_block(input);
        assert_eq!(extracted, r#"{"validity": "valid"}"#);
    }
}
