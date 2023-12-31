mod client;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use std::io::Stdout;
use std::time::Duration;
use tui_textarea::{CursorMove, Input, Key};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use client::{Task, TaskSpawner};

type CrosstermTerminal = Terminal<CrosstermBackend<Stdout>>;

struct State<'a> {
    textarea: tui_textarea::TextArea<'a>,
    messages: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = init_terminal()?;

    let mut textarea = tui_textarea::TextArea::default();
    textarea.set_placeholder_text("Enter some text.");
    textarea.set_block(Block::default().borders(Borders::ALL));

    let mut state = State {
        textarea,
        messages: vec![],
    };

    let (spawner, mut rx) = TaskSpawner::new();

    loop {
        terminal.draw(|f| ui(f, &state))?;

        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        event::KeyCode::Char(ch) => {
                            state.textarea.insert_char(ch);
                        }
                        event::KeyCode::Backspace => {
                            state.textarea.delete_char();
                        }
                        event::KeyCode::Esc => {
                            break;
                        }
                        event::KeyCode::Enter => {
                            let msg = &state.textarea.lines()[0];
                            if !msg.is_empty() {
                                spawner.spawn_task(Task::send(msg));
                                // clear line
                                state.textarea.move_cursor(CursorMove::End);
                                state.textarea.delete_line_by_head();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        match rx.try_recv() {
            Ok(msg) => {
                state.messages.push(msg);
            }
            Err(_) => {}
        }
    }

    reset_terminal(&mut terminal)
}

fn ui(f: &mut Frame, state: &State) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(4)
        .constraints([Constraint::Percentage(80), Constraint::Max(3)])
        .split(f.size());

    let messages = state
        .messages
        .iter()
        .map(|msg| Line::from(msg.clone()))
        .collect::<Vec<_>>();
    f.render_widget(
        Paragraph::new(messages).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    f.render_widget(state.textarea.widget(), chunks[1]);
}

fn reset_terminal(terminal: &mut CrosstermTerminal) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(terminal.show_cursor()?)
}

fn init_terminal() -> Result<CrosstermTerminal, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}
