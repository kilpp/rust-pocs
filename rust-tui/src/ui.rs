use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    fn create_layout(f: &mut Frame) -> std::rc::Rc<[ratatui::layout::Rect]> {
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
    }

    fn render_left_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
        let panel_block = Block::default()
            .title(" Menu ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let menu_items = Self::create_menu_items(app);

        let content = Paragraph::new(menu_items)
            .block(panel_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(content, area);
    }

    fn create_menu_items(app: &App) -> Vec<Line<'static>> {
        app.items
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
            .collect()
    }

    fn render_central_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
        let panel_block = Block::default()
            .title(" Content ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let content = Self::create_content(app);

        let content_widget = Paragraph::new(content)
            .block(panel_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(content_widget, area);
    }

    fn create_content(app: &App) -> Vec<Line<'static>> {
        let selected_item = &app.items[app.selected_item];
        vec![
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
        ]
    }
}
