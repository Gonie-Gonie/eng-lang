use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde_json::{json, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewDocumentError {
    ExpectedObject,
    MissingSemanticHash,
    MissingSectionHashes,
}

impl fmt::Display for ReviewDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedObject => formatter
                .write_str("expected a ReviewDocument JSON object or a review.json wrapper"),
            Self::MissingSemanticHash => {
                formatter.write_str("ReviewDocument is missing a non-empty string `semantic_hash`")
            }
            Self::MissingSectionHashes => {
                formatter.write_str("ReviewDocument is missing an object `section_hashes`")
            }
        }
    }
}

impl Error for ReviewDocumentError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewSemanticDiffError {
    InvalidPrevious(ReviewDocumentError),
    InvalidCurrent(ReviewDocumentError),
}

impl fmt::Display for ReviewSemanticDiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrevious(error) => write!(formatter, "invalid previous review: {error}"),
            Self::InvalidCurrent(error) => write!(formatter, "invalid current review: {error}"),
        }
    }
}

impl Error for ReviewSemanticDiffError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPrevious(error) | Self::InvalidCurrent(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewDocumentRefreshError {
    InvalidBaseline(ReviewDocumentError),
    InvalidRuntime(ReviewDocumentError),
}

impl fmt::Display for ReviewDocumentRefreshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseline(error) => write!(formatter, "invalid baseline review: {error}"),
            Self::InvalidRuntime(error) => write!(formatter, "invalid runtime review: {error}"),
        }
    }
}

impl Error for ReviewDocumentRefreshError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidBaseline(error) | Self::InvalidRuntime(error) => Some(error),
        }
    }
}

pub fn extract_review_document(value: &Value) -> Result<&Value, ReviewDocumentError> {
    let document = value.get("review_document").unwrap_or(value);
    let Some(object) = document.as_object() else {
        return Err(ReviewDocumentError::ExpectedObject);
    };
    if object
        .get("semantic_hash")
        .and_then(Value::as_str)
        .is_none_or(|hash| hash.trim().is_empty())
    {
        return Err(ReviewDocumentError::MissingSemanticHash);
    }
    if !object.get("section_hashes").is_some_and(Value::is_object) {
        return Err(ReviewDocumentError::MissingSectionHashes);
    }
    Ok(document)
}

pub fn review_semantic_diff(
    previous_input: &Value,
    current_input: &Value,
) -> Result<Value, ReviewSemanticDiffError> {
    let previous = extract_review_document(previous_input)
        .map_err(ReviewSemanticDiffError::InvalidPrevious)?;
    let current =
        extract_review_document(current_input).map_err(ReviewSemanticDiffError::InvalidCurrent)?;
    let previous_hash = json_string(previous, "semantic_hash");
    let current_hash = json_string(current, "semantic_hash");
    let changed_sections = review_changed_sections(previous, current);
    let section_changes = review_section_changes(previous, current, &changed_sections);
    let status = if previous_hash == current_hash && changed_sections.is_empty() {
        "unchanged"
    } else {
        "changed"
    };
    Ok(json!({
        "format": "eng-review-semantic-diff-preview-1",
        "status": status,
        "semantic_hash_before": previous_hash,
        "semantic_hash_after": current_hash,
        "changed_sections": changed_sections,
        "section_changes": section_changes
    }))
}

