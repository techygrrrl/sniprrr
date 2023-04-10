use std::{error::Error, io};

use crate::file_utils::{load_messages_from_file, write_messages_to_file};
use arboard::Clipboard;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::widgets::{Cell, Row, Table, TableState};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

use crate::models::Snippet;

mod file_utils;
mod models;

enum InputMode {
    Normal,
    Editing,
}

const MAX_INPUT_COUNT: i8 = 2;
const INPUT_TITLE_INDEX: i8 = 0;
const INPUT_DESCRIPTION_INDEX: i8 = 1;

/// App holds the state of the application
struct AppState {
    title_input: String,
    description_input: String,
    focused_input_index: i8,
    input_mode: InputMode,
    messages: Vec<Snippet>,
    table_state: TableState,
}

impl AppState {
    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.messages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

impl Default for AppState {
    fn default() -> AppState {
        AppState {
            title_input: String::new(),
            description_input: String::new(),
            focused_input_index: INPUT_TITLE_INDEX,
            input_mode: InputMode::Normal,
            table_state: TableState::default(),
            messages: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::default();

    // Load from disk
    let messages = load_messages_from_file();
    app_state.messages = messages;

    let res = run_app(&mut terminal, app_state);

    // restore terminal / tear down
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app_state: AppState) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app_state))?;

        if let Event::Key(key) = event::read()? {
            match app_state.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app_state.focused_input_index = INPUT_TITLE_INDEX;
                        app_state.input_mode = InputMode::Editing;
                    }
                    KeyCode::Delete | KeyCode::Backspace => {
                        let selected = app_state.table_state.selected();
                        if let Some(selected) = selected {
                            app_state.messages.remove(selected);

                            let json_string =
                                serde_json::to_string::<Vec<Snippet>>(&app_state.messages).unwrap();
                            write_messages_to_file(&json_string)?
                        }
                    }
                    KeyCode::Char('c') => {
                        match Clipboard::new() {
                            Ok(mut clipboard) => {
                                let selected_snippet = get_selected_snippet(&app_state);
                                if selected_snippet.is_none() {
                                    return Ok(());
                                }

                                let selected_snippet = selected_snippet.unwrap();

                                match clipboard.set_text(&selected_snippet.description) {
                                    Ok(_) => return Ok(()),
                                    Err(_error) => {
                                        // TODO: handle copy error? - output to console instead
                                        // println!("{}", error)
                                    }
                                }
                            }
                            Err(error) => {
                                // TODO: Output to console
                                println!("{}", error)
                            }
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => app_state.next(),
                    KeyCode::Up | KeyCode::Char('k') => app_state.previous(),
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                },
                InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Tab => {
                        app_state.focused_input_index =
                            (app_state.focused_input_index + 1) % MAX_INPUT_COUNT
                    }
                    KeyCode::Enter => {
                        // If we are not on the last field, enter moves to the next field
                        if app_state.focused_input_index == MAX_INPUT_COUNT - 1 {
                            // Last field index
                            let snippet = Snippet {
                                title: app_state.title_input.clone(),
                                description: app_state.description_input.clone(),
                            };

                            app_state.messages.push(snippet);

                            app_state.title_input.clear();
                            app_state.description_input.clear();
                            app_state.input_mode = InputMode::Normal;

                            let json_string =
                                serde_json::to_string::<Vec<Snippet>>(&app_state.messages).unwrap();

                            write_messages_to_file(&json_string)?;
                        } else {
                            // Not the last field
                            // Move to next field
                            app_state.focused_input_index =
                                (app_state.focused_input_index + 1) % MAX_INPUT_COUNT
                        }
                    }
                    KeyCode::Char(c) => {
                        match app_state.focused_input_index {
                            INPUT_TITLE_INDEX => app_state.title_input.push(c),
                            INPUT_DESCRIPTION_INDEX => app_state.description_input.push(c),
                            _ => {}
                        };
                    }
                    KeyCode::Backspace => {
                        match app_state.focused_input_index {
                            INPUT_TITLE_INDEX => {
                                app_state.title_input.pop();
                            }
                            INPUT_DESCRIPTION_INDEX => {
                                app_state.description_input.pop();
                            }
                            _ => {}
                        };
                    }
                    KeyCode::Esc => {
                        app_state.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn get_selected_snippet(app: &AppState) -> Option<&Snippet> {
    let selected_index = app.table_state.selected()?;
    app.messages.get(selected_index)
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(6),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    // Split remaining chunk
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    // Render the title input
    let title_input = Paragraph::new(app.title_input.as_ref())
        .style(match (&app.input_mode, app.focused_input_index) {
            (InputMode::Editing, INPUT_TITLE_INDEX) => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Title"));

    f.render_widget(title_input, inner_chunks[0]);

    // Render the description input
    let description_input = Paragraph::new(app.description_input.as_ref())
        .style(match (&app.input_mode, app.focused_input_index) {
            (InputMode::Editing, INPUT_DESCRIPTION_INDEX) => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Description"));

    f.render_widget(description_input, inner_chunks[1]);

    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            match app.focused_input_index {
                INPUT_TITLE_INDEX => {
                    f.set_cursor(
                        chunks[1].x + app.title_input.width() as u16 + 1,
                        chunks[1].y + 1,
                    );
                }
                INPUT_DESCRIPTION_INDEX => {
                    f.set_cursor(
                        inner_chunks[1].x + app.description_input.width() as u16 + 1,
                        inner_chunks[1].y + 1,
                    );
                }
                _ => {}
            };
        }
    }

    let normal_style = Style::default().bg(Color::Rgb(0xff, 0x00, 0xff));
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);

    // Create rows for the data

    let header_cells = vec!["Title", "Description"];
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);

    let rows = app.messages.iter().map(|snippet| {
        let height = snippet.description.chars().filter(|c| *c == '\n').count() + 1;

        let title_cell = Cell::from(snippet.title.clone());
        let description_cell = Cell::from(snippet.description.clone());

        Row::new(vec![title_cell, description_cell]).height(height as u16)
    });

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Snippets"))
        .highlight_style(selected_style)
        // .highlight_symbol("🦀 ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);
}
