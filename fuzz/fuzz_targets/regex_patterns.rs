#![no_main]
use libfuzzer_sys::fuzz_target;
use towl::config::ParsingConfig;
use towl::parser::{parse_content, validate_patterns};

fuzz_target!(|data: &[u8]| {
    if let Ok(pattern) = std::str::from_utf8(data) {
        let mut config = ParsingConfig {
            file_extensions: ["rs".to_string()].into_iter().collect(),
            exclude_patterns: vec![],
            include_context_lines: 3,
            comment_prefixes: vec![r"//".to_string()],
            todo_patterns: vec![pattern.to_string()],
            function_patterns: vec![],
        };

        let _ = validate_patterns(&config);

        config.todo_patterns = vec![r"(?i)\bTODO:\s*(.*)".to_string()];
        config.comment_prefixes = vec![pattern.to_string()];
        let _ = validate_patterns(&config);

        config.comment_prefixes = vec![r"//".to_string()];
        config.function_patterns = vec![pattern.to_string()];
        let _ = validate_patterns(&config);

        config.function_patterns = vec![];
        config.exclude_patterns = vec![pattern.to_string()];

        if validate_patterns(&config).is_ok() {
            let test_content = r#"
                // TODO: test
                fn main() {
                    // FIXME: something
                }
            "#;

            use std::path::Path;
            let _ = parse_content(&config, Path::new("test.rs"), test_content);

            if let Ok(content) = std::str::from_utf8(data) {
                let _ = parse_content(&config, Path::new("fuzz.rs"), content);
            }
        }
    }

    if data.len() >= 3 {
        let chunk_size = data.len() / 3;
        let patterns: Vec<String> = (0..3)
            .filter_map(|i| {
                let start = i * chunk_size;
                let end = if i == 2 { data.len() } else { (i + 1) * chunk_size };
                std::str::from_utf8(&data[start..end])
                    .ok()
                    .map(|s| s.to_string())
            })
            .collect();

        if patterns.len() == 3 {
            let config = ParsingConfig {
                file_extensions: ["rs".to_string()].into_iter().collect(),
                exclude_patterns: vec![],
                include_context_lines: 3,
                comment_prefixes: vec![patterns[0].clone()],
                todo_patterns: vec![patterns[1].clone()],
                function_patterns: vec![patterns[2].clone()],
            };

            let _ = validate_patterns(&config);
        }
    }
});
