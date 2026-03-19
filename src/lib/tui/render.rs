use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use ratatui::Frame;

use crate::comment::todo::TodoComment;
use crate::llm::types::Validity;

use super::app::{App, AppMode, CreatingState, DoneState, PeekState, SortField};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_filter_bar(frame, chunks[1], app);
    draw_todo_list(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);

    match app.mode() {
        AppMode::Peek(state) => draw_peek_popup(frame, state),
        AppMode::Confirm => draw_confirm_popup(frame),
        AppMode::Creating(state) => draw_creating_popup(frame, state),
        AppMode::Done(state) => draw_done_popup(frame, state),
        AppMode::DeleteConfirm(todos) => draw_delete_confirm_popup(frame, todos),
        AppMode::Browse => {}
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.todos().len();
    let visible = app.filtered_indices().len();
    let selected = app.selected_count();

    let text = format!(" tOwl | {visible}/{total} TODOs shown | {selected} selected");
    let block = Block::default().borders(Borders::ALL).title(" Todo Owl ");
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_filter_bar(frame: &mut Frame, area: Rect, app: &App) {
    let filter_text = app.filter_type().map_or_else(
        || " Filter: All".to_string(), // clone: &str → owned String for widget
        |t| format!(" Filter: {t}"),
    );

    let sort_text = match app.sort_field() {
        SortField::File => "File",
        SortField::Line => "Line",
        SortField::Priority => "Priority",
        SortField::Type => "Type",
    };

    let direction = if app.sort_ascending() { "asc" } else { "desc" };
    let line = Line::from(vec![
        Span::styled(filter_text, Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(
            format!("Sort: {sort_text} ({direction})"),
            Style::default().fg(Color::Yellow),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn validity_indicator(todo: &TodoComment) -> (&str, Option<Color>) {
    match todo.analysis.as_ref().map(|a| a.validity) {
        Some(Validity::Valid) => ("V", Some(Color::Green)),
        Some(Validity::Invalid) => ("I", Some(Color::Red)),
        Some(Validity::Uncertain) => ("?", Some(Color::Yellow)),
        None => ("-", None),
    }
}

fn build_todo_list_item(todo: &TodoComment, is_selected: bool) -> ListItem {
    let marker = if is_selected { "[x]" } else { "[ ]" };
    let (status, validity_colour) = validity_indicator(todo);
    let file_display = todo
        .file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let text = format!(
        " {marker} {status} [{:5}] {}:{} - {}",
        todo.todo_type, file_display, todo.line_number, todo.description,
    );

    let style = if let Some(colour) = validity_colour {
        Style::default().fg(colour)
    } else if is_selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    ListItem::new(text).style(style)
}

fn draw_todo_list(frame: &mut Frame, area: Rect, app: &App) {
    let todos = app.todos();
    let items: Vec<ListItem> = app
        .filtered_indices()
        .iter()
        .map(|&idx| build_todo_list_item(&todos[idx], app.is_selected(idx)))
        .collect();

    let item_count = items.len();
    let block = Block::default().borders(Borders::ALL).title(" TODOs ");
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let mut list_state = ListState::default().with_selected(Some(app.cursor()));
    frame.render_stateful_widget(list, area, &mut list_state);

    let mut scrollbar_state = ScrollbarState::new(item_count).position(app.cursor());
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area,
        &mut scrollbar_state,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = match app.mode() {
        AppMode::Browse => {
            " j/k:nav  space:toggle  a:all  n:none  f:filter  s:sort  r:reverse  p:peek  d:delete  enter:confirm  q:quit"
        }
        AppMode::Peek(_) => " j/k:scroll  p/esc:close",
        AppMode::Confirm => " y/enter:yes  n/esc:no",
        AppMode::Creating(_) => " Creating issues...",
        AppMode::Done(_) => " q/enter:quit",
        AppMode::DeleteConfirm(_) => " y/enter:delete  n/esc:cancel",
    };

    let line = Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray)));
    frame.render_widget(Paragraph::new(line), area);
}

const fn popup_area(frame: &Frame, width_pct: u16, height: u16) -> Rect {
    let area = frame.area();
    let popup_width = area.width.saturating_mul(width_pct) / 100;
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    Rect::new(x, y, popup_width, height)
}

const fn popup_area_pct(frame: &Frame, width_pct: u16, height_pct: u16) -> Rect {
    let area = frame.area();
    let w = area.width.saturating_mul(width_pct) / 100;
    let h = area.height.saturating_mul(height_pct) / 100;
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w, h)
}

fn draw_peek_popup(frame: &mut Frame, state: &PeekState) {
    let area = popup_area_pct(frame, 85, 80);
    frame.render_widget(Clear, area);

    let title = format!(" {} L{} ", state.file, state.todo_line);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Cyan));

    let inner_height = usize::from(area.height.saturating_sub(2));
    let mut visible: Vec<Line> = state
        .lines
        .iter()
        .skip(state.scroll)
        .take(inner_height)
        .map(|(line_num, content)| {
            let is_todo_line = *line_num == state.todo_line;
            let num_span = Span::styled(
                format!("{line_num:>4} "),
                Style::default().fg(Color::DarkGray),
            );
            let code_style = if is_todo_line {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let code_span = Span::styled(content.as_str(), code_style);
            Line::from(vec![num_span, code_span])
        })
        .collect();

    if let Some(ref analysis) = state.analysis {
        let separator = Line::from(Span::styled(
            "─── AI Analysis ───",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        visible.push(separator);

        let (validity_text, colour) = match analysis.validity {
            Validity::Valid => ("Valid", Color::Green),
            Validity::Invalid => ("Invalid", Color::Red),
            Validity::Uncertain => ("Uncertain", Color::Yellow),
        };
        visible.push(Line::from(vec![
            Span::styled(" Validity: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{validity_text} ({:.0}%)", analysis.confidence * 100.0),
                Style::default().fg(colour),
            ),
        ]));
        let inner_width = area.width.saturating_sub(2);
        visible.extend(wrap_labelled_text(
            " Reasoning: ",
            &analysis.reasoning,
            inner_width,
        ));
    }

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, area);

    let mut scrollbar_state = ScrollbarState::new(state.lines.len()).position(state.scroll);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area,
        &mut scrollbar_state,
    );
}

