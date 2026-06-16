//! ASA logo banner — static highlight

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub fn render_logo(_frame_count: u64) -> Vec<Line<'static>> {
    let raw = [
        " █████╗    ███████╗     █████╗  ",
        "██╔══██╗   ██╔════╝    ██╔══██╗ ",
        "███████║   ███████╗    ███████║ ",
        "██╔══██║   ╚════██║    ██╔══██║ ",
        "██║  ██║   ███████║    ██║  ██║ ",
        "╚═╝  ╚═╝   ╚══════╝    ╚═╝  ╚═╝ ",
    ];

    let bright = Color::Rgb(80, 220, 255);

    let mut lines = Vec::new();
    for row_str in &raw {
        let mut spans = Vec::new();
        for ch in row_str.chars() {
            if ch == ' ' {
                spans.push(Span::styled("  ".to_string(), Style::default()));
            } else {
                spans.push(Span::styled(
                    format!("{}{}", ch, ch),
                    Style::default().fg(bright),
                ));
            }
        }
        let line = Line::from(spans);
        lines.push(line.clone()); // Double vertically
        lines.push(line);
    }
    lines
}
