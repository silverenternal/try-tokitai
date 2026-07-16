//! Research Pipeline State Machine
//!
//! Implements the AI Scientist workflow:
//!   Problem → Literature Review → Hypothesis → Experiment → Validation → Paper

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ReviewerFeedbackEntry {
    pub reviewer: String,
    pub linked_run_id: String,
    pub score: Option<u8>,
    pub comment: String,
    pub resolved: bool,
}

/// Research pipeline phases
#[derive(Debug, Clone, PartialEq)]
pub enum ResearchPhase {
    /// Initial state — waiting for research topic
    Idle,
    /// Understanding the problem, searching literature
    LiteratureReview,
    /// Identifying knowledge gaps, generating hypotheses
    HypothesisGeneration,
    /// Designing experiments and evaluation protocols
    ExperimentDesign,
    /// Executing experiments / running analysis
    Execution,
    /// Analyzing results, validating hypotheses
    Validation,
    /// Writing structured paper output
    PaperWriting,
    /// Human review & iteration
    Review,
    /// Research complete
    Complete,
}

impl ResearchPhase {
    pub fn label(&self) -> &str {
        match self {
            ResearchPhase::Idle => "Idle",
            ResearchPhase::LiteratureReview => "Literature Review",
            ResearchPhase::HypothesisGeneration => "Generating Hypothesis",
            ResearchPhase::ExperimentDesign => "Designing Experiment",
            ResearchPhase::Execution => "Executing Experiment",
            ResearchPhase::Validation => "Validating Results",
            ResearchPhase::PaperWriting => "Writing Paper",
            ResearchPhase::Review => "Review",
            ResearchPhase::Complete => "Complete",
        }
    }

    pub fn next(&self) -> ResearchPhase {
        match self {
            ResearchPhase::Idle => ResearchPhase::LiteratureReview,
            ResearchPhase::LiteratureReview => ResearchPhase::HypothesisGeneration,
            ResearchPhase::HypothesisGeneration => ResearchPhase::ExperimentDesign,
            ResearchPhase::ExperimentDesign => ResearchPhase::Execution,
            ResearchPhase::Execution => ResearchPhase::Validation,
            ResearchPhase::Validation => ResearchPhase::PaperWriting,
            ResearchPhase::PaperWriting => ResearchPhase::Review,
            ResearchPhase::Review => ResearchPhase::Complete,
            ResearchPhase::Complete => ResearchPhase::Idle,
        }
    }

    /// System prompt snippet for this phase
    pub fn system_instruction(&self) -> &str {
        match self {
            ResearchPhase::Idle => "",
            ResearchPhase::LiteratureReview => {
                r#"
## Current Phase: Literature Review
- Search for relevant papers using search_web or search_arxiv
- Extract key methods, datasets, and findings
- Identify the state-of-the-art and current limitations
- Summarize: what approaches exist, what gaps remain
- Output format: structured literature summary with citations
"#
            }
            ResearchPhase::HypothesisGeneration => {
                r#"
## Current Phase: Hypothesis Generation
- Based on the literature review, identify 3-5 concrete research gaps
- For each gap, formulate a specific, testable hypothesis
- Each hypothesis must include:
  * The claim (what you propose)
  * The rationale (why it should work)
  * The expected outcome (what would confirm/disconfirm it)
- Rank hypotheses by novelty and feasibility
- Output format: numbered hypotheses with clear statements
"#
            }
            ResearchPhase::ExperimentDesign => {
                r#"
## Current Phase: Experiment Design
- Select the most promising hypothesis to test
- Design a rigorous experiment:
  * Datasets: what data to use (must be accessible)
  * Baselines: what existing methods to compare against
  * Metrics: quantitative measures of success
  * Protocol: step-by-step experimental procedure
- Consider confounding factors and controls
- Output format: structured experiment plan
"#
            }
            ResearchPhase::Execution => {
                r#"
## Current Phase: Execution
- Implement or simulate the experiment
- Write code to:
  * Load and preprocess data
  * Implement the proposed method
  * Run baselines
  * Compute metrics
- Execute the code and collect results
- Document any implementation details
"#
            }
            ResearchPhase::Validation => {
                r#"
## Current Phase: Validation
- Analyze experimental results:
  * Compare against baselines using the defined metrics
  * Perform statistical significance tests
  * Identify limitations and edge cases
- Determine if the hypothesis is supported or rejected
- If rejected: identify why and suggest revisions
- If supported: quantify the improvement
"#
            }
            ResearchPhase::PaperWriting => {
                r#"
## Current Phase: Paper Writing
- Write a complete research paper with these sections:
  * Title (concise, descriptive)
  * Abstract (150-200 words)
  * 1. Introduction & Problem Statement
  * 2. Related Work (with citations)
  * 3. Proposed Method (with algorithm details)
  * 4. Experiments (datasets, baselines, metrics, results)
  * 5. Analysis & Discussion
  * 6. Conclusion & Future Work
  * References (real papers, properly cited)
- Use academic writing style, clear and precise
"#
            }
            ResearchPhase::Review => {
                r#"
## Current Phase: Review
- Critically review the generated paper:
  * Is the hypothesis clearly stated?
  * Are the experiments reproducible?
  * Are the conclusions supported by the data?
  * Are there logical gaps or overclaims?
- Suggest specific improvements
- Mark sections that need revision
"#
            }
            ResearchPhase::Complete => {
                r#"
## Research Complete
- All phases completed
- Final paper ready for submission
- Summary of contributions
"#
            }
        }
    }
}

