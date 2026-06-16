//! Conversation graph with explicit session branch metadata.
//!
//! The renderer intentionally avoids guessing merges from lane disappearance.
//! Forks are drawn from `SessionBranch.parent_id` and `fork_msg_index`; merges
//! are shown only when `SessionBranch.merged_into` is set.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::collections::{HashMap, HashSet};

use super::message_block::MessageBlock;
use crate::tui::session::SessionBranch;

const COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

#[derive(Clone)]
struct GNode {
    text: String,
    branch: String,
    lane: usize,
    color: Color,
}

fn normalize_branch_id(branch_id: &str) -> &str {
    if branch_id.is_empty() {
        "main"
    } else {
        branch_id
    }
}

fn user_branch_at(messages: &[MessageBlock], user_node_idx: usize) -> String {
    messages
        .iter()
        .filter_map(|msg| match msg {
            MessageBlock::User { branch_id, .. } => {
                Some(normalize_branch_id(branch_id).to_string())
            }
            _ => None,
        })
        .nth(user_node_idx)
        .unwrap_or_else(|| "main".to_string())
}

fn build_branch_map(branches: &[SessionBranch]) -> HashMap<String, SessionBranch> {
    let mut map = HashMap::new();
    map.insert("main".to_string(), SessionBranch::main());
    for branch in branches {
        map.insert(branch.id.clone(), branch.clone());
    }
    map
}

fn branch_depth(branch_id: &str, branches: &HashMap<String, SessionBranch>) -> usize {
    let mut depth = 0usize;
    let mut current = branch_id;
    let mut seen = HashSet::new();

    while current != "main" && seen.insert(current.to_string()) {
        let Some(branch) = branches.get(current) else {
            break;
        };
        let parent = normalize_branch_id(&branch.parent_id);
        if parent == current {
            break;
        }
        depth += 1;
        current = parent;
    }

    depth
}

fn build_lanes(branches: &HashMap<String, SessionBranch>) -> HashMap<String, usize> {
    let mut ordered: Vec<&SessionBranch> = branches.values().collect();
    ordered.sort_by_key(|branch| {
        (
            branch_depth(&branch.id, branches),
            branch.fork_msg_index,
            branch.id.clone(),
        )
    });

    let mut lanes = HashMap::new();
    lanes.insert("main".to_string(), 0);

    let mut next_lane = 1usize;
    for branch in ordered {
        if branch.id == "main" {
            continue;
        }
        lanes.entry(branch.id.clone()).or_insert_with(|| {
            let lane = next_lane;
            next_lane += 1;
            lane
        });
    }

    lanes
}

fn build_graph(messages: &[MessageBlock], branches: &[SessionBranch]) -> Vec<GNode> {
    let branch_map = build_branch_map(branches);
    let lanes = build_lanes(&branch_map);
    let mut branch_color: HashMap<String, Color> = HashMap::new();

    for branch in branch_map.values() {
        branch_color.insert(branch.id.clone(), COLORS[branch.color_idx % COLORS.len()]);
    }
    branch_color.insert("main".to_string(), COLORS[0]);

    let mut nodes = Vec::new();
    for msg in messages
        .iter()
        .filter(|msg| matches!(msg, MessageBlock::User { .. }))
    {
        if let MessageBlock::User { content, branch_id } = msg {
            let branch = normalize_branch_id(branch_id).to_string();
            let lane = *lanes.get(&branch).unwrap_or(&0);
            let color = *branch_color
                .get(&branch)
                .unwrap_or(&COLORS[lane % COLORS.len()]);
            nodes.push(GNode {
                text: content.clone(),
                branch,
                lane,
                color,
            });
        }
    }

    nodes
}

fn active_lanes(
    nodes: &[GNode],
    from: usize,
    branches: &[SessionBranch],
    last_node_by_branch: &HashMap<String, usize>,
) -> Vec<usize> {
    let mut lanes: Vec<usize> = nodes[from..].iter().map(|node| node.lane).collect();
    for branch in branches {
        if branch.id == "main" {
            continue;
        }
        let forked = branch.fork_msg_index.saturating_sub(1) < from;
        let ended = if branch.merged_into.is_some() {
            last_node_by_branch
                .get(&branch.id)
                .map(|&last| from > last)
                .unwrap_or(false)
        } else {
            false
        };
        if forked && !ended {
            if let Some(lane) = nodes
                .iter()
                .find(|node| node.branch == branch.id)
                .map(|node| node.lane)
            {
                lanes.push(lane);
            }
        }
    }
    lanes.sort_unstable();
    lanes.dedup();
    lanes
}

