use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::AstItem;
use crate::parser::ParsedProgram;
use crate::semantic::{ArgValueInfo, SemanticProgram, WithOptionInfo};
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetQueryParam {
    pub key: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetRequestInfo {
    pub binding: String,
    pub method: String,
    pub url_literal: String,
    pub url_value: String,
    pub query: Vec<NetQueryParam>,
    pub retry: Option<usize>,
    pub cache: bool,
    pub expected_sha256: Option<String>,
    pub timeout: Option<String>,
    pub fixture: Option<String>,
    pub status_code: Option<u16>,
    pub response_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetDownloadInfo {
    pub url_literal: String,
    pub url_value: String,
    pub target_literal: String,
    pub target_value: String,
    pub query: Vec<NetQueryParam>,
    pub retry: Option<usize>,
    pub cache: bool,
    pub expected_sha256: Option<String>,
    pub timeout: Option<String>,
    pub fixture: Option<String>,
    pub status_code: Option<u16>,
    pub response_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetAnalysis {
    pub requests: Vec<NetRequestInfo>,
    pub downloads: Vec<NetDownloadInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_net_boundaries(
    parsed: &ParsedProgram,
    source_base: Option<&Path>,
    program: &SemanticProgram,
) -> NetAnalysis {
    let mut analysis = NetAnalysis::default();
    for item in &parsed.items {
        match item {
            AstItem::FastBinding(binding) => {
                let Some(url_literal) = parse_http_get_expression(&binding.expression) else {
                    continue;
                };
                let options = net_options_for_owner(program, binding.line);
                let boundary = build_request(
                    &binding.name,
                    &url_literal,
                    binding.line,
                    &options,
                    source_base,
                    &program.arg_values,
                    &mut analysis.diagnostics,
                );
                analysis.requests.push(boundary);
            }
            AstItem::NetDownload(download) => {
                let options = net_options_for_owner(program, download.line);
                let boundary = build_download(
                    &download.url,
                    &download.target,
                    download.line,
                    &options,
                    source_base,
                    &program.arg_values,
                    &mut analysis.diagnostics,
                );
                analysis.downloads.push(boundary);
            }
            _ => {}
        }
    }
    analysis
}

pub fn is_http_get_expression(expression: &str) -> bool {
    parse_http_get_expression(expression).is_some()
}

fn parse_http_get_expression(expression: &str) -> Option<String> {
    let trimmed = expression.trim();
    let rest = trimmed.strip_prefix("http get ")?;
    let source = rest
        .split_once(" with ")
        .map(|(left, _)| left)
        .unwrap_or(rest)
        .trim();
    (!source.is_empty()).then(|| source.to_owned())
}

fn build_request(
    binding: &str,
    url_literal: &str,
    line: usize,
    options: &[WithOptionInfo],
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> NetRequestInfo {
    let url_value =
        resolve_value(url_literal, arg_values).unwrap_or_else(|| url_literal.to_owned());
    validate_url(&url_value, line, diagnostics);
    let fixture = option_value(options, "fixture")
        .map(|value| resolve_value(value, arg_values).unwrap_or_else(|| value.to_owned()));
    let fixture_read = fixture
        .as_ref()
        .and_then(|fixture| read_fixture(source_base, fixture));
    NetRequestInfo {
        binding: binding.to_owned(),
        method: "GET".to_owned(),
        url_literal: url_literal.to_owned(),
        url_value,
        query: query_params(options, arg_values),
        retry: option_value(options, "retry").and_then(parse_usize),
        cache: option_value(options, "cache").is_some_and(parse_bool),
        expected_sha256: option_value(options, "expected_sha256").map(str::to_owned),
        timeout: option_value(options, "timeout").map(str::to_owned),
        fixture,
        status_code: option_value(options, "status_code")
            .and_then(parse_u16)
            .or_else(|| fixture_read.as_ref().map(|_| 200)),
        response_hash: fixture_read.as_ref().map(|read| read.hash.clone()),
        status: fixture_read
            .as_ref()
            .map(|read| read.status.clone())
            .unwrap_or_else(|| "declared".to_owned()),
        line,
    }
}

fn build_download(
    url_literal: &str,
    target_literal: &str,
    line: usize,
    options: &[WithOptionInfo],
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> NetDownloadInfo {
    let url_value =
        resolve_value(url_literal, arg_values).unwrap_or_else(|| url_literal.to_owned());
    validate_url(&url_value, line, diagnostics);
    let target_value =
        resolve_value(target_literal, arg_values).unwrap_or_else(|| target_literal.to_owned());
    let fixture = option_value(options, "fixture")
        .map(|value| resolve_value(value, arg_values).unwrap_or_else(|| value.to_owned()));
    let fixture_read = fixture
        .as_ref()
        .and_then(|fixture| read_fixture(source_base, fixture));
    NetDownloadInfo {
        url_literal: url_literal.to_owned(),
        url_value,
        target_literal: target_literal.to_owned(),
        target_value,
        query: query_params(options, arg_values),
        retry: option_value(options, "retry").and_then(parse_usize),
        cache: option_value(options, "cache").is_some_and(parse_bool),
        expected_sha256: option_value(options, "expected_sha256").map(str::to_owned),
        timeout: option_value(options, "timeout").map(str::to_owned),
        fixture,
        status_code: option_value(options, "status_code")
            .and_then(parse_u16)
            .or_else(|| fixture_read.as_ref().map(|_| 200)),
        response_hash: fixture_read.as_ref().map(|read| read.hash.clone()),
        status: fixture_read
            .as_ref()
            .map(|read| read.status.clone())
            .unwrap_or_else(|| "declared".to_owned()),
        line,
    }
}

fn net_options_for_owner(program: &SemanticProgram, owner_line: usize) -> Vec<WithOptionInfo> {
    program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.options.iter().cloned())
        .filter(|option| option.status == "accepted")
        .collect()
}

fn query_params(options: &[WithOptionInfo], arg_values: &[ArgValueInfo]) -> Vec<NetQueryParam> {
    options
        .iter()
        .filter(|option| !is_net_control_option(&option.key))
        .filter(|option| option.key != "query" && option.key != "}")
        .map(|option| {
            let (value, redacted) = resolve_query_value(&option.value, arg_values);
            NetQueryParam {
                key: option.key.clone(),
                value,
                redacted,
            }
        })
        .collect()
}

pub fn is_net_control_option(key: &str) -> bool {
    matches!(
        key,
        "query"
            | "retry"
            | "cache"
            | "expected_sha256"
            | "timeout"
            | "fixture"
            | "status_code"
            | "body_size_limit"
            | "response_body_limit"
    )
}

fn option_value<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|option| option.key == key && option.status == "accepted")
        .map(|option| option.value.as_str())
}

fn resolve_query_value(value: &str, arg_values: &[ArgValueInfo]) -> (String, bool) {
    if is_secret_expression(value) {
        return ("<redacted>".to_owned(), true);
    }
    (
        resolve_value(value, arg_values).unwrap_or_else(|| strip_string_literal(value)),
        false,
    )
}

fn resolve_value(value: &str, arg_values: &[ArgValueInfo]) -> Option<String> {
    let trimmed = value.trim();
    if let Some(arg_name) = trimmed.strip_prefix("args.") {
        return arg_values
            .iter()
            .find(|arg| arg.name == arg_name)
            .map(|arg| arg.value.clone());
    }
    if let Some(value) = strip_call_string_arg(trimmed, "url") {
        return Some(value);
    }
    if let Some(value) = strip_call_string_arg(trimmed, "file") {
        return Some(value);
    }
    if trimmed.starts_with('"') {
        return Some(strip_string_literal(trimmed));
    }
    None
}

fn strip_call_string_arg(expression: &str, function_name: &str) -> Option<String> {
    let prefix = format!("{function_name}(");
    let inner = expression.strip_prefix(&prefix)?.strip_suffix(')')?.trim();
    Some(strip_string_literal(inner))
}

fn strip_string_literal(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        inner.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn is_secret_expression(value: &str) -> bool {
    value.trim().starts_with("secret ")
}

fn validate_url(value: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    if value.starts_with("http://") || value.starts_with("https://") {
        return;
    }
    diagnostics.push(Diagnostic::error(
        "E-NET-INVALID-URL",
        line,
        &format!("Network boundary URL `{value}` is not an absolute HTTP(S) URL."),
        Some("Use `url(\"https://...\")` or an Args value containing an absolute HTTP(S) URL."),
    ));
}

struct FixtureRead {
    hash: String,
    status: String,
}

fn read_fixture(source_base: Option<&Path>, fixture: &str) -> Option<FixtureRead> {
    let path = resolve_source_relative_path(fixture, source_base);
    let source = fs::read(&path).ok()?;
    Some(FixtureRead {
        hash: hash_bytes(&source),
        status: "fixture".to_owned(),
    })
}

fn resolve_source_relative_path(source: &str, source_base: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(source);
    if path.is_absolute() {
        path
    } else {
        source_base.unwrap_or_else(|| Path::new(".")).join(path)
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

fn parse_usize(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok()
}

fn parse_u16(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok()
}

fn hash_bytes(source: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
