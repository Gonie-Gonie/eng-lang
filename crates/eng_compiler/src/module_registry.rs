use std::fmt;
use std::fs;
use std::path::Path;

const BUNDLED_MODULE_REGISTRY: &str = include_str!("../../../stdlib/eng/modules.toml");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRegistry {
    pub modules: Vec<ModuleRegistryEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRegistryEntry {
    pub name: String,
    pub status: String,
    pub backing: String,
    pub purpose: String,
    pub artifacts: Vec<String>,
    pub diagnostics: Vec<String>,
    pub examples: Vec<String>,
    pub tests: Vec<String>,
    pub symbols: Vec<String>,
}

impl ModuleRegistryEntry {
    pub fn status_label(&self) -> &'static str {
        module_status_label(&self.status)
    }

    pub fn status_detail(&self) -> &'static str {
        module_status_detail(&self.status)
    }

    pub fn completion_detail(&self) -> String {
        format!(
            "{}: {}",
            module_completion_status_label(&self.status),
            module_completion_purpose(&self.purpose)
        )
    }
}

fn module_completion_status_label(status: &str) -> &'static str {
    match status {
        "supported" | "supported_narrow" => "Supported",
        "native_preview" => "Native",
        "planned" => "Planned",
        "internal_planned" => "Internal target",
        "internal" => "Internal",
        _ => "Unknown",
    }
}

fn module_completion_purpose(purpose: &str) -> String {
    let trimmed = purpose.trim();
    for marker in ["; broader ", "; broad "] {
        if let Some(index) = trimmed.find(marker) {
            if trimmed[index + marker.len()..].contains("planned") {
                return trimmed[..index].trim_end_matches('.').to_owned();
            }
        }
    }
    trimmed.to_owned()
}

pub fn module_status_label(status: &str) -> &'static str {
    match status {
        "supported" => "Supported",
        "supported_narrow" => "Supported narrow",
        "native_preview" => "Native workflow support",
        "planned" => "Planned",
        "internal_planned" => "Internal target",
        "internal" => "Internal",
        _ => "Unknown",
    }
}

pub fn module_status_detail(status: &str) -> &'static str {
    match status {
        "supported" => "Public built-in surface supported by compiler/runtime.",
        "supported_narrow" => "Supported for the listed syntax forms and review artifacts.",
        "native_preview" => {
            "Native runtime path is implemented for the listed workflow commands and artifacts; unsupported combinations report diagnostics."
        }
        "planned" => "Documented target module; not yet executable as a public stdlib API.",
        "internal_planned" => "Internal target, not a public stdlib API.",
        "internal" => "Internal compiler/runtime vocabulary outside the public stdlib API.",
        _ => "Unrecognized registry status.",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRegistryError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ModuleRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "module registry: {}", self.message)
        } else {
            write!(
                formatter,
                "module registry line {}: {}",
                self.line, self.message
            )
        }
    }
}

impl std::error::Error for ModuleRegistryError {}

pub fn bundled_module_registry() -> Result<ModuleRegistry, ModuleRegistryError> {
    parse_module_registry(BUNDLED_MODULE_REGISTRY)
}

pub fn load_module_registry(path: &Path) -> Result<ModuleRegistry, ModuleRegistryError> {
    let source = fs::read_to_string(path).map_err(|error| ModuleRegistryError {
        line: 0,
        message: format!("could not read {}: {error}", path.display()),
    })?;
    parse_module_registry(&source)
}

pub fn parse_module_registry(source: &str) -> Result<ModuleRegistry, ModuleRegistryError> {
    let mut modules = Vec::new();
    let mut current: Option<PartialModuleRegistryEntry> = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("[module.\"") {
            if let Some(entry) = current.take() {
                modules.push(entry.finish(line_number)?);
            }
            current = Some(PartialModuleRegistryEntry {
                name: parse_module_header(line, line_number)?,
                status: None,
                backing: None,
                purpose: None,
                artifacts: None,
                diagnostics: None,
                examples: None,
                tests: None,
                symbols: None,
            });
            continue;
        }

        let Some(entry) = current.as_mut() else {
            return Err(registry_error(
                line_number,
                "expected a [module.\"eng.name\"] section before key-value pairs",
            ));
        };
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| registry_error(line_number, "expected key = value"))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "status" => entry.status = Some(parse_quoted_string(value, line_number)?),
            "backing" => entry.backing = Some(parse_quoted_string(value, line_number)?),
            "purpose" => entry.purpose = Some(parse_quoted_string(value, line_number)?),
            "artifacts" => entry.artifacts = Some(parse_string_array(value, line_number)?),
            "diagnostics" => entry.diagnostics = Some(parse_string_array(value, line_number)?),
            "examples" => entry.examples = Some(parse_string_array(value, line_number)?),
            "tests" => entry.tests = Some(parse_string_array(value, line_number)?),
            "symbols" => entry.symbols = Some(parse_string_array(value, line_number)?),
            other => {
                return Err(registry_error(
                    line_number,
                    &format!("unsupported module registry key `{other}`"),
                ))
            }
        }
    }

    if let Some(entry) = current.take() {
        modules.push(entry.finish(source.lines().count())?);
    }
    if modules.is_empty() {
        return Err(registry_error(0, "registry contains no modules"));
    }
    modules.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(ModuleRegistry { modules })
}

