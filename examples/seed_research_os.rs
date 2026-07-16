use ai_assistant::research_os::execute_mutation;
use serde_json::json;
use std::path::PathBuf;

fn main() {
    let workspace = PathBuf::from(std::env::args().nth(1).expect("workspace path required"));

    let hyp = execute_mutation(
        &workspace,
        "create_hypothesis",
        &json!({
            "title": "Larger batch size improves throughput without hurting accuracy",
            "description": "Testing whether batch size 256 vs 64 changes validation accuracy on the benchmark task",
            "domain_id": "ai-ml",
        }),
    )
    .expect("create_hypothesis failed");
    let hyp_id = hyp["id"].as_str().unwrap().to_string();
    println!("created hypothesis {}", hyp_id);

    let exp = execute_mutation(
        &workspace,
        "create_experiment",
        &json!({
            "title": "Batch size sweep run 1",
            "domain_id": "ai-ml",
            "hypothesis_id": hyp_id,
            "parameters": {"batch_size": 256, "lr": 0.001},
        }),
    )
    .expect("create_experiment failed");
    let exp_id = exp["id"].as_str().unwrap().to_string();
    println!("created experiment {}", exp_id);

    execute_mutation(
        &workspace,
        "update_experiment",
        &json!({"id": exp_id, "status": "completed", "artifacts": ["runs/exp1/metrics.json"]}),
    )
    .expect("update_experiment failed");

    let evidence = execute_mutation(
        &workspace,
        "create_evidence",
        &json!({
            "kind": "experimental",
            "summary": "Validation accuracy stayed within 0.3pt of baseline at batch size 256",
            "strength": 0.82,
            "supports": true,
            "hypothesis_id": hyp_id,
            "experiment_id": exp_id,
            "source_path": "runs/exp1/metrics.json",
        }),
    )
    .expect("create_evidence failed");
    println!("created evidence {}", evidence["id"]);

    let updated_hyp = execute_mutation(
        &workspace,
        "update_hypothesis",
        &json!({"id": hyp_id, "status": "validated"}),
    )
    .expect("update_hypothesis failed");
    println!("hypothesis now {}", updated_hyp["status"]);

    let neg = execute_mutation(
        &workspace,
        "create_negative_result",
        &json!({
            "title": "Batch size 1024 causes OOM on single GPU",
            "description": "Ran out of memory when attempting batch size 1024 on a 24GB GPU",
            "failure_mode": "resource_exhaustion",
            "domain_id": "ai-ml",
            "learned": "Cap batch size at 512 for this GPU or use gradient accumulation",
            "hypothesis_id": hyp_id,
        }),
    )
    .expect("create_negative_result failed");
    println!("created negative result {}", neg["id"]);

    let decision = execute_mutation(
        &workspace,
        "create_decision",
        &json!({
            "title": "Adopt batch size 256 as default",
            "context": "Balances throughput gains against OOM risk observed at larger sizes",
            "options": [
                {"id": "opt-256", "label": "Batch size 256", "pros": ["Stable", "22% faster"], "cons": [], "estimated_cost": "low"},
                {"id": "opt-1024", "label": "Batch size 1024", "pros": ["Fastest"], "cons": ["OOM on 24GB GPU"], "estimated_cost": "high"}
            ],
            "chosen_option_id": "opt-256",
            "decision_score": 0.78,
            "rationale": "256 is the largest batch size that avoided OOM across all runs while improving throughput.",
        }),
    )
    .expect("create_decision failed");
    println!("created decision {}", decision["id"]);

    let memory = execute_mutation(
        &workspace,
        "create_memory",
        &json!({
            "content": "GPU memory ceiling for this benchmark model is batch size 512 on 24GB cards; use gradient accumulation beyond that.",
            "importance": 0.75,
        }),
    )
    .expect("create_memory failed");
    println!("created memory {}", memory["id"]);

    let publication = execute_mutation(
        &workspace,
        "create_publication",
        &json!({"title": "Batch Size Scaling Notes"}),
    )
    .expect("create_publication failed");
    let pub_id = publication["id"].as_str().unwrap().to_string();

    let evidence_id = evidence["id"].as_str().unwrap().to_string();
    let updated_pub = execute_mutation(
        &workspace,
        "update_publication",
        &json!({"id": pub_id, "status": "ready", "evidence_ids": [evidence_id], "hypothesis_ids": [hyp_id]}),
    )
    .expect("update_publication failed");
    println!("publication now {}", updated_pub["status"]);

    println!("seed complete");
}