/// Full research pipeline state
#[derive(Debug, Clone)]
pub struct ResearchPipeline {
    /// Current phase
    pub phase: ResearchPhase,
    /// Research topic / question
    pub topic: String,
    /// Accumulated research context (literature, hypotheses, results)
    pub context: Vec<String>,
    /// Whether the pipeline is active
    pub active: bool,
    /// Workspace directory (auto-created on start)
    pub workspace: Option<String>,
    /// Error-fix attempt counter
    pub error_retries: usize,
    /// Knowledge graph (paper-method-dataset relations)
    pub knowledge_graph: crate::tui::scientist_tools::KnowledgeGraph,
    /// BibTeX citations collected
    pub citations: Vec<serde_json::Value>,
    /// Competition mode: pause at checkpoints for human approval
    pub competition_mode: bool,
    /// Waiting for human approval before advancing
    pub waiting_approval: bool,
    /// Reviewer feedback panel visibility
    pub reviewer_panel_visible: bool,
    /// Reviewer feedback entries bound to the current research flow
    pub reviewer_feedback: Vec<ReviewerFeedbackEntry>,
    /// Current run identifier or review target
    pub current_run_id: Option<String>,
}

impl ResearchPipeline {
    pub fn new() -> Self {
        Self {
            phase: ResearchPhase::Idle,
            topic: String::new(),
            context: Vec::new(),
            active: false,
            workspace: None,
            error_retries: 0,
            knowledge_graph: crate::tui::scientist_tools::KnowledgeGraph::new(),
            citations: Vec::new(),
            competition_mode: false,
            waiting_approval: false,
            reviewer_panel_visible: false,
            reviewer_feedback: Vec::new(),
            current_run_id: None,
        }
    }

    /// Start a new research pipeline with a given topic
    pub fn start(&mut self, topic: String) {
        self.topic = topic.clone();
        self.phase = ResearchPhase::LiteratureReview;
        self.context.clear();
        self.active = true;
        self.reviewer_feedback.clear();
        self.reviewer_panel_visible = false;
        self.current_run_id = None;

        // Create workspace
        match crate::tui::research_workspace::ResearchWorkspace::create(&topic) {
            Ok(ws) => {
                self.workspace = Some(ws.project_dir.display().to_string());
            }
            Err(e) => {
                tracing::warn!("Failed to create research workspace: {}", e);
                self.workspace = None;
            }
        }
    }

    /// Advance to the next phase
    pub fn advance(&mut self) {
        self.phase = self.phase.next();
    }

    /// Record output from the current phase into context
    pub fn record(&mut self, output: String) {
        self.context
            .push(format!("[{}]\n{}", self.phase.label(), output));
    }

    /// Stop the pipeline
    pub fn stop(&mut self) {
        self.active = false;
        self.phase = ResearchPhase::Idle;
        self.waiting_approval = false;
        self.reviewer_panel_visible = false;
        self.current_run_id = None;
    }

    /// Get the full research context as a string
    pub fn full_context(&self) -> String {
        let mut ctx = format!("# Research Pipeline: {}\n\n", self.topic);
        for (i, c) in self.context.iter().enumerate() {
            ctx.push_str(&format!("## Phase {} Output\n{}\n\n", i + 1, c));
        }
        ctx
    }

