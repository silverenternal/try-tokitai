//! ASA logo banner with a compact terminal-safe pseudo-3D look.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub fn render_logo(_frame_count: u64) -> Vec<Line<'static>> {
    let face = Color::Rgb(90, 230, 255);
    let shadow = Color::Rgb(30, 90, 120);

    let rows = [
        (
            "    ___      _____      ___    ",
            "      /_/     /____/      /_/   ",
        ),
        (
            "   /   |    / ___/     /   |   ",
            "     /_/      /___/      /_/    ",
        ),
        (
            "  /_/|_|   /_/        /_/|_|   ",
            "   /_/      /____/     /_/      ",
        ),
    ];

    let mut lines = Vec::new();
    for (main, drop_shadow) in rows {
        lines.push(Line::from(vec![Span::styled(
            drop_shadow.to_string(),
            Style::default().fg(shadow),
        )]));
        lines.push(Line::from(vec![Span::styled(
            main.to_string(),
            Style::default().fg(face),
        )]));
    }
    lines
}
