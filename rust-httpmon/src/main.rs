mod monitor;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::Terminal;
use tokio::sync::mpsc;

use monitor::{EndpointStatus, poll_endpoints, build_client};

#[derive(Parser)]
#[command(name = "httpmon", about = "Live HTTP endpoint health monitor")]
struct Args {
    /// URLs to monitor (space-separated)
    #[arg(required = true)]
    urls: Vec<String>,

    /// Polling interval in seconds
    #[arg(short, long, default_value = "5")]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut endpoints: Vec<EndpointStatus> = args
        .urls
        .iter()
        .map(|u| EndpointStatus::new(u.clone()))
        .collect();

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let client = build_client()?;
    let interval = Duration::from_secs(args.interval);
    let mut last_poll = Instant::now() - interval;

    loop {
        if last_poll.elapsed() >= interval {
            last_poll = Instant::now();
            poll_endpoints(&endpoints, &client, &tx);
        }

        while let Ok(result) = rx.try_recv() {
            if let Some(ep) = endpoints.get_mut(result.index) {
                ep.apply_result(result);
            }
        }

        terminal.draw(|frame| ui::draw(frame, &endpoints, last_poll, interval))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => last_poll = Instant::now() - interval,
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    println!("Goodbye!");
    Ok(())
}