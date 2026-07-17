//! Incremental Checkpoint Tests

use super::*;
use tempfile::TempDir;

fn create_test_manager() -> (IncrementalCheckpointManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();
    (manager, temp_dir)
}

#[test]
fn test_full_checkpoint_creation() {
    let (mut manager, _temp_dir) = create_test_manager();

    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    state.insert("key2".to_string(), b"value2".to_vec());
    state.insert("key3".to_string(), b"value3".to_vec());

    let checkpoint_id = manager.create_full_checkpoint(&state, Some("Test full checkpoint")).unwrap();

    assert!(checkpoint_id.starts_with("ckpt_"));

    let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
    assert!(matches!(checkpoint.checkpoint_type, CheckpointType::Full));
    assert_eq!(checkpoint.entries.len(), 3);
    assert_eq!(checkpoint.metadata.total_entries, 3);
    assert_eq!(checkpoint.metadata.put_count, 3);
}

#[test]
fn test_incremental_checkpoint_creation() {
    let (mut manager, _temp_dir) = create_test_manager();

    // First create a full checkpoint
    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    let _ = manager.create_full_checkpoint(&state, Some("Base")).unwrap();

    // Create incremental checkpoint with changes
    let changes = vec![
        CheckpointEntry::Put {
            key: "key2".to_string(),
            value: b"value2".to_vec(),
            timestamp: 1000,
        },
        CheckpointEntry::Delete {
            key: "key1".to_string(),
            timestamp: 1001,
        },
    ];

    let checkpoint_id = manager.create_incremental_checkpoint(changes, Some("Test incremental")).unwrap();

    let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
    assert!(matches!(checkpoint.checkpoint_type, CheckpointType::Incremental { .. }));
    assert_eq!(checkpoint.entries.len(), 2);
    assert_eq!(checkpoint.metadata.put_count, 1);
    assert_eq!(checkpoint.metadata.delete_count, 1);
}

#[test]
fn test_compute_diff() {
    let mut old_state: HashMap<String, Vec<u8>> = HashMap::new();
    old_state.insert("key1".to_string(), b"value1".to_vec());
    old_state.insert("key2".to_string(), b"value2".to_vec());

    let mut new_state: HashMap<String, Vec<u8>> = HashMap::new();
    new_state.insert("key1".to_string(), b"value1_modified".to_vec());
    new_state.insert("key3".to_string(), b"value3".to_vec());

    let changes = IncrementalCheckpointManager::compute_diff(&old_state, &new_state);

    assert_eq!(changes.len(), 3);

    let mut has_delete = false;
    let mut has_modify = false;
    let mut has_put = false;

    for change in &changes {
        match change {
            CheckpointEntry::Delete { key, .. } if key == "key2" => has_delete = true,
            CheckpointEntry::Modify { key, .. } if key == "key1" => has_modify = true,
            CheckpointEntry::Put { key, .. } if key == "key3" => has_put = true,
            _ => {}
        }
    }

    assert!(has_delete);
    assert!(has_modify);
    assert!(has_put);
}

#[test]
fn test_restore_from_full_checkpoint() {
    let (mut manager, _temp_dir) = create_test_manager();

    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    state.insert("key2".to_string(), b"value2".to_vec());

    let checkpoint_id = manager.create_full_checkpoint(&state, None).unwrap();
    let restored = manager.restore(&checkpoint_id).unwrap();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored.get("key1"), Some(&b"value1".to_vec()));
    assert_eq!(restored.get("key2"), Some(&b"value2".to_vec()));
}

#[test]
fn test_restore_from_incremental_checkpoint() {
    let (mut manager, _temp_dir) = create_test_manager();

    // Create base full checkpoint
    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    let _ = manager.create_full_checkpoint(&state, None).unwrap();

    // Create incremental checkpoint
    let changes = vec![
        CheckpointEntry::Put {
            key: "key2".to_string(),
            value: b"value2".to_vec(),
            timestamp: 1000,
        },
        CheckpointEntry::Delete {
            key: "key1".to_string(),
            timestamp: 1001,
        },
    ];
    let incr_id = manager.create_incremental_checkpoint(changes, None).unwrap();

    // Restore from incremental
    let restored = manager.restore(&incr_id).unwrap();

    assert_eq!(restored.len(), 1);
    assert_eq!(restored.get("key1"), None); // Deleted
    assert_eq!(restored.get("key2"), Some(&b"value2".to_vec()));
}

