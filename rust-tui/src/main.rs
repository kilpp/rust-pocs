use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use sysinfo::{Disks, Networks, System};
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
    cpu_cores: Vec<f32>,
    mem_total: u64,
    mem_used: u64,
    mem_available: u64,
    mem_swap_total: u64,
    mem_swap_used: u64,
    disks_info: Vec<(String, u64, u64)>, // (mount_point, total, available)
    networks_info: Vec<(String, u64, u64, String)>, // (name, rx_bps, tx_bps, kind)
    tick: usize,
}

impl App {
    fn new() -> Self {
        App {
            selected_item: 0,
            items: vec![
                "CPU".to_string(),
                "Memory".to_string(),
                "Disk".to_string(),
                "Network".to_string(),
            ],
            cpu_history: Vec::new(),
            mem_history: Vec::new(),
            disk_history: Vec::new(),
            disk_available: 0,
            cpu_cores: Vec::new(),
            mem_total: 0,
            mem_used: 0,
            mem_available: 0,
            mem_swap_total: 0,
            mem_swap_used: 0,
            disks_info: Vec::new(),
            networks_info: Vec::new(),
            tick: 0,
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
    let mut networks = Networks::new_with_refreshed_list();
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

        // Per-core CPU usage
        app.cpu_cores = sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        // Memory breakdown
        app.mem_total = sys.total_memory();
        app.mem_used = sys.used_memory();
        app.mem_available = sys.available_memory();
        app.mem_swap_total = sys.total_swap();
        app.mem_swap_used = sys.used_swap();

        // Disk usage: refresh disks and compute aggregate usage/available
        disks.refresh();
        let mut total_disk: u64 = 0;
        let mut avail_disk: u64 = 0;
        app.disks_info.clear();
        for d in disks.list() {
            total_disk = total_disk.saturating_add(d.total_space());
            avail_disk = avail_disk.saturating_add(d.available_space());
            let mount = d.mount_point().to_string_lossy().to_string();
            app.disks_info.push((mount, d.total_space(), d.available_space()));
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

        // Networks: refresh and compute approximate speeds (bytes/sec)
        networks.refresh();
        app.networks_info.clear();
        for (name, net) in networks.list() {
            // net.received()/transmitted() give bytes since last refresh; our loop polls ~500ms
            let rx = net.received();
            let tx = net.transmitted();
            // convert to bytes/sec assuming ~500ms interval
            let rx_bps = rx.saturating_mul(2);
            let tx_bps = tx.saturating_mul(2);
            let kind = {
                // Prefer checking sysfs on Linux to detect wireless interfaces reliably
                #[cfg(target_os = "linux")]
                {
                    use std::path::Path;
                    let wireless_path = format!("/sys/class/net/{}/wireless", name);
                    if Path::new(&wireless_path).exists() {
                        "Wi-Fi".to_string()
                    } else {
                        // If the device directory exists it's likely a physical interface (Ethernet)
                        let device_path = format!("/sys/class/net/{}/device", name);
                        if Path::new(&device_path).exists() {
                            "Ethernet".to_string()
                        } else {
                            // Fallback to name heuristics
                            if name.contains("wl") || name.to_lowercase().contains("wifi") || name.to_lowercase().contains("wlan") {
                                "Wi-Fi".to_string()
                            } else if name.contains("en") || name.to_lowercase().contains("eth") || name.to_lowercase().contains("enp") {
                                "Ethernet".to_string()
                            } else {
                                "Unknown".to_string()
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    // Non-Linux fallback heuristics
                    if name.contains("wl") || name.to_lowercase().contains("wifi") || name.to_lowercase().contains("wlan") {
                        "Wi-Fi".to_string()
                    } else if name.contains("en") || name.to_lowercase().contains("eth") || name.to_lowercase().contains("enp") {
                        "Ethernet".to_string()
                    } else {
                        "Unknown".to_string()
                    }
                }
            };
            app.networks_info.push((name.clone(), rx_bps, tx_bps, kind));
        }

        // Animation tick for simple indicator
        app.tick = app.tick.wrapping_add(1);

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
