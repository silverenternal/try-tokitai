//! Pre-chat configuration screen with model selector.

use crate::tui::model_config::ModelRegistry;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigField {
    Competition,
    DeepThink,
    KeyInput,
    ModelSelect,
    Privacy,
    ToolPermission,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityLevelChoice {
    Strict,
    Standard,
    Permissive,
}

impl SecurityLevelChoice {
    pub fn next(self) -> Self {
        match self {
            SecurityLevelChoice::Strict => SecurityLevelChoice::Standard,
            SecurityLevelChoice::Standard => SecurityLevelChoice::Permissive,
            SecurityLevelChoice::Permissive => SecurityLevelChoice::Strict,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SecurityLevelChoice::Strict => "Strict - confirm every tool call",
            SecurityLevelChoice::Standard => "Standard - auto safe, confirm moderate + low",
            SecurityLevelChoice::Permissive => "Permissive - auto all tools",
        }
    }

    pub fn auto_approve_enabled(&self) -> bool {
        !matches!(self, SecurityLevelChoice::Strict)
    }

    pub fn max_auto_risk(&self) -> crate::tool_matrix::matrix::RiskLevel {
        match self {
            SecurityLevelChoice::Strict => crate::tool_matrix::matrix::RiskLevel::Safe,
            SecurityLevelChoice::Standard => crate::tool_matrix::matrix::RiskLevel::Safe,
            SecurityLevelChoice::Permissive => crate::tool_matrix::matrix::RiskLevel::Low,
        }
    }
}

pub struct ConfigScreenState {
    pub selected_field: ConfigField,
    pub model_name: String,
    pub provider_name: String,
    pub api_key_preview: String,
    pub deep_think: bool,
    pub competition_mode: bool,
    pub privacy_mode: bool,
    pub security_level: SecurityLevelChoice,
    pub model_index: usize,
    pub available_models: Vec<crate::tui::model_config::ModelInfo>,
    pub custom_key: String,
    pub editing_key: bool,
}

impl ConfigScreenState {
    pub fn new(model: String, provider: String, api_key_preview: String) -> Self {
        let available_models = ModelRegistry::all_models();
        let model_index = available_models
            .iter()
            .position(|m| m.model == model)
            .unwrap_or(0);
        Self {
            selected_field: ConfigField::DeepThink,
            model_name: model,
            provider_name: provider,
            api_key_preview,
            deep_think: false,
            competition_mode: false,
            privacy_mode: false,
            security_level: SecurityLevelChoice::Standard,
            model_index,
            available_models,
            custom_key: String::new(),
            editing_key: false,
        }
    }

    pub fn select_next(&mut self) {
        self.editing_key = false;
        self.selected_field = match self.selected_field {
            ConfigField::DeepThink => ConfigField::Competition,
            ConfigField::Competition => ConfigField::Privacy,
            ConfigField::Privacy => ConfigField::ToolPermission,
            ConfigField::ToolPermission => ConfigField::ModelSelect,
            ConfigField::ModelSelect => ConfigField::KeyInput,
            ConfigField::KeyInput => ConfigField::Start,
            ConfigField::Start => ConfigField::DeepThink,
        };
    }

    pub fn select_prev(&mut self) {
        self.editing_key = false;
        self.selected_field = match self.selected_field {
            ConfigField::DeepThink => ConfigField::Start,
            ConfigField::Competition => ConfigField::DeepThink,
            ConfigField::Privacy => ConfigField::Competition,
            ConfigField::ToolPermission => ConfigField::Privacy,
            ConfigField::ModelSelect => ConfigField::ToolPermission,
            ConfigField::KeyInput => ConfigField::ModelSelect,
            ConfigField::Start => ConfigField::KeyInput,
        };
    }

    pub fn next_model(&mut self) {
        self.model_index = (self.model_index + 1) % self.available_models.len();
        self.apply_model();
    }

    pub fn prev_model(&mut self) {
        if self.model_index == 0 {
            self.model_index = self.available_models.len() - 1;
        } else {
            self.model_index -= 1;
        }
        self.apply_model();
    }

    fn apply_model(&mut self) {
        if let Some(m) = self.available_models.get(self.model_index) {
            self.model_name = m.model.to_string();
            self.provider_name = m.provider.to_string();
        }
    }

    pub fn selected_model_info(&self) -> Option<&crate::tui::model_config::ModelInfo> {
        self.available_models.get(self.model_index)
    }

    pub fn push_key_char(&mut self, c: char) {
        self.custom_key.push(c);
    }

    pub fn pop_key_char(&mut self) {
        self.custom_key.pop();
    }
}

pub struct ConfigScreen;

impl ConfigScreen {
    pub fn render(frame: &mut Frame, area: Rect, state: &ConfigScreenState, frame_count: u64) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Atlas AI ")
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(mode) = std::env::var_os("ATLAS_CONFIG_DEBUG") {
            let mode = mode.to_string_lossy();
            return match mode.as_ref() {
                "title" => render_debug_title(frame, inner),
                "compact" => render_compact(frame, inner, state),
                "no-logo" => render_rich(frame, inner, state, false),
                _ => render_rich(frame, inner, state, true),
            };
        }

        if inner.width < 64 || inner.height < 24 {
            render_compact(frame, inner, state);
            return;
        }

        render_rich(frame, inner, state, frame_count != u64::MAX);
    }
}