fn max_lane(nodes: &[GNode]) -> usize {
    nodes.iter().map(|node| node.lane).max().unwrap_or(0)
}

pub fn render_graph(
    frame: &mut ratatui::Frame,
    area: Rect,
    messages: &[MessageBlock],
    branches: &[SessionBranch],
    selected: usize,
) -> usize {
    let nodes = build_graph(messages, branches);
    let mut last_node_by_branch: HashMap<String, usize> = HashMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        last_node_by_branch.insert(node.branch.clone(), idx);
    }
    let total = nodes.len();
    let max_l = max_lane(&nodes);
    let col_w = 3u16;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Conversation Graph - {} questions ", total));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if nodes.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "\n  No user messages yet.",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return 0;
    }

    let x = inner.x + 2;
    let mut y = inner.y + 1;
    let rows_per = 2;
    let visible = (inner.height.saturating_sub(3) / rows_per as u16) as usize;

    let sel = selected.min(total.saturating_sub(1));
    let start = if total <= visible {
        0
    } else if sel < visible / 2 {
        0
    } else {
        (sel - visible / 2).min(total.saturating_sub(visible))
    };

    for i in start..total {
        if y + 2 >= inner.y + inner.height {
            break;
        }

        let node = &nodes[i];
        let is_sel = i == sel;
        let is_last = i == total - 1;
        let lanes = active_lanes(&nodes, i, branches, &last_node_by_branch);

        if i > start {
            let lanes_prev = active_lanes(&nodes, i - 1, branches, &last_node_by_branch);
            let lanes_curr = &lanes;
            let mut spans = Vec::new();
            for lane in 0..=max_l {
                let in_prev = lanes_prev.contains(&lane);
                let in_curr = lanes_curr.contains(&lane);
                let color = nodes
                    .iter()
                    .find(|node| node.lane == lane)
                    .map(|node| node.color)
                    .unwrap_or(Color::DarkGray);
                if in_prev && in_curr {
                    spans.push(Span::styled(" │ ", Style::default().fg(color)));
                } else if !in_prev && in_curr {
                    spans.push(Span::styled(" ╲ ", Style::default().fg(color)));
                } else if in_prev && !in_curr {
                    spans.push(Span::styled(" ╱ ", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw("   "));
                }
            }
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(x, y, col_w * (max_l as u16 + 1), 1),
            );
            y += 1;
        }

        let mut spans = Vec::new();
        for lane in 0..=max_l {
            if lane == node.lane {
                let marker_style = if is_sel {
                    Style::default()
                        .fg(Color::Black)
                        .bg(node.color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(node.color).add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(" ● ", marker_style));
            } else if lanes.contains(&lane) {
                let color = nodes
                    .iter()
                    .find(|node| node.lane == lane)
                    .map(|node| node.color)
                    .unwrap_or(Color::DarkGray);
                spans.push(Span::styled(" │ ", Style::default().fg(color)));
            } else {
                spans.push(Span::raw("   "));
            }
        }

        let text_style = if is_sel {
            Style::default().fg(Color::Black).bg(node.color)
        } else {
            Style::default().fg(Color::White)
        };

        spans.push(Span::styled(
            format!("#{:<2}", i + 1),
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(truncate(&node.text, 60), text_style));
        spans.push(Span::styled(
            format!(" ({})", node.branch),
            Style::default().fg(Color::DarkGray),
        ));

        if is_last {
            spans.push(Span::styled(" ★", Style::default().fg(Color::Yellow)));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(x, y, inner.width.saturating_sub(4), 1),
        );
        y += 1;
    }

    if y + 1 < inner.y + inner.height {
        let mut legend = Vec::new();
        let mut seen = HashSet::new();
        for node in &nodes {
            if seen.insert(node.branch.clone()) {
                legend.push(Span::styled(
                    format!(" ●{}  ", node.branch),
                    Style::default().fg(node.color),
                ));
            }
        }
        frame.render_widget(
            Paragraph::new(Line::from(legend)),
            Rect::new(
                inner.x + 2,
                inner.y + inner.height.saturating_sub(2),
                inner.width.saturating_sub(4),
                1,
            ),
        );
    }

    let selected_branch = user_branch_at(messages, sel);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(
                " Up/Down select  Enter resume/fork  Q back  D delete  selected: {}",
                selected_branch
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Rect::new(
            inner.x + 2,
            inner.y + inner.height.saturating_sub(1),
            inner.width.saturating_sub(4),
            1,
        ),
    );

    total
}

fn truncate(s: &str, max: usize) -> String {
    let truncated: String = s.chars().take(max).collect();
    if truncated.len() < s.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}
