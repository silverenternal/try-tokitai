use super::util::{ext, read_source, selected_path, source_for_path, stable_id, workspace_files};
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationEdge, VisualizationFrame,
    VisualizationNode, VisualizationSource, VisualizationTypeDescriptor,
};
use crate::visualization::{type_descriptor, VisualizationAdapter, VisualizationContext};
use anyhow::Result;
use lopdf::Document;
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

pub struct PaperAdapter;

impl VisualizationAdapter for PaperAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor(
            "paper",
            "Paper",
            "Paper structure, citations, method flow, and concepts parsed from manuscripts.",
            "atlas.paper.workspace",
        )
    }

    fn discover(&self, context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        Ok(workspace_files(
            context.workspace_root,
            |path| {
                let name = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                matches!(ext(path).as_str(), "pdf" | "tex" | "bib")
                    || (matches!(ext(path).as_str(), "md" | "txt" | "json")
                        && [
                            "paper",
                            "manuscript",
                            "article",
                            "literature",
                            "reference",
                            "research",
                        ]
                        .iter()
                        .any(|needle| name.contains(needle)))
            },
            100,
        )
        .iter()
        .map(|path| {
            source_for_path(
                context.workspace_root,
                path,
                "paper",
                paper_source_type(path),
            )
        })
        .collect())
    }

    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        let path = selected_path(context.workspace_root, context.source_id)?;
        let source = source_for_path(
            context.workspace_root,
            &path,
            "paper",
            paper_source_type(&path),
        );
        let mut document = VisualizationDocument::empty(
            "paper",
            path.file_name().and_then(|v| v.to_str()).unwrap_or("Paper"),
            source,
        );
        let raw = if ext(&path) == "pdf" {
            extract_pdf(&path)?
        } else {
            read_source(&path)?
        };
        parse_paper_text(&raw, ext(&path).as_str(), &mut document);
        if document.nodes.is_empty() {
            document.diagnostics.push(VisualizationDiagnostic {
                level: "info".to_string(),
                message: "No paper sections, citations, or method concepts were detected."
                    .to_string(),
                metadata: BTreeMap::new(),
            });
        }
        Ok(document)
    }
}

fn paper_source_type(path: &Path) -> &'static str {
    match ext(path).as_str() {
        "pdf" => "pdf",
        "tex" => "latex",
        "bib" => "bibliography",
        "json" => "paper-metadata",
        _ => "manuscript",
    }
}

