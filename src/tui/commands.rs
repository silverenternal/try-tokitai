//! Slash command registry
//!
//! Provides a `/command` system similar to Claude Code's slash commands.

use crate::tui::app::{AppMode, TuiApp};
use crate::tui::components::message_block::MessageBlock;

/// Result of handling a slash command
pub enum CommandResult {
    /// Command was handled, no further action
    Handled,
    /// Add a system message to the conversation
    Message(String),
    /// Send this text as a user message (triggers LLM)
    SendMessage(String),
    /// Quit the application
    Quit,
}

/// A slash command definition
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

/// Registry of slash commands
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: vec![
                Command {
                    name: "help",
                    description: "Show available commands",
                },
                Command {
                    name: "clear",
                    description: "Clear the conversation",
                },
                Command {
                    name: "quit",
                    description: "Exit the application",
                },
                Command {
                    name: "exit",
                    description: "Exit the application",
                },
                Command {
                    name: "model",
                    description: "Show current model info",
                },
                Command {
                    name: "tools",
                    description: "List available tools",
                },
                Command {
                    name: "auto-approve",
                    description: "Toggle auto-approve tool calls",
                },
                Command {
                    name: "config",
                    description: "Re-open configuration screen",
                },
                Command {
                    name: "agents",
                    description: "List available AI agents/skills",
                },
                Command {
                    name: "agent",
                    description: "Switch to a specific agent (/agent <name>)",
                },
                Command {
                    name: "research",
                    description: "Start AI Scientist research (/research <topic>)",
                },
                Command {
                    name: "next",
                    description: "Advance research to next phase",
                },
                Command {
                    name: "phase",
                    description: "Show current research phase",
                },
                Command {
                    name: "stop",
                    description: "Stop the research pipeline",
                },
                Command {
                    name: "fix",
                    description: "Analyze last error and suggest fix",
                },
                Command {
                    name: "stats",
                    description: "Run statistical tests (after experiment)",
                },
                Command {
                    name: "cite",
                    description: "Generate BibTeX references",
                },
                Command {
                    name: "kgraph",
                    description: "Show knowledge graph",
                },
                Command {
                    name: "approve",
                    description: "Approve current phase and continue (competition mode)",
                },
                Command {
                    name: "privacy",
                    description: "Show privacy/security status and audit log",
                },
                Command {
                    name: "sessions",
                    description: "List saved conversation sessions",
                },
                Command {
                    name: "resume",
                    description: "Resume a session (/resume <id>)",
                },
                Command {
                    name: "new",
                    description: "Start a new conversation session",
                },
                Command {
                    name: "summarize",
                    description: "Generate AI summary of this conversation",
                },
                Command {
                    name: "rename",
                    description: "Rename current session (/rename <title>)",
                },
                Command {
                    name: "info",
                    description: "Show current session info and summary",
                },
                Command {
                    name: "branches",
                    description: "List branches in current session",
                },
                Command {
                    name: "fork",
                    description: "Fork conversation at a message (/fork [msg#])",
                },
                Command {
                    name: "merge",
                    description: "Merge a branch into another (/merge <branch-id> [target])",
                },
                Command {
                    name: "delete",
                    description: "Delete a session (/delete <id>)",
                },
            ],
        }
    }

    /// Try to match input as a slash command name.
    pub fn match_command(&self, input: &str) -> Option<&str> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let parts: Vec<&str> = trimmed[1..].splitn(2, ' ').collect();
        let command_name = parts[0];
        for cmd in &self.commands {
            if cmd.name == command_name {
                return Some(cmd.name);
            }
        }
        None
    }

    /// Execute a command by name on the app state.
    pub fn execute(command_name: &str, app: &mut TuiApp, _args: &str) -> CommandResult {
        match command_name {
            "help" => {
                let help_text = "\
Available commands:
  /help           Show this help
  /clear          Clear the conversation
  /quit, /exit    Exit the application
  /model          Show current model info
  /tools          List available tools
  /auto-approve   Toggle auto-approve tool calls
  /config         Re-open configuration
  /sessions       List saved conversation sessions
  /resume <id>    Resume a previous session
  /new            Start a new conversation
  /delete <id>    Delete a session
  /info           Show current session info
  /rename <title> Rename current session
  /summarize      Generate AI summary
  /branches       List branches in current session
  /fork [msg#]    Fork conversation at message
  /merge <branch> [target]
                  Merge a branch into another (default: main)
  /agents         List available AI agents
  /agent <name>   Switch to an agent
  /research <t>   Start AI Scientist pipeline
  /next           Advance research phase
  /phase          Show research status
  /stop           Stop research pipeline";
                app.add_message(MessageBlock::System {
                    content: help_text.to_string(),
                });
                CommandResult::Handled
            }
            "clear" => {
                app.clear_messages();
                CommandResult::Handled
            }
            "quit" | "exit" => CommandResult::Quit,
            "model" => {
                let info = format!(
                    "Model: {}\nProvider: {}",
                    app.status_bar.model, app.status_bar.provider,
                );
                app.add_message(MessageBlock::System { content: info });
                CommandResult::Handled
            }
            "tools" => {
                if let Some(tools) = &app.tool_definitions {
                    let names: Vec<String> = tools
                        .iter()
                        .filter_map(|t| {
                            t.get("function")?
                                .get("name")?
                                .as_str()
                                .map(|s| s.to_string())
                        })
                        .collect();
                    let list = if names.is_empty() {
                        "No tools available.".to_string()
                    } else {
                        format!(
                            "Available tools ({}):\n  {}",
                            names.len(),
                            names.join("\n  ")
                        )
                    };
                    app.add_message(MessageBlock::System { content: list });
                } else {
                    app.add_message(MessageBlock::System {
                        content: "Tool definitions not loaded.".to_string(),
                    });
                }
                CommandResult::Handled
            }
            "auto-approve" => {
                app.auto_approve_tools = !app.auto_approve_tools;
                let status = if app.auto_approve_tools {
                    "ON 鈥?tool calls will be auto-approved"
                } else {
                    "OFF 鈥?tool calls require confirmation"
                };
                app.add_message(MessageBlock::System {
                    content: format!("Auto-approve: {}", status),
                });
                CommandResult::Handled
            }
            "config" => {
                app.mode = AppMode::Config;
                CommandResult::Handled
            }
            "agents" => {
                let agents = app.agent_loader.list_agents();
                if agents.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "No agents found.\nLooking in: {}",
                            app.agent_loader.skills_dir().display()
                        ),
                    });
                } else {
                    let mut list = format!("Available agents ({}):\n\n", agents.len());
                    for (name, desc) in &agents {
                        list.push_str(&format!("  /agent {} 鈥?{}\n", name, desc));
                    }
                    list.push_str("\nUsage: /agent <name> to switch");
                    app.add_message(MessageBlock::System { content: list });
                }
                CommandResult::Handled
            }
            "agent" => {
                let name = _args.trim();
                if name.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: "Usage: /agent <name>\nUse /agents to list available agents."
                            .to_string(),
                    });
                } else if let Some(def) = app.agent_loader.load_agent(name) {
                    app.active_agent = crate::tui::agent_loader::ActiveAgent::from_def(&def);
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "Switched to agent: **{}**\n{}",
                            def.name, def.description
                        ),
                    });
                } else {
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "Agent '{}' not found.\nLooked in: {}\nUse /agents to list available agents.",
                            name,
                            app.agent_loader.skills_dir().display()
                        ),
                    });
                }
                CommandResult::Handled
            }
            "research" => {
                let topic = _args.trim();
                if topic.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: "Usage: /research <topic>\nExample: /research Identify anomalies in EEG data".to_string(),
                    });
                    CommandResult::Handled
                } else {
                    app.research.start(topic.to_string());
                    app.auto_approve_tools = true;
                    let phase = app.research.phase.label().to_string();
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "Research pipeline started!\nTopic: **{}**\nPhase: **{}**\n\nUse /next to advance phases, /stop to end.",
                            app.research.topic, phase,
                        ),
                    });
                    CommandResult::SendMessage(format!(
                        "I am an AI Scientist. My research topic is: {}\n\nCurrent phase: {}\n{}",
                        app.research.topic,
                        phase,
                        app.research.phase.system_instruction()
                    ))
                }
            }
            "next" => {
                if !app.research.active {
                    app.add_message(MessageBlock::System {
                        content: "No active research. Use /research <topic> to start.".to_string(),
                    });
                    CommandResult::Handled
                } else {
                    for msg in app.messages.iter().rev() {
                        if let MessageBlock::Assistant { content } = msg {
                            if !content.is_empty() {
                                app.research.record(content.clone());
                                break;
                            }
                        }
                    }
                    app.research.advance();
                    if app.research.phase == crate::tui::research_pipeline::ResearchPhase::Complete
                    {
                        let ctx = app.research.full_context();
                        app.research.stop();
                        app.add_message(MessageBlock::System {
                            content: format!("Pipeline complete!\n\n{}", ctx),
                        });
                        CommandResult::Handled
                    } else {
                        let phase = app.research.phase.label().to_string();
                        app.add_message(MessageBlock::System {
                            content: format!("Advanced to: **{}**", phase),
                        });
                        CommandResult::SendMessage(format!(
                            "Continue the research. Current phase: {}\n{}",
                            phase,
                            app.research.phase.system_instruction()
                        ))
                    }
                }
            }
            "phase" => {
                if app.research.active {
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "Topic: **{}**\nPhase: **{}** ({} of 7)",
                            app.research.topic,
                            app.research.phase.label(),
                            app.research.context.len() + 1,
                        ),
                    });
                } else {
                    app.add_message(MessageBlock::System {
                        content: "No active research. Use /research <topic> to start.".to_string(),
                    });
                }
                CommandResult::Handled
            }
            "stop" => {
                if app.research.active {
                    let ctx = app.research.full_context();
                    app.research.stop();
                    app.add_message(MessageBlock::System {
                        content: format!("Research stopped.\n\n{}", ctx),
                    });
                } else {
                    app.add_message(MessageBlock::System {
                        content: "No active research to stop.".to_string(),
                    });
                }
                CommandResult::Handled
            }
            "fix" => {
                if !app.research.active {
                    app.add_message(MessageBlock::System {
                        content: "No active research. Start with /research first.".to_string(),
                    });
                    CommandResult::Handled
                } else if let Some(ref ws) = app.research.workspace {
                    let hint = format!(
                        "Error fix mode (attempt {}/3).\n\n\
                         Read the code in `{}/code/` and the error log in `{}/results/`.\n\
                         Identify the bug, fix the code, and re-run the experiment.\n\
                         Common issues: missing imports, wrong paths, package not installed, data format mismatch.\n\n\
                         After fixing, run the experiment again.",
                        app.research.error_retries + 1, ws, ws,
                    );
                    app.research.error_retries += 1;
                    CommandResult::SendMessage(hint)
                } else {
                    app.add_message(MessageBlock::System {
                        content: "No workspace found.".to_string(),
                    });
                    CommandResult::Handled
                }
            }
            "stats" => {
                if !app.research.active {
                    app.add_message(MessageBlock::System {
                        content: "No active research. Start with /research first.".to_string(),
                    });
                    CommandResult::Handled
                } else {
                    CommandResult::SendMessage(
                        "Run statistical analysis on the experiment results.\n\n\
                         Use scipy.stats for t-tests, ANOVA, effect sizes.\n\
                         Read the results from `results/` directory.\n\
                         Output formatted JSON with descriptive stats, normality tests,\n\
                         t-tests with p-values, Cohen's d effect sizes, and a summary conclusion."
                            .to_string(),
                    )
                }
            }
            "cite" => {
                let citations = &app.research.citations;
                if citations.is_empty() {
                    let hint = format!(
                        "Generate BibTeX citations for papers referenced in this research.\n\n\
                         For each paper, provide:\n\
                         - key (e.g., smith2024attention)\n\
                         - authors\n\
                         - title\n\
                         - venue/journal\n\
                         - year\n\
                         - url/doi\n\n\
                         Write them to `{}/paper/references.bib`",
                        app.research.workspace.as_deref().unwrap_or("workspace")
                    );
                    CommandResult::SendMessage(hint)
                } else {
                    let bib =
                        crate::tui::scientist_tools::CitationManager::generate_bib_file(citations);
                    app.add_message(MessageBlock::System {
                        content: format!("References:\n\n```bib\n{}\n```", bib),
                    });
                    CommandResult::Handled
                }
            }
            "privacy" => {
                let report = app.privacy.report();
                app.add_message(MessageBlock::System { content: report });
                CommandResult::Handled
            }
            "approve" => {
                if !app.research.active {
                    app.add_message(MessageBlock::System {
                        content: "No active research.".to_string(),
                    });
                    CommandResult::Handled
                } else if !app.research.waiting_approval {
                    app.add_message(MessageBlock::System {
                        content: "Not waiting for approval. Research is auto-advancing."
                            .to_string(),
                    });
                    CommandResult::Handled
                } else {
                    app.research.waiting_approval = false;
                    app.research.advance();
                    if app.research.phase == crate::tui::research_pipeline::ResearchPhase::Complete
                    {
                        let ctx = app.research.full_context();
                        app.research.stop();
                        app.add_message(MessageBlock::System {
                            content: format!("Pipeline complete!\n\n{}", ctx),
                        });
                        CommandResult::Handled
                    } else {
                        let phase = app.research.phase.label().to_string();
                        app.add_message(MessageBlock::System {
                            content: format!("Approved. Continuing to: **{}**", phase),
                        });
                        let instructions = app.research.phase.system_instruction().to_string();
                        CommandResult::SendMessage(format!(
                            "Continue research. Phase: {}\n{}",
                            phase, instructions
                        ))
                    }
                }
            }
            "kgraph" => {
                let kg = &app.research.knowledge_graph;
                let mermaid = kg.to_mermaid();
                let json = kg.to_json();
                app.add_message(MessageBlock::System {
                    content: format!(
                        "Knowledge Graph ({} entities, {} relations):\n\n{}\n```json\n{}\n```",
                        kg.entities.len(),
                        kg.relations.len(),
                        mermaid,
                        serde_json::to_string_pretty(&json).unwrap_or_default()
                    ),
                });
                CommandResult::Handled
            }
            "fork" => {
                let idx_str = _args.trim();
                let at = if idx_str.is_empty() {
                    // Fork at current graph selection
                    app.graph_selected
                } else {
                    match idx_str.parse::<usize>() {
                        Ok(n) if n > 0 => n - 1, // Convert 1-based to 0-indexed
                        _ => {
                            app.add_message(MessageBlock::System {
                                content: "Usage: /fork [message-number]\nFork at current selection or the given message number (e.g. /fork 3)."
                                    .to_string(),
                            });
                            return CommandResult::Handled;
                        }
                    }
                };

                // Count user messages to validate index
                let user_count = app
                    .messages
                    .iter()
                    .filter(|m| matches!(m, MessageBlock::User { .. }))
                    .count();
                if at >= user_count {
                    app.add_message(MessageBlock::System {
                        content: format!(
                            "Invalid message number: {}. There are {} user messages (1-{}).",
                            at + 1,
                            user_count,
                            user_count
                        ),
                    });
                    return CommandResult::Handled;
                }

                let current_branch = app.current_branch_id.clone();
                match app.session_manager.fork_at_node(at, &current_branch) {
                    Ok(fork_id) => {
                        app.current_branch_id = fork_id.clone();
                        // Add a fork-tagged user message to trigger LLM on the new branch
                        app.add_message(MessageBlock::User {
                            content: format!("[forked at #{}]", at + 1),
                            branch_id: fork_id.clone(),
                        });
                        app.add_message(MessageBlock::System {
                            content: format!(
                                "Forked at message #{} → branch `{}` (parent: {})",
                                at + 1, fork_id, current_branch,
                            ),
                        });
                    }
                    Err(e) => {
                        app.add_message(MessageBlock::System {
                            content: format!("Fork failed: {}", e),
                        });
                    }
                }
                CommandResult::Handled
            }
            "merge" => {
                let args: Vec<&str> = _args.split_whitespace().collect();
                if args.is_empty() {
                    let branches = app.session_manager.list_branches();
                    let mut list = String::from("Usage: /merge <branch-id> [target-branch]\n\nAvailable branches:\n");
                    for b in &branches {
                        let merged = b.merged_into.as_ref()
                            .map(|t| format!(" (merged into {})", t))
                            .unwrap_or_default();
                        list.push_str(&format!("  {} — parent: {}{}\n", b.id, if b.parent_id.is_empty() { "main" } else { &b.parent_id }, merged));
                    }
                    app.add_message(MessageBlock::System { content: list });
                } else {
                    let branch_id = args[0];
                    let target = if args.len() > 1 { args[1] } else { "main" };
                    match app
                        .session_manager
                        .mark_branch_merged(branch_id, target)
                    {
                        Ok(()) => {
                            app.add_message(MessageBlock::System {
                                content: format!(
                                    "Branch `{}` merged into `{}`",
                                    branch_id, target
                                ),
                            });
                        }
                        Err(e) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Merge failed: {}", e),
                            });
                        }
                    }
                }
                CommandResult::Handled
            }
            "branches" => {
                let branches = app.session_manager.list_branches();
                if branches.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: "No branches in current session.".to_string(),
                    });
                } else {
                    let mut list = format!("Branches ({}):\n\n", branches.len());
                    for b in branches {
                        let current = if b.id == app.current_branch_id {
                            " *"
                        } else {
                            ""
                        };
                        let parent = if b.parent_id.is_empty() {
                            "main"
                        } else {
                            &b.parent_id
                        };
                        let merged = b
                            .merged_into
                            .as_ref()
                            .map(|target| format!(" merged into {}", target))
                            .unwrap_or_default();
                        list.push_str(&format!(
                            "  {} ({}){} - parent: {}, forked at msg #{}{}\n",
                            b.name, b.id, current, parent, b.fork_msg_index, merged,
                        ));
                    }
                    app.add_message(MessageBlock::System { content: list });
                }
                CommandResult::Handled
            }
            "delete" => {
                let id = _args.trim();
                if id.is_empty() {
                    app.add_message(MessageBlock::System {
                        content:
                            "Usage: /delete <session-id>\nUse /sessions to list saved sessions."
                                .to_string(),
                    });
                } else {
                    match app.session_manager.delete_session(id) {
                        Ok(()) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Deleted session: `{}`", id),
                            });
                        }
                        Err(e) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Failed to delete: {}", e),
                            });
                        }
                    }
                }
                CommandResult::Handled
            }
            "info" => {
                let id = app.session_manager.current_id.clone();
                if let Some(id) = id {
                    if let Some(s) = app.session_manager.index.iter().find(|m| m.id == id) {
                        let mut info = format!(
                            "Session: `{}`\nTitle: {}\nModel: {}\nMessages: {}\nCreated: {}\nUpdated: {}",
                            s.id, s.title, s.model, s.message_count, s.created_at, s.updated_at,
                        );
                        if !s.summary.is_empty() {
                            info.push_str(&format!("\nSummary: {}", s.summary));
                        } else {
                            info.push_str("\nSummary: (none 鈥?use /summarize to generate)");
                        }
                        app.add_message(MessageBlock::System { content: info });
                    } else {
                        app.add_message(MessageBlock::System {
                            content: "Session not found in index.".to_string(),
                        });
                    }
                } else {
                    app.add_message(MessageBlock::System {
                        content: "No active session.".to_string(),
                    });
                }
                CommandResult::Handled
            }
            "sessions" => {
                let sessions = app.session_manager.list_recent(20);
                if sessions.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: "No saved sessions.".to_string(),
                    });
                } else {
                    let mut list = format!("Saved sessions ({}):\n\n", sessions.len());
                    for s in sessions {
                        let current = app.session_manager.current_id.as_deref() == Some(&s.id);
                        let marker = if current { " *" } else { "" };
                        let mut entry = format!(
                            "  `{}`{}  鈥?{}\n    {} messages | {} | {}",
                            s.id, marker, s.title, s.message_count, s.model, s.updated_at,
                        );
                        if !s.summary.is_empty() {
                            entry.push_str(&format!("\n    {}", s.summary));
                        }
                        list.push_str(&format!("{}\n\n", entry));
                    }
                    list.push_str("Use /resume <id> to restore a session, /new to start fresh.");
                    app.add_message(MessageBlock::System { content: list });
                }
                CommandResult::Handled
            }
            "resume" => {
                let id = _args.trim();
                if id.is_empty() {
                    app.add_message(MessageBlock::System {
                        content:
                            "Usage: /resume <session-id>\nUse /sessions to list saved sessions."
                                .to_string(),
                    });
                    CommandResult::Handled
                } else {
                    // Save current session first
                    let _ = app.session_manager.save_messages(&app.messages);
                    match app.session_manager.resume_session(id) {
                        Ok(msgs) => {
                            // Update model from session meta
                            if let Some(meta) =
                                app.session_manager.index.iter().find(|m| m.id == id)
                            {
                                app.status_bar.model = meta.model.clone();
                            }
                            app.messages = msgs;
                            app.add_message(MessageBlock::System {
                                content: format!("Resumed session: `{}`", id),
                            });
                        }
                        Err(e) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Failed to load session: {}", e),
                            });
                        }
                    }
                    CommandResult::Handled
                }
            }
            "summarize" => {
                // Mark that the next assistant response should be saved as summary
                app.pending_summarize = true;
                CommandResult::SendMessage(
                    "Please write a one-sentence summary (max 100 chars) of what we've discussed \
                     and accomplished in this conversation. Focus on the key question and outcome. \
                     Output ONLY the summary text, no prefix or quotes."
                        .to_string(),
                )
            }
            "rename" => {
                let title = _args.trim();
                if title.is_empty() {
                    app.add_message(MessageBlock::System {
                        content: "Usage: /rename <new-title>\nSets a custom title for the current session.".to_string(),
                    });
                } else {
                    match app.session_manager.set_title(title) {
                        Ok(()) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Session renamed to: {}", title),
                            });
                        }
                        Err(e) => {
                            app.add_message(MessageBlock::System {
                                content: format!("Failed to rename: {}", e),
                            });
                        }
                    }
                }
                CommandResult::Handled
            }
            "new" => {
                // Save current session, start new one
                let _ = app.session_manager.save_messages(&app.messages);
                let model = app.status_bar.model.clone();
                let _ = app.session_manager.switch_to_new(&app.messages, &model);
                app.messages.clear();
                app.add_message(MessageBlock::System {
                    content: "New conversation started.".to_string(),
                });
                CommandResult::Handled
            }
            _ => CommandResult::Message(format!(
                "Unknown command: /{}\nType /help for available commands.",
                command_name
            )),
        }
    }

    /// Get list of command names and descriptions for completion
    pub fn completions(&self) -> Vec<(&'static str, &'static str)> {
        self.commands
            .iter()
            .map(|c| (c.name, c.description))
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
