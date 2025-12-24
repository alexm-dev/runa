use crate::app::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

pub enum InputKey {
    Char(char),
    Name(&'static str),
}

pub fn get_pane_block(title: &str, app: &AppState) -> Block<'static> {
    let mut b = Block::default();
    if app.config().display().is_split() {
        b = b
            .borders(Borders::ALL)
            .border_style(app.config().theme().accent().as_style());
        if app.config().display().titles() {
            b = b.title(title.to_string());
        }
    }
    b
}

pub fn draw_separator(frame: &mut Frame, area: Rect, style: Style) {
    frame.render_widget(
        Block::default().borders(Borders::LEFT).border_style(style),
        area,
    );
}

pub fn draw_confirm_popup(frame: &mut Frame, area: Rect, prompt: &str) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(vertical_chunks[1])[1];

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Red));

    let text = Paragraph::new(format!("\n{}", prompt))
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, popup_area);
}
