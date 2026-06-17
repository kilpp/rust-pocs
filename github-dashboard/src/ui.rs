use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap,
};

use crate::app::{App, PR_LIST_LIMIT, Status};

/// Braille frames for the loading spinner.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_body(frame, app, chunks[1]);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = match &app.status {
        Status::Loading => {
            let frame = SPINNER[(app.tick as usize) % SPINNER.len()];
            let elapsed = app
                .loading_since
                .map(|t| (Local::now() - t).num_seconds().max(0))
                .unwrap_or(0);
            (format!("{frame} loading… {elapsed}s"), Color::Yellow)
        }
        Status::Ready => {
            let refreshed = app
                .last_refresh
                .map(|t| t.format("%H:%M:%S").to_string())
                .unwrap_or_default();
            (
                format!(
                    "{} languages · {} PR contributions · filter: {} · sort: {} · refreshed {}",
                    app.stats.len(),
                    app.total_prs(),
                    app.filter_label(),
                    app.sort_mode.label(),
                    refreshed
                ),
                Color::Green,
            )
        }
        Status::Error(e) => (format!("error: {e}"), Color::Red),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " github-dashboard ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "· users: {} · window: last {} days",
                app.users.join(", "),
                app.timeline_days
            )),
        ]),
        Line::from(vec![
            Span::styled("API: ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.base_url.clone()),
            Span::raw("  "),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
    ];

    let header = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Contributions "));
    frame.render_widget(header, area);
}

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Left column: contributions on top, language breakdown below.
    let contrib_height = ((app.users.len() as u16) * 2 + 2).clamp(4, 14);
    let left = Layout::vertical([Constraint::Length(contrib_height), Constraint::Min(0)])
        .split(cols[0]);

    draw_contributions(frame, app, left[0]);
    draw_languages(frame, app, left[1]);
    draw_summary(frame, app, cols[1]);
}

/// Render a compact unicode sparkline for a series of counts.
fn sparkline(data: &[u64]) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if data.is_empty() {
        return String::new();
    }
    let max = data.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return "▁".repeat(data.len());
    }
    data.iter()
        .map(|&v| BARS[(((v * 7) / max) as usize).min(7)])
        .collect()
}

fn draw_contributions(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Contributions ");

    let visible = app.visible_contributions();
    if visible.is_empty() {
        let msg = match &app.status {
            Status::Loading => "Fetching contributions…",
            Status::Error(_) => "Could not load contributions (see header).",
            Status::Ready => "No contributions in this window.",
        };
        let placeholder = Paragraph::new(msg)
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for c in visible {
        let mut headline = vec![
            Span::styled(
                c.user.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {} contributions", c.total)),
        ];
        let spark = sparkline(&c.daily);
        if !spark.is_empty() {
            headline.push(Span::raw("  "));
            headline.push(Span::styled(spark, Style::default().fg(Color::Green)));
        }
        lines.push(Line::from(headline));

        let mut breakdown = vec![
            Span::raw("  commits "),
            Span::styled(c.commits.to_string(), Style::default().fg(Color::Green)),
            Span::raw("  PRs "),
            Span::styled(c.prs.to_string(), Style::default().fg(Color::Blue)),
            Span::raw("  reviews "),
            Span::styled(c.reviews.to_string(), Style::default().fg(Color::Yellow)),
            Span::raw("  issues "),
            Span::styled(c.issues.to_string(), Style::default().fg(Color::Magenta)),
        ];
        if c.private > 0 {
            breakdown.push(Span::raw("  private "));
            breakdown.push(Span::styled(
                c.private.to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(breakdown));
    }

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn draw_languages(frame: &mut Frame, app: &App, area: Rect) {
    if app.stats.is_empty() {
        let msg = match &app.status {
            Status::Loading => "Fetching pull requests…",
            Status::Error(_) => "Could not load data (see header).",
            Status::Ready => "No pull requests in this window.",
        };
        let placeholder = Paragraph::new(msg)
            .block(Block::default().borders(Borders::ALL).title(" Languages "))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, area);
        return;
    }

    let max_total = app.stats.iter().map(|s| s.total()).max().unwrap_or(1).max(1);

    let rows: Vec<Row> = app
        .stats
        .iter()
        .map(|s| {
            let bar_width = 12usize;
            let filled = (s.total() * bar_width).div_ceil(max_total).min(bar_width);
            let bar: String = "█".repeat(filled);

            Row::new(vec![
                Cell::from(s.language.clone()),
                Cell::from(Span::styled(
                    s.prs_open.to_string(),
                    Style::default().fg(Color::Green),
                )),
                Cell::from(Span::styled(
                    s.prs_closed.to_string(),
                    Style::default().fg(Color::Magenta),
                )),
                Cell::from(s.total().to_string()),
                Cell::from(Span::styled(bar, Style::default().fg(Color::Cyan))),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(13),
        ],
    )
    .header(
        Row::new(vec!["Language", "Open", "Closed", "Total", ""])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title(" Languages "))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_summary(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.selected_lang() {
        Some(l) => format!(" What were the {} PRs about? ", l.language),
        None => " Summary ".to_string(),
    };

    let block = Block::default().borders(Borders::ALL).title(title);

    let body: Vec<Line> = if app.summarizing {
        vec![Line::from(Span::styled(
            "Summarizing with claude…",
            Style::default().fg(Color::Yellow),
        ))]
    } else if let Some(text) = &app.summary {
        text.lines().map(Line::from).collect()
    } else if let Some(lang) = app.selected_lang() {
        let mut lines = vec![
            Line::from(Span::styled(
                format!(
                    "{} PRs ({} open, {} closed)",
                    lang.total(),
                    lang.prs_open,
                    lang.prs_closed
                ),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ];
        for (i, pr) in lang.prs.iter().take(PR_LIST_LIMIT).enumerate() {
            let marker = if pr.is_open() { "○" } else { "●" };
            let selected = i == app.selected_pr;
            let cursor = if selected { "▶ " } else { "  " };
            let title_style = if selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(Color::Cyan)),
                Span::styled(format!("{marker} "), Style::default().fg(Color::DarkGray)),
                Span::styled(pr.title.clone(), title_style),
                Span::styled(
                    format!("  ({})", pr.repo),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "←/→ pick PR · o open in browser · s summarize with claude",
            Style::default().fg(Color::Cyan),
        )));
        lines
    } else {
        vec![Line::from(Span::styled(
            "Select a language to see its PRs.",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let para = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let key = |k: &'static str| Span::styled(k, Style::default().fg(Color::Yellow).bold());
    let footer = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        key("↑/↓"),
        Span::raw(" lang  "),
        key("←/→"),
        Span::raw(" PR  "),
        key("o"),
        Span::raw(" open  "),
        key("s"),
        Span::raw(" summarize  "),
        key("u"),
        Span::raw(" user  "),
        key("t"),
        Span::raw(" sort  "),
        key("r"),
        Span::raw(" refresh  "),
        key("q"),
        Span::raw(" quit"),
    ]));
    frame.render_widget(footer, area);
}