pub fn refresh_runtime_review_document_hashes(
    baseline_input: &Value,
    runtime_input: &mut Value,
) -> Result<Vec<String>, ReviewDocumentRefreshError> {
    let baseline = extract_review_document(baseline_input)
        .map_err(ReviewDocumentRefreshError::InvalidBaseline)?;
    let runtime = extract_review_document(runtime_input)
        .map_err(ReviewDocumentRefreshError::InvalidRuntime)?;
    let baseline_hashes = baseline
        .get("section_hashes")
        .and_then(Value::as_object)
        .expect("validated ReviewDocument section hashes");
    let runtime_hashes = runtime
        .get("section_hashes")
        .and_then(Value::as_object)
        .expect("validated ReviewDocument section hashes");
    let section_names = baseline_hashes
        .keys()
        .cloned()
        .chain(runtime_hashes.keys().cloned())
        .collect::<BTreeSet<_>>();
    let changed_sections = section_names
        .iter()
        .filter(|section| baseline.get(*section) != runtime.get(*section))
        .cloned()
        .collect::<Vec<_>>();
    if changed_sections.is_empty() {
        return Ok(changed_sections);
    }

    let mut refreshed_hashes = runtime_hashes.clone();
    for section in &section_names {
        if changed_sections.binary_search(section).is_ok() {
            refreshed_hashes.insert(
                section.clone(),
                Value::String(hash_json_value(runtime.get(section))),
            );
        } else if let Some(hash) = baseline_hashes.get(section) {
            refreshed_hashes.insert(section.clone(), hash.clone());
        }
    }
    let semantic_hash = runtime_review_semantic_hash(runtime, &refreshed_hashes);
    let runtime = extract_review_document_mut(runtime_input)
        .expect("runtime ReviewDocument was validated before mutation");
    let object = runtime
        .as_object_mut()
        .expect("validated ReviewDocument must be an object");
    object.insert("section_hashes".to_owned(), Value::Object(refreshed_hashes));
    object.insert("semantic_hash".to_owned(), Value::String(semantic_hash));
    object.insert(
        "semantic_hash_scope".to_owned(),
        Value::String("runtime_enriched".to_owned()),
    );
    Ok(changed_sections)
}

fn extract_review_document_mut(value: &mut Value) -> Result<&mut Value, ReviewDocumentError> {
    extract_review_document(value)?;
    if value.get("review_document").is_some() {
        Ok(value
            .get_mut("review_document")
            .expect("review_document key was present during validation"))
    } else {
        Ok(value)
    }
}

fn hash_json_value(value: Option<&Value>) -> String {
    let serialized = serde_json::to_string(value.unwrap_or(&Value::Null))
        .expect("serde_json::Value must serialize");
    hash_review_text(&serialized)
}

fn runtime_review_semantic_hash(
    document: &Value,
    section_hashes: &serde_json::Map<String, Value>,
) -> String {
    let mut digest = String::from("eng-runtime-review-document-v1");
    digest.push('|');
    digest.push_str(json_string(document, "workflow_signature").unwrap_or(""));
    for (section, hash) in section_hashes {
        digest.push('|');
        digest.push_str(section);
        digest.push('=');
        digest.push_str(hash.as_str().unwrap_or(""));
    }
    hash_review_text(&digest)
}

fn hash_review_text(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn review_section_changes(
    previous: &Value,
    current: &Value,
    changed_sections: &[Value],
) -> Vec<Value> {
    changed_sections
        .iter()
        .filter_map(|row| json_string(row, "section"))
        .filter_map(|section| review_array_section_change(section, previous, current))
        .collect()
}

fn review_array_section_change(section: &str, previous: &Value, current: &Value) -> Option<Value> {
    let previous_items = previous
        .get(section)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let current_items = current
        .get(section)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if previous_items.is_empty() && current_items.is_empty() {
        return None;
    }

    let previous_map = review_item_map(section, previous_items);
    let current_map = review_item_map(section, current_items);
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, current_item) in &current_map {
        match previous_map.get(key) {
            None => added.push(review_diff_item(key, current_item)),
            Some(previous_item) if *previous_item != *current_item => {
                changed.push(json!({
                    "key": key,
                    "before": *previous_item,
                    "after": *current_item
                }));
            }
            _ => {}
        }
    }
    for (key, previous_item) in &previous_map {
        if !current_map.contains_key(key) {
            removed.push(review_diff_item(key, previous_item));
        }
    }

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return None;
    }
    Some(json!({
        "section": section,
        "added": added,
        "removed": removed,
        "changed": changed
    }))
}

fn review_item_map<'a>(section: &str, items: &'a [Value]) -> BTreeMap<String, &'a Value> {
    let mut map = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let key = review_item_key(section, item, index);
        map.insert(key, item);
    }
    map
}

fn review_diff_item(key: &str, item: &Value) -> Value {
    json!({
        "key": key,
        "item": item
    })
}

