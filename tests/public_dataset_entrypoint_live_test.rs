use ai_assistant::scientist::tools::data::DataTools;

#[test]
fn public_dataset_search_returns_dataset_entrypoints_from_direct_providers() {
    let payload = DataTools
        .search_public_datasets("image classification".to_string(), Some(3))
        .expect("search_public_datasets should succeed against direct providers");

    assert_eq!(payload["status"], "success");
    assert_eq!(payload["provider"], "direct-official-dataset-databases");
    assert_eq!(
        payload["dataset_source_policy"],
        "direct_official_databases_only"
    );
    assert_eq!(payload["paper_source_policy"], "official_paper_apis_only");
    assert!(payload["total"].as_u64().unwrap_or(0) >= 1);

    let datasets = payload["datasets"]
        .as_array()
        .expect("datasets should be an array");
    assert!(
        !datasets.is_empty(),
        "expected at least one dataset entrypoint"
    );

    for dataset in datasets {
        let url = dataset["url"].as_str().unwrap_or("");
        let provider = dataset["provider"].as_str().unwrap_or("");
        assert!(
            url.contains("huggingface.co")
                || url.contains("openml.org")
                || url.contains("paperswithcode.com")
                || url.contains("kaggle.com"),
            "unexpected dataset URL: {}",
            url
        );
        assert_ne!(provider, "web", "dataset provider should be classified");
        assert!(
            !url.contains("arxiv.org"),
            "paper result should not leak into dataset entrypoints: {}",
            url
        );
    }
}