fn render_debug_title(frame: &mut Frame, inner: Rect) {
    let x = inner.x.saturating_add(2);
    let y = inner.y.saturating_add(1);
    let w = inner.width.saturating_sub(4);
    frame.render_widget(
        Paragraph::new("Configure before starting"),
        Rect::new(x, y, w, 1),
    );
}

fn render_rich(
    frame: &mut Frame,
    inner: Rect,
    state: &ConfigScreenState,
    show_logo_override: bool,
) {
    let logo_lines = if show_logo_override {
        super::logo::render_logo(0)
    } else {
        Vec::new()
    };
    let logo_h = logo_lines.len() as u16;
    let show_logo = show_logo_override && inner.height > logo_h + 15 && inner.width >= 78;

    if show_logo {
        for (i, line) in logo_lines.into_iter().enumerate() {
            let y = inner.y + 1 + i as u16;
            if y < inner.y + inner.height {
                frame.render_widget(
                    Paragraph::new(line),
                    Rect::new(inner.x + 2, y, inner.width.saturating_sub(4), 1),
                );
            }
        }
    }

    let logo_offset = if show_logo { logo_h + 2 } else { 0 };
    let x = inner.x + 2;
    let mut y = inner.y + 1 + logo_offset;
    let w = inner.width.saturating_sub(4);
    let bottom = inner.y + inner.height;

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Configure before starting",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  tuned for a safe quick start",
                Style::default().fg(Color::DarkGray),
            ),
        ])),
        Rect::new(x, y, w, 1),
    );
    y += 1;

    render_toggle_row(
        frame,
        Rect::new(x, y, w, 2),
        "Deep Think",
        state.selected_field == ConfigField::DeepThink,
        state.deep_think,
        if state.deep_think {
            "Max reasoning depth, higher token budget, slower but more thorough."
        } else {
            "Balanced mode for faster replies and lower token use."
        },
    );
    y += 2;

    render_toggle_row(
        frame,
        Rect::new(x, y, w, 2),
        "Competition Mode",
        state.selected_field == ConfigField::Competition,
        state.competition_mode,
        if state.competition_mode {
            "Adds human checkpoints between research phases and waits for approval."
        } else {
            "Lets the agent continue end to end without manual checkpoints."
        },
    );
    y += 2;

    render_toggle_row(
        frame,
        Rect::new(x, y, w, 2),
        "Privacy Guard",
        state.selected_field == ConfigField::Privacy,
        state.privacy_mode,
        if state.privacy_mode {
            "Blocks cloud-only paths for sensitive work and prefers safer local handling."
        } else {
            "No additional privacy restrictions beyond the normal tool policy."
        },
    );
    y += 2;

    let permission_text = format!("[ {} ]", state.security_level.label());
    render_value_row(
        frame,
        Rect::new(x, y, w, 2),
        "Tool Permission",
        state.selected_field == ConfigField::ToolPermission,
        &permission_text,
        "Left/Right cycles how aggressively tool calls are auto-approved.",
    );
    y += 2;

    if let Some(info) = state.selected_model_info() {
        let primary = format!("[ {} ]  {}", info.model, info.provider);
        let secondary = format!(
            "{} | {} output tokens | Left/Right to change",
            info.display_name, info.max_output_tokens
        );
        render_value_row(
            frame,
            Rect::new(x, y, w, 2),
            "Model",
            state.selected_field == ConfigField::ModelSelect,
            &primary,
            &secondary,
        );

        let counter = format!("{}/{}", state.model_index + 1, state.available_models.len());
        frame.render_widget(
            Paragraph::new(counter).alignment(Alignment::Right),
            Rect::new(x, y, w, 1),
        );
    }
    y += 2;

    let key_display = if state.editing_key {
        format!("{}_", state.custom_key)
    } else if !state.custom_key.is_empty() {
        "******** (custom for this session)".to_string()
    } else {
        format!("{} (from env)", state.api_key_preview)
    };
    render_value_row(
        frame,
        Rect::new(x, y, w, 2),
        "API Key",
        state.selected_field == ConfigField::KeyInput,
        &key_display,
        "Press Enter to edit. Leave empty to keep using the environment variable.",
    );
    y += 2;

    if y < bottom.saturating_sub(3) {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "[ Start Chat ]",
                if state.selected_field == ConfigField::Start {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                },
            )])),
            Rect::new(x, y, w, 1),
        );
    }

    if bottom > inner.y + 1 {
        let hint_y = bottom.saturating_sub(2);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
                Span::styled(" move  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Left/Right", Style::default().fg(Color::Yellow)),
                Span::styled(" toggle  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" edit/select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::styled(" quit", Style::default().fg(Color::DarkGray)),
            ])),
            Rect::new(x, hint_y, w, 1),
        );
    }
}

