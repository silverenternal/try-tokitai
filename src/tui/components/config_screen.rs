//! Pre-chat configuration screen with model selector

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::model_config::ModelRegistry;

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

/// Security level choices for the config screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityLevelChoice {
    /// Ask for all tool calls
    Strict,
    /// Auto-approve Safe, ask for Moderate+
    Standard,
    /// Auto-approve all tools (Safe + Moderate + Low)
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
            SecurityLevelChoice::Strict => "Strict — confirm every tool call",
            SecurityLevelChoice::Standard => "Standard — auto safe, confirm moderate + low",
            SecurityLevelChoice::Permissive => "Permissive — auto all tools",
        }
    }

    /// Does this level auto-approve any tools?
    pub fn auto_approve_enabled(&self) -> bool {
        !matches!(self, SecurityLevelChoice::Strict)
    }

    /// Maximum risk level that gets auto-approved
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
    /// Security level for tool permission control
    pub security_level: SecurityLevelChoice,
    /// Index into ModelRegistry::all_models()
    pub model_index: usize,
    /// Available models for quick selection
    pub available_models: Vec<crate::tui::model_config::ModelInfo>,
    /// Custom API key override (empty = use default from env)
    pub custom_key: String,
    /// Whether editing the key field
    pub editing_key: bool,
}

impl ConfigScreenState {
    pub fn new(model: String, provider: String, api_key_preview: String) -> Self {
        let available_models = ModelRegistry::all_models();
        let model_index = available_models.iter()
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

    /// Visual order: DeepThink → Competition → Privacy → ToolPermission → ModelSelect → KeyInput → Start
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
            .title(" Tokitai AI ")
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // ── Logo banner ──
        let logo_lines = super::logo::render_logo(frame_count);
        let logo_h = logo_lines.len() as u16;
        if inner.height > logo_h + 10 {
            for (i, line) in logo_lines.into_iter().enumerate() {
                let y = inner.y + 1 + i as u16;
                if y < inner.y + inner.height {
                    frame.render_widget(
                        Paragraph::new(line).centered(),
                        ratatui::layout::Rect::new(inner.x + 2, y, inner.width.saturating_sub(4), 1),
                    );
                }
            }
        }

        let logo_offset: u16 = if inner.height > logo_h + 10 { logo_h + 2 } else { 0 };
        let mut y = inner.y + 1 + logo_offset;
        let x = inner.x + 2;
        let w = inner.width.saturating_sub(4);

        // Title
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Configure before starting",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ))),
            Rect::new(x, y, w, 1),
        );
        y += 3;

        // ── Deep Think ──
        let hl = |field: &ConfigField, target: ConfigField| -> Style {
            if *field == target { Style::default().fg(Color::Black).bg(Color::Cyan) }
            else { Style::default().fg(Color::White) }
        };
        let on_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
        let off_style = Style::default().fg(Color::DarkGray);

        let (dt_on, dt_label) = if state.deep_think {
            (on_style, "[ ON ]  Deep Think — max reasoning depth")
        } else {
            (off_style, "[ OFF ] Deep Think — faster responses")
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Deep Think: ", Style::default().fg(Color::Gray)),
                Span::styled(dt_label, hl(&state.selected_field, ConfigField::DeepThink)),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                if state.deep_think { "  ON  — Max output tokens, higher creativity" }
                else { "  OFF — Standard output" },
                if state.deep_think { Style::default().fg(Color::Green) }
                else { Style::default().fg(Color::DarkGray) },
            )),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Competition ──
        let (cp_on, cp_label) = if state.competition_mode {
            (on_style, "[ ON ]  Competition Mode — human checkpoints")
        } else {
            (off_style, "[ OFF ] Competition Mode — fully autonomous")
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Competition: ", Style::default().fg(Color::Gray)),
                Span::styled(cp_label, hl(&state.selected_field, ConfigField::Competition)),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                if state.competition_mode { "  ON  — Pauses between phases, requires /approve" }
                else { "  OFF — Fully autonomous" },
                if state.competition_mode { Style::default().fg(Color::Green) }
                else { Style::default().fg(Color::DarkGray) },
            )),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Privacy ──
        let (pv_on, pv_label) = if state.privacy_mode {
            (on_style, "[ ON ]  Privacy Guard — local model for confidential phases")
        } else {
            (off_style, "[ OFF ] Privacy Guard — no restrictions")
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Privacy: ", Style::default().fg(Color::Gray)),
                Span::styled(pv_label, hl(&state.selected_field, ConfigField::Privacy)),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                if state.privacy_mode { "  ON  — Blocks cloud API for confidential/strict phases" }
                else { "  OFF — No restrictions" },
                if state.privacy_mode { Style::default().fg(Color::Green) }
                else { Style::default().fg(Color::DarkGray) },
            )),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Tool Permission ──
        let sec_text = state.security_level.label();
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Permission: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("[ {} ]", sec_text),
                    hl(&state.selected_field, ConfigField::ToolPermission),
                ),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  Strict: confirm all  |  Standard: auto safe  |  Permissive: auto all  ← → to cycle",
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Model Select ──
        let model_hl = hl(&state.selected_field, ConfigField::ModelSelect);
        if let Some(info) = state.selected_model_info() {
            let desc = format!(
                "{} — {} ({} output)",
                info.display_name, info.description, info.max_output_tokens
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(" Model: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("[ {} ]  {}", info.model, info.provider),
                        model_hl.add_modifier(Modifier::BOLD),
                    ),
                ])),
                Rect::new(x, y, w, 1),
            );
            y += 1;
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(&desc, Style::default().fg(Color::DarkGray)),
                    Span::styled("  ← → to change", Style::default().fg(Color::DarkGray)),
                ])),
                Rect::new(x, y, w, 1),
            );
            // Show the model count
            let pos = format!("{}/{}", state.model_index + 1, state.available_models.len());
            frame.render_widget(
                Paragraph::new(Span::styled(pos, Style::default().fg(Color::DarkGray))),
                Rect::new(x + w - 10, y - 1, 8, 1),
            );
        }
        y += 2;

        // ── API Key ──
        let key_hl = hl(&state.selected_field, ConfigField::KeyInput);
        let key_display = if state.editing_key {
            format!("{}▌", state.custom_key)
        } else if !state.custom_key.is_empty() {
            "•••••••• (custom)".to_string()
        } else {
            format!("{} (from env)", state.api_key_preview)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" API Key: ", Style::default().fg(Color::Gray)),
                Span::styled(key_display, key_hl),
                Span::styled("  Enter to edit", Style::default().fg(Color::DarkGray)),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  Leave empty to use environment variable. Custom key is used for this session only.",
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Start ──
        let s_style = if state.selected_field == ConfigField::Start {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ▶ Start Chat ", s_style.add_modifier(Modifier::BOLD)),
            ])),
            Rect::new(x, y, w, 1),
        );
        y += 2;

        // ── Key hints ──
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
                Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
                Span::styled(" ←→", Style::default().fg(Color::Yellow)),
                Span::styled(" toggle/model  ", Style::default().fg(Color::DarkGray)),
                Span::styled(" Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" select/edit  ", Style::default().fg(Color::DarkGray)),
                Span::styled(" q", Style::default().fg(Color::Yellow)),
                Span::styled(" quit", Style::default().fg(Color::DarkGray)),
            ])),
            Rect::new(x, inner.y + inner.height - 2, w, 1),
        );
    }
}
