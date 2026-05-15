//! DPS Tracker — Segment Tree POC
//!
//! Simulates a WoW-style raid with 7 players. Each player's per-tick damage
//! is stored in a Segment Tree (N_TICKS leaves). Rolling DPS is a single
//! O(log n) range-sum query: query(tick - WINDOW, tick).
//!
//! Layout:
//!   ┌─── DPS Meter ─────────────────────────────────────────────┐
//!   │  Arcane Mage  ████████████████████████  7.2k  100%        │
//!   │  Fire Mage    ██████████████████████    6.8k   94%        │
//!   │  ...                                                       │
//!   ├─── DPS Timeline ──────────────────────────────────────────┤
//!   │  [braille line chart — one coloured line per player]       │
//!   ├─── Status ────────────────────────────────────────────────┤
//!   │  Fight: 0:42  |  Raid DPS: 38.9k  |  [Space]=pause  ...  │
//!   └───────────────────────────────────────────────────────────┘
//!
//! Controls:
//!   Space       pause / resume
//!   R           reset fight
//!   Q / Esc     quit

use std::io;
use std::time::{Duration, Instant};

use rand::{Rng, RngExt};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Bar, BarChart, BarGroup, Block, Chart, Dataset, GraphType, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use rust_segment_tree::SegmentTree;

// ---------- simulation constants -----------------------------------------------

const N_TICKS: usize = 512;      // segment tree leaf count (fight history)
const TICK: Duration = Duration::from_millis(250); // wall-clock tick
const DPS_WINDOW: usize = 16;    // rolling DPS = last 4 s of ticks
const MAX_FIGHT: usize = 480;    // fight ends at ~2 minutes

// ---------- player configs -------------------------------------------------------

struct PlayerCfg {
    name: &'static str,
    color: Color,
    base: f64,       // average damage per tick
    variance: f64,   // ±relative jitter
    crit_rate: f64,
    crit_mult: f64,
    // burst cooldown: every cd_period ticks do cd_mult damage for cd_duration ticks
    cd_period: usize,
    cd_duration: usize,
    cd_mult: f64,
}

static CFGS: &[PlayerCfg] = &[
    PlayerCfg { name: "Sorcerer", color: Color::Cyan,        base: 1_200.0, variance: 0.50, crit_rate: 0.35, crit_mult: 3.0, cd_period: 40, cd_duration:  8, cd_mult: 2.5 },
    PlayerCfg { name: "Rogue",    color: Color::Yellow,      base: 1_000.0, variance: 0.12, crit_rate: 0.25, crit_mult: 2.0, cd_period: 60, cd_duration:  6, cd_mult: 3.0 },
    PlayerCfg { name: "Archer",   color: Color::Green,       base:   950.0, variance: 0.18, crit_rate: 0.22, crit_mult: 2.2, cd_period: 30, cd_duration:  5, cd_mult: 2.0 },
    PlayerCfg { name: "Paladin",  color: Color::LightYellow, base:   800.0, variance: 0.22, crit_rate: 0.18, crit_mult: 2.8, cd_period: 50, cd_duration:  4, cd_mult: 2.5 },
    PlayerCfg { name: "Priest",   color: Color::Magenta,     base:   500.0, variance: 0.15, crit_rate: 0.12, crit_mult: 1.8, cd_period: 80, cd_duration:  3, cd_mult: 3.0 },
];

// ---------- per-player state -----------------------------------------------------

struct Player {
    cfg: &'static PlayerCfg,
    /// Damage dealt at each tick slot; queried with range sums for DPS.
    tree: SegmentTree,
    total_damage: u64,
    /// (time_seconds, dps) history for the line chart.
    chart_data: Vec<(f64, f64)>,
}

impl Player {
    fn new(cfg: &'static PlayerCfg) -> Self {
        Self {
            cfg,
            tree: SegmentTree::new(&vec![0_i32; N_TICKS]),
            total_damage: 0,
            chart_data: Vec::new(),
        }
    }

    fn sim_tick(&mut self, tick: usize, rng: &mut impl Rng) {
        let cfg = self.cfg;
        let mut dmg = cfg.base * (1.0 - cfg.variance + rng.random::<f64>() * 2.0 * cfg.variance);
        if cfg.cd_period > 0 && tick % cfg.cd_period < cfg.cd_duration {
            dmg *= cfg.cd_mult;
        }
        if rng.random::<f64>() < cfg.crit_rate {
            dmg *= cfg.crit_mult;
        }
        let dmg_i = dmg.round() as i32;
        self.tree.update(tick, dmg_i);
        self.total_damage += dmg_i as u64;
    }

    /// Rolling DPS via a single segment tree range query.
    fn dps(&self, tick: usize) -> f64 {
        if tick == 0 {
            return 0.0;
        }
        let end = tick.min(N_TICKS);
        let start = end.saturating_sub(DPS_WINDOW);
        let total = self.tree.query(start, end); // O(log N_TICKS)
        total as f64 / ((end - start) as f64 * TICK.as_secs_f64())
    }

    fn push_chart_point(&mut self, tick: usize) {
        let t = tick as f64 * TICK.as_secs_f64();
        self.chart_data.push((t, self.dps(tick)));
    }
}

// ---------- app state ------------------------------------------------------------

struct App {
    players: Vec<Player>,
    fight_tick: usize,
    paused: bool,
    quit: bool,
    rng: rand::rngs::ThreadRng,
}

