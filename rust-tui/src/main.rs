use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use sysinfo::{Disks, System};
use std::io;

mod ui;
use ui::UIRenderer;

pub struct App {
    selected_item: usize,
    items: Vec<String>,
    cpu_history: Vec<u64>,
    mem_history: Vec<u64>,
    disk_history: Vec<u64>,
    disk_available: u64,
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
            cpu_history: Vec::new(),
            mem_history: Vec::new(),
            disk_history: Vec::new(),
            disk_available: 0,
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
    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();
    const HISTORY_LEN: usize = 100;

    loop {
        // Refresh system metrics
        sys.refresh_cpu();
        sys.refresh_memory();

        // CPU usage (percentage)
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let cpu_pct = cpu_usage.round() as u64;
        app.cpu_history.push(cpu_pct);
        if app.cpu_history.len() > HISTORY_LEN {
            app.cpu_history.remove(0);
        }

        // Memory usage (percentage)
        let total_mem = sys.total_memory() as f64;
        let used_mem = sys.used_memory() as f64;
        let mem_pct = if total_mem > 0.0 { ((used_mem / total_mem) * 100.0).round() as u64 } else { 0 };
        app.mem_history.push(mem_pct);
        if app.mem_history.len() > HISTORY_LEN {
            app.mem_history.remove(0);
        }

        // Disk usage: refresh disks and compute aggregate usage/available
        disks.refresh();
        let mut total_disk: u64 = 0;
        let mut avail_disk: u64 = 0;
        for d in disks.list() {
            total_disk = total_disk.saturating_add(d.total_space());
            avail_disk = avail_disk.saturating_add(d.available_space());
        }
        let used_disk = total_disk.saturating_sub(avail_disk);
        let disk_pct = if total_disk > 0 {
            ((used_disk as f64 / total_disk as f64) * 100.0).round() as u64
        } else {
            0
        };
        app.disk_history.push(disk_pct);
        if app.disk_history.len() > HISTORY_LEN {
            app.disk_history.remove(0);
        }
        app.disk_available = avail_disk;

        // Draw UI
        terminal.draw(|f| UIRenderer::render(f, &app))?;

        // Handle input events
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
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
