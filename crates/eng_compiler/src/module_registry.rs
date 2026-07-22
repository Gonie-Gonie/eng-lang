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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleFunctionParameter {
    pub name: String,
    pub label: String,
    pub type_name: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleFunctionSignature {
    pub name: String,
    pub label: String,
    pub parameters: Vec<ModuleFunctionParameter>,
    pub return_type: String,
    pub return_display_unit: Option<String>,
}

impl ModuleRegistryEntry {
    pub fn is_public_api(&self) -> bool {
        matches!(
            self.status.as_str(),
            "supported" | "supported_narrow" | "native_preview"
        )
    }

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

    pub fn function_signatures(&self) -> Vec<ModuleFunctionSignature> {
        self.symbols
            .iter()
            .filter(|symbol| looks_like_function_signature(symbol))
            .filter_map(|symbol| parse_module_function_signature(symbol).ok())
            .collect()
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
            "symbols" => {
                let symbols = parse_string_array(value, line_number)?;
                for symbol in &symbols {
                    if looks_like_function_signature(symbol) {
                        parse_module_function_signature_at(symbol, line_number)?;
                    }
                }
                entry.symbols = Some(symbols);
            }
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

pub fn parse_module_function_signature(
    value: &str,
) -> Result<ModuleFunctionSignature, ModuleRegistryError> {
    parse_module_function_signature_at(value, 0)
}

fn parse_module_function_signature_at(
    value: &str,
    line_number: usize,
) -> Result<ModuleFunctionSignature, ModuleRegistryError> {
    let value = value.trim();
    let (callable, return_value) = value.split_once(" -> ").ok_or_else(|| {
        registry_error(
            line_number,
            "function symbol must use name(parameters) -> ReturnType",
        )
    })?;
    let open = callable.find('(').ok_or_else(|| {
        registry_error(
            line_number,
            "function symbol must include an opening parenthesis",
        )
    })?;
    let close = callable
        .rfind(')')
        .filter(|close| *close + 1 == callable.len())
        .ok_or_else(|| {
            registry_error(
                line_number,
                "function symbol must end its parameter list with a closing parenthesis",
            )
        })?;
    let name = callable[..open].trim();
    if !is_registry_identifier(name) {
        return Err(registry_error(
            line_number,
            "function symbol name must be an identifier",
        ));
    }

    let parameters = split_signature_parameters(&callable[open + 1..close], line_number)?
        .into_iter()
        .map(|parameter| parse_signature_parameter(parameter, line_number))
        .collect::<Result<Vec<_>, _>>()?;
    let (return_type, return_display_unit) =
        parse_signature_return(return_value.trim(), line_number)?;

    Ok(ModuleFunctionSignature {
        name: name.to_owned(),
        label: value.to_owned(),
        parameters,
        return_type,
        return_display_unit,
    })
}

fn looks_like_function_signature(value: &str) -> bool {
    value.contains('(') || value.contains(')') || value.contains("->")
}

fn split_signature_parameters(
    value: &str,
    line_number: usize,
) -> Result<Vec<&str>, ModuleRegistryError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut parameters = Vec::new();
    let mut start = 0usize;
    let mut bracket_depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '[' => bracket_depth += 1,
            ']' => {
                bracket_depth = bracket_depth.checked_sub(1).ok_or_else(|| {
                    registry_error(
                        line_number,
                        "function parameter type has an unmatched closing bracket",
                    )
                })?;
            }
            ',' if bracket_depth == 0 => {
                parameters.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if bracket_depth != 0 {
        return Err(registry_error(
            line_number,
            "function parameter type has an unmatched opening bracket",
        ));
    }
    parameters.push(value[start..].trim());
    if parameters.iter().any(|parameter| parameter.is_empty()) {
        return Err(registry_error(
            line_number,
            "function signature contains an empty parameter",
        ));
    }
    Ok(parameters)
}

fn parse_signature_parameter(
    value: &str,
    line_number: usize,
) -> Result<ModuleFunctionParameter, ModuleRegistryError> {
    let (raw_name, type_name) = value.split_once(':').ok_or_else(|| {
        registry_error(line_number, "function parameter must use name: Type syntax")
    })?;
    let raw_name = raw_name.trim();
    let (name, optional) = raw_name
        .strip_suffix('?')
        .map(|name| (name.trim(), true))
        .unwrap_or((raw_name, false));
    let type_name = type_name.trim();
    if !is_registry_identifier(name) {
        return Err(registry_error(
            line_number,
            "function parameter name must be an identifier",
        ));
    }
    if type_name.is_empty() {
        return Err(registry_error(
            line_number,
            "function parameter type cannot be empty",
        ));
    }
    Ok(ModuleFunctionParameter {
        name: name.to_owned(),
        label: value.to_owned(),
        type_name: type_name.to_owned(),
        optional,
    })
}

fn parse_signature_return(
    value: &str,
    line_number: usize,
) -> Result<(String, Option<String>), ModuleRegistryError> {
    if value.is_empty() {
        return Err(registry_error(
            line_number,
            "function return type cannot be empty",
        ));
    }
    if let Some(unit_start) = value.rfind(" [") {
        if value.ends_with(']') {
            let return_type = value[..unit_start].trim();
            let display_unit = value[unit_start + 2..value.len() - 1].trim();
            if return_type.is_empty() || display_unit.is_empty() {
                return Err(registry_error(
                    line_number,
                    "function return type and display unit cannot be empty",
                ));
            }
            return Ok((return_type.to_owned(), Some(display_unit.to_owned())));
        }
    }
    Ok((value.to_owned(), None))
}

fn is_registry_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
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

fn purpose_repeats_status(status: &str, purpose: &str) -> bool {
    let repeated_word = match status {
        "supported" | "supported_narrow" => "supported",
        "native_preview" => "native",
        "planned" => "planned",
        "internal_planned" | "internal" => "internal",
        _ => return false,
    };
    purpose
        .split_whitespace()
        .next()
        .map(|word| word.trim_end_matches(|character: char| character.is_ascii_punctuation()))
        .is_some_and(|word| word.eq_ignore_ascii_case(repeated_word))
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
        if purpose_repeats_status(&status, &purpose) {
            return Err(registry_error(
                line,
                &format!(
                    "module purpose must describe the capability without repeating status `{status}`"
                ),
            ));
        }
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
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "typed_payload.numeric_values")
                && module
                    .symbols
                    .iter()
                    .any(|symbol| symbol.starts_with("duration_above(series: TimeSeries"))
                && module
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic == "E-STATS-DURATION-CALL-001")
                && module
                    .tests
                    .iter()
                    .any(|test| test.contains("computes_heat_rate_statistics_and_integral"))
        }));
        assert!(registry.modules.iter().any(|module| {
            module.name == "eng.timeseries"
                && module.status == "native_preview"
                && module
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "typed_payload.numeric_values")
                && module.tests.iter().any(|test| {
                    test.contains(
                        "run_file_materializes_explicit_computed_scalars_in_declared_units",
                    )
                })
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
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "typed_payload.numeric_values")
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
    fn bundled_registry_distinguishes_public_api_modules() {
        let registry = bundled_module_registry().expect("bundled registry should parse");

        for name in ["eng.path", "eng.net", "eng.uncertainty"] {
            let module = registry
                .modules
                .iter()
                .find(|module| module.name == name)
                .unwrap_or_else(|| panic!("{name} should be registered"));
            assert!(module.is_public_api(), "{name} should be public");
        }
        for name in ["eng.building", "eng.system", "eng.ml"] {
            let module = registry
                .modules
                .iter()
                .find(|module| module.name == name)
                .unwrap_or_else(|| panic!("{name} should be registered"));
            assert!(!module.is_public_api(), "{name} should not be public");
        }
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
    fn registry_rejects_status_repeated_in_user_facing_purpose() {
        let error = parse_module_registry(
            r#"
[module."eng.case"]
status = "native_preview"
backing = "compiler_runtime_builtin"
purpose = "Native: case execution."
artifacts = []
diagnostics = []
examples = []
tests = []
"#,
        )
        .expect_err("status-prefixed purpose should fail");
        assert!(error
            .message
            .contains("without repeating status `native_preview`"));
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

    #[test]
    fn parses_typed_function_symbols_with_optional_parameters_and_return_units() {
        let signature = parse_module_function_signature(
            "duration_above(series: TimeSeries[Time], threshold: Quantity, axis?: TimeAxis) -> Duration [s]",
        )
        .expect("typed function symbol should parse");

        assert_eq!(signature.name, "duration_above");
        assert_eq!(signature.parameters.len(), 3);
        assert_eq!(signature.parameters[0].type_name, "TimeSeries[Time]");
        assert_eq!(signature.parameters[2].name, "axis");
        assert!(signature.parameters[2].optional);
        assert_eq!(signature.return_type, "Duration");
        assert_eq!(signature.return_display_unit.as_deref(), Some("s"));
    }

    #[test]
    fn registry_rejects_malformed_function_symbols() {
        let error = parse_module_registry(
            r#"
[module."eng.stats"]
status = "native_preview"
backing = "compiler_runtime_builtin"
purpose = "Typed statistics."
artifacts = []
diagnostics = []
examples = []
tests = []
symbols = ["mean(series TimeSeries) -> Quantity"]
"#,
        )
        .expect_err("malformed function symbol should fail");

        assert_eq!(error.line, 10);
        assert!(error.message.contains("name: Type"));
    }
}
