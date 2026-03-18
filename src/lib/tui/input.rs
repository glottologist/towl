use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, AppMode};

/// Result of processing a keyboard event.
pub enum Action {
    /// No state change; continue the event loop.
    Continue,
    /// Exit the TUI.
    Quit,
}

/// Polls for keyboard input and dispatches to mode-specific handlers.
///
/// # Errors
/// Returns `std::io::Error` if polling or reading terminal events fails.
pub fn handle_input(app: &mut App, timeout: std::time::Duration) -> std::io::Result<Action> {
    if !event::poll(timeout)? {
        return Ok(Action::Continue);
    }

    let event = event::read()?;
    let Event::Key(key) = event else {
        return Ok(Action::Continue);
    };

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(Action::Quit);
    }

    Ok(match app.mode() {
        AppMode::Browse => handle_browse(app, key),
        AppMode::Peek(_) => handle_peek(app, key),
        AppMode::Confirm => handle_confirm(app, key),
        AppMode::Creating(_) => Action::Continue,
        AppMode::Done(_) => handle_done(key),
    })
}

fn handle_browse(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char(' ') => app.toggle_select(),
        KeyCode::Char('a') => app.select_all_visible(),
        KeyCode::Char('n') => app.deselect_all(),
        KeyCode::Char('f') => app.cycle_filter(),
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('r') => app.reverse_sort(),
        KeyCode::Char('p') => app.enter_peek(),
        KeyCode::Enter => app.enter_confirm(),
        _ => {}
    }
    Action::Continue
}

fn handle_peek(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('p' | 'q') | KeyCode::Esc => app.exit_peek(),
        KeyCode::Char('j') | KeyCode::Down => app.peek_scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.peek_scroll_up(),
        _ => {}
    }
    Action::Continue
}

fn handle_confirm(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            app.start_creating();
        }
        KeyCode::Char('n' | 'q') | KeyCode::Esc => {
            app.cancel_confirm();
        }
        _ => {}
    }
    Action::Continue
}

const fn handle_done(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => Action::Quit,
        _ => Action::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;
    use crate::comment::todo::TodoType;
    use rstest::rstest;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn test_app() -> App {
        App::new(vec![
            TestTodoBuilder::new()
                .todo_type(TodoType::Todo)
                .file_path("test.rs")
                .line_number(1)
                .column_start(3)
                .column_end(20)
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Fixme)
                .file_path("test2.rs")
                .line_number(5)
                .column_start(3)
                .column_end(20)
                .build(),
        ])
    }

    #[rstest]
    #[case(KeyCode::Char('q'), true)]
    #[case(KeyCode::Esc, true)]
    #[case(KeyCode::Char('j'), false)]
    #[case(KeyCode::Down, false)]
    #[case(KeyCode::Char('k'), false)]
    #[case(KeyCode::Up, false)]
    #[case(KeyCode::Char(' '), false)]
    #[case(KeyCode::Char('a'), false)]
    #[case(KeyCode::Char('n'), false)]
    #[case(KeyCode::Char('f'), false)]
    #[case(KeyCode::Char('s'), false)]
    #[case(KeyCode::Char('r'), false)]
    #[case(KeyCode::Enter, false)]
    fn test_browse_key_mapping(#[case] code: KeyCode, #[case] is_quit: bool) {
        let mut app = test_app();
        let result = handle_browse(&mut app, make_key(code));
        assert_eq!(matches!(result, Action::Quit), is_quit);
    }

    #[rstest]
    #[case(KeyCode::Char('y'), true)]
    #[case(KeyCode::Enter, true)]
    #[case(KeyCode::Char('n'), false)]
    #[case(KeyCode::Esc, false)]
    #[case(KeyCode::Char('q'), false)]
    fn test_confirm_key_mapping(#[case] code: KeyCode, #[case] starts_creating: bool) {
        let mut app = test_app();
        app.toggle_select();
        app.enter_confirm();
        assert!(matches!(app.mode(), AppMode::Confirm));

        handle_confirm(&mut app, make_key(code));
        assert_eq!(matches!(app.mode(), AppMode::Creating(_)), starts_creating,);
    }

    #[rstest]
    #[case(KeyCode::Char('p'), true)]
    #[case(KeyCode::Char('q'), true)]
    #[case(KeyCode::Esc, true)]
    #[case(KeyCode::Char('j'), false)]
    #[case(KeyCode::Down, false)]
    #[case(KeyCode::Char('k'), false)]
    #[case(KeyCode::Up, false)]
    #[case(KeyCode::Char('x'), false)]
    fn test_peek_key_mapping(#[case] code: KeyCode, #[case] exits_peek: bool) {
        let mut app = test_app();
        app.enter_peek();
        handle_peek(&mut app, make_key(code));
        assert_eq!(matches!(app.mode(), AppMode::Browse), exits_peek);
    }

    #[rstest]
    #[case(KeyCode::Char('q'), true)]
    #[case(KeyCode::Esc, true)]
    #[case(KeyCode::Enter, true)]
    #[case(KeyCode::Char('x'), false)]
    fn test_done_key_mapping(#[case] code: KeyCode, #[case] is_quit: bool) {
        let result = handle_done(make_key(code));
        assert_eq!(matches!(result, Action::Quit), is_quit);
    }
}
