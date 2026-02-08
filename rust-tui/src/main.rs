use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

struct App {
    selected_item: usize,
    items: Vec<String>,
}

impl App {
    fn new() -> Self {
        App {
            selected_item: 0,
            items: vec![
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
                "Item 4".to_string(),
            ],
        }
    }

    fn next_item(&mut self) {
        if self.selected_item < self.items.len() - 1 {
            self.selected_item += 1;
        }
    }

    fn previous_item(&mut self) {
        if self.selected_item > 0 {
            self.selected_item -= 1;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next_item();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.previous_item();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(25), // Left panel
                Constraint::Percentage(75), // Central panel
            ]
            .as_ref(),
        )
        .split(f.area());

    // Left Panel - Menu/Navigation
    let left_panel = Block::default()
        .title(" Menu ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let menu_items: Vec<Line> = app
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            if idx == app.selected_item {
                Line::from(vec![Span::styled(
                    format!("► {}", item),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )])
            } else {
                Line::from(vec![Span::raw(format!("  {}", item))])
            }
        })
        .collect();

    let left_content = Paragraph::new(menu_items)
        .block(left_panel)
        .style(Style::default().fg(Color::White));

    f.render_widget(left_content, chunks[0]);

    // Central Panel - Main Content
    let central_panel = Block::default()
        .title(" Content ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let selected_item = &app.items[app.selected_item];
    let content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("Selected: {}", selected_item),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Use ↑/↓ (or j/k) to navigate"),
        Line::from("Press 'q' or ESC to quit"),
        Line::from(""),
        Line::from("This is the main content area!"),
    ];

    let central_content = Paragraph::new(content)
        .block(central_panel)
        .style(Style::default().fg(Color::White));

    f.render_widget(central_content, chunks[1]);
}