impl App {
    fn new() -> Self {
        Self {
            players: CFGS.iter().map(Player::new).collect(),
            fight_tick: 0,
            paused: false,
            quit: false,
            rng: rand::rng(),
        }
    }

    fn reset(&mut self) {
        self.players = CFGS.iter().map(Player::new).collect();
        self.fight_tick = 0;
        self.paused = false;
    }

    fn advance(&mut self) {
        if self.paused || self.fight_tick >= MAX_FIGHT {
            return;
        }
        let t = self.fight_tick;
        for p in &mut self.players {
            p.sim_tick(t, &mut self.rng);
        }
        self.fight_tick += 1;
        for p in &mut self.players {
            p.push_chart_point(self.fight_tick);
        }
    }

    fn fight_done(&self) -> bool {
        self.fight_tick >= MAX_FIGHT
    }

    fn fight_time(&self) -> String {
        let secs = (self.fight_tick as f64 * TICK.as_secs_f64()) as u64;
        format!("{}:{:02}", secs / 60, secs % 60)
    }

    fn raid_dps(&self) -> f64 {
        self.players.iter().map(|p| p.dps(self.fight_tick)).sum()
    }

    /// Indices of players sorted descending by current DPS.
    fn ranked(&self) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..self.players.len()).collect();
        idx.sort_by(|&a, &b| {
            self.players[b]
                .dps(self.fight_tick)
                .partial_cmp(&self.players[a].dps(self.fight_tick))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        idx
    }
}

// ---------- main loop ------------------------------------------------------------

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();
    let mut last_tick = Instant::now();
    while !app.quit {
        terminal.draw(|f| draw(f, &app))?;
        let timeout = TICK.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    handle_key(&mut app, k);
                }
            }
        }
        if last_tick.elapsed() >= TICK {
            app.advance();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, k: event::KeyEvent) {
    match k.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Char(' ') => app.paused = !app.paused,
        KeyCode::Char('r') | KeyCode::Char('R') => app.reset(),
        _ => {}
    }
}

// ---------- rendering ------------------------------------------------------------

fn fmt_k(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(11),   // DPS meter
        Constraint::Fill(1),   // DPS timeline chart
        Constraint::Length(3), // status bar
    ])
    .split(f.area());

    draw_meter(f, app, chunks[0]);
    draw_chart(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);
}

fn draw_meter(f: &mut Frame, app: &App, area: Rect) {
    let ranked = app.ranked();
    let dpses: Vec<f64> = ranked.iter().map(|&i| app.players[i].dps(app.fight_tick)).collect();
    let max_dps = dpses.first().cloned().unwrap_or(1.0).max(1.0);

    let bars: Vec<Bar> = ranked
        .iter()
        .zip(&dpses)
        .map(|(&pi, &dps)| {
            let p = &app.players[pi];
            let pct = (dps / max_dps * 100.0).round() as u64;
            Bar::default()
                .label(Line::from(format!("{:<13}", p.cfg.name)))
                .value(dps.round() as u64)
                .text_value(format!("{:>6}  {:3}%", fmt_k(dps), pct))
                .style(Style::default().fg(p.cfg.color))
                .value_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        })
        .collect();

    let chart = BarChart::default()
        .direction(Direction::Horizontal)
        .data(BarGroup::default().bars(&bars))
        .bar_width(1)
        .bar_gap(0)
        .max(max_dps.round() as u64)
        .block(Block::bordered().title(Line::from(vec![
            Span::raw(" DPS Meter  "),
            Span::styled(
                "segment tree: query(tick-16, tick)",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
        ])));

    f.render_widget(chart, area);
}

fn draw_chart(f: &mut Frame, app: &App, area: Rect) {
    let now_s = app.fight_tick as f64 * TICK.as_secs_f64();
    let window_s = 30.0_f64;
    let x_min = (now_s - window_s).max(0.0);
    let x_max = now_s.max(window_s);

    let max_y = app
        .players
        .iter()
        .flat_map(|p| p.chart_data.iter().map(|&(_, y)| y))
        .fold(100.0_f64, f64::max);

    let datasets: Vec<Dataset<'_>> = app
        .players
        .iter()
        .map(|p| {
            Dataset::default()
                .name(p.cfg.name)
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(p.cfg.color))
                .data(&p.chart_data)
        })
        .collect();

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(" DPS Timeline (last 30 s) "))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([x_min, x_max])
                .labels(vec![
                    Span::raw(format!("{:.0}s", x_min)),
                    Span::raw(format!("{:.0}s", (x_min + x_max) / 2.0)),
                    Span::raw(format!("{:.0}s", x_max)),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, max_y * 1.1])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(fmt_k(max_y / 2.0)),
                    Span::raw(fmt_k(max_y)),
                ]),
        );

    f.render_widget(chart, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let (state_text, state_style) = if app.fight_done() {
        (
            "VICTORY!  Press R to reset".to_string(),
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
        )
    } else if app.paused {
        (
            "PAUSED".to_string(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            format!("Raid DPS: {}", fmt_k(app.raid_dps())),
            Style::default().fg(Color::Cyan),
        )
    };

    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Fight: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.fight_time(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  "),
        Span::styled(state_text, state_style),
        Span::raw("  |  "),
        Span::styled(
            "[Space]=pause  [R]=reset  [Q]=quit",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    f.render_widget(
        Paragraph::new(line).block(Block::bordered().title(" Status ")),
        area,
    );
}