fn extract_pdf(path: &Path) -> Result<String> {
    let document = Document::load(path)?;
    let mut text = String::new();
    for page in document.get_pages().keys().take(300) {
        if let Ok(page_text) = document.extract_text(&[*page]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }
    Ok(text)
}

fn parse_paper_text(raw: &str, format: &str, document: &mut VisualizationDocument) {
    if format == "bib" {
        parse_bibliography(raw, document);
        return;
    }
    let markdown_heading = Regex::new(r"(?m)^(#{1,6})\s+(.+?)\s*$").unwrap();
    let latex_heading =
        Regex::new(r"(?m)^\s*\\(section|subsection|subsubsection)\*?\{([^}]+)\}").unwrap();
    let numbered_heading =
        Regex::new(r"(?m)^\s*(\d+(?:\.\d+)*)[.)]?\s+([A-Z][^\n]{2,100})$").unwrap();
    let mut sections = Vec::<(String, usize, usize)>::new();
    for capture in markdown_heading.captures_iter(raw) {
        sections.push((
            capture[2].trim().to_string(),
            capture.get(0).unwrap().start(),
            capture[1].len(),
        ));
    }
    for capture in latex_heading.captures_iter(raw) {
        let depth = match &capture[1] {
            "section" => 1,
            "subsection" => 2,
            _ => 3,
        };
        sections.push((
            capture[2].trim().to_string(),
            capture.get(0).unwrap().start(),
            depth,
        ));
    }
    if sections.is_empty() {
        for capture in numbered_heading.captures_iter(raw) {
            sections.push((
                capture[2].trim().to_string(),
                capture.get(0).unwrap().start(),
                capture[1].matches('.').count() + 1,
            ));
        }
    }
    sections.sort_by_key(|(_, offset, _)| *offset);
    sections.dedup_by_key(|(_, offset, _)| *offset);
    let mut parent_for_depth = HashMap::<usize, String>::new();
    for (index, (title, offset, depth)) in sections.iter().enumerate().take(250) {
        let id = format!("section:{index}");
        let end = sections
            .get(index + 1)
            .map(|(_, next, _)| *next)
            .unwrap_or(raw.len());
        let body = &raw[*offset..end.min(raw.len())];
        let mut node = VisualizationNode::new(&id, title, "section");
        node.parent_id = (1..*depth)
            .rev()
            .find_map(|level| parent_for_depth.get(&level).cloned());
        node.metrics
            .insert("characters".to_string(), body.chars().count() as f64);
        node.metadata
            .insert("depth".to_string(), Value::from(*depth));
        node.metadata
            .insert("summary".to_string(), Value::String(first_sentence(body)));
        document.nodes.push(node);
        parent_for_depth.insert(*depth, id.clone());
        parent_for_depth.retain(|level, _| *level <= *depth);
        if index > 0 {
            document.edges.push(VisualizationEdge::new(
                format!("section-flow:{index}"),
                format!("section:{}", index - 1),
                &id,
                "next",
                "structure",
            ));
        }
    }
    parse_citations(raw, &sections, document);
    parse_method_flow(raw, &sections, document);
    parse_concept_graph(raw, &sections, document);
    document.frames = document
        .nodes
        .iter()
        .filter(|node| node.category == "section")
        .enumerate()
        .map(|(sequence, node)| VisualizationFrame {
            id: format!("paper-frame:{sequence}"),
            sequence,
            label: node.label.clone(),
            active_nodes: vec![node.id.clone()],
            active_edges: document
                .edges
                .iter()
                .filter(|edge| edge.source == node.id || edge.target == node.id)
                .map(|edge| edge.id.clone())
                .collect(),
            metrics: BTreeMap::new(),
        })
        .collect();
}

fn parse_concept_graph(
    raw: &str,
    sections: &[(String, usize, usize)],
    document: &mut VisualizationDocument,
) {
    let word = Regex::new(r"(?u)\b[\p{L}][\p{L}\p{N}_-]{3,}\b").unwrap();
    let stop_words: HashSet<&str> = [
        "about", "after", "again", "also", "among", "because", "before", "being", "between",
        "both", "could", "during", "each", "from", "further", "have", "into", "more", "most",
        "other", "over", "paper", "results", "section", "should", "such", "than", "that", "their",
        "these", "they", "this", "those", "through", "under", "using", "very", "were", "which",
        "while", "with", "would",
    ]
    .into_iter()
    .collect();
    let mut section_terms = Vec::<Vec<String>>::new();
    let mut global_counts = HashMap::<String, usize>::new();
    for (index, (_, start, _)) in sections.iter().enumerate() {
        let end = sections
            .get(index + 1)
            .map(|(_, next, _)| *next)
            .unwrap_or(raw.len());
        let body = &raw[*start..end.min(raw.len())];
        let mut counts = HashMap::<String, usize>::new();
        for value in word.find_iter(body).take(20_000) {
            let term = value.as_str().to_lowercase();
            if stop_words.contains(term.as_str()) || term.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            *counts.entry(term.clone()).or_default() += 1;
            *global_counts.entry(term).or_default() += 1;
        }
        let mut terms = counts.into_iter().collect::<Vec<_>>();
        terms.sort_by_key(|(term, count)| {
            std::cmp::Reverse((*count, global_counts.get(term).copied().unwrap_or_default()))
        });
        section_terms.push(terms.into_iter().take(8).map(|(term, _)| term).collect());
    }
    let mut ranked = global_counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let selected = ranked
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .take(60)
        .map(|(term, count)| (term, count))
        .collect::<HashMap<_, _>>();
    for (term, count) in &selected {
        let mut node = VisualizationNode::new(stable_id("concept", term), term, "concept");
        node.metrics.insert("mentions".to_string(), *count as f64);
        document.nodes.push(node);
    }
    let mut cooccurrence = HashMap::<(String, String), usize>::new();
    let mut edge_index = 0usize;
    for (section_index, terms) in section_terms.iter().enumerate() {
        let terms = terms
            .iter()
            .filter(|term| selected.contains_key(term.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for term in &terms {
            document.edges.push(VisualizationEdge::new(
                format!("section-concept:{edge_index}"),
                format!("section:{section_index}"),
                stable_id("concept", term),
                "mentions",
                "knowledge",
            ));
            edge_index += 1;
        }
        for left in 0..terms.len() {
            for right in left + 1..terms.len() {
                let pair = if terms[left] <= terms[right] {
                    (terms[left].clone(), terms[right].clone())
                } else {
                    (terms[right].clone(), terms[left].clone())
                };
                *cooccurrence.entry(pair).or_default() += 1;
            }
        }
    }
    for ((source, target), weight) in cooccurrence {
        if weight < 2 {
            continue;
        }
        let mut edge = VisualizationEdge::new(
            format!("concept-relation:{edge_index}"),
            stable_id("concept", &source),
            stable_id("concept", &target),
            "co-occurs",
            "knowledge",
        );
        edge.weight = weight as f64;
        document.edges.push(edge);
        edge_index += 1;
    }
}

fn parse_bibliography(raw: &str, document: &mut VisualizationDocument) {
    let entry = Regex::new(r"(?ims)@(\w+)\s*\{\s*([^,]+),(.*?)(?=\n@|\z)").unwrap();
    let title = Regex::new(r#"(?i)title\s*=\s*[\{\"]([^}\"]+)"#).unwrap();
    for (index, capture) in entry.captures_iter(raw).take(500).enumerate() {
        let key = capture[2].trim();
        let body = capture
            .get(3)
            .map(|value| value.as_str())
            .unwrap_or_default();
        let label = title
            .captures(body)
            .and_then(|value| value.get(1))
            .map(|value| value.as_str().trim())
            .unwrap_or(key);
        let mut node = VisualizationNode::new(stable_id("citation", key), label, "citation");
        node.metadata
            .insert("key".to_string(), Value::String(key.to_string()));
        node.metadata.insert(
            "entry_type".to_string(),
            Value::String(capture[1].to_string()),
        );
        node.metadata
            .insert("sequence".to_string(), Value::from(index));
        document.nodes.push(node);
    }
}

fn parse_citations(
    raw: &str,
    sections: &[(String, usize, usize)],
    document: &mut VisualizationDocument,
) {
    let bracket = Regex::new(r"\[((?:\d+[a-z]?(?:\s*[-,;]\s*)?)+)\]").unwrap();
    let latex = Regex::new(r"\\cite\w*\{([^}]+)\}").unwrap();
    let mut keys = HashSet::<String>::new();
    let mut occurrences = Vec::<(usize, String)>::new();
    for capture in bracket.captures_iter(raw) {
        for value in Regex::new(r"\d+[a-z]?").unwrap().find_iter(&capture[1]) {
            keys.insert(value.as_str().to_string());
            occurrences.push((capture.get(0).unwrap().start(), value.as_str().to_string()));
        }
    }
    for capture in latex.captures_iter(raw) {
        for key in capture[1]
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            keys.insert(key.to_string());
            occurrences.push((capture.get(0).unwrap().start(), key.to_string()));
        }
    }
    for key in &keys {
        document.nodes.push(VisualizationNode::new(
            stable_id("citation", key),
            format!("[{key}]"),
            "citation",
        ));
    }
    for (index, (offset, key)) in occurrences.into_iter().enumerate() {
        let section_index = sections
            .iter()
            .enumerate()
            .take_while(|(_, (_, start, _))| *start <= offset)
            .map(|(index, _)| index)
            .last();
        if let Some(section_index) = section_index {
            document.edges.push(VisualizationEdge::new(
                format!("citation-edge:{index}"),
                format!("section:{section_index}"),
                stable_id("citation", &key),
                "cites",
                "citation",
            ));
        }
    }
}

fn parse_method_flow(
    raw: &str,
    sections: &[(String, usize, usize)],
    document: &mut VisualizationDocument,
) {
    let transition =
        Regex::new(r"(?i)\b(first|then|next|subsequently|finally|input|output|stage|step|phase)\b")
            .unwrap();
    let method_section = sections.iter().enumerate().find(|(_, (title, _, _))| {
        let title = title.to_ascii_lowercase();
        [
            "method",
            "methodology",
            "approach",
            "architecture",
            "framework",
            "pipeline",
        ]
        .iter()
        .any(|needle| title.contains(needle))
    });
    let Some((section_index, (_, start, _))) = method_section else {
        return;
    };
    let end = sections
        .get(section_index + 1)
        .map(|(_, start, _)| *start)
        .unwrap_or(raw.len());
    let body = &raw[*start..end.min(raw.len())];
    let sentences = body
        .split(['.', ';', '\n'])
        .map(str::trim)
        .filter(|sentence| sentence.len() > 20 && transition.is_match(sentence))
        .take(40)
        .collect::<Vec<_>>();
    let mut previous = format!("section:{section_index}");
    for (index, sentence) in sentences.into_iter().enumerate() {
        let id = format!("method-step:{index}");
        let mut node = VisualizationNode::new(
            &id,
            sentence.chars().take(90).collect::<String>(),
            "method-step",
        );
        node.parent_id = Some(format!("section:{section_index}"));
        document.nodes.push(node);
        document.edges.push(VisualizationEdge::new(
            format!("method-flow:{index}"),
            previous,
            &id,
            "next",
            "method-flow",
        ));
        previous = id;
    }
}

fn first_sentence(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("\\section"))
        .collect::<Vec<_>>()
        .join(" ")
        .split(['.', '。'])
        .next()
        .unwrap_or_default()
        .chars()
        .take(240)
        .collect()
}
