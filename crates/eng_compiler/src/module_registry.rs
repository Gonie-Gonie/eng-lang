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
    pub fn completion_detail(&self) -> String {
        format!("{}: {}", self.status, self.purpose)
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
        Ok(ModuleRegistryEntry {
            name: self.name,
            status: self
                .status
                .ok_or_else(|| registry_error(line, "module is missing status"))?,
            backing: self
                .backing
                .ok_or_else(|| registry_error(line, "module is missing backing"))?,
            purpose: self
                .purpose
                .ok_or_else(|| registry_error(line, "module is missing purpose"))?,
            artifacts: self
                .artifacts
                .ok_or_else(|| registry_error(line, "module is missing artifacts"))?,
            diagnostics: self
                .diagnostics
                .ok_or_else(|| registry_error(line, "module is missing diagnostics"))?,
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
            .any(|module| module.name == "eng.net" && module.status == "supported_seed"));
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.cache" && module.status == "supported_seed"));
        assert!(registry
            .modules
            .iter()
            .any(|module| module.name == "eng.case" && !module.artifacts.is_empty()));
        let net_module = registry
            .modules
            .iter()
            .find(|module| module.name == "eng.net")
            .expect("eng.net should be registered");
        assert!(net_module
            .diagnostics
            .iter()
            .any(|code| code == "E-NET-INVALID-URL"));
        assert!(net_module
            .examples
            .iter()
            .any(|example| example == "examples/workflows/01_weather_api_to_standard_file_hybrid"));
        assert!(net_module
            .tests
            .iter()
            .any(|test| test == "cargo test -p eng_compiler net_"));
        assert!(registry.modules.iter().all(|module| {
            module.diagnostics.iter().all(|value| value != "")
                && module.examples.iter().all(|value| value != "")
                && module.tests.iter().all(|value| value != "")
        }));
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
