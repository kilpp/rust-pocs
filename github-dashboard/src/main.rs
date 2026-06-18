mod app;
mod claude;
mod config;
mod github;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use reqwest::Client;
use tokio::sync::mpsc::{self, UnboundedSender};

use app::{App, AppEvent, DashboardData};
use config::{Args, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = match Config::load(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    let client = match github::build_client(config.token()) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &config, client).await;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result?;
    Ok(())
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: &Config,
    client: Arc<Client>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(config);
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    spawn_fetch(&tx, &client, config);

    loop {
        while let Ok(event) = rx.try_recv() {
            app.apply(event);
        }

        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Char('o') | KeyCode::Enter => {
                        if let Some(url) = app.selected_pr_url() {
                            let _ = open_url(&url);
                        }
                    }
                    KeyCode::Tab | KeyCode::Char('u') => app.cycle_user(),
                    KeyCode::Char('r') => {
                        app.begin_loading();
                        spawn_fetch(&tx, &client, config);
                    }
                    KeyCode::Char('s') => spawn_summary(&tx, &mut app),
                    _ => {}
                }
            }
        } else {
            // Timed out with no input: advance the spinner animation.
            app.tick = app.tick.wrapping_add(1);
        }
    }

    Ok(())
}

/// Open a URL in the system browser, detached and silent so it can't disturb
/// the TUI. Best-effort: failures are ignored by the caller.
fn open_url(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};

    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(all(unix, not(target_os = "macos")))]
    let program = "xdg-open";

    Command::new(program)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

fn spawn_fetch(tx: &UnboundedSender<AppEvent>, client: &Arc<Client>, config: &Config) {
    let tx = tx.clone();
    let client = Arc::clone(client);
    let base_url = config.base_url.clone();
    let users = config.users.clone();
    let days = config.timeline_days;

    tokio::spawn(async move {
        let (prs_res, contrib_res) = tokio::join!(
            github::fetch_prs(&client, &base_url, &users, days),
            github::fetch_contributions(&client, &base_url, &users, days),
        );

        let result = match (prs_res, contrib_res) {
            (Ok(prs), Ok(contributions)) => Ok(DashboardData { prs, contributions }),
            (Err(e), _) | (_, Err(e)) => Err(e),
        };
        let _ = tx.send(AppEvent::FetchDone(result));
    });
}

fn spawn_summary(tx: &UnboundedSender<AppEvent>, app: &mut App) {
    if app.summarizing {
        return;
    }
    let Some(pr) = app.selected_pr() else {
        return;
    };

    let pr = pr.clone();
    let key = format!("{}#{}", pr.repo, pr.number);
    app.summarizing = true;
    app.summary = None;

    let tx = tx.clone();
    tokio::spawn(async move {
        let result = claude::summarize(&pr).await;
        let _ = tx.send(AppEvent::SummaryDone { key, result });
    });
}
