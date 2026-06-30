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
    pub binding: String,
    pub command: String,
    pub tool_version: Option<String>,
    pub args: Vec<String>,
    pub cwd: String,
    pub expected_output_count: usize,
    pub expected_output_status: String,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub success: bool,
    pub status: String,
    pub line: usize,
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
            binding: "run_case".to_owned(),
            command: "sim".to_owned(),
            tool_version: Some("sim 1.0".to_owned()),
            args: vec!["--input".to_owned(), "case.in".to_owned()],
            cwd: "outputs/case_001".to_owned(),
            expected_output_count: 2,
            expected_output_status: "satisfied".to_owned(),
            stdout_hash: "out".to_owned(),
            stderr_hash: "err".to_owned(),
            success: true,
            status: "process-ok".to_owned(),
            line: 12,
        };

        assert!(record.success);
        assert_eq!(record.expected_output_count, 2);
    }
}
