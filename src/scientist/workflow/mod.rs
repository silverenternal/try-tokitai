//! AI Scientist Workflow
//!
//! Declarative workflow definition for the full AI Scientist pipeline.

mod paper_workflow;

/// Path to the workflow TOML file
pub const AI_SCIENTIST_WORKFLOW_TOML: &str = include_str!("ai_scientist.toml");

pub use paper_workflow::{run_paper_workflow, PaperWorkflowRequest, PaperWorkflowResult};

/// Available workflow stages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchStage {
    LiteratureReview,
    ProblemFormulation,
    ImplementationOrBenchmarkDesign,
    Verification,
    ResultAnalysis,
    ReportGeneration,
}

impl ResearchStage {
    pub fn id(&self) -> &str {
        match self {
            Self::LiteratureReview => "literature_review",
            Self::ProblemFormulation => "problem_formulation",
            Self::ImplementationOrBenchmarkDesign => "implementation_or_benchmark_design",
            Self::Verification => "verification",
            Self::ResultAnalysis => "result_analysis",
            Self::ReportGeneration => "report_generation",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::LiteratureReview => "CS Literature Review",
            Self::ProblemFormulation => "Problem Formulation",
            Self::ImplementationOrBenchmarkDesign => "Implementation Or Benchmark Design",
            Self::Verification => "Verification",
            Self::ResultAnalysis => "Result Analysis",
            Self::ReportGeneration => "Report Generation",
        }
    }

    pub fn order(&self) -> usize {
        match self {
            Self::LiteratureReview => 0,
            Self::ProblemFormulation => 1,
            Self::ImplementationOrBenchmarkDesign => 2,
            Self::Verification => 3,
            Self::ResultAnalysis => 4,
            Self::ReportGeneration => 5,
        }
    }

    pub fn all() -> Vec<ResearchStage> {
        vec![
            Self::LiteratureReview,
            Self::ProblemFormulation,
            Self::ImplementationOrBenchmarkDesign,
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
                "problem_formulation",
                "implementation_or_benchmark_design",
                "verification",
                "result_analysis",
                "report_generation",
            ]
        );
        assert_eq!(
            labels,
            vec![
                "CS Literature Review",
                "Problem Formulation",
                "Implementation Or Benchmark Design",
                "Verification",
                "Result Analysis",
                "Report Generation",
            ]
        );
        assert_eq!(orders, vec![0, 1, 2, 3, 4, 5]);
    }
}
