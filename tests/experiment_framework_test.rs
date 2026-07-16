//! Experiment Framework Tests

use ai_assistant::experiments::ExperimentGroup;

#[test]
fn test_experiment_group_configuration() {
    // Test Control group
    let control = ExperimentGroup::Control;
    assert_eq!(control.log_dir_name(), "control");
    assert!(!control.has_evolution());

    // Test OursFull group
    let full = ExperimentGroup::OursFull;
    assert_eq!(full.log_dir_name(), "ours_full");
    assert!(full.has_evolution());
    assert!(full.has_multi_agent());
    assert!(full.has_cot());
    assert!(full.has_self_fix());

    // Test OursSingle group
    let single = ExperimentGroup::OursSingle;
    assert_eq!(single.log_dir_name(), "ours_single");
    assert!(single.has_evolution());
    assert!(!single.has_multi_agent());

    // Test OursNoCoT group
    let nocot = ExperimentGroup::OursNoCoT;
    assert_eq!(nocot.log_dir_name(), "ours_nocot");
    assert!(nocot.has_evolution());
    assert!(!nocot.has_cot());

    // Test OursNoFix group
    let nofix = ExperimentGroup::OursNoFix;
    assert_eq!(nofix.log_dir_name(), "ours_nofix");
    assert!(nofix.has_evolution());
    assert!(!nofix.has_self_fix());

    println!("Experiment group configuration test passed!");
}

#[test]
fn test_group_comparison() {
    // Test that different groups have different configurations
    let control = ExperimentGroup::Control;
    let full = ExperimentGroup::OursFull;

    // Control should not have evolution, Full should
    assert!(!control.has_evolution());
    assert!(full.has_evolution());

    // Verify log dir names are different
    assert_ne!(control.log_dir_name(), full.log_dir_name());

    println!("Group comparison test passed!");
}

#[test]
fn test_group_descriptions() {
    let control = ExperimentGroup::Control;
    let full = ExperimentGroup::OursFull;

    // Verify descriptions are not empty
    assert!(!control.description().is_empty());
    assert!(!full.description().is_empty());

    // Descriptions should be different
    assert_ne!(control.description(), full.description());

    println!("Group descriptions test passed!");
}