fn parse_module_header(line: &str, line_number: usize) -> Result<String, ModuleRegistryError> {
    let Some(name) = line
        .strip_prefix("[module.\"")
        .and_then(|value| value.strip_suffix("\"]"))
    else {
        return Err(registry_error(
            line_number,
            "module section must be [module.\"eng.name\"]",
        ));
    };
    if !name.starts_with("eng.") {
        return Err(registry_error(
            line_number,
            "module names must use the eng.* namespace",
        ));
    }
    Ok(name.to_owned())
}

fn parse_quoted_string(value: &str, line_number: usize) -> Result<String, ModuleRegistryError> {
    let Some(inner) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(registry_error(line_number, "expected quoted string"));
    };
    if inner.contains('"') || inner.contains('\\') {
        return Err(registry_error(
            line_number,
            "registry strings currently support only plain ASCII text",
        ));
    }
    Ok(inner.to_owned())
}

fn parse_string_array(value: &str, line_number: usize) -> Result<Vec<String>, ModuleRegistryError> {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Err(registry_error(line_number, "expected string array"));
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let mut rest = inner;
    while !rest.is_empty() {
        let Some(after_open_quote) = rest.strip_prefix('"') else {
            return Err(registry_error(line_number, "expected quoted string"));
        };
        let Some(close_quote) = after_open_quote.find('"') else {
            return Err(registry_error(line_number, "expected quoted string"));
        };
        let item = &after_open_quote[..close_quote];
        if item.contains('\\') {
            return Err(registry_error(
                line_number,
                "registry strings currently support only plain ASCII text",
            ));
        }
        items.push(item.to_owned());
        rest = after_open_quote[close_quote + 1..].trim_start();
        if rest.is_empty() {
            break;
        }
        let Some(after_comma) = rest.strip_prefix(',') else {
            return Err(registry_error(
                line_number,
                "expected comma between strings",
            ));
        };
        rest = after_comma.trim_start();
    }
    Ok(items)
}

