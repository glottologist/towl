use std::collections::HashSet;

use crate::comment::todo::{TodoComment, TodoType};
use crate::github::types::CreatedIssue;

const PEEK_CONTEXT: usize = 10;

/// State for the source-code peek overlay showing lines around a TODO.
#[derive(Debug)]
pub struct PeekState {
    pub lines: Vec<(usize, String)>,
    pub file: String,
    pub todo_line: usize,
    pub scroll: usize,
    pub analysis: Option<crate::llm::types::AnalysisResult>,
}

/// State tracked during background GitHub issue creation.
#[derive(Debug)]
pub struct CreatingState {
    pub phase: String,
    pub progress: usize,
    pub total: usize,
    pub errors: Vec<String>,
    pub created_issues: Vec<CreatedIssue>,
}

/// Final state after issue creation completes, showing results and errors.
#[derive(Debug)]
pub struct DoneState {
    pub created_issues: Vec<CreatedIssue>,
    pub errors: Vec<String>,
}

/// The current UI mode, determining which view is rendered and which keys are active.
#[derive(Debug)]
pub enum AppMode {
    Browse,
    Peek(PeekState),
    Confirm,
    Creating(CreatingState),
    Done(DoneState),
    DeleteConfirm(Vec<TodoComment>),
}

/// Field used to sort the TODO list. Cycle with the `s` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    File,
    Line,
    Priority,
    Type,
}

const ALL_TYPES: [TodoType; 5] = [
    TodoType::Todo,
    TodoType::Fixme,
    TodoType::Hack,
    TodoType::Note,
    TodoType::Bug,
];

/// Core TUI application state: TODO list, selection, filtering, sorting, and mode.
pub struct App {
    todos: Vec<TodoComment>,
    filtered: Vec<usize>,
    selected: HashSet<usize>,
    cursor: usize,
    filter_type: Option<TodoType>,
    sort_field: SortField,
    sort_ascending: bool,
    mode: AppMode,
    pending_delete: Option<Vec<TodoComment>>,
}

impl App {
    #[must_use]
    pub fn new(todos: Vec<TodoComment>) -> Self {
        let filtered: Vec<usize> = (0..todos.len()).collect();
        Self {
            todos,
            filtered,
            selected: HashSet::new(),
            cursor: 0,
            filter_type: None,
            sort_field: SortField::File,
            sort_ascending: true,
            mode: AppMode::Browse,
            pending_delete: None,
        }
    }

    #[must_use]
    pub fn todos(&self) -> &[TodoComment] {
        &self.todos
    }