fn wrap_labelled_text<'a>(label: &'a str, text: &'a str, max_width: u16) -> Vec<Line<'a>> {
    let width = usize::from(max_width);
    let label_len = label.len();
    let content_width = width.saturating_sub(label_len);

    if content_width < 10 {
        return vec![Line::from(vec![
            Span::styled(label, Style::default().fg(Color::DarkGray)),
            Span::raw(text),
        ])];
    }

    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut current = String::new();
    let mut is_first = true;

    for word in text.split_whitespace() {
        let needed = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };

        if needed > content_width && !current.is_empty() {
            let prefix = if is_first {
                is_first = false;
                Span::styled(label, Style::default().fg(Color::DarkGray))
            } else {
                Span::raw(" ".repeat(label_len))
            };
            lines.push(Line::from(vec![prefix, Span::raw(current)]));
            current = word.to_string();
        } else if current.is_empty() {
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    let prefix = if is_first {
        Span::styled(label, Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(" ".repeat(label_len))
    };
    lines.push(Line::from(vec![prefix, Span::raw(current)]));
    lines
}

fn draw_confirm_popup(frame: &mut Frame) {
    let area = popup_area(frame, 50, 5);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .style(Style::default().fg(Color::Yellow));

    let text =
        Paragraph::new(" Create GitHub issues for selected TODOs?\n y/enter = yes, n/esc = no")
            .block(block);
    frame.render_widget(text, area);
}

fn draw_creating_popup(frame: &mut Frame, state: &CreatingState) {
    let area = popup_area(frame, 70, 7);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Creating Issues ")
        .style(Style::default().fg(Color::Cyan));

    let text = Paragraph::new(format!(
        " {}\n Progress: {}/{}\n Errors: {}",
        state.phase,
        state.progress,
        state.total,
        state.errors.len()
    ))
    .block(block);
    frame.render_widget(text, area);
}

fn draw_done_popup(frame: &mut Frame, state: &DoneState) {
    let error_lines: Vec<String> = state.errors.iter().map(|e| format!("   {e}")).collect();
    let error_count = error_lines.len();
    let popup_height = u16::try_from(error_count.saturating_add(5).clamp(7, 20)).unwrap_or(20);

    let area = popup_area(frame, 70, popup_height);
    frame.render_widget(Clear, area);

    let border_color = if error_count > 0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Done ")
        .style(Style::default().fg(border_color));

    let mut body = format!(
        " Created: {} issues\n Errors: {error_count}",
        state.created_issues.len(),
    );
    for line in &error_lines {
        body.push('\n');
        body.push_str(line);
    }
    body.push_str("\n\n Press q or enter to exit");

    let text = Paragraph::new(body).block(block);
    frame.render_widget(text, area);
}

fn draw_delete_confirm_popup(frame: &mut Frame, todos: &[TodoComment]) {
    let list_lines: Vec<String> = todos
        .iter()
        .take(10)
        .map(|t| {
            format!(
                "  {}:{} - {}",
                t.file_path.display(),
                t.line_number,
                t.description
            )
        })
        .collect();
    let extra = if todos.len() > 10 {
        format!("\n  ... and {} more", todos.len() - 10)
    } else {
        String::new()
    };

    let popup_height = u16::try_from((list_lines.len() + 5).clamp(7, 20)).unwrap_or(20);
    let area = popup_area(frame, 70, popup_height);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Delete {} invalid TODOs? ", todos.len()))
        .style(Style::default().fg(Color::Red));

    let mut body = String::new();
    for line in &list_lines {
        body.push_str(line);
        body.push('\n');
    }
    body.push_str(&extra);
    body.push_str("\n\n y/enter = delete, n/esc = cancel");

    let text = Paragraph::new(body).block(block);
    frame.render_widget(text, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;
    use crate::comment::todo::TodoType;
    use crate::github::types::CreatedIssue;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_todos() -> Vec<TodoComment> {
        vec![
            TestTodoBuilder::new()
                .todo_type(TodoType::Todo)
                .file_path("src/main.rs")
                .line_number(10)
                .column_start(3)
                .column_end(30)
                .description("implement error handling")
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Fixme)
                .file_path("src/lib.rs")
                .line_number(25)
                .column_start(5)
                .column_end(40)
                .description("fix memory leak")
                .build(),
            TestTodoBuilder::new()
                .todo_type(TodoType::Bug)
                .file_path("tests/integration.rs")
                .line_number(42)
                .column_start(1)
                .column_end(20)
                .description("race condition in tests")
                .build(),
        ]
    }

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut lines = Vec::new();
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            lines.push(line.trim_end().to_string());
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines.join("\n")
    }

    fn render_to_string(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        buffer_to_string(terminal.backend().buffer())
    }

    #[test]
    fn snapshot_browse_mode() {
        let app = App::new(sample_todos());
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_browse_empty() {
        let app = App::new(vec![]);
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_confirm_popup() {
        let mut app = App::new(sample_todos());
        app.toggle_select();
        app.set_mode(AppMode::Confirm);
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_creating_popup() {
        let mut app = App::new(sample_todos());
        app.set_mode(AppMode::Creating(CreatingState {
            phase: "Creating issues...".to_string(),
            progress: 2,
            total: 5,
            errors: Vec::new(),
            created_issues: Vec::new(),
        }));
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_done_success() {
        let mut app = App::new(sample_todos());
        app.set_mode(AppMode::Done(DoneState {
            created_issues: vec![
                CreatedIssue {
                    number: 1,
                    title: "TODO: implement error handling".to_string(),
                    html_url: "https://github.com/owner/repo/issues/1".to_string(),
                    todo_id: "src/main.rs_L10".to_string(),
                },
                CreatedIssue {
                    number: 2,
                    title: "FIXME: fix memory leak".to_string(),
                    html_url: "https://github.com/owner/repo/issues/2".to_string(),
                    todo_id: "src/lib.rs_L25".to_string(),
                },
            ],
            errors: Vec::new(),
        }));
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_done_with_errors() {
        let mut app = App::new(sample_todos());
        app.set_mode(AppMode::Done(DoneState {
            created_issues: vec![CreatedIssue {
                number: 1,
                title: "TODO: implement error handling".to_string(),
                html_url: "https://github.com/owner/repo/issues/1".to_string(),
                todo_id: "src/main.rs_L10".to_string(),
            }],
            errors: vec!["Failed to create issue: rate limited".to_string()],
        }));
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_peek_popup() {
        let mut app = App::new(sample_todos());
        app.enter_peek();
        let output = render_to_string(&app, 80, 20);
        insta::assert_snapshot!(output);
    }
}