fn review_item_key(section: &str, item: &Value, index: usize) -> String {
    let kind = json_string(item, "kind").unwrap_or(section);
    for field in ["name", "binding", "target", "source"] {
        if let Some(value) = json_string(item, field) {
            return format!("{kind}:{field}:{value}");
        }
    }
    if let Some(line) = item.get("line").and_then(Value::as_u64) {
        return format!("{kind}:line:{line}");
    }
    if let Some(line) = item.get("source_line").and_then(Value::as_u64) {
        return format!("{kind}:source_line:{line}");
    }
    if let Some(expression) = json_string(item, "expression") {
        return format!("{kind}:expression:{expression}");
    }
    if let Some(category) = json_string(item, "category") {
        return format!("{kind}:category:{category}:{index}");
    }
    format!("{section}:{index}")
}

fn review_changed_sections(previous: &Value, current: &Value) -> Vec<Value> {
    let mut sections = Vec::new();
    let previous_hashes = previous.get("section_hashes").and_then(Value::as_object);
    let current_hashes = current.get("section_hashes").and_then(Value::as_object);
    let section_names = previous_hashes
        .into_iter()
        .flat_map(|hashes| hashes.keys().cloned())
        .chain(
            current_hashes
                .into_iter()
                .flat_map(|hashes| hashes.keys().cloned()),
        )
        .collect::<BTreeSet<_>>();
    for section in section_names {
        let previous_hash = previous_hashes.and_then(|hashes| hashes.get(&section));
        let current_hash = current_hashes.and_then(|hashes| hashes.get(&section));
        if previous_hash != current_hash {
            sections.push(json!({
                "section": section,
                "before": previous_hash.cloned().unwrap_or(Value::Null),
                "after": current_hash.cloned().unwrap_or(Value::Null)
            }));
        }
    }
    sections
}

