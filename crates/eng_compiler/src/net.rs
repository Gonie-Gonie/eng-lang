use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ast::AstItem;
use crate::lexer::{Symbol, TokenKind};
use crate::parser::ParsedProgram;
use crate::semantic::{ArgValueInfo, SemanticProgram, WithOptionInfo};
use crate::Diagnostic;

const MAX_RETRY_ATTEMPTS: usize = 5;
const HTTP_REQUEST_METHODS: &[(&str, &str)] = &[
    ("get", "GET"),
    ("post", "POST"),
    ("put", "PUT"),
    ("patch", "PATCH"),
    ("head", "HEAD"),
    ("request", "REQUEST"),
    ("fetch", "FETCH"),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetQueryParam {
    pub key: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetHeaderParam {
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
    pub body: Option<String>,
    pub query: Vec<NetQueryParam>,
    pub headers: Vec<NetHeaderParam>,
    pub retry: Option<usize>,
    pub cache: bool,
    pub expected_sha256: Option<String>,
    pub timeout: Option<String>,
    pub body_size_limit_bytes: Option<usize>,
    pub offline_response: Option<String>,
    pub status_code: Option<u16>,
    pub status_class: String,
    pub response_hash: Option<String>,
    pub response_source: String,
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
    pub body_size_limit_bytes: Option<usize>,
    pub offline_response: Option<String>,
    pub status_code: Option<u16>,
    pub status_class: String,
    pub response_hash: Option<String>,
    pub response_source: String,
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
                let Some((method, url_literal)) =
                    parse_http_request_expression(&binding.expression)
                else {
                    continue;
                };
                let options = net_options_for_owner(program, binding.line);
                let boundary = build_request(
                    &binding.name,
                    &method,
                    &url_literal,
                    binding.line,
                    &options,
                    parsed,
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
                    parsed,
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

pub fn is_http_request_expression(expression: &str) -> bool {
    parse_http_request_expression(expression).is_some()
}

fn parse_http_request_expression(expression: &str) -> Option<(String, String)> {
    let trimmed = expression.trim();
    let rest = trimmed.strip_prefix("http ")?;
    let method_label = rest.split_whitespace().next()?;
    let method = http_method_label(method_label)?;
    let rest = rest[method_label.len()..].trim();
    let source = rest
        .split_once(" with ")
        .map(|(left, _)| left)
        .unwrap_or(rest)
        .trim();
    (!source.is_empty()).then(|| (method.to_owned(), source.to_owned()))
}

fn http_method_label(label: &str) -> Option<&'static str> {
    HTTP_REQUEST_METHODS
        .iter()
        .find_map(|(syntax, method)| label.eq_ignore_ascii_case(syntax).then_some(*method))
}

fn build_request(
    binding: &str,
    method: &str,
    url_literal: &str,
    line: usize,
    options: &[WithOptionInfo],
    parsed: &ParsedProgram,
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> NetRequestInfo {
    let url_value =
        resolve_value(url_literal, arg_values).unwrap_or_else(|| url_literal.to_owned());
    validate_url(&url_value, line, diagnostics);
    let offline_response = offline_response_value(options)
        .map(|value| resolve_value(value, arg_values).unwrap_or_else(|| value.to_owned()));
    let offline_response_read = offline_response
        .as_ref()
        .and_then(|offline_response| read_offline_response(source_base, offline_response));
    let expected_sha256 = expected_sha256_option(options);
    let hash_valid = validate_expected_sha256(
        expected_sha256
            .as_ref()
            .map(|(value, _line)| value.as_str()),
        offline_response_read.as_ref(),
        expected_sha256
            .as_ref()
            .map(|(_value, line)| *line)
            .unwrap_or(line),
        diagnostics,
    );
    let status_code = option_value(options, "status_code")
        .and_then(parse_u16)
        .or_else(|| offline_response_read.as_ref().map(|_| 200));
    NetRequestInfo {
        binding: binding.to_owned(),
        method: method.to_owned(),
        url_literal: url_literal.to_owned(),
        url_value,
        body: request_body_option(method, options, arg_values, diagnostics),
        query: query_params(parsed, options, arg_values),
        headers: header_params(parsed, options, arg_values),
        retry: retry_policy(options, diagnostics),
        cache: option_value(options, "cache").is_some_and(parse_bool),
        expected_sha256: expected_sha256.map(|(value, _line)| value),
        timeout: timeout_policy(options, diagnostics),
        body_size_limit_bytes: body_size_limit_policy(options, diagnostics),
        offline_response,
        status_code,
        status_class: http_status_class(status_code).to_owned(),
        response_hash: offline_response_read.as_ref().map(|read| read.hash.clone()),
        response_source: offline_response_status(offline_response_read.as_ref(), hash_valid),
        line,
    }
}

fn build_download(
    url_literal: &str,
    target_literal: &str,
    line: usize,
    options: &[WithOptionInfo],
    parsed: &ParsedProgram,
    source_base: Option<&Path>,
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> NetDownloadInfo {
    let url_value =
        resolve_value(url_literal, arg_values).unwrap_or_else(|| url_literal.to_owned());
    validate_url(&url_value, line, diagnostics);
    let target_value =
        resolve_value(target_literal, arg_values).unwrap_or_else(|| target_literal.to_owned());
    let offline_response = offline_response_value(options)
        .map(|value| resolve_value(value, arg_values).unwrap_or_else(|| value.to_owned()));
    let offline_response_read = offline_response
        .as_ref()
        .and_then(|offline_response| read_offline_response(source_base, offline_response));
    let expected_sha256 = expected_sha256_option(options);
    let hash_valid = validate_expected_sha256(
        expected_sha256
            .as_ref()
            .map(|(value, _line)| value.as_str()),
        offline_response_read.as_ref(),
        expected_sha256
            .as_ref()
            .map(|(_value, line)| *line)
            .unwrap_or(line),
        diagnostics,
    );
    let status_code = option_value(options, "status_code")
        .and_then(parse_u16)
        .or_else(|| offline_response_read.as_ref().map(|_| 200));
    NetDownloadInfo {
        url_literal: url_literal.to_owned(),
        url_value,
        target_literal: target_literal.to_owned(),
        target_value,
        query: query_params(parsed, options, arg_values),
        retry: retry_policy(options, diagnostics),
        cache: option_value(options, "cache").is_some_and(parse_bool),
        expected_sha256: expected_sha256.map(|(value, _line)| value),
        timeout: timeout_policy(options, diagnostics),
        body_size_limit_bytes: body_size_limit_policy(options, diagnostics),
        offline_response,
        status_code,
        status_class: http_status_class(status_code).to_owned(),
        response_hash: offline_response_read.as_ref().map(|read| read.hash.clone()),
        response_source: offline_response_status(offline_response_read.as_ref(), hash_valid),
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

fn query_params(
    parsed: &ParsedProgram,
    options: &[WithOptionInfo],
    arg_values: &[ArgValueInfo],
) -> Vec<NetQueryParam> {
    let header_ranges = option_map_ranges(parsed, options, "headers");
    options
        .iter()
        .filter(|option| !is_net_control_option(&option.key))
        .filter(|option| option.key != "query" && option.key != "}")
        .filter(|option| !line_in_ranges(option.line, &header_ranges))
        .map(|option| {
            let (value, redacted) = resolve_net_param_value(&option.value, arg_values);
            NetQueryParam {
                key: option.key.clone(),
                value,
                redacted,
            }
        })
        .collect()
}

fn header_params(
    parsed: &ParsedProgram,
    options: &[WithOptionInfo],
    arg_values: &[ArgValueInfo],
) -> Vec<NetHeaderParam> {
    let header_ranges = option_map_ranges(parsed, options, "headers");
    if header_ranges.is_empty() {
        return Vec::new();
    }
    options
        .iter()
        .filter(|option| option.key != "headers" && option.key != "query" && option.key != "}")
        .filter(|option| line_in_ranges(option.line, &header_ranges))
        .map(|option| {
            let (value, redacted) = resolve_net_param_value(&option.value, arg_values);
            NetHeaderParam {
                key: option.key.clone(),
                value,
                redacted,
            }
        })
        .collect()
}

fn option_map_ranges(
    parsed: &ParsedProgram,
    options: &[WithOptionInfo],
    key: &str,
) -> Vec<(usize, usize)> {
    options
        .iter()
        .filter(|option| option.key == key && option.value.trim_start().starts_with('{'))
        .filter_map(|option| option_map_range(parsed, option.line))
        .collect()
}

fn option_map_range(parsed: &ParsedProgram, start_line: usize) -> Option<(usize, usize)> {
    let mut depth = 0i32;
    let mut seen_start = false;
    for line in parsed.lines.iter().filter(|line| line.line >= start_line) {
        seen_start |= line.line == start_line;
        if !seen_start {
            continue;
        }
        depth += line
            .tokens
            .iter()
            .map(|token| match token.kind {
                TokenKind::Symbol(Symbol::LBrace) => 1,
                TokenKind::Symbol(Symbol::RBrace) => -1,
                _ => 0,
            })
            .sum::<i32>();
        if depth <= 0 {
            return Some((start_line, line.line));
        }
    }
    seen_start.then_some((start_line, usize::MAX))
}

fn line_in_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| line > *start && line < *end)
}

pub fn is_net_control_option(key: &str) -> bool {
    matches!(
        key,
        "query"
            | "body"
            | "retry"
            | "cache"
            | "cache_dir"
            | "cache_key"
            | "expected_sha256"
            | "headers"
            | "timeout"
            | "offline_response"
            | "fixture"
            | "status_code"
            | "body_size_limit"
            | "response_body_limit"
    )
}

fn request_body_option(
    method: &str,
    options: &[WithOptionInfo],
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let option = option_for_key(options, "body")?;
    if !matches!(method, "POST" | "PUT" | "PATCH") {
        diagnostics.push(Diagnostic::error(
            "E-NET-BODY-METHOD",
            option.line,
            &format!("HTTP request body is not supported for method `{method}`."),
            Some("Use `http post`, `http put`, or `http patch` when sending a request body."),
        ));
        return None;
    }
    if is_secret_expression(&option.value) {
        diagnostics.push(Diagnostic::error(
            "E-NET-BODY-POLICY",
            option.line,
            "HTTP request body currently supports string literals and non-secret args values only.",
            Some("Bind request bodies with a string literal or a non-secret `args.<name>` value."),
        ));
        return None;
    }
    Some(
        resolve_value(&option.value, arg_values)
            .unwrap_or_else(|| strip_string_literal(&option.value)),
    )
}

fn option_value<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a str> {
    option_for_key(options, key).map(|option| option.value.as_str())
}

fn offline_response_value(options: &[WithOptionInfo]) -> Option<&str> {
    option_for_key(options, "offline_response")
        .or_else(|| option_for_key(options, "fixture"))
        .map(|option| option.value.as_str())
}

fn option_for_key<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a WithOptionInfo> {
    options
        .iter()
        .find(|option| option.key == key && option.status == "accepted")
}

fn option_for_any_key<'a>(
    options: &'a [WithOptionInfo],
    keys: &[&str],
) -> Option<&'a WithOptionInfo> {
    options
        .iter()
        .find(|option| option.status == "accepted" && keys.iter().any(|key| option.key == *key))
}

fn expected_sha256_option(options: &[WithOptionInfo]) -> Option<(String, usize)> {
    let option = option_for_key(options, "expected_sha256")?;
    Some((normalize_sha256(&option.value), option.line))
}

fn retry_policy(options: &[WithOptionInfo], diagnostics: &mut Vec<Diagnostic>) -> Option<usize> {
    let option = option_for_key(options, "retry")?;
    let raw = option.value.trim();
    let parsed = match raw.parse::<usize>() {
        Ok(value) => value,
        Err(_) => {
            diagnostics.push(Diagnostic::error(
                "E-NET-RETRY-POLICY",
                option.line,
                &format!("Network retry policy `{raw}` is not a whole number."),
                Some("Use `retry = 0` to disable retries or an integer from 1 to 5."),
            ));
            return None;
        }
    };
    if parsed > MAX_RETRY_ATTEMPTS {
        diagnostics.push(Diagnostic::error(
            "E-NET-RETRY-POLICY",
            option.line,
            &format!(
                "Network retry policy `{parsed}` exceeds the maximum of {MAX_RETRY_ATTEMPTS}."
            ),
            Some("Use a retry count from 0 to 5."),
        ));
        return None;
    }
    Some(parsed)
}

fn timeout_policy(options: &[WithOptionInfo], diagnostics: &mut Vec<Diagnostic>) -> Option<String> {
    let option = option_for_key(options, "timeout")?;
    match normalize_timeout_duration(&option.value) {
        Ok(timeout) => Some(timeout),
        Err(message) => {
            diagnostics.push(Diagnostic::error(
                "E-NET-TIMEOUT",
                option.line,
                &format!(
                    "Network timeout policy `{}` is invalid.",
                    option.value.trim()
                ),
                Some(&message),
            ));
            None
        }
    }
}

fn normalize_timeout_duration(value: &str) -> Result<String, String> {
    let (amount, unit) = parse_number_with_suffix(value)
        .ok_or_else(|| "Use a timeout such as `500 ms`, `30 s`, `10 min`, or `1 h`.".to_owned())?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err("Use a positive finite timeout duration.".to_owned());
    }
    let unit = unit.unwrap_or("s").to_ascii_lowercase();
    let seconds = match unit.as_str() {
        "ms" | "msec" | "millisecond" | "milliseconds" => amount / 1000.0,
        "s" | "sec" | "secs" | "second" | "seconds" => amount,
        "m" | "min" | "mins" | "minute" | "minutes" => amount * 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => amount * 3600.0,
        _ => {
            return Err("Supported timeout units are ms, s, min, and h.".to_owned());
        }
    };
    Ok(format!("{} s", format_duration_number(seconds)))
}

fn body_size_limit_policy(
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<usize> {
    let option = option_for_any_key(options, &["body_size_limit", "response_body_limit"])?;
    match normalize_body_size_limit(&option.value) {
        Ok(limit) => Some(limit),
        Err(message) => {
            diagnostics.push(Diagnostic::error(
                "E-NET-BODY-SIZE-LIMIT",
                option.line,
                &format!(
                    "Network response body size limit `{}` is invalid.",
                    option.value.trim()
                ),
                Some(&message),
            ));
            None
        }
    }
}

fn normalize_body_size_limit(value: &str) -> Result<usize, String> {
    let (amount, unit) = parse_number_with_suffix(value).ok_or_else(|| {
        "Use a size such as `512 KB`, `10 MB`, `1 GiB`, or a raw byte count.".to_owned()
    })?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err("Use a positive finite response body size limit.".to_owned());
    }
    let unit = unit.unwrap_or("B").to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kilobyte" | "kilobytes" => 1_000.0,
        "m" | "mb" | "megabyte" | "megabytes" => 1_000_000.0,
        "g" | "gb" | "gigabyte" | "gigabytes" => 1_000_000_000.0,
        "kib" | "kibibyte" | "kibibytes" => 1_024.0,
        "mib" | "mebibyte" | "mebibytes" => 1_048_576.0,
        "gib" | "gibibyte" | "gibibytes" => 1_073_741_824.0,
        _ => {
            return Err("Supported size units are B, KB, MB, GB, KiB, MiB, and GiB.".to_owned());
        }
    };
    let bytes = amount * multiplier;
    if !bytes.is_finite() || bytes <= 0.0 {
        return Err("Use a positive finite response body size limit.".to_owned());
    }
    if bytes > usize::MAX as f64 {
        return Err(
            "Response body size limit exceeds the maximum supported byte count.".to_owned(),
        );
    }
    let rounded = bytes.round();
    if (bytes - rounded).abs() > 0.000001 {
        return Err("Use a size that resolves to a whole number of bytes.".to_owned());
    }
    Ok(rounded as usize)
}

fn resolve_net_param_value(value: &str, arg_values: &[ArgValueInfo]) -> (String, bool) {
    if is_secret_expression(value) {
        return ("<redacted>".to_owned(), true);
    }
    if let Some(arg_name) = value.trim().strip_prefix("args.") {
        if let Some(arg) = arg_values.iter().find(|arg| arg.name == arg_name) {
            if arg.redacted {
                return ("<redacted>".to_owned(), true);
            }
        }
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
            .map(|arg| resolve_literal_value(&arg.value).unwrap_or_else(|| arg.value.clone()));
    }
    resolve_literal_value(trimmed)
}

fn resolve_literal_value(trimmed: &str) -> Option<String> {
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

fn validate_expected_sha256(
    expected_sha256: Option<&str>,
    offline_response_read: Option<&OfflineResponseRead>,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(expected_sha256) = expected_sha256 else {
        return true;
    };
    if !is_sha256_hex(expected_sha256) {
        diagnostics.push(Diagnostic::error(
            "E-NET-HASH-MISMATCH",
            line,
            &format!("Expected response SHA256 `{expected_sha256}` is invalid."),
            Some("Use a 64-character hexadecimal SHA-256 digest."),
        ));
        return false;
    }
    let Some(offline_response_read) = offline_response_read else {
        return true;
    };
    if expected_sha256 != offline_response_read.hash {
        diagnostics.push(Diagnostic::error(
            "E-NET-HASH-MISMATCH",
            line,
            &format!(
                "Expected response SHA256 `{expected_sha256}` but offline response SHA256 was `{}`.",
                offline_response_read.hash
            ),
            Some("Update `expected_sha256` or the offline response file so the digest matches."),
        ));
        return false;
    }
    true
}

fn normalize_sha256(value: &str) -> String {
    let stripped = strip_string_literal(value).trim().to_ascii_lowercase();
    stripped
        .strip_prefix("sha256:")
        .unwrap_or(&stripped)
        .to_owned()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

pub fn request_body_sha256(body: &str) -> String {
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

struct OfflineResponseRead {
    hash: String,
    status: String,
}

fn offline_response_status(
    offline_response_read: Option<&OfflineResponseRead>,
    hash_valid: bool,
) -> String {
    match offline_response_read {
        Some(read) if hash_valid => read.status.clone(),
        Some(_) => "hash_mismatch".to_owned(),
        None => "declared".to_owned(),
    }
}

fn read_offline_response(
    source_base: Option<&Path>,
    offline_response: &str,
) -> Option<OfflineResponseRead> {
    let path = resolve_source_relative_path(offline_response, source_base);
    let source = fs::read(&path).ok()?;
    Some(OfflineResponseRead {
        hash: hash_bytes(&source),
        status: "offline_response".to_owned(),
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

fn parse_number_with_suffix(value: &str) -> Option<(f64, Option<&str>)> {
    let trimmed = value.trim();
    let mut split_at = 0usize;
    let mut saw_digit = false;
    let mut previous = '\0';
    for (index, character) in trimmed.char_indices() {
        let allowed = character.is_ascii_digit()
            || character == '.'
            || ((character == '-' || character == '+')
                && (index == 0 || previous == 'e' || previous == 'E'))
            || ((character == 'e' || character == 'E') && saw_digit);
        if !allowed {
            break;
        }
        if character.is_ascii_digit() {
            saw_digit = true;
        }
        split_at = index + character.len_utf8();
        previous = character;
    }
    if !saw_digit {
        return None;
    }
    let amount = trimmed[..split_at].parse::<f64>().ok()?;
    let unit = trimmed[split_at..].trim();
    Some((amount, (!unit.is_empty()).then_some(unit)))
}

fn format_duration_number(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn parse_u16(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok()
}

pub fn http_status_class(status_code: Option<u16>) -> &'static str {
    match status_code {
        Some(100..=199) => "informational",
        Some(200..=299) => "success",
        Some(300..=399) => "redirect",
        Some(400..=499) => "client_error",
        Some(500..=599) => "server_error",
        Some(_) | None => "unknown",
    }
}

fn hash_bytes(source: &[u8]) -> String {
    format!("{:x}", Sha256::digest(source))
}
