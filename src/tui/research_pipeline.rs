//! Research Pipeline State Machine
//!
//! Implements the AI Scientist workflow:
//!   Problem → Literature Review → Hypothesis → Experiment → Validation → Paper

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
            ResearchPhase::LiteratureReview => r#"
## Current Phase: Literature Review
- Search for relevant papers using search_web or search_arxiv
- Extract key methods, datasets, and findings
- Identify the state-of-the-art and current limitations
- Summarize: what approaches exist, what gaps remain
- Output format: structured literature summary with citations
"#,
            ResearchPhase::HypothesisGeneration => r#"
## Current Phase: Hypothesis Generation
- Based on the literature review, identify 3-5 concrete research gaps
- For each gap, formulate a specific, testable hypothesis
- Each hypothesis must include:
  * The claim (what you propose)
  * The rationale (why it should work)
  * The expected outcome (what would confirm/disconfirm it)
- Rank hypotheses by novelty and feasibility
- Output format: numbered hypotheses with clear statements
"#,
            ResearchPhase::ExperimentDesign => r#"
## Current Phase: Experiment Design
- Select the most promising hypothesis to test
- Design a rigorous experiment:
  * Datasets: what data to use (must be accessible)
  * Baselines: what existing methods to compare against
  * Metrics: quantitative measures of success
  * Protocol: step-by-step experimental procedure
- Consider confounding factors and controls
- Output format: structured experiment plan
"#,
            ResearchPhase::Execution => r#"
## Current Phase: Execution
- Implement or simulate the experiment
- Write code to:
  * Load and preprocess data
  * Implement the proposed method
  * Run baselines
  * Compute metrics
- Execute the code and collect results
- Document any implementation details
"#,
            ResearchPhase::Validation => r#"
## Current Phase: Validation
- Analyze experimental results:
  * Compare against baselines using the defined metrics
  * Perform statistical significance tests
  * Identify limitations and edge cases
- Determine if the hypothesis is supported or rejected
- If rejected: identify why and suggest revisions
- If supported: quantify the improvement
"#,
            ResearchPhase::PaperWriting => r#"
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
"#,
            ResearchPhase::Review => r#"
## Current Phase: Review
- Critically review the generated paper:
  * Is the hypothesis clearly stated?
  * Are the experiments reproducible?
  * Are the conclusions supported by the data?
  * Are there logical gaps or overclaims?
- Suggest specific improvements
- Mark sections that need revision
"#,
            ResearchPhase::Complete => r#"
## Research Complete
- All phases completed
- Final paper ready for submission
- Summary of contributions
"#,
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
        }
    }

    /// Start a new research pipeline with a given topic
    pub fn start(&mut self, topic: String) {
        self.topic = topic.clone();
        self.phase = ResearchPhase::LiteratureReview;
        self.context.clear();
        self.active = true;

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
        self.context.push(format!(
            "[{}]\n{}",
            self.phase.label(),
            output
        ));
    }

    /// Stop the pipeline
    pub fn stop(&mut self) {
        self.active = false;
        self.phase = ResearchPhase::Idle;
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
}

impl Default for ResearchPipeline {
    fn default() -> Self {
        Self::new()
    }
}
