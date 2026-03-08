use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table};

use crate::monitor::EndpointStatus;

pub fn draw(frame: &mut Frame, endpoints: &[EndpointStatus], last_poll: Instant, interval: Duration) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, chunks[0]);
    draw_endpoints(frame, endpoints, chunks[1]);
    draw_footer(frame, chunks[2], last_poll, interval);
}

fn draw_header(frame: &mut Frame, area: ratatui::layout::Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" httpmon ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("- Live HTTP Health Monitor"),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Status Dashboard "));
    frame.render_widget(header, area);
}

fn draw_endpoints(frame: &mut Frame, endpoints: &[EndpointStatus], area: ratatui::layout::Rect) {
    let row_constraints: Vec<_> = std::iter::repeat_n(Constraint::Length(4), endpoints.len())
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();
    let rows = Layout::vertical(row_constraints).split(area);

    for (i, ep) in endpoints.iter().enumerate() {
        if i >= rows.len() - 1 {
            break;
        }
        let row_chunks = Layout::horizontal([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(rows[i]);

        draw_endpoint_table(frame, ep, row_chunks[0]);
        draw_sparkline(frame, ep, row_chunks[1]);
    }
}

fn draw_endpoint_table(frame: &mut Frame, ep: &EndpointStatus, area: ratatui::layout::Rect) {
    let (status_str, status_color) = match ep.status {
        Some(code) if (200..400).contains(&code) => (format!("{code}"), Color::Green),
        Some(code) => (format!("{code}"), Color::Red),
        None if ep.error.is_some() => ("ERR".to_string(), Color::Red),
        None => ("---".to_string(), Color::DarkGray),
    };

    let time_str = match ep.response_time_ms {
        Some(ms) => format!("{ms}ms"),
        None => "---".to_string(),
    };

    let uptime_str = if ep.total_count > 0 {
        format!("{:.1}%", ep.uptime_pct())
    } else {
        "---".to_string()
    };

    let last_check_str = ep.last_check.clone().unwrap_or("never".to_string());

    let rows = vec![Row::new(vec![
        Cell::from(Span::styled(&status_str, Style::default().fg(status_color).bold())),
        Cell::from(&*time_str),
        Cell::from(&*uptime_str),
        Cell::from(&*last_check_str),
    ])];

    let title = if let Some(err) = &ep.error {
        let short = if err.len() > 40 { &err[..40] } else { err };
        format!(" {} - {} ", ep.url, short)
    } else {
        format!(" {} ", ep.url)
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["Status", "Latency", "Uptime", "Last Check"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(status_color)),
    );
    frame.render_widget(table, area);
}

fn draw_sparkline(frame: &mut Frame, ep: &EndpointStatus, area: ratatui::layout::Rect) {
    let spark = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Response Time ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(&ep.history)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(spark, area);
}

fn draw_footer(frame: &mut Frame, area: ratatui::layout::Rect, last_poll: Instant, interval: Duration) {
    let next_poll = if last_poll.elapsed() < interval {
        interval.as_secs() - last_poll.elapsed().as_secs()
    } else {
        0
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).bold()),
        Span::raw(" quit  "),
        Span::styled("r", Style::default().fg(Color::Yellow).bold()),
        Span::raw(" force refresh  "),
        Span::raw(format!("next poll in {next_poll}s")),
    ]));
    frame.render_widget(footer, area);
}