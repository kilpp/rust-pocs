//! Interactive terminal visualizer for `SegmentTree`.
//!
//! Layout:
//!     ┌──── tree ─────────────────────────────────────┐
//!     │                                               │
//!     │  ascii drawing of the heap, one row per level │
//!     │                                               │
//!     ├──── input ────────────────────────────────────┤
//!     │ > update 2 7                                  │
//!     ├──── log ──────────────────────────────────────┤
//!     │ > query 1 5  ->  sum [1, 5) = 14              │
//!     └───────────────────────────────────────────────┘
//!
//! Commands:
//!   update <p> <value>     rewrite leaf p
//!   query  <l> <r>         half-open sum on [l, r)
//!   reset                  reload the initial array
//!   quit                   exit (Esc / Ctrl-C also quit)

use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use rust_segment_tree::SegmentTree;

const INITIAL: [i32; 16] = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3];
const CELL_W: usize = 6;
const FLASH_TTL: u32 = 24;
const TICK: Duration = Duration::from_millis(40);
const LOG_CAP: usize = 64;

#[derive(Clone, Copy)]
enum Tag {
    Update,
    Query,
}

struct Highlight {
    tag: Tag,
    ttl: u32,
}

struct App {
    tree: SegmentTree,
    input: String,
    log: VecDeque<String>,
    highlights: HashMap<usize, Highlight>,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            tree: SegmentTree::new(&INITIAL),
            input: String::new(),
            log: VecDeque::new(),
            highlights: HashMap::new(),
            quit: false,
        }
    }

    fn submit(&mut self) {
        let cmd: String = self.input.trim().to_string();
        self.input.clear();
        if cmd.is_empty() {
            return;
        }
        let line = match self.dispatch(&cmd) {
            Ok(msg) => format!("> {cmd}  ->  {msg}"),
            Err(e) => format!("> {cmd}  !!  {e}"),
        };
        self.log.push_back(line);
        while self.log.len() > LOG_CAP {
            self.log.pop_front();
        }
    }

    fn dispatch(&mut self, line: &str) -> Result<String, String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["quit"] | ["q"] | ["exit"] => {
                self.quit = true;
                Ok("bye".into())
            }
            ["reset"] => {
                self.tree = SegmentTree::new(&INITIAL);
                self.highlights.clear();
                Ok("reloaded initial array".into())
            }
            ["update", p, v] => {
                let p: usize = p.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                let v: i32 = v.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                if p >= self.tree.size() {
                    return Err(format!("p out of range (n={})", self.tree.size()));
                }
                let path = self.tree.update_traced(p, v);
                for n in path {
                    self.highlights.insert(
                        n,
                        Highlight {
                            tag: Tag::Update,
                            ttl: FLASH_TTL,
                        },
                    );
                }
                Ok(format!("set [{p}] = {v}"))
            }
            ["query", l, r] => {
                let l: usize = l.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                let r: usize = r.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                if l > r || r > self.tree.size() {
                    return Err(format!("range out of bounds (n={})", self.tree.size()));
                }
                let (sum, visited) = self.tree.query_traced(l, r);
                for n in visited {
                    self.highlights.insert(
                        n,
                        Highlight {
                            tag: Tag::Query,
                            ttl: FLASH_TTL,
                        },
                    );
                }
                Ok(format!("sum [{l}, {r}) = {sum}"))
            }
            _ => Err("usage: update <p> <v> | query <l> <r> | reset | quit".into()),
        }
    }

    fn tick(&mut self) {
        self.highlights.retain(|_, h| {
            h.ttl = h.ttl.saturating_sub(1);
            h.ttl > 0
        });
    }

    fn style_for(&self, idx: usize) -> Style {
        match self.highlights.get(&idx) {
            Some(h) => {
                let frac = h.ttl as f32 / FLASH_TTL as f32;
                let color = match h.tag {
                    Tag::Update => {
                        if frac > 0.66 {
                            Color::LightRed
                        } else if frac > 0.33 {
                            Color::Red
                        } else {
                            Color::Rgb(120, 40, 40)
                        }
                    }
                    Tag::Query => {
                        if frac > 0.66 {
                            Color::LightGreen
                        } else if frac > 0.33 {
                            Color::Green
                        } else {
                            Color::Rgb(40, 100, 40)
                        }
                    }
                };
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            }
            None => Style::default().fg(Color::Gray),
        }
    }
}

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
            app.tick();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => app.quit = true,
        KeyCode::Enter => app.submit(),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => app.input.clear(),
        KeyCode::Char(c) => app.input.push(c),
        _ => {}
    }
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(3),
        Constraint::Length(10),
    ])
    .split(f.area());

    draw_tree(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_log(f, app, chunks[2]);
}

