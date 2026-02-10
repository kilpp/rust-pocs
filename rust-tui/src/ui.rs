use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

use crate::App;

pub struct UIRenderer;

impl UIRenderer {
    pub fn render(f: &mut Frame, app: &App) {
        Self::render_layout(f, app);
    }

    fn render_layout(f: &mut Frame, app: &App) {
        let chunks = Self::create_layout(f);

        Self::render_left_panel(f, app, chunks[0]);
        Self::render_central_panel(f, app, chunks[1]);
    }

    fn create_layout(f: &mut Frame) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(25), // Left panel
                    Constraint::Percentage(75), // Central panel
                ]
                .as_ref(),
            )
            .split(f.area())
            .to_vec()
    }

    fn render_left_panel(f: &mut Frame, app: &App, area: Rect) {
        let panel_block = Block::default()
            .title(" Computer Resources ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        // Show CPU, Memory, Disk with current percentages
        let cpu = app.cpu_history.last().cloned().unwrap_or(0);
        let mem = app.mem_history.last().cloned().unwrap_or(0);

        let disk = app.disk_history.last().cloned().unwrap_or(0);

        let lines = vec![
            Line::from(vec![Span::styled(
                format!("CPU: {}%", cpu),
                if app.selected_item == 0 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]),
            Line::from(vec![Span::styled(
                format!("Memory: {}%", mem),
                if app.selected_item == 1 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]),
            Line::from(vec![Span::styled(
                format!("Disk: {}%  Avail: {}", disk, Self::format_bytes(app.disk_available)),
                if app.selected_item == 2 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )]),
        ];

        let content = Paragraph::new(lines).block(panel_block);
        f.render_widget(content, area);
    }

    fn render_central_panel(f: &mut Frame, app: &App, area: Rect) {
        let panel_block = Block::default()
            .title(" Resource Graphs ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        // Render a detailed view for the selected resource using more space
        match app.selected_item {
            0 => Self::render_cpu_view(f, app, area, panel_block),
            1 => Self::render_mem_view(f, app, area, panel_block),
            2 => Self::render_disk_view(f, app, area, panel_block),
            _ => {
                let empty = Paragraph::new("No resource selected").block(panel_block);
                f.render_widget(empty, area);
            }
        }
    }

    fn render_cpu_view(f: &mut Frame, app: &App, area: Rect, panel_block: Block) {
        // CPU detailed: big sparkline + gauge + current value
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let cpu_data: Vec<u64> = app.cpu_history.clone();
        let spark = Sparkline::default()
            .block(Block::default().title("CPU % (history)").borders(Borders::ALL))
            .data(&cpu_data)
            .style(Style::default().fg(Color::Magenta));
        f.render_widget(spark, chunks[0]);

        let current = app.cpu_history.last().cloned().unwrap_or(0) as f64 / 100.0;
        let gauge = Gauge::default()
            .block(Block::default().title("CPU Usage").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Magenta))
            .ratio(current)
            .label(format!("{}%", app.cpu_history.last().cloned().unwrap_or(0)));
        f.render_widget(gauge, chunks[1]);

        let info = Paragraph::new(vec![
            Line::from(format!("Current CPU: {}%", app.cpu_history.last().cloned().unwrap_or(0))),
            Line::from("Per-core details not shown."),
        ])
        .block(panel_block);
        f.render_widget(info, chunks[2]);
    }

    fn render_mem_view(f: &mut Frame, app: &App, area: Rect, panel_block: Block) {
        // Memory detailed: sparkline + gauge + usage text
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(6), Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let mem_data: Vec<u64> = app.mem_history.clone();
        let spark = Sparkline::default()
            .block(Block::default().title("Memory % (history)").borders(Borders::ALL))
            .data(&mem_data)
            .style(Style::default().fg(Color::Green));
        f.render_widget(spark, chunks[0]);

        let current = app.mem_history.last().cloned().unwrap_or(0) as f64 / 100.0;
        let gauge = Gauge::default()
            .block(Block::default().title("Memory Usage").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(current)
            .label(format!("{}%", app.mem_history.last().cloned().unwrap_or(0)));
        f.render_widget(gauge, chunks[1]);

        let info = Paragraph::new(vec![
            Line::from(format!("Current Memory: {}%", app.mem_history.last().cloned().unwrap_or(0))),
            Line::from("Total/available details are not shown."),
        ])
        .block(panel_block);
        f.render_widget(info, chunks[2]);
    }

    fn render_disk_view(f: &mut Frame, app: &App, area: Rect, panel_block: Block) {
        // Disk detailed: sparkline + gauge + available bytes
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(6), Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let disk_data: Vec<u64> = app.disk_history.clone();
        let spark = Sparkline::default()
            .block(Block::default().title("Disk % (history)").borders(Borders::ALL))
            .data(&disk_data)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(spark, chunks[0]);

        let current = app.disk_history.last().cloned().unwrap_or(0) as f64 / 100.0;
        let gauge = Gauge::default()
            .block(Block::default().title("Disk Usage").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Yellow))
            .ratio(current)
            .label(format!("{}%", app.disk_history.last().cloned().unwrap_or(0)));
        f.render_widget(gauge, chunks[1]);

        let info = Paragraph::new(vec![
            Line::from(format!("Available: {}", Self::format_bytes(app.disk_available))),
            Line::from("Aggregated across mounts."),
        ])
        .block(panel_block);
        f.render_widget(info, chunks[2]);
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        let b = bytes as f64;
        if b >= GB {
            format!("{:.1} GiB", b / GB)
        } else if b >= MB {
            format!("{:.1} MiB", b / MB)
        } else if b >= KB {
            format!("{:.1} KiB", b / KB)
        } else {
            format!("{} B", bytes)
        }
    }
}
