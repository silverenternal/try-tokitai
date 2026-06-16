//! Chat panel with rich message rendering
//!
//! Renders `MessageBlock`s with virtual scrolling, auto-scroll behavior,
//! and distinct visual styling per message type.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::message_block::{MessageBlock, ToolCallStatus};

/// Chat panel renderer
pub struct ChatPanel;

impl ChatPanel {
    /// Render the chat area with all messages (no border box)
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        messages: &[MessageBlock],
        scroll_offset: usize,
        frame_count: u64,
        status_word: &str,
    ) {
        let visible_height = area.height as usize;
        if visible_height == 0 {
            return;
        }

        // Build all rendered lines
        let mut all_lines = Vec::<Line>::new();
        for block_msg in messages {
            Self::render_block(block_msg, &mut all_lines, area.width, frame_count, status_word);
        }

        // Scroll logic
        let total_lines = all_lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = scroll_offset.min(max_scroll);

        // Slice visible lines
        let visible_lines: Vec<Line> = all_lines
            .iter()
            .skip(scroll)
            .take(visible_height)
            .cloned()
            .collect();

        let content = if visible_lines.is_empty() && messages.is_empty() {
            vec![Line::from(Span::styled(
                "Type a message to start. Use /help for commands.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            visible_lines
        };

        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, area);

        // Scrollbar
        if total_lines > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state =
                ScrollbarState::new(total_lines).position(scroll);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    /// Render a single MessageBlock into lines.
    /// Only shows User and Assistant messages — tool calls, results,
    /// errors, and system messages are hidden to keep the UI clean.
    fn render_block(block: &MessageBlock, lines: &mut Vec<Line<'_>>, width: u16, _frame_count: u64, _status_word: &str) {
        match block {
            // Tool call — compact inline display like Claude Code
            MessageBlock::ToolCall { name, args, status, .. } => {
                let is_done = matches!(status, ToolCallStatus::Complete);
                let icon = if is_done { "✓" } else { "⚙" };
                let color = if is_done { Color::Green } else { Color::Yellow };
                let args_str = serde_json::to_string(args).unwrap_or_default();
                let args_short = if args_str.chars().count() > 100 { format!("{}...", args_str.chars().take(97).collect::<String>()) } else { args_str };
                let tool_name = name.clone();
                lines.push(Line::from(vec![
                    Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                    Span::styled(tool_name, Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD)),
                    Span::styled(format!("  {}", args_short), Style::default().fg(Color::DarkGray)),
                ]));
                lines.push(Line::from(""));
            }
            // Tool result — compact, truncated
            MessageBlock::ToolResult { result, success, .. } => {
                let color = if *success { Color::Green } else { Color::Red };
                let icon = if *success { "✓" } else { "✗" };
                let preview: String = if result.chars().count() > 300 {
                    format!("{}...", result.chars().take(297).collect::<String>())
                } else { result.clone() };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {} Result: ", icon), Style::default().fg(color)),
                ]));
                for line in preview.lines().take(5) {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                if result.lines().count() > 5 {
                    lines.push(Line::from(Span::styled("   ...", Style::default().fg(Color::DarkGray))));
                }
                lines.push(Line::from(""));
            }
            MessageBlock::Error { .. } => return,
            MessageBlock::Thinking { content, collapsed } => {
                let content = content.clone();
                let collapsed = *collapsed;
                let toggle = if collapsed { "[+]" } else { "[-]" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", toggle),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::styled(
                        "Thinking…",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(ratatui::style::Modifier::ITALIC),
                    ),
                ]));
                if !collapsed {
                    for line in content.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::Gray).add_modifier(ratatui::style::Modifier::ITALIC),
                        )));
                    }
                }
                lines.push(Line::from(""));
            }
            // System messages are visible (used for /agents, /help, etc.)
            MessageBlock::Diff { diff } => {
                for line in diff.render() {
                    lines.push(line);
                }
                lines.push(Line::from(""));
            }
            MessageBlock::System { content } => {
                for line in content.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(""));
            }
            MessageBlock::User { content, .. } => {
                let content = content.clone();
                lines.push(Line::from(vec![
                    Span::styled("▌", Style::default().fg(Color::Cyan)),
                    Span::styled(" You", Style::default().fg(Color::Cyan)),
                ]));
                append_markdown_lines(lines, &content, width, Color::White);
                lines.push(Line::from(""));
            }
            MessageBlock::Assistant { content } => {
                let content = content.clone();
                append_markdown_lines(lines, &content, width, Color::White);
                lines.push(Line::from(""));
            }
            MessageBlock::AssistantStreaming { content } => {
                let content = content.clone();
                append_markdown_lines(lines, &content, width, Color::White);
                if !content.is_empty() {
                    if let Some(last) = lines.last_mut() {
                        last.spans.push(Span::styled("▌", Style::default().fg(Color::Cyan)));
                    }
                }
                lines.push(Line::from(""));
            }
        }
    }
}

