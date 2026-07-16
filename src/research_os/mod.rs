//! Research OS - Unified research object graph and persistence layer
//!
//! Connects 16 specialized research domains with persistent research objects:
//! - Hypotheses, Evidence, Experiments, Negative Results
//! - Research Diary, Knowledge Graph, Decision Records
//! - Research Memory, Timeline, Publication Pipeline

pub mod object_graph;
pub mod hypothesis;
pub mod evidence;
pub mod experiment_lineage;
pub mod negative_results;
pub mod diary;
pub mod knowledge_graph;
pub mod decision_engine;
pub mod memory;
pub mod timeline;
pub mod publication;
pub mod ingestion;
pub mod mutation;

pub use object_graph::{ResearchObjectId, ResearchObjectType};
pub use hypothesis::{Hypothesis, HypothesisStatus, create_hypothesis, update_hypothesis, list_hypotheses, get_hypothesis};
pub use evidence::{Evidence, EvidenceKind, create_evidence, list_evidence, get_evidence};
pub use experiment_lineage::{ExperimentNode, ExperimentStatus, create_experiment, update_experiment, list_experiments, get_experiment};
pub use negative_results::{NegativeResult, create_negative_result, list_negative_results, get_negative_result};
pub use diary::{DiaryEntry, DiaryEntryType, create_diary_entry, create_diary_entry_with_source, list_diary_entries, get_diary_entry};
pub use knowledge_graph::{KnowledgeGraphNode, KnowledgeGraphEdge, NodeType, EdgeType, create_kg_node, create_kg_edge, get_knowledge_graph};
pub use decision_engine::{DecisionRecord, DecisionOption, create_decision, list_decisions, get_decision};
pub use memory::{ResearchMemoryEntry, create_memory_entry, search_memory, get_memory_entry, list_memory_entries};
pub use timeline::{TimelineEvent, EventType, create_timeline_event, list_timeline_events};
pub use publication::{PublicationDraft, PublicationSection, PublicationStatus, create_publication, update_publication, list_publications, get_publication};
pub use ingestion::{ingest_agent_turn, ingest_domain_task};
pub use mutation::execute_mutation;
