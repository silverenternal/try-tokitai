use ai_assistant::scientist::tools::literature::LiteratureTools;
use serde_json::json;

#[test]
fn remote_fetch_returns_structured_document_from_primary_source() {
    let tool = LiteratureTools;

    let search = tool
        .call_tool(
            "search_paper",
            &json!({
                "query": "transformer compression survey",
                "source": "auto",
                "limit": 5
            }),
        )
        .expect("remote search should succeed");

    assert_eq!(search["status"], "success");
    assert_eq!(search["mode"], "remote");
    assert!(search["total"].as_u64().unwrap_or(0) >= 1);
    let mut fetched = None;
    for item in search["results"].as_array().unwrap_or(&Vec::new()) {
        let Some(paper_id) = item["paper_id"].as_str() else {
            continue;
        };
        if let Ok(payload) = tool.call_tool(
            "fetch_paper",
            &json!({
                "paper_id": paper_id
            }),
        ) {
            fetched = Some(payload);
            break;
        }
    }
    let fetched = fetched.expect("at least one remote paper should fetch successfully");

    println!(
        "remote_paper title={:?} paper_id={:?} provider={:?}",
        fetched["paper"]["title"], fetched["paper"]["paper_id"], fetched["provider"]
    );
    println!(
        "structured_document body_len={} section_count={} reference_count={} content_source={:?} completeness={:?} extraction_path={:?}",
        fetched["structured_document"]["body_text"]
            .as_str()
            .map(|text| text.len())
            .unwrap_or(0),
        fetched["structured_document"]["sections"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0),
        fetched["structured_document"]["references"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or(0),
        fetched["structured_document"]["provenance"]["content_source"],
        fetched["structured_document"]["quality"]["completeness"],
        fetched["structured_document"]["quality"]["extraction_path"],
    );
    println!(
        "fulltext status={:?} provider={:?} body_text_chars={:?} attempted_pdf_url={:?}",
        fetched["fulltext"]["status"],
        fetched["fulltext"]["provider"],
        fetched["fulltext"]["body_text_chars"],
        fetched["fulltext"]["attempted_pdf_url"],
    );

    assert_eq!(fetched["status"], "success");
    assert_eq!(fetched["mode"], "remote");
    assert_eq!(
        fetched["structured_document"]["schema_version"],
        "structured_paper_document_v1"
    );
    assert_eq!(
        fetched["structured_document"]["provenance"]["primary_source"],
        "remote"
    );
    assert_eq!(fetched["fulltext"]["primary_source"], "remote");
    assert_eq!(
        fetched["fulltext"]["completeness"],
        fetched["structured_document"]["quality"]["completeness"]
    );
    assert!(fetched["structured_document"]["quality"]["completeness"].is_string());
    assert!(
        fetched["structured_document"]["body_text"]
            .as_str()
            .unwrap_or("")
            .len()
            > 200
    );
    assert!(fetched["structured_document"]["sections"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false));
}
