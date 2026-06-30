use std::path::{Path, PathBuf};

use eng_compiler::canonical_path_text;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtifactRecord {
    pub kind: String,
    pub class: String,
    pub path: String,
    pub hash: String,
    pub status: String,
    pub validation: ArtifactValidation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceRecord {
    pub kind: String,
    pub binding: String,
    pub path: String,
    pub hash: Option<String>,
    pub schema: Option<String>,
    pub row_count: Option<usize>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExternalBoundaryRecord {
    pub kind: String,
    pub binding: String,
    pub command: String,
    pub target: String,
    pub tool_version: Option<String>,
    pub args: Vec<String>,
    pub cwd: String,
    pub output_paths: Vec<String>,
    pub expected_output_count: usize,
    pub expected_output_status: String,
    pub response_hash: Option<String>,
    pub expected_hash: Option<String>,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub success: bool,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelArtifactRecord {
    pub artifact: ArtifactRecord,
    pub binding: String,
    pub kind: String,
    pub source: Option<String>,
    pub target: Option<String>,
    pub target_quantity: Option<String>,
    pub target_unit: String,
    pub training_data_hash: Option<String>,
    pub model_artifact_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OutputArtifact {
    pub kind: String,
    pub path: String,
    pub hash: String,
    pub absolute_path: PathBuf,
    pub validation: ArtifactValidation,
}

impl OutputArtifact {
    pub(crate) fn new(
        kind: String,
        path: String,
        hash: String,
        absolute_path: PathBuf,
        validation: ArtifactValidation,
    ) -> Self {
        Self {
            kind,
            path,
            hash,
            absolute_path,
            validation,
        }
    }
}

pub(crate) struct OutputManifest<'a> {
    pub runtime_version: &'a str,
    pub source_path: &'a Path,
    pub working_dir: &'a Path,
    pub output_dir: &'a Path,
    pub execution_profile: &'a str,
    pub artifacts: &'a [ArtifactRecord],
    pub artifact_registry_json: String,
    pub profile_diagnostics_json: String,
}

impl OutputManifest<'_> {
    pub(crate) fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str("  \"format\": \"eng-output-manifest-v1\",\n");
        json.push_str(&format!(
            "  \"runtime_version\": \"{}\",\n",
            json_escape(self.runtime_version)
        ));
        json.push_str(&format!(
            "  \"source_path\": \"{}\",\n",
            json_escape(&path_for_manifest(self.source_path))
        ));
        json.push_str(&format!(
            "  \"working_dir\": \"{}\",\n",
            json_escape(&path_for_manifest(self.working_dir))
        ));
        json.push_str(&format!(
            "  \"output_dir\": \"{}\",\n",
            json_escape(&path_for_manifest(self.output_dir))
        ));
        json.push_str(&format!(
            "  \"execution_profile\": \"{}\",\n",
            json_escape(self.execution_profile)
        ));
        json.push_str(&format!(
            "  \"artifact_count\": {},\n",
            self.artifacts.len()
        ));
        json.push_str("  \"artifacts\": [\n");
        push_output_artifacts_json(&mut json, self.artifacts);
        json.push_str("\n  ],\n");
        json.push_str("  \"artifact_registry\": {\n");
        json.push_str(&self.artifact_registry_json);
        json.push_str("\n  },\n");
        json.push_str("  \"profile_diagnostics\": [\n");
        json.push_str(&self.profile_diagnostics_json);
        json.push_str("\n  ]\n");
        json.push_str("}\n");
        json
    }
}

fn path_for_manifest(path: &Path) -> String {
    canonical_path_text(&path.display().to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtifactValidation {
    pub status: String,
    pub rule: String,
    pub message: String,
}

impl ArtifactValidation {
    pub(crate) fn new(status: &str, rule: &str, message: &str) -> Self {
        Self {
            status: status.to_owned(),
            rule: rule.to_owned(),
            message: message.to_owned(),
        }
    }
}

fn push_output_artifacts_json(json: &mut String, artifacts: &[ArtifactRecord]) {
    for (index, artifact) in artifacts.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"kind\": \"{}\",\n",
            json_escape(&artifact.kind)
        ));
        json.push_str(&format!(
            "      \"class\": \"{}\",\n",
            json_escape(&artifact.class)
        ));
        json.push_str(&format!(
            "      \"path\": \"{}\",\n",
            json_escape(&artifact.path)
        ));
        json.push_str(&format!(
            "      \"hash\": \"{}\",\n",
            json_escape(&artifact.hash)
        ));
        json.push_str(&format!(
            "      \"status\": \"{}\",\n",
            json_escape(&artifact.status)
        ));
        push_artifact_validation_json(json, &artifact.validation, 6);
        json.push_str("    }");
    }
}