fn render_compact(frame: &mut Frame, inner: Rect, state: &ConfigScreenState) {
    let x = inner.x.saturating_add(2);
    let mut y = inner.y.saturating_add(1);
    let w = inner.width.saturating_sub(4);
    let bottom = inner.y.saturating_add(inner.height);

    frame.render_widget(
        Paragraph::new("Configure before starting"),
        Rect::new(x, y, w, 1),
    );
    y += 2;

    let selected_style = |selected: bool| {
        if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let mut render_text = |text: Line<'static>, y_pos: &mut u16| {
        if *y_pos < bottom.saturating_sub(2) {
            frame.render_widget(Paragraph::new(text), Rect::new(x, *y_pos, w, 1));
        }
        *y_pos = y_pos.saturating_add(1);
    };

    let deep_label = if state.deep_think {
        "[ ON ]  Deep Think"
    } else {
        "[ OFF ] Deep Think"
    };
    render_text(
        Line::from(vec![
            Span::styled("Deep Think: ", Style::default().fg(Color::Gray)),
            Span::styled(
                deep_label.to_string(),
                selected_style(state.selected_field == ConfigField::DeepThink),
            ),
        ]),
        &mut y,
    );
    y = y.saturating_add(1);

    let competition_label = if state.competition_mode {
        "[ ON ]  Competition Mode"
    } else {
        "[ OFF ] Competition Mode"
    };
    render_text(
        Line::from(vec![
            Span::styled("Competition: ", Style::default().fg(Color::Gray)),
            Span::styled(
                competition_label.to_string(),
                selected_style(state.selected_field == ConfigField::Competition),
            ),
        ]),
        &mut y,
    );
    y = y.saturating_add(1);

    let privacy_label = if state.privacy_mode {
        "[ ON ]  Privacy Guard"
    } else {
        "[ OFF ] Privacy Guard"
    };
    render_text(
        Line::from(vec![
            Span::styled("Privacy: ", Style::default().fg(Color::Gray)),
            Span::styled(
                privacy_label.to_string(),
                selected_style(state.selected_field == ConfigField::Privacy),
            ),
        ]),
        &mut y,
    );
    y = y.saturating_add(1);

    render_text(
        Line::from(vec![
            Span::styled("Permission: ", Style::default().fg(Color::Gray)),
            Span::styled(
                state.security_level.label().to_string(),
                selected_style(state.selected_field == ConfigField::ToolPermission),
            ),
        ]),
        &mut y,
    );
    y = y.saturating_add(1);

    if let Some(info) = state.selected_model_info() {
        render_text(
            Line::from(vec![
                Span::styled("Model: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} ({})", info.model, info.provider),
                    selected_style(state.selected_field == ConfigField::ModelSelect),
                ),
            ]),
            &mut y,
        );
        y = y.saturating_add(1);
    }

    let key_display = if state.editing_key {
        format!("{}_", state.custom_key)
    } else if !state.custom_key.is_empty() {
        "(custom key for this session)".to_string()
    } else {
        format!("{} (from env)", state.api_key_preview)
    };
    render_text(
        Line::from(vec![
            Span::styled("API Key: ", Style::default().fg(Color::Gray)),
            Span::styled(
                key_display,
                selected_style(state.selected_field == ConfigField::KeyInput),
            ),
        ]),
        &mut y,
    );
    y = y.saturating_add(1);

    render_text(
        Line::from(vec![Span::styled(
            "Start Chat",
            if state.selected_field == ConfigField::Start {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            },
        )]),
        &mut y,
    );

    let hint_y = bottom.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
            Span::styled(" move  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Left/Right", Style::default().fg(Color::Yellow)),
            Span::styled(" toggle  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" select/edit  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(" quit", Style::default().fg(Color::DarkGray)),
        ])),
        Rect::new(x, hint_y, w, 1),
    );
}

fn render_toggle_row(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    selected: bool,
    enabled: bool,
    desc: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let value = if enabled { "[ ON ]" } else { "[ OFF ]" };
    let value_style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else if enabled {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {}: ", label), Style::default().fg(Color::Gray)),
            Span::styled(format!("{}  {}", value, label), value_style),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );
    if area.height > 1 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                desc.to_string(),
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }
}

fn render_value_row(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    selected: bool,
    value: &str,
    desc: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let value_style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" {}: ", label), Style::default().fg(Color::Gray)),
            Span::styled(value.to_string(), value_style),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );
    if area.height > 1 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                desc.to_string(),
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }
}