fn json_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::{
        extract_review_document, refresh_runtime_review_document_hashes, review_semantic_diff,
        ReviewDocumentError,
    };
    use serde_json::json;

    #[test]
    fn extracts_bare_and_wrapped_review_documents() {
        let document = json!({
            "semantic_hash": "same",
            "section_hashes": {}
        });
        assert_eq!(extract_review_document(&document), Ok(&document));

        let wrapped = json!({ "review_document": document });
        assert_eq!(
            extract_review_document(&wrapped).expect("wrapped review"),
            &wrapped["review_document"]
        );
    }

    #[test]
    fn rejects_incomplete_review_documents() {
        assert_eq!(
            extract_review_document(&json!([])),
            Err(ReviewDocumentError::ExpectedObject)
        );
        assert_eq!(
            extract_review_document(&json!({ "section_hashes": {} })),
            Err(ReviewDocumentError::MissingSemanticHash)
        );
        assert_eq!(
            extract_review_document(&json!({ "semantic_hash": "hash" })),
            Err(ReviewDocumentError::MissingSectionHashes)
        );
    }

    #[test]
    fn semantic_diff_reports_changed_sections_and_items() {
        let previous = json!({
            "semantic_hash": "before",
            "section_hashes": {
                "inputs": "same",
                "calculations": "old"
            },
            "calculations": [{
                "kind": "binding",
                "name": "Q_total",
                "expression": "Q + 1 kW",
                "quantity_kind": "HeatRate",
                "line": 3
            }]
        });
        let current = json!({
            "semantic_hash": "after",
            "section_hashes": {
                "inputs": "same",
                "calculations": "new"
            },
            "calculations": [{
                "kind": "binding",
                "name": "Q_total",
                "expression": "Q + 2 kW",
                "quantity_kind": "HeatRate",
                "line": 3
            }]
        });

        let diff = review_semantic_diff(&previous, &current).expect("semantic diff");

        assert_eq!(diff["status"], "changed");
        assert_eq!(diff["changed_sections"][0]["section"], "calculations");
        assert_eq!(diff["changed_sections"][0]["before"], "old");
        assert_eq!(diff["changed_sections"][0]["after"], "new");
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["key"],
            "binding:name:Q_total"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["before"]["expression"],
            "Q + 1 kW"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["after"]["expression"],
            "Q + 2 kW"
        );
    }

    #[test]
    fn semantic_diff_compares_wrapped_workflow_modules() {
        let previous = json!({
            "review_document": {
                "semantic_hash": "before",
                "section_hashes": { "workflow_modules": "old" },
                "workflow_modules": [{
                    "kind": "native_module",
                    "name": "eng.net",
                    "status": "planned"
                }]
            }
        });
        let current = json!({
            "semantic_hash": "after",
            "section_hashes": { "workflow_modules": "new" },
            "workflow_modules": [{
                "kind": "native_module",
                "name": "eng.net",
                "status": "native_preview"
            }]
        });

        let diff = review_semantic_diff(&previous, &current).expect("semantic diff");

        assert_eq!(diff["changed_sections"][0]["section"], "workflow_modules");
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["before"]["status"],
            "planned"
        );
        assert_eq!(
            diff["section_changes"][0]["changed"][0]["after"]["status"],
            "native_preview"
        );
    }

    #[test]
    fn semantic_diff_reports_unchanged_document() {
        let previous = json!({
            "semantic_hash": "same",
            "section_hashes": { "inputs": "a" }
        });

        let diff = review_semantic_diff(&previous, &previous).expect("semantic diff");

        assert_eq!(diff["status"], "unchanged");
        assert_eq!(diff["changed_sections"].as_array().map(Vec::len), Some(0));
        assert_eq!(diff["section_changes"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn semantic_diff_reports_removed_sections_and_items() {
        let previous = json!({
            "semantic_hash": "before",
            "section_hashes": { "calculations": "old" },
            "calculations": [{
                "kind": "binding",
                "name": "Q_total",
                "expression": "Q + 1 kW"
            }]
        });
        let current = json!({
            "semantic_hash": "after",
            "section_hashes": {}
        });

        let diff = review_semantic_diff(&previous, &current).expect("semantic diff");

        assert_eq!(diff["changed_sections"][0]["section"], "calculations");
        assert!(diff["changed_sections"][0]["after"].is_null());
        assert_eq!(
            diff["section_changes"][0]["removed"][0]["key"],
            "binding:name:Q_total"
        );
    }

    #[test]
    fn runtime_hash_refresh_preserves_unchanged_static_hashes() {
        let baseline = json!({
            "review_document": {
                "workflow_signature": "workflow",
                "semantic_hash": "static-semantic",
                "section_hashes": {
                    "calculations": "static-calculations",
                    "schemas": "static-schemas"
                },
                "calculations": [{ "kind": "binding", "name": "Q" }],
                "schemas": [{ "name": "Input" }]
            }
        });
        let mut runtime = baseline.clone();

        let changed =
            refresh_runtime_review_document_hashes(&baseline, &mut runtime).expect("refresh");

        assert!(changed.is_empty());
        assert_eq!(
            runtime["review_document"]["semantic_hash"],
            "static-semantic"
        );
        assert!(runtime["review_document"]
            .get("semantic_hash_scope")
            .is_none());
    }

    #[test]
    fn runtime_hash_refresh_updates_only_changed_sections() {
        let baseline = json!({
            "review_document": {
                "workflow_signature": "workflow",
                "semantic_hash": "static-semantic",
                "section_hashes": {
                    "calculations": "static-calculations",
                    "schemas": "static-schemas"
                },
                "calculations": [{ "kind": "binding", "name": "Q" }],
                "schemas": [{ "name": "Input" }]
            }
        });
        let mut runtime = baseline.clone();
        runtime["review_document"]["calculations"][0]["runtime_value"] = json!(42.0);

        let changed =
            refresh_runtime_review_document_hashes(&baseline, &mut runtime).expect("refresh");
        let first_hash = runtime["review_document"]["semantic_hash"].clone();

        assert_eq!(changed, vec!["calculations"]);
        assert_ne!(
            runtime["review_document"]["section_hashes"]["calculations"],
            "static-calculations"
        );
        assert_eq!(
            runtime["review_document"]["section_hashes"]["schemas"],
            "static-schemas"
        );
        assert_ne!(first_hash, "static-semantic");
        assert_eq!(
            runtime["review_document"]["semantic_hash_scope"],
            "runtime_enriched"
        );

        let mut repeated = baseline.clone();
        repeated["review_document"]["calculations"][0]["runtime_value"] = json!(42.0);
        refresh_runtime_review_document_hashes(&baseline, &mut repeated).expect("repeat refresh");
        assert_eq!(repeated["review_document"]["semantic_hash"], first_hash);
    }
}