fn draw_tree(f: &mut Frame, app: &App, area: Rect) {
    let nodes = app.tree.nodes();
    let total = nodes.len(); // 2n
    let max_idx = total - 1;
    let max_level = (usize::BITS - 1 - max_idx.leading_zeros()) as usize; // floor(log2)
    let leaves_in_grid = 1usize << max_level;
    let total_w = leaves_in_grid * CELL_W;

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(max_level * 2 + 1);
    for level in 0..=max_level {
        let span_w = total_w / (1usize << level);
        let mut spans: Vec<Span<'static>> = Vec::new();
        for k in 0..(1usize << level) {
            let idx = (1usize << level) + k;
            if idx >= total {
                spans.push(Span::raw(" ".repeat(span_w)));
                continue;
            }
            let label = format!("{}", nodes[idx]);
            let cell = center_in(&label, span_w);
            spans.push(Span::styled(cell, app.style_for(idx)));
        }
        lines.push(Line::from(spans));

        if level < max_level {
            lines.push(Line::from(connector_row(level, total, total_w)));
        }
    }

    // index banner under the leaves so users know which `p` maps where
    lines.push(Line::from(""));
    lines.push(Line::from(index_banner(app, total, total_w, max_level)));

    let title = Line::from(vec![
        Span::raw(" segment tree  "),
        Span::styled("n=", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", app.tree.size()), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("update", Style::default().fg(Color::LightRed))
            .add_modifier(Modifier::BOLD),
        Span::raw(" / "),
        Span::styled("query", Style::default().fg(Color::LightGreen))
            .add_modifier(Modifier::BOLD),
        Span::raw(" "),
    ]);

    let para = Paragraph::new(lines)
        .block(Block::bordered().title(title))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

/// Center `label` inside a `span_w`-wide cell. The label's center column
/// matches `span_w / 2`, which is also where connector rows aim — so values
/// always sit directly under the `┴` of the connector above.
fn center_in(label: &str, span_w: usize) -> String {
    if label.len() >= span_w {
        return label.to_string();
    }
    let center = span_w / 2;
    let lpad = center - label.len() / 2;
    let rpad = span_w - lpad - label.len();
    format!("{}{}{}", " ".repeat(lpad), label, " ".repeat(rpad))
}

/// Build one row of box-drawing connectors between `level` and `level + 1`.
/// For each parent with two children we paint:
///
/// ```text
///         ┴            <- parent column
///    ┌────┴────┐
/// ```
///
/// collapsed onto a single row: `┌` at the left child's column, `┐` at the
/// right child's, `─` filling the gap, and `┴` at the parent column. Parents
/// that only have a left child (possible at the leaf boundary when `n` is
/// not a power of two) get a `┌─┘` corner instead.
fn connector_row(level: usize, total: usize, total_w: usize) -> Vec<Span<'static>> {
    let parent_span = total_w / (1usize << level);
    let child_span = parent_span / 2;
    let mut row: Vec<char> = vec![' '; total_w];
    for k in 0..(1usize << level) {
        let parent_idx = (1usize << level) + k;
        if parent_idx >= total {
            continue;
        }
        let lc = 2 * parent_idx;
        let rc = lc + 1;
        if lc >= total {
            continue; // this "parent" is actually a leaf — nothing below
        }

        let parent_left = k * parent_span;
        let parent_col = parent_left + parent_span / 2;
        let left_col = parent_left + child_span / 2;
        let right_col = parent_left + child_span + child_span / 2;

        if rc < total {
            for col in (left_col + 1)..right_col {
                if col < total_w {
                    row[col] = '─';
                }
            }
            if left_col < total_w {
                row[left_col] = '┌';
            }
            if right_col < total_w {
                row[right_col] = '┐';
            }
            if parent_col < total_w {
                row[parent_col] = '┴';
            }
        } else {
            // only-left-child corner: ┌─┘
            for col in (left_col + 1)..parent_col {
                if col < total_w {
                    row[col] = '─';
                }
            }
            if left_col < total_w {
                row[left_col] = '┌';
            }
            if parent_col < total_w {
                row[parent_col] = '┘';
            }
        }
    }
    vec![Span::styled(
        row.into_iter().collect::<String>(),
        Style::default().fg(Color::DarkGray),
    )]
}

fn index_banner(app: &App, total: usize, total_w: usize, max_level: usize) -> Vec<Span<'static>> {
    // Show the input array index `p` directly under each leaf (heap idx p+n).
    let leaves_in_grid = 1usize << max_level;
    let span_w = total_w / leaves_in_grid;
    let mut spans: Vec<Span<'static>> = Vec::new();
    for k in 0..leaves_in_grid {
        let heap_idx = leaves_in_grid + k;
        if heap_idx >= total {
            spans.push(Span::raw(" ".repeat(span_w)));
            continue;
        }
        let p = heap_idx - app.tree.size();
        spans.push(Span::styled(
            center_in(&format!("{p}"), span_w),
            Style::default().fg(Color::DarkGray),
        ));
    }
    spans
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let prompt = "> ";
    let line = Line::from(vec![
        Span::styled(prompt, Style::default().fg(Color::Cyan)),
        Span::raw(app.input.clone()),
    ]);
    let para = Paragraph::new(line)
        .block(Block::bordered().title(" input  (Enter = run, Esc = quit) "));
    f.render_widget(para, area);

    // Place the cursor right after the input text.
    let cursor_x = area.x + 1 + prompt.len() as u16 + app.input.len() as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width.saturating_sub(1) {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let visible = area.height.saturating_sub(2) as usize;
    let start = app.log.len().saturating_sub(visible);
    let lines: Vec<Line> = app
        .log
        .iter()
        .skip(start)
        .map(|s| {
            let style = if s.contains("!!") {
                Style::default().fg(Color::LightRed)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(s.clone(), style))
        })
        .collect();
    let para = Paragraph::new(lines).block(Block::bordered().title(" log "));
    f.render_widget(para, area);
}
