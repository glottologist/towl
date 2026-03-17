use super::types::Parser;

const BACKWARD_SEARCH_LINES: usize = 50;
const FORWARD_SEARCH_LINES: usize = 3;

impl Parser {
    pub(super) fn extract_context(&self, lines: &[&str], current_line: usize) -> Vec<String> {
        let mut context = Vec::new();

        let start = current_line.saturating_sub(self.context_lines);

        let end = std::cmp::min(current_line + self.context_lines + 1, lines.len());

        for (i, line) in lines.iter().enumerate().take(end).skip(start) {
            if i != current_line {
                context.push(format!("{}: {}", i + 1, line));
            }
        }

        context
    }

    pub(super) fn match_function_name<'a>(&self, line: &'a str) -> Option<&'a str> {
        for pattern in &self.function_patterns {
            if let Some(captures) = pattern.captures(line) {
                for j in 1..captures.len() {
                    if let Some(name) = captures.get(j) {
                        let name_str = name.as_str();
                        if !name_str.is_empty()
                            && name_str.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            return Some(name_str);
                        }
                    }
                }
            }
        }
        None
    }

    pub(super) fn find_function_context(
        &self,
        lines: &[&str],
        current_line: usize,
    ) -> Option<String> {
        let search_start = current_line.saturating_sub(BACKWARD_SEARCH_LINES);
        for i in (search_start..=current_line).rev() {
            if let Some(name) = self.match_function_name(lines[i]) {
                return Some(format!("{name}:{}", i + 1));
            }
        }

        let search_end = std::cmp::min(current_line + FORWARD_SEARCH_LINES + 1, lines.len());
        for (i, line) in lines
            .iter()
            .enumerate()
            .take(search_end)
            .skip(current_line + 1)
        {
            if let Some(name) = self.match_function_name(line) {
                return Some(format!("{name}:{} (below)", i + 1));
            }
        }

        None
    }
}