/// Append markdown text as styled Lines to the given vector.
/// Supports: # headers, **bold**, *italic*, `inline code`, ```code blocks```,
/// and preserves inline styling when wrapping long lines.
fn append_markdown_lines(out: &mut Vec<Line<'_>>, text: &str, width: u16, base_color: Color) {
    let max_w = (width.saturating_sub(2)) as usize;
    if max_w == 0 {
        out.push(Line::from(Span::styled(text.to_string(), Style::default().fg(base_color))));
        return;
    }

    let mut in_code_block = false;

    for raw_line in text.lines() {
        // Code block fences
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            for chunk in char_chunks(raw_line, max_w) {
                out.push(Line::from(Span::styled(
                    format!("  {}", chunk),
                    Style::default().fg(Color::Gray),
                )));
            }
            continue;
        }

        if raw_line.is_empty() {
            out.push(Line::from(""));
            continue;
        }

        // Check for header at line start
        if let Some(header_line) = try_render_header(raw_line) {
            out.push(header_line);
            continue;
        }

        // Parse inline markdown and wrap preserving styles
        render_wrapped_spans(out, raw_line, max_w, base_color);
    }
}

/// Try to render a markdown header line. Returns Some if the line starts with #.
fn try_render_header(line: &str) -> Option<Line<'static>> {
    let trimmed = line.trim_start();
    let (level, content) = if trimmed.starts_with("### ") {
        (3, &trimmed[4..])
    } else if trimmed.starts_with("## ") {
        (2, &trimmed[3..])
    } else if trimmed.starts_with("# ") {
        (1, &trimmed[2..])
    } else {
        return None;
    };

    let prefix = match level {
        1 => "# ",
        2 => "## ",
        _ => "### ",
    };
    Some(Line::from(vec![
        Span::styled(
            prefix.to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            content.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]))
}