fn is_module_registry_diagnostic_code(value: &str) -> bool {
    let Some((prefix, rest)) = value.split_once('-') else {
        return false;
    };
    matches!(prefix, "E" | "W")
        && !rest.is_empty()
        && rest.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

fn registry_error(line: usize, message: &str) -> ModuleRegistryError {
    ModuleRegistryError {
        line,
        message: message.to_owned(),
    }
}

#[derive(Clone, Debug)]
struct PartialModuleRegistryEntry {
    name: String,
    status: Option<String>,
    backing: Option<String>,
    purpose: Option<String>,
    artifacts: Option<Vec<String>>,
    diagnostics: Option<Vec<String>>,
    examples: Option<Vec<String>>,
    tests: Option<Vec<String>>,
    symbols: Option<Vec<String>>,
}

impl PartialModuleRegistryEntry {
    fn finish(self, line: usize) -> Result<ModuleRegistryEntry, ModuleRegistryError> {
        let status = self
            .status
            .ok_or_else(|| registry_error(line, "module is missing status"))?;
        let backing = self
            .backing
            .ok_or_else(|| registry_error(line, "module is missing backing"))?;
        let purpose = self
            .purpose
            .ok_or_else(|| registry_error(line, "module is missing purpose"))?;
        let artifacts = self
            .artifacts
            .ok_or_else(|| registry_error(line, "module is missing artifacts"))?;
        let diagnostics = self
            .diagnostics
            .ok_or_else(|| registry_error(line, "module is missing diagnostics"))?;
        for diagnostic in &diagnostics {
            if !is_module_registry_diagnostic_code(diagnostic) {
                return Err(registry_error(
                    line,
                    &format!("module diagnostic `{diagnostic}` must use an E-/W- diagnostic code"),
                ));
            }
        }
        Ok(ModuleRegistryEntry {
            name: self.name,
            status,
            backing,
            purpose,
            artifacts,
            diagnostics,
            examples: self
                .examples
                .ok_or_else(|| registry_error(line, "module is missing examples"))?,
            tests: self
                .tests
                .ok_or_else(|| registry_error(line, "module is missing tests"))?,
            symbols: self.symbols.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_registry_loads_workflow_modules() {
        let registry = bundled_module_registry().expect("bundled registry should parse");
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.path" && module.status == "supported"));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.path"
                && module
                    .symbols
                    .iter()
                    .any(|symbol| symbol.starts_with("file(path: String)"))
                && module
                    .symbols
                    .iter()
                    .any(|symbol| symbol.starts_with("exists(path:"))
        }));
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.net" && module.status == "native_preview"));
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.cache" && module.status == "native_preview"));
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.workflow"
                && module.status == "native_preview"
                && module.backing == "compiler_runtime_builtin"));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.uncertainty"
                && module.status == "native_preview"
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "report_spec.confidence_band")
        }));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.stats"
                && module.status == "native_preview"
                && module.backing == "compiler_runtime_builtin"
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "typed_payload.statistics")
                && module
                    .tests
                    .iter()
                    .any(|test| test.contains("computes_heat_rate_statistics_and_integral"))
        }));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.quality"
                && module.status == "native_preview"
                && module
                    .symbols
                    .iter()
                    .any(|symbol| symbol.starts_with("rmse(left: TimeSeries"))
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "typed_payload.metrics")
                && module
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic == "E-RMSE-CALL-001")
        }));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.case"
                && module.status == "native_preview"
                && module.backing == "compiler_runtime_builtin"
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "object_store.CaseRunResult")
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "cache_manifest")
                && module
                    .tests
                    .iter()
                    .any(|test| test.contains("run_file_executes_and_collects_native_case_results"))
        }));
        let net_module = registry
            .modules
            .iter()
            .find(|module| module.name == "eng.net")
            .expect("eng.net should be registered");
        assert_eq!(net_module.status_label(), "Native workflow support");
        assert!(net_module.completion_detail().starts_with("Native:"));
        assert!(!net_module.completion_detail().contains("native_preview"));
        assert!(!net_module
            .completion_detail()
            .contains("Native workflow support"));
        let cache_module = registry
            .modules
            .iter()
            .find(|module| module.name == "eng.cache")
            .expect("eng.cache should be registered");
        assert!(cache_module.completion_detail().starts_with("Native:"));
        assert!(cache_module
            .completion_detail()
            .contains("verified native case-result cache"));
        assert!(cache_module
            .examples
            .iter()
            .any(|example| example == "examples/workflows/02_native_surrogate_case_workflow"));
        assert!(!cache_module.completion_detail().contains("broader"));
        assert!(net_module
            .diagnostics
            .iter()
            .any(|code| code == "E-NET-INVALID-URL"));
        assert!(net_module
            .examples
            .iter()
            .any(|example| example == "examples/workflows/01_weather_api_to_standard_file"));
        assert!(net_module
            .tests
            .iter()
            .any(|test| test == "cargo test -p eng_compiler net_"));
        assert!(registry.modules.iter().all(|module| {
            module.diagnostics.iter().all(|value| !value.is_empty())
                && module.examples.iter().all(|value| !value.is_empty())
                && module.tests.iter().all(|value| !value.is_empty())
        }));
    }

    #[test]
    fn registry_rejects_placeholder_diagnostics() {
        let error = parse_module_registry(
            r#"
[module."eng.report"]
status = "native_preview"
backing = "eng_report"
purpose = "Report artifacts without direct diagnostics."
artifacts = []
diagnostics = ["none_current"]
examples = []
tests = []
"#,
        )
        .expect_err("placeholder diagnostics should fail");
        assert!(error.message.contains("must use an E-/W- diagnostic code"));
    }

    #[test]
    fn registry_rejects_missing_required_fields() {
        let error = parse_module_registry(
            r#"
[module."eng.net"]
status = "planned"
"#,
        )
        .expect_err("missing fields should fail");
        assert!(error.message.contains("missing backing"));
    }
}