    #[must_use]
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered
    }

    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    #[must_use]
    pub const fn filter_type(&self) -> Option<TodoType> {
        self.filter_type
    }

    #[must_use]
    pub const fn sort_field(&self) -> SortField {
        self.sort_field
    }

    #[must_use]
    pub const fn sort_ascending(&self) -> bool {
        self.sort_ascending
    }

    #[must_use]
    pub fn is_selected(&self, todo_idx: usize) -> bool {
        self.selected.contains(&todo_idx)
    }

    #[must_use]
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    #[must_use]
    pub fn selected_todos(&self) -> Vec<TodoComment> {
        self.selected
            .iter()
            .filter_map(|&idx| self.todos.get(idx).cloned()) // clone: caller needs owned copies for GitHub issue creation
            .collect()
    }

    #[must_use]
    pub const fn mode(&self) -> &AppMode {
        &self.mode
    }

    pub fn set_creation_phase(&mut self, phase: String) {
        if let AppMode::Creating(state) = &mut self.mode {
            state.phase = phase;
        }
    }

    pub fn set_creation_progress(&mut self, current: usize, total: usize) {
        if let AppMode::Creating(state) = &mut self.mode {
            state.progress = current;
            state.total = total;
        }
    }

    pub fn push_creation_error(&mut self, msg: String) {
        if let AppMode::Creating(state) = &mut self.mode {
            state.errors.push(msg);
        }
    }

    pub fn push_created_issue(&mut self, issue: CreatedIssue) {
        if let AppMode::Creating(state) = &mut self.mode {
            state.created_issues.push(issue);
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered.is_empty() {
            self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
            if self.cursor + 1 < self.filtered.len() {
                self.cursor += 1;
            }
        }
    }

    pub fn toggle_select(&mut self) {
        if let Some(&todo_idx) = self.filtered.get(self.cursor) {
            if self.selected.contains(&todo_idx) {
                self.selected.remove(&todo_idx);
            } else {
                self.selected.insert(todo_idx);
            }
        }
    }

    pub fn select_all_visible(&mut self) {
        for &idx in &self.filtered {
            self.selected.insert(idx);
        }
    }

    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    pub fn cycle_filter(&mut self) {
        self.filter_type = self.filter_type.map_or_else(
            || Some(ALL_TYPES[0]),
            |current| {
                ALL_TYPES
                    .iter()
                    .position(|&t| t == current)
                    .and_then(|i| ALL_TYPES.get(i + 1).copied())
            },
        );
        self.rebuild_filtered();
    }

    pub fn cycle_sort(&mut self) {
        self.sort_field = match self.sort_field {
            SortField::File => SortField::Line,
            SortField::Line => SortField::Priority,
            SortField::Priority => SortField::Type,
            SortField::Type => SortField::File,
        };
        self.sort_filtered();
    }

    pub fn reverse_sort(&mut self) {
        self.sort_ascending = !self.sort_ascending;
        self.sort_filtered();
    }

    pub fn enter_confirm(&mut self) {
        if !self.selected.is_empty() {
            self.mode = AppMode::Confirm;
        }
    }

    pub fn cancel_confirm(&mut self) {
        self.mode = AppMode::Browse;
    }

    pub fn enter_delete_confirm(&mut self) {
        use crate::llm::types::Validity;

        let invalid_selected: Vec<TodoComment> = self
            .selected
            .iter()
            .filter_map(|&idx| self.todos.get(idx))
            .filter(|t| {
                matches!(
                    t.analysis.as_ref().map(|a| a.validity),
                    Some(Validity::Invalid)
                )
            })
            .cloned() // clone: owned copies for confirmation display
            .collect();
        if !invalid_selected.is_empty() {
            self.mode = AppMode::DeleteConfirm(invalid_selected);
        }
    }

    pub fn cancel_delete(&mut self) {
        self.mode = AppMode::Browse;
    }

    pub fn start_deleting(&mut self) {
        let old = std::mem::replace(&mut self.mode, AppMode::Browse);
        if let AppMode::DeleteConfirm(todos) = old {
            let count = todos.len();
            self.pending_delete = Some(todos);
            self.mode = AppMode::Creating(CreatingState {
                phase: format!("Deleting {count} invalid TODOs..."),
                progress: 0,
                total: count,
                errors: Vec::new(),
                created_issues: Vec::new(),
            });
        }
    }

    pub fn take_pending_delete(&mut self) -> Option<Vec<TodoComment>> {
        self.pending_delete.take()
    }

    pub fn start_creating(&mut self) {
        self.mode = AppMode::Creating(CreatingState {
            phase: "Starting...".to_string(), // clone: &str → owned String for struct field
            progress: 0,
            total: self.selected.len(),
            errors: Vec::new(),
            created_issues: Vec::new(),
        });
    }

    pub fn finish_creating(&mut self) {
        let old = std::mem::replace(&mut self.mode, AppMode::Browse);
        if let AppMode::Creating(state) = old {
            self.mode = AppMode::Done(DoneState {
                created_issues: state.created_issues,
                errors: state.errors,
            });
        }
    }

    pub fn enter_peek(&mut self) {
        let Some(&todo_idx) = self.filtered.get(self.cursor) else {
            return;
        };
        let todo = &self.todos[todo_idx];
        let file = todo.file_path.display().to_string(); // clone: Display → owned String
        let todo_line = todo.line_number;

        let start = todo.line_number.saturating_sub(PEEK_CONTEXT + 1);
        let end = todo.line_number.saturating_add(PEEK_CONTEXT);

        let lines = match todo.file_path.canonicalize() {
            Ok(canonical) => match std::fs::read_to_string(&canonical) {
                Ok(content) => content
                    .lines()
                    .enumerate()
                    .filter(|&(i, _)| i >= start && i < end)
                    .map(|(i, line)| (i + 1, line.to_string())) // clone: &str → owned String
                    .collect(),
                Err(e) => vec![(0, format!("Could not read file: {e}"))],
            },
            Err(e) => vec![(0, format!("Could not read file: {e}"))],
        };

        let scroll = lines
            .iter()
            .position(|(n, _)| *n == todo_line)
            .unwrap_or(0)
            .saturating_sub(PEEK_CONTEXT / 2);

        let analysis = todo.analysis.clone(); // clone: owned copy for PeekState

        self.mode = AppMode::Peek(PeekState {
            lines,
            file,
            todo_line,
            scroll,
            analysis,
        });
    }

    pub fn exit_peek(&mut self) {
        self.mode = AppMode::Browse;
    }

    pub fn peek_scroll_up(&mut self) {
        if let AppMode::Peek(state) = &mut self.mode {
            state.scroll = state.scroll.saturating_sub(1);
        }
    }

    pub fn peek_scroll_down(&mut self) {
        if let AppMode::Peek(state) = &mut self.mode {
            if state.scroll + 1 < state.lines.len() {
                state.scroll += 1;
            }
        }
    }

    fn rebuild_filtered(&mut self) {
        self.filtered = (0..self.todos.len())
            .filter(|&i| match self.filter_type {
                None => true,
                Some(t) => self.todos[i].todo_type == t,
            })
            .collect();
        self.sort_filtered();
        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
    }

    fn sort_filtered(&mut self) {
        let todos = &self.todos;
        let field = self.sort_field;
        let ascending = self.sort_ascending;

        self.filtered.sort_by(|&a, &b| {
            let ord = match field {
                SortField::File => todos[a]
                    .file_path
                    .cmp(&todos[b].file_path)
                    .then(todos[a].line_number.cmp(&todos[b].line_number)),
                SortField::Line => todos[a].line_number.cmp(&todos[b].line_number),
                SortField::Priority => todos[a]
                    .todo_type
                    .priority()
                    .cmp(&todos[b].todo_type.priority()),
                SortField::Type => todos[a]
                    .todo_type
                    .as_filter_str()
                    .cmp(todos[b].todo_type.as_filter_str()),
            };
            if ascending {
                ord
            } else {
                ord.reverse()
            }
        });
    }
}