    /// Build the system prompt for the current phase
    pub fn system_prompt(&self) -> String {
        if !self.active {
            return String::new();
        }

        let mut prompt = String::new();
        prompt.push_str(&format!(
            "You are an AI Scientist conducting research on: **{}**\n\n",
            self.topic
        ));

        // Workspace context
        if let Some(ref ws_path) = self.workspace {
            prompt.push_str(&format!(
                "## Research Workspace\n\
                 Your dedicated workspace is at: `{}`\n\
                 - `{}/code/` — Write experiment scripts here\n\
                 - `{}/data/` — Store datasets here\n\
                 - `{}/results/` — Save results, figures, logs here\n\
                 - `{}/paper/` — Write paper drafts here\n\n\
                 You can run Python scripts with: `python {}/code/script.py`\n\
                 Install packages with: `pip install <package>`\n\n",
                ws_path, ws_path, ws_path, ws_path, ws_path, ws_path,
            ));
        }

        prompt.push_str(&format!("Current Phase: **{}**\n", self.phase.label()));
        prompt.push_str(self.phase.system_instruction());
        prompt.push_str("\n\n## Research Context So Far\n");
        prompt.push_str(&self.full_context());
        prompt.push_str("\n\nContinue the research. Focus ONLY on the current phase.\n");
        prompt.push_str("When the current phase is complete, say 'PHASE_COMPLETE'.\n");

        prompt
    }

    pub fn set_current_run_id(&mut self, run_id: impl Into<String>) {
        let run_id = run_id.into();
        if run_id.trim().is_empty() {
            self.current_run_id = None;
        } else {
            self.current_run_id = Some(run_id);
        }
    }

    pub fn toggle_reviewer_panel(&mut self) -> bool {
        self.reviewer_panel_visible = !self.reviewer_panel_visible;
        self.reviewer_panel_visible
    }

    pub fn show_reviewer_panel(&mut self) {
        self.reviewer_panel_visible = true;
    }

    pub fn add_reviewer_feedback(
        &mut self,
        reviewer: impl Into<String>,
        score: Option<u8>,
        comment: impl Into<String>,
        linked_run_id: Option<String>,
    ) {
        let linked_run_id = linked_run_id
            .or_else(|| self.current_run_id.clone())
            .unwrap_or_default()
            .trim()
            .to_string();
        self.reviewer_feedback.push(ReviewerFeedbackEntry {
            reviewer: reviewer.into().trim().to_string(),
            linked_run_id,
            score,
            comment: comment.into().trim().to_string(),
            resolved: false,
        });
        self.reviewer_panel_visible = true;
    }

    pub fn resolve_reviewer_feedback(&mut self, index: usize) -> bool {
        if let Some(entry) = self.reviewer_feedback.get_mut(index) {
            entry.resolved = true;
            true
        } else {
            false
        }
    }

    pub fn unresolved_feedback_count(&self) -> usize {
        self.reviewer_feedback
            .iter()
            .filter(|entry| !entry.resolved)
            .count()
    }

    pub fn reviewer_feedback_summary(&self) -> String {
        let run_label = self
            .current_run_id
            .clone()
            .unwrap_or_else(|| "unbound".to_string());
        format!(
            "reviewer_feedback: {} total, {} unresolved, current run {}",
            self.reviewer_feedback.len(),
            self.unresolved_feedback_count(),
            run_label
        )
    }
}

impl Default for ResearchPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewer_feedback_entries_bind_to_current_run() {
        let mut pipeline = ResearchPipeline::new();
        pipeline.set_current_run_id("run-42");
        pipeline.add_reviewer_feedback(
            "panel-a",
            Some(88),
            "Need clearer latency attribution.",
            None,
        );

        assert_eq!(pipeline.reviewer_feedback.len(), 1);
        assert_eq!(pipeline.reviewer_feedback[0].linked_run_id, "run-42");
        assert!(pipeline.reviewer_panel_visible);
        assert_eq!(pipeline.unresolved_feedback_count(), 1);
    }

    #[test]
    fn reviewer_feedback_can_be_resolved() {
        let mut pipeline = ResearchPipeline::new();
        pipeline.add_reviewer_feedback(
            "panel-a",
            Some(90),
            "Looks good after repair.",
            Some("run-9".to_string()),
        );
        assert!(pipeline.resolve_reviewer_feedback(0));
        assert_eq!(pipeline.unresolved_feedback_count(), 0);
    }
}
