pub const SYSTEM_PROMPT: &str = r#"You are a code analyst. Given a TODO comment and its surrounding code context, determine whether the TODO is still valid.

Respond with ONLY a JSON object (no markdown, no explanation outside the JSON):

{
  "validity": "valid" | "invalid" | "uncertain",
  "reasoning": "Brief explanation of why the TODO is valid, invalid, or uncertain",
  "is_resolved": true/false,
  "is_relevant": true/false,
  "is_actionable": true/false,
  "confidence": 0.0-1.0,
  "enrichment": "Enhanced description suitable for a GitHub issue body"
}

Rules:
- "invalid" means the TODO has been resolved (code already does what it asks), the code it references no longer exists, or the TODO is nonsensical
- "valid" means the TODO describes work that still needs to be done
- "uncertain" means you cannot determine validity from the available context
- "is_resolved": true if the surrounding code already implements what the TODO asks
- "is_relevant": true if the code/feature the TODO references still exists
- "is_actionable": true if the TODO describes a clear, specific task
- "enrichment" should be a 2-3 sentence description that adds context beyond the original TODO text, suitable for a GitHub issue body"#;

/// Formats TODO metadata and code context into XML-tagged user content for the LLM.
pub fn build_user_content(
    description: &str,
    file_path: &str,
    line_number: usize,
    expanded_context: &[String],
    function_body: Option<&str>,
) -> String {
    let mut content = format!(
        "<todo_comment>\nDescription: {description}\nFile: {file_path}\nLine: {line_number}\n</todo_comment>\n\n"
    );
    content.push_str("<surrounding_code>\n");
    for line in expanded_context {
        content.push_str(line);
        content.push('\n');
    }
    content.push_str("</surrounding_code>\n");

    if let Some(body) = function_body {
        content.push_str("\n<enclosing_function>\n");
        content.push_str(body);
        content.push_str("\n</enclosing_function>\n");
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_build_user_content_structure(
            desc in "[a-zA-Z0-9 ]{1,50}",
            file in "[a-zA-Z0-9_/\\.]{1,30}",
            line in 1usize..10000,
        ) {
            let context = vec!["some context line".to_string()];
            let content = build_user_content(&desc, &file, line, &context, None);
            prop_assert!(content.contains("<todo_comment>"));
            prop_assert!(content.contains("</todo_comment>"));
            prop_assert!(content.contains("<surrounding_code>"));
            prop_assert!(content.contains("</surrounding_code>"));
            prop_assert!(content.contains(&desc));
            prop_assert!(content.contains(&file));
            prop_assert!(!content.contains("<enclosing_function>"));

            let with_func = build_user_content(&desc, &file, line, &context, Some("fn test()"));
            prop_assert!(with_func.contains("<enclosing_function>"));
            prop_assert!(with_func.contains("</enclosing_function>"));
        }
    }

    #[test]
    fn test_build_user_content_empty_context() {
        let content = build_user_content("Fix this", "main.rs", 1, &[], None);
        assert!(content.contains("<todo_comment>"));
        assert!(content.contains("<surrounding_code>\n</surrounding_code>"));
    }
}