#[cfg(test)]
impl App {
    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;
    use proptest::prelude::*;
    use rstest::rstest;

    fn sample_todos() -> Vec<TodoComment> {
        vec![
            TestTodoBuilder::new()
                .todo_type(TodoType::Todo)
                .file_path("a.rs")
                .line_number(10)
                .column_start(3)
                .column_end(20)
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Fixme)
                .file_path("b.rs")
                .line_number(5)
                .column_start(3)
                .column_end(20)
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Hack)
                .file_path("a.rs")
                .line_number(20)
                .column_start(3)
                .column_end(20)
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Bug)
                .file_path("c.rs")
                .line_number(1)
                .column_start(3)
                .column_end(20)
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Note)
                .file_path("b.rs")
                .line_number(15)
                .column_start(3)
                .column_end(20)
                .build(),
        ]
    }

    #[rstest]
    #[case(Some(TodoType::Todo), 1)]
    #[case(Some(TodoType::Fixme), 1)]
    #[case(Some(TodoType::Bug), 1)]
    #[case(Some(TodoType::Hack), 1)]
    #[case(Some(TodoType::Note), 1)]
    #[case(None, 5)]
    fn test_filter_by_type(#[case] filter: Option<TodoType>, #[case] expected: usize) {
        let mut app = App::new(sample_todos());
        app.filter_type = filter;
        app.rebuild_filtered();
        assert_eq!(app.filtered.len(), expected);
    }

    #[test]
    fn test_mode_transitions() {
        let mut app = App::new(sample_todos());

        assert!(matches!(app.mode(), AppMode::Browse));

        app.enter_confirm();
        assert!(
            matches!(app.mode(), AppMode::Browse),
            "no selection = stays Browse"
        );

        app.toggle_select();
        app.enter_confirm();
        assert!(matches!(app.mode(), AppMode::Confirm));

        app.cancel_confirm();
        assert!(matches!(app.mode(), AppMode::Browse));

        app.enter_confirm();
        app.start_creating();
        assert!(matches!(app.mode(), AppMode::Creating(_)));

        app.finish_creating();
        assert!(matches!(app.mode(), AppMode::Done(_)));
    }

    #[test]
    fn test_navigation_bounds() {
        let mut app = App::new(sample_todos());

        app.move_up();
        assert_eq!(app.cursor, 0);

        for _ in 0..20 {
            app.move_down();
        }
        assert_eq!(app.cursor, 4);

        app.move_up();
        assert_eq!(app.cursor, 3);
    }

    #[test]
    fn test_select_all_deselect_all() {
        let mut app = App::new(sample_todos());

        app.select_all_visible();
        assert_eq!(app.selected_count(), 5);

        app.deselect_all();
        assert_eq!(app.selected_count(), 0);
    }

    #[test]
    fn test_peek_mode_transitions() {
        let mut app = App::new(sample_todos());

        app.enter_peek();
        {
            let AppMode::Peek(state) = app.mode() else {
                panic!("expected Peek mode");
            };
            assert_eq!(state.file, "a.rs");
            assert_eq!(state.todo_line, 10);
            assert!(
                !state.lines.is_empty(),
                "should have error message for missing file"
            );
        }

        app.exit_peek();
        assert!(matches!(app.mode(), AppMode::Browse));
    }

    #[test]
    fn test_peek_on_empty_filtered_stays_browse() {
        let mut app = App::new(sample_todos());
        app.filter_type = Some(TodoType::Todo);
        app.rebuild_filtered();
        app.deselect_all();

        app.filter_type = Some(TodoType::Todo);
        app.rebuild_filtered();
        assert_eq!(app.filtered.len(), 1);

        app.filtered.clear();
        app.enter_peek();
        assert!(
            matches!(app.mode(), AppMode::Browse),
            "no items = can't peek"
        );
    }

    proptest! {
        #[test]
        fn prop_filter_preserves_items(
            filter_steps in 0usize..20,
        ) {
            let mut app = App::new(sample_todos());
            let total = app.todos.len();

            for _ in 0..filter_steps {
                app.cycle_filter();
                let visible: HashSet<usize> = app.filtered.iter().copied().collect();
                for &idx in &visible {
                    prop_assert!(idx < total);
                }
            }

            while app.filter_type.is_some() {
                app.cycle_filter();
            }
            prop_assert_eq!(app.filtered.len(), total);
        }

        #[test]
        fn prop_select_deselect_idempotent(
            toggles in prop::collection::vec(0usize..5, 0..20),
        ) {
            let mut app = App::new(sample_todos());

            for idx in &toggles {
                app.cursor = (*idx).min(app.filtered.len().saturating_sub(1));
                app.toggle_select();
            }

            let selected = app.selected_count();
            prop_assert!(selected <= app.todos.len());
        }

        #[test]
        fn prop_sort_preserves_all_items(
            sort_steps in 0usize..10,
            reverse_steps in 0usize..5,
        ) {
            let mut app = App::new(sample_todos());
            let total = app.todos.len();

            for _ in 0..sort_steps {
                app.cycle_sort();
            }
            for _ in 0..reverse_steps {
                app.reverse_sort();
            }

            prop_assert_eq!(app.filtered.len(), total);
            let unique: HashSet<usize> = app.filtered.iter().copied().collect();
            prop_assert_eq!(unique.len(), total);
        }

        #[test]
        fn prop_sort_ordering_correct(
            sort_steps in 1usize..10,
            do_reverse in proptest::bool::ANY,
        ) {
            let mut app = App::new(sample_todos());

            for _ in 0..sort_steps {
                app.cycle_sort();
            }
            if do_reverse {
                app.reverse_sort();
            }

            let todos = app.todos();
            let indices = app.filtered_indices();
            for window in indices.windows(2) {
                let a = window[0];
                let b = window[1];
                let ord = match app.sort_field() {
                    SortField::File => todos[a].file_path.cmp(&todos[b].file_path)
                        .then(todos[a].line_number.cmp(&todos[b].line_number)),
                    SortField::Line => todos[a].line_number.cmp(&todos[b].line_number),
                    SortField::Priority => todos[a].todo_type.priority()
                        .cmp(&todos[b].todo_type.priority()),
                    SortField::Type => todos[a].todo_type.as_filter_str()
                        .cmp(todos[b].todo_type.as_filter_str()),
                };
                let expected = if app.sort_ascending() { ord } else { ord.reverse() };
                prop_assert!(
                    expected.is_le(),
                    "Sort violation: {:?} should come before {:?} with field={:?} asc={}",
                    todos[a].file_path, todos[b].file_path,
                    app.sort_field(), app.sort_ascending()
                );
            }
        }

        #[test]
        fn prop_peek_scroll_bounds(
            up_steps in 0usize..30,
            down_steps in 0usize..30,
        ) {
            let mut app = App::new(sample_todos());
            app.enter_peek();

            let line_count = match app.mode() {
                AppMode::Peek(state) => state.lines.len(),
                _ => panic!("expected Peek mode"),
            };

            for _ in 0..down_steps {
                app.peek_scroll_down();
            }
            let scroll = match app.mode() {
                AppMode::Peek(state) => state.scroll,
                _ => panic!("expected Peek mode"),
            };
            prop_assert!(scroll < line_count || line_count == 0);

            for _ in 0..up_steps {
                app.peek_scroll_up();
            }
            let scroll = match app.mode() {
                AppMode::Peek(state) => state.scroll,
                _ => panic!("expected Peek mode"),
            };
            prop_assert!(scroll < line_count || line_count == 0);
        }
    }
}
