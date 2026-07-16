//! Research OS - Unified research object graph and persistence layer
//!
//! Connects 16 specialized research domains with persistent research objects:
//! - Hypotheses, Evidence, Experiments, Negative Results
//! - Research Diary, Knowledge Graph, Decision Records
//! - Research Memory, Timeline, Publication Pipeline

pub mod decision_engine;
pub mod diary;
pub mod evidence;
pub mod experiment_lineage;
pub mod hypothesis;
pub mod ingestion;
pub mod knowledge_graph;
pub mod memory;
pub mod mutation;
pub mod negative_results;
pub mod object_graph;
pub mod publication;
pub mod timeline;

pub use decision_engine::{
    create_decision, get_decision, list_decisions, DecisionOption, DecisionRecord,
};
pub use diary::{
    create_diary_entry, create_diary_entry_with_source, get_diary_entry, list_diary_entries,
    DiaryEntry, DiaryEntryType,
};
pub use evidence::{create_evidence, get_evidence, list_evidence, Evidence, EvidenceKind};
pub use experiment_lineage::{
    create_experiment, get_experiment, list_experiments, update_experiment, ExperimentNode,
    ExperimentStatus,
};
pub use hypothesis::{
    create_hypothesis, get_hypothesis, list_hypotheses, update_hypothesis, Hypothesis,
    HypothesisStatus,
};
pub use ingestion::{ingest_agent_turn, ingest_domain_task};
pub use knowledge_graph::{
    create_kg_edge, create_kg_node, get_knowledge_graph, EdgeType, KnowledgeGraphEdge,
    KnowledgeGraphNode, NodeType,
};
pub use memory::{
    create_memory_entry, get_memory_entry, list_memory_entries, search_memory, ResearchMemoryEntry,
};
pub use mutation::execute_mutation;
pub use negative_results::{
    create_negative_result, get_negative_result, list_negative_results, NegativeResult,
};
pub use object_graph::{ResearchObjectId, ResearchObjectType};
pub use publication::{
    create_publication, get_publication, list_publications, update_publication, PublicationDraft,
    PublicationSection, PublicationStatus,
};
pub use timeline::{create_timeline_event, list_timeline_events, EventType, TimelineEvent};
