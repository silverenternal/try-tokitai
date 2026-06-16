use ai_assistant::context::{MergeStrategy, ParallelContextManager, ParallelContextManagerConfig};
use ai_assistant::{FileKV, FileKVConfig};
use tempfile::TempDir;

#[test]
fn test_parallel_context_branch_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let context_root = temp_dir.path().join(".context");

    let mut manager = ParallelContextManager::new(ParallelContextManagerConfig {
        context_root: context_root.clone(),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        auto_cleanup_abandoned: false,
        branch_ttl_hours: None,
    })
    .unwrap();

    let feature = manager.create_branch("feature-ai", "main").unwrap();
    let feature_id = feature.branch_id.clone();
    manager.checkout(&feature_id).unwrap();

    let current = manager.get_current_branch().unwrap();
    std::fs::write(current.short_term_dir.join("note.txt"), "feature branch note").unwrap();

    manager.checkout("main").unwrap();
    let merge = manager.merge(&feature_id, "main", None).unwrap();
    assert!(merge.success);

    let current = manager.get_current_branch().unwrap();
    assert!(current.short_term_dir.join("note.txt").exists());
}

#[test]
fn test_filekv_open_put_get_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = FileKVConfig::default();
    config.segment_dir = temp_dir.path().join("segments");
    config.wal_dir = temp_dir.path().join("wal");
    config.index_dir = temp_dir.path().join("index");
    config.checkpoint_dir = temp_dir.path().join("checkpoints");
    config.enable_wal = false;

    let kv = FileKV::open(config).unwrap();
    kv.put("context-key", b"context-value").unwrap();
    let value = kv.get("context-key").unwrap().unwrap();
    assert_eq!(value.as_ref(), b"context-value");
}