/// Chunk a string into max_w character pieces (Unicode-safe).
fn char_chunks(s: &str, max_w: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut chunks = Vec::new();
    let mut pos = 0;
    while pos < chars.len() {
        let end = (pos + max_w).min(chars.len());
        chunks.push(chars[pos..end].iter().collect());
        pos = end;
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

/// Parse inline markdown and render as wrapped Lines, preserving styles.
fn render_wrapped_spans(out: &mut Vec<Line<'_>>, text: &str, max_w: usize, base: Color) {
    let spans = parse_inline_markdown(text, base);
    // Flatten spans into (char, style) pairs for precise wrapping
    let mut char_styles: Vec<(char, Style)> = Vec::new();
    for (s, style) in &spans {
        for ch in s.chars() {
            char_styles.push((ch, *style));
        }
    }

    if char_styles.is_empty() {
        out.push(Line::from(""));
        return;
    }

    let mut line_start = 0;
    while line_start < char_styles.len() {
        let mut line_end = (line_start + max_w).min(char_styles.len());

        // Try to break at a word boundary (space) — walk back from line_end
        if line_end < char_styles.len() {
            let mut break_at = line_end;
            while break_at > line_start && char_styles[break_at - 1].0 != ' ' {
                break_at -= 1;
            }
            // Only break at word boundary if we found a space within reasonable range
            if break_at > line_start && (line_end - break_at) < (max_w / 2) {
                line_end = break_at;
            }
        }

        // Build spans for this line by merging adjacent same-style chars
        let mut line_spans: Vec<Span> = Vec::new();
        let mut seg_start = line_start;
        while seg_start < line_end {
            let seg_style = char_styles[seg_start].1;
            let mut seg_end = seg_start + 1;
            while seg_end < line_end && char_styles[seg_end].1 == seg_style {
                seg_end += 1;
            }
            let seg_text: String = char_styles[seg_start..seg_end]
                .iter().map(|(ch, _)| *ch).collect();
            line_spans.push(Span::styled(seg_text, seg_style));
            seg_start = seg_end;
        }

        out.push(Line::from(line_spans));
        line_start = line_end;
        // Skip leading space on continuation lines
        while line_start < char_styles.len() && char_styles[line_start].0 == ' ' {
            line_start += 1;
        }
    }
}

/// Parse inline markdown using char-based indexing (safe for Unicode).
/// Returns styled spans with markdown markers stripped.
fn parse_inline_markdown(text: &str, base: Color) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    fn flush(buf: &mut String, out: &mut Vec<(String, Style)>, color: Color) {
        if !buf.is_empty() {
            out.push((std::mem::take(buf), Style::default().fg(color)));
        }
    }

    while i < len {
        // Check for *** (bold+italic) first — 3 markers
        if i + 2 < len && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            let start = i + 3;
            let mut end = start;
            while end + 2 < len && !(chars[end] == '*' && chars[end + 1] == '*' && chars[end + 2] == '*') {
                end += 1;
            }
            if end > start && end + 2 < len {
                flush(&mut buf, &mut out, base);
                let text: String = chars[start..end].iter().collect();
                out.push((text, Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::ITALIC)));
                i = end + 3;
                continue;
            }
        }

        // **bold** or __bold__
        if i + 1 < len && ((chars[i] == '*' && chars[i + 1] == '*') || (chars[i] == '_' && chars[i + 1] == '_')) {
            let marker = chars[i];
            // Skip if it's *** (handled above)
            if marker == '*' && i + 2 < len && chars[i + 2] == '*' {
                buf.push(chars[i]);
                i += 1;
                continue;
            }
            let start = i + 2;
            let mut end = start;
            while end + 1 < len && !(chars[end] == marker && chars[end + 1] == marker) {
                end += 1;
            }
            if end > start && end + 1 < len {
                flush(&mut buf, &mut out, base);
                let bold_text: String = chars[start..end].iter().collect();
                out.push((bold_text, Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD)));
                i = end + 2;
                continue;
            }
        }

        // *italic* or _italic_ (single marker, not part of ** or __)
        if chars[i] == '*' || chars[i] == '_' {
            let marker = chars[i];
            // Skip if next char is same marker (handled as bold above) or if it's ***
            if i + 1 < len && chars[i + 1] == marker {
                buf.push(chars[i]);
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut end = start;
            while end < len && chars[end] != marker {
                end += 1;
            }
            if end > start && end < len {
                flush(&mut buf, &mut out, base);
                let italic_text: String = chars[start..end].iter().collect();
                out.push((italic_text, Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(ratatui::style::Modifier::ITALIC)));
                i = end + 1;
                continue;
            }
        }

        // `inline code`
        if chars[i] == '`' {
            let start = i + 1;
            let mut end = start;
            while end < len && chars[end] != '`' {
                end += 1;
            }
            if end < len {
                flush(&mut buf, &mut out, base);
                let code_text: String = chars[start..end].iter().collect();
                out.push((code_text, Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::DarkGray)));
                i = end + 1;
                continue;
            }
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush(&mut buf, &mut out, base);

    if out.is_empty() {
        out.push((text.to_string(), Style::default().fg(base)));
    }

    out
}
