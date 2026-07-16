use ai_assistant::scientist::tools::literature::LiteratureTools;
use serde_json::json;

#[test]
fn remote_batch_fetch_returns_three_structured_documents() {
    let tool = LiteratureTools;

    let search = tool
        .call_tool(
            "search_paper",
            &json!({
                "query": "transformer language model",
                "source": "arxiv",
                "limit": 3
            }),
        )
        .expect("remote search should succeed");

    assert_eq!(search["status"], "success");
    assert_eq!(search["mode"], "remote");
    assert!(
        search["results"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0)
            >= 3
    );

    let ids = search["results"]
        .as_array()
        .unwrap()
        .iter()
        .take(3)
        .filter_map(|item| item["paper_id"].as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    let fetched = tool
        .call_tool(
            "fetch_papers",
            &json!({
                "paper_ids": ids,
                "limit": 3
            }),
        )
        .expect("remote batch fetch should succeed");

    println!(
        "batch_fetch fetched_count={} titles={:?}",
        fetched["fetched_count"],
        fetched["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|item| item["paper"]["title"].as_str())
            .collect::<Vec<_>>()
    );
    println!("batch_fulltext bundle={}", fetched["fulltext_bundle"]);

    assert_eq!(fetched["status"], "success");
    assert_eq!(fetched["fetched_count"], 3);
    assert_eq!(fetched["limit_applied"], 3);
    assert_eq!(fetched["results"].as_array().unwrap().len(), 3);
    assert_eq!(fetched["fulltext_bundle"]["requested_documents"], 3);
    for item in fetched["results"].as_array().unwrap() {
        assert_eq!(
            item["structured_document"]["provenance"]["primary_source"],
            "remote"
        );
        assert_eq!(item["fulltext"]["primary_source"], "remote");
        assert!(
            item["structured_document"]["body_text"]
                .as_str()
                .unwrap_or("")
                .len()
                > 100
        );
    }
}
