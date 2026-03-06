use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreatedIssue {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub todo_id: String,
}

impl CreatedIssue {
    #[must_use]
    pub fn new(number: u64, title: String, html_url: String, todo_id: String) -> Self {
        Self {
            number,
            title,
            html_url,
            todo_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_created_issue_json_roundtrip(
            number in 1u64..100_000,
            title in "[a-zA-Z0-9 ]{1,50}",
            todo_id in "[a-zA-Z0-9_]{1,30}"
        ) {
            let url = format!("https://github.com/owner/repo/issues/{number}");
            let issue = CreatedIssue::new(
                number,
                title.clone(), // clone: proptest needs owned value for assertion
                url.clone(), // clone: proptest needs owned value for assertion
                todo_id.clone(), // clone: proptest needs owned value for assertion
            );

            let json = serde_json::to_string(&issue).unwrap();
            let decoded: CreatedIssue = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(decoded, issue);
        }
    }
}
