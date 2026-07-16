use super::{Recommendation, RecommendationCategory, RecommendationScore};
use crate::atlas_core::{
    AtlasCore, AtlasEvent, AtlasEventKind, LifecycleState, RelationshipKind, ScientificObject,
};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RecommendationEngine {
    core: Arc<AtlasCore>,
}

impl RecommendationEngine {
    pub fn new(core: Arc<AtlasCore>) -> Self {
        Self { core }
    }

    pub fn evaluate(
        &self,
        focus_object_id: Option<&str>,
        actor: &str,
    ) -> Result<Vec<Recommendation>> {
        let objects = self.core.list()?;
        let focus = focus_object_id.and_then(|id| objects.iter().find(|object| object.id == id));
        let experiments = objects
            .iter()
            .filter(|object| object.object_type.0 == "experiment")
            .collect::<Vec<_>>();
        let evidence = objects
            .iter()
            .filter(|object| object.object_type.0 == "evidence")
            .collect::<Vec<_>>();
        let hypotheses = objects
            .iter()
            .filter(|object| object.object_type.0 == "hypothesis")
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        if hypotheses.iter().any(|object| object.evidence.is_empty()) || evidence.is_empty() {
            candidates.push(candidate(
                RecommendationCategory::NextExperiment,
                "Run an evidence-producing experiment",
                "Current hypotheses lack linked experimental evidence.",
                0.82,
                focus,
            ));
        }
        if experiments
            .iter()
            .any(|object| object.lifecycle == LifecycleState::Failed)
        {
            candidates.push(candidate(
                RecommendationCategory::NextAblation,
                "Isolate the last failure",
                "A failed experiment should be converted into a controlled ablation before retrying.",
                0.76,
                focus,
            ));
        }
        if experiments
            .iter()
            .any(|object| object.lifecycle == LifecycleState::Completed)
            && !objects
                .iter()
                .any(|object| object.object_type.0 == "visualization")
        {
            candidates.push(candidate(
                RecommendationCategory::NextVisualization,
                "Visualize completed experiment evidence",
                "Completed results exist without an object-backed visualization mapping.",
                0.64,
                focus,
            ));
        }
        if evidence.len() >= 2
            && !objects
                .iter()
                .any(|object| object.object_type.0 == "publication")
        {
            candidates.push(candidate(
                RecommendationCategory::NextPublication,
                "Create a publication object",
                "Multiple evidence objects are available and can anchor a publication structure.",
                0.58,
                focus,
            ));
        }
        for recommendation in &mut candidates {
            recommendation.score.total = weighted_total(&recommendation.score);
            let mut object = ScientificObject::new("recommendation", &recommendation.title, actor);
            object.id = recommendation.object_id.clone();
            object.description = recommendation.reason.clone();
            object.metadata.insert(
                "recommendation".into(),
                serde_json::to_value(&recommendation)?,
            );
            object.rebuild_search_index();
            let object = self.core.sync_external(object, actor)?;
            if let Some(focus) = focus {
                self.core.relate(
                    &object.id,
                    &focus.id,
                    RelationshipKind::RelatedTo,
                    actor,
                    BTreeMap::new(),
                )?;
            }
            self.core.record_event(AtlasEvent::new(
                AtlasEventKind::RecommendationGenerated,
                actor,
                vec![object.id],
            ))?;
        }
        candidates.sort_by(|left, right| right.score.total.total_cmp(&left.score.total));
        Ok(candidates)
    }

    pub fn approve(&self, recommendation_id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.update(
            recommendation_id,
            &json!({"lifecycle": "completed", "metadata": {"decision": "approved"}}),
            actor,
        )
    }

    pub fn reject(&self, recommendation_id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.update(
            recommendation_id,
            &json!({"lifecycle": "archived", "metadata": {"decision": "rejected"}}),
            actor,
        )
    }

    pub fn delay(&self, recommendation_id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.update(
            recommendation_id,
            &json!({"metadata": {"decision": "delayed"}}),
            actor,
        )
    }
}

fn candidate(
    category: RecommendationCategory,
    title: &str,
    reason: &str,
    confidence: f64,
    focus: Option<&ScientificObject>,
) -> Recommendation {
    let risk = 0.35;
    let failure_probability = 0.3;
    let focus_key = focus
        .map(|object| object.id.as_str())
        .unwrap_or("workspace");
    let object_id = blake3::hash(format!("recommendation:{category:?}:{focus_key}").as_bytes())
        .to_hex()[..32]
        .to_string();
    Recommendation {
        object_id,
        category,
        title: title.into(),
        reason: reason.into(),
        score: RecommendationScore {
            expected_gain: confidence,
            novelty: 0.55,
            risk,
            scientific_confidence: confidence,
            gpu_cost: 0.25,
            execution_time: 0.35,
            paper_support: 0.5,
            failure_probability,
            recommendation_confidence: confidence,
            total: 0.0,
        },
        evidence_object_ids: focus
            .map(|object| {
                object
                    .evidence
                    .iter()
                    .map(|item| item.evidence_object_id.clone())
                    .collect()
            })
            .unwrap_or_default(),
        related_object_ids: focus
            .map(|object| vec![object.id.clone()])
            .unwrap_or_default(),
        estimated_runtime_hours: 1.0,
        expected_improvement: confidence * 0.2,
    }
}

fn weighted_total(score: &RecommendationScore) -> f64 {
    (score.expected_gain * 0.24
        + score.novelty * 0.12
        + (1.0 - score.risk) * 0.12
        + score.scientific_confidence * 0.18
        + (1.0 - score.gpu_cost) * 0.06
        + (1.0 - score.execution_time) * 0.06
        + score.paper_support * 0.08
        + (1.0 - score.failure_probability) * 0.06
        + score.recommendation_confidence * 0.08)
        .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn recommendations_are_ranked_and_stable() {
        let directory = tempdir().unwrap();
        let core = Arc::new(AtlasCore::open(directory.path()).unwrap());
        let engine = RecommendationEngine::new(core.clone());
        let first = engine.evaluate(None, "agent").unwrap();
        let second = engine.evaluate(None, "agent").unwrap();
        assert!(!first.is_empty());
        assert_eq!(first[0].object_id, second[0].object_id);
        assert_eq!(
            core.list()
                .unwrap()
                .iter()
                .filter(|object| object.object_type.0 == "recommendation")
                .count(),
            first.len()
        );
    }
}