fn push_artifact_validation_json(
    json: &mut String,
    validation: &ArtifactValidation,
    indent: usize,
) {
    let padding = " ".repeat(indent);
    json.push_str(&format!("{padding}\"validation\": {{\n"));
    json.push_str(&format!(
        "{padding}  \"status\": \"{}\",\n",
        json_escape(&validation.status)
    ));
    json.push_str(&format!(
        "{padding}  \"rule\": \"{}\",\n",
        json_escape(&validation.rule)
    ));
    json.push_str(&format!(
        "{padding}  \"message\": \"{}\"\n",
        json_escape(&validation.message)
    ));
    json.push_str(&format!("{padding}}}\n"));
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_record_preserves_validation_contract() {
        let record = ArtifactRecord {
            kind: "standard_file".to_owned(),
            class: "generated_file".to_owned(),
            path: "outputs/weather.txt".to_owned(),
            hash: "abc123".to_owned(),
            status: "generated".to_owned(),
            validation: ArtifactValidation::new("passed", "content_hash", "hashed"),
        };

        assert_eq!(record.kind, "standard_file");
        assert_eq!(record.validation.status, "passed");
    }

    #[test]
    fn external_boundary_record_keeps_process_review_fields() {
        let record = ExternalBoundaryRecord {
            kind: "process".to_owned(),
            binding: "run_case".to_owned(),
            command: "sim".to_owned(),
            target: "sim".to_owned(),
            tool_version: Some("sim 1.0".to_owned()),
            args: vec!["--input".to_owned(), "case.in".to_owned()],
            cwd: "outputs/case_001".to_owned(),
            output_paths: vec!["case.out".to_owned(), "case.log".to_owned()],
            expected_output_count: 2,
            expected_output_status: "satisfied".to_owned(),
            response_hash: None,
            expected_hash: None,
            stdout_hash: "out".to_owned(),
            stderr_hash: "err".to_owned(),
            success: true,
            status: "process-ok".to_owned(),
            line: 12,
        };

        assert!(record.success);
        assert_eq!(record.expected_output_count, 2);
    }

    #[test]
    fn output_manifest_writer_preserves_artifact_contract() {
        let artifacts = vec![ArtifactRecord {
            kind: "result".to_owned(),
            class: "review_artifact".to_owned(),
            path: "result.engres".to_owned(),
            hash: "hash123".to_owned(),
            status: "generated".to_owned(),
            validation: ArtifactValidation::new("passed", "content_hash", "hashed"),
        }];
        let manifest = OutputManifest {
            runtime_version: "0.1.0",
            source_path: Path::new("main.eng"),
            working_dir: Path::new("."),
            output_dir: Path::new("build/result"),
            execution_profile: "normal",
            artifacts: &artifacts,
            artifact_registry_json: "    \"format\": \"eng-artifact-registry-v1\"".to_owned(),
            profile_diagnostics_json: String::new(),
        }
        .to_json();

        assert!(manifest.contains("\"format\": \"eng-output-manifest-v1\""));
        assert!(manifest.contains("\"source_path\": \"main.eng\""));
        assert!(manifest.contains("\"working_dir\": \".\""));
        assert!(manifest.contains("\"output_dir\": \"build/result\""));
        assert!(manifest.contains("\"artifact_count\": 1"));
        assert!(manifest.contains("\"kind\": \"result\""));
        assert!(manifest.contains("\"validation\""));
        assert!(manifest.contains("\"artifact_registry\""));
    }
}
