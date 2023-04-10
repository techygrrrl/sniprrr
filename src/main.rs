use std::{error::Error, io};

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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

use crate::models::Snippet;

mod models;

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct AppState {
    /// Current value of the input box
    title_input: String,
    description_input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<Snippet>,
    /// The state of the table
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
            input_mode: InputMode::Normal,
            table_state: TableState::default(),
            messages: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = AppState::default();
    let res = run_app(&mut terminal, app);

    // restore terminal
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: AppState) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('c') => {
                        match Clipboard::new() {
                            Ok(mut clipboard) => {
                                let selected_snippet = get_selected_snippet(&app);
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
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                },
                InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Enter => {
                        let snippet = Snippet {
                            title: app.title_input.clone(),
                            description: app.title_input.clone(), // TODO: Collect user input for this
                        };

                        app.messages.push(snippet);
                    }
                    KeyCode::Char(c) => {
                        app.title_input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.title_input.pop();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
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
                Constraint::Length(3),
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

    let input = Paragraph::new(app.title_input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask ratatui to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.title_input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    let normal_style = Style::default().bg(Color::Blue);
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

        Row::new(vec![title_cell, description_cell])
            .height(height as u16)
            .bottom_margin(1)
    });

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Snippets"))
        .highlight_style(selected_style)
        .highlight_symbol("🦀 ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);
}
