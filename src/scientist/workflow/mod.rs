//! AI Scientist Workflow
//!
//! Declarative workflow definition for the full AI Scientist pipeline.

/// Path to the workflow TOML file
pub const AI_SCIENTIST_WORKFLOW_TOML: &str = include_str!("ai_scientist.toml");

/// Available workflow stages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchStage {
    LiteratureReview,
    HypothesisGeneration,
    ExperimentDesign,
    Verification,
    ResultAnalysis,
    ReportGeneration,
}

impl ResearchStage {
    pub fn id(&self) -> &str {
        match self {
            Self::LiteratureReview => "literature_review",
            Self::HypothesisGeneration => "hypothesis_generation",
            Self::ExperimentDesign => "experiment_design",
            Self::Verification => "verification",
            Self::ResultAnalysis => "result_analysis",
            Self::ReportGeneration => "report_generation",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::LiteratureReview => "Literature Review",
            Self::HypothesisGeneration => "Hypothesis Generation",
            Self::ExperimentDesign => "Experiment Design",
            Self::Verification => "Verification",
            Self::ResultAnalysis => "Result Analysis",
            Self::ReportGeneration => "Report Generation",
        }
    }

    pub fn order(&self) -> usize {
        match self {
            Self::LiteratureReview => 0,
            Self::HypothesisGeneration => 1,
            Self::ExperimentDesign => 2,
            Self::Verification => 3,
            Self::ResultAnalysis => 4,
            Self::ReportGeneration => 5,
        }
    }

    pub fn all() -> Vec<ResearchStage> {
        vec![
            Self::LiteratureReview,
            Self::HypothesisGeneration,
            Self::ExperimentDesign,
            Self::Verification,
            Self::ResultAnalysis,
            Self::ReportGeneration,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::ResearchStage;

    #[test]
    fn research_stages_have_stable_ids_and_order() {
        let stages = ResearchStage::all();
        let ids: Vec<&str> = stages.iter().map(ResearchStage::id).collect();
        let labels: Vec<&str> = stages.iter().map(ResearchStage::label).collect();
        let orders: Vec<usize> = stages.iter().map(ResearchStage::order).collect();

        assert_eq!(
            ids,
            vec![
                "literature_review",
                "hypothesis_generation",
                "experiment_design",
                "verification",
                "result_analysis",
                "report_generation",
            ]
        );
        assert_eq!(
            labels,
            vec![
                "Literature Review",
                "Hypothesis Generation",
                "Experiment Design",
                "Verification",
                "Result Analysis",
                "Report Generation",
            ]
        );
        assert_eq!(orders, vec![0, 1, 2, 3, 4, 5]);
    }
}