#[test]
fn test_checkpoint_chain_restore() {
    let (mut manager, _temp_dir) = create_test_manager();

    // Create full checkpoint
    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("a".to_string(), b"1".to_vec());
    let _ = manager.create_full_checkpoint(&state, None).unwrap();

    // First incremental
    let changes1 = vec![
        CheckpointEntry::Put {
            key: "b".to_string(),
            value: b"2".to_vec(),
            timestamp: 1000,
        },
    ];
    let _ = manager.create_incremental_checkpoint(changes1, None).unwrap();

    // Second incremental
    let changes2 = vec![
        CheckpointEntry::Put {
            key: "c".to_string(),
            value: b"3".to_vec(),
            timestamp: 2000,
        },
        CheckpointEntry::Delete {
            key: "a".to_string(),
            timestamp: 2001,
        },
    ];
    let incr2_id = manager.create_incremental_checkpoint(changes2, None).unwrap();

    // Restore from latest incremental
    let restored = manager.restore(&incr2_id).unwrap();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored.get("a"), None);
    assert_eq!(restored.get("b"), Some(&b"2".to_vec()));
    assert_eq!(restored.get("c"), Some(&b"3".to_vec()));
}

#[test]
fn test_checkpoint_persistence() {
    let temp_dir = TempDir::new().unwrap();

    // Create manager and checkpoints
    {
        let mut manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();

        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        let _ = manager.create_full_checkpoint(&state, None).unwrap();

        let changes = vec![
            CheckpointEntry::Put {
                key: "key2".to_string(),
                value: b"value2".to_vec(),
                timestamp: 1000,
            },
        ];
        let _ = manager.create_incremental_checkpoint(changes, None).unwrap();
    }

    // Create new manager (should load existing checkpoints)
    let manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();

    assert_eq!(manager.list_checkpoints().len(), 2);
    assert_eq!(manager.get_chain().checkpoint_ids.len(), 2);
}

#[test]
fn test_checkpoint_stats() {
    let (mut manager, _temp_dir) = create_test_manager();

    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    let _ = manager.create_full_checkpoint(&state, None).unwrap();

    let changes = vec![
        CheckpointEntry::Put {
            key: "key2".to_string(),
            value: b"value2".to_vec(),
            timestamp: 1000,
        },
    ];
    let _ = manager.create_incremental_checkpoint(changes, None).unwrap();

    let stats = manager.get_stats();

    assert_eq!(stats.total_checkpoints, 2);
    assert_eq!(stats.full_checkpoints, 1);
    assert_eq!(stats.incremental_checkpoints, 1);
    assert!(stats.total_size_bytes > 0);
    assert_eq!(stats.total_entries, 2);
}

#[test]
fn test_checkpoint_compaction() {
    let (mut manager, _temp_dir) = create_test_manager();

    // Create one full checkpoint
    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key0".to_string(), b"value0".to_vec());
    let _ = manager.create_full_checkpoint(&state, None).unwrap();

    // Create incremental checkpoints
    for i in 1..6 {
        let changes = vec![
            CheckpointEntry::Put {
                key: format!("key{}", i),
                value: format!("value{}", i).into_bytes(),
                timestamp: i as u64 * 1000,
            },
        ];
        let _ = manager.create_incremental_checkpoint(changes, None).unwrap();
    }

    assert_eq!(manager.list_checkpoints().len(), 6);

    // Compact, keeping last 3
    let deleted = manager.compact(3).unwrap();

    assert!(deleted >= 2);
    assert!(manager.list_checkpoints().len() <= 4); // At least the full checkpoint is preserved
}

#[test]
fn test_checkpoint_integrity() {
    let (mut manager, _temp_dir) = create_test_manager();

    let mut state: HashMap<String, Vec<u8>> = HashMap::new();
    state.insert("key1".to_string(), b"value1".to_vec());
    let checkpoint_id = manager.create_full_checkpoint(&state, None).unwrap();

    let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
    assert!(checkpoint.content_hash.starts_with("0x"));
    assert!(checkpoint.content_hash.len() > 10);
}
