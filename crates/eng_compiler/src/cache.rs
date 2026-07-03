use crate::ml::MlInfo;
use crate::semantic::{ArgValueInfo, SemanticProgram, WithOptionInfo};
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheRecordInfo {
    pub owner_kind: String,
    pub owner_name: String,
    pub cache_key: String,
    pub cache_key_parts: Vec<String>,
    pub cache_key_hash: String,
    pub cache_path: String,
    pub cache_dir: String,
    pub cache_ttl: Option<String>,
    pub source_hash: String,
    pub expected_hash: Option<String>,
    pub observed_hash: Option<String>,
    pub status: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheAnalysis {
    pub records: Vec<CacheRecordInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze_cache_records(program: &SemanticProgram, source_hash: &str) -> CacheAnalysis {
    let mut analysis = CacheAnalysis::default();

    for process in &program.process_runs {
        let options = options_for_owner(program, process.line);
        if let Some(record) = build_cache_record(
            "process",
            &process.binding,
            process.line,
            &options,
            process_cache_parts(program, process.line, &process.command),
            source_hash,
            None,
            None,
            "declared",
            &program.arg_values,
            &mut analysis.diagnostics,
        ) {
            analysis.records.push(record);
        }
    }

    for model in &program.ml_infos {
        let options = options_for_owner(program, model.line);
        if let Some(record) = build_cache_record(
            "model",
            &model.binding,
            model.line,
            &options,
            model_cache_parts(model),
            source_hash,
            None,
            None,
            "declared",
            &program.arg_values,
            &mut analysis.diagnostics,
        ) {
            analysis.records.push(record);
        }
    }

    for request in &program.net_requests {
        let options = options_for_owner(program, request.line);
        if let Some(record) = build_cache_record(
            "network_request",
            &request.binding,
            request.line,
            &options,
            network_request_cache_parts(request),
            source_hash,
            request.expected_sha256.clone(),
            request.response_hash.clone(),
            if request.response_hash.is_some() {
                "offline_response_available"
            } else {
                "declared"
            },
            &program.arg_values,
            &mut analysis.diagnostics,
        ) {
            analysis.records.push(record);
        }
    }

    for download in &program.net_downloads {
        let options = options_for_owner(program, download.line);
        if let Some(record) = build_cache_record(
            "network_download",
            &download.target_value,
            download.line,
            &options,
            network_download_cache_parts(download),
            source_hash,
            download.expected_sha256.clone(),
            download.response_hash.clone(),
            if download.response_hash.is_some() {
                "offline_response_available"
            } else {
                "declared"
            },
            &program.arg_values,
            &mut analysis.diagnostics,
        ) {
            analysis.records.push(record);
        }
    }

    analysis
}

fn build_cache_record(
    owner_kind: &str,
    owner_name: &str,
    line: usize,
    options: &[WithOptionInfo],
    default_parts: Vec<String>,
    source_hash: &str,
    expected_hash: Option<String>,
    observed_hash: Option<String>,
    status: &str,
    arg_values: &[ArgValueInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CacheRecordInfo> {
    let cache_enabled = option_value(options, "cache").is_some_and(parse_bool);
    let raw_cache_key = option_value(options, "cache_key");
    if !cache_enabled && raw_cache_key.is_none() {
        return None;
    }

    let key_line = options
        .iter()
        .find(|option| option.key == "cache_key")
        .map(|option| option.line)
        .unwrap_or(line);
    let mut cache_key_parts = if let Some(raw_cache_key) = raw_cache_key {
        let parts = parse_cache_key_parts(raw_cache_key, arg_values);
        validate_cache_key_parts(raw_cache_key, &parts, key_line, diagnostics);
        parts
    } else {
        default_parts
    };
    cache_key_parts.push(format!("source_hash={source_hash}"));
    let cache_key = serialize_cache_key(&cache_key_parts);
    let cache_key_hash = hash_text(&cache_key);
    let cache_dir = option_value(options, "cache_dir")
        .map(strip_string_literal)
        .unwrap_or_else(|| "cache".to_owned());
    let cache_ttl = cache_ttl_policy(options, diagnostics);
    let cache_path = format!("{}/{}", cache_dir.trim_end_matches('/'), cache_key_hash);

    Some(CacheRecordInfo {
        owner_kind: owner_kind.to_owned(),
        owner_name: owner_name.to_owned(),
        cache_key,
        cache_key_parts,
        cache_key_hash,
        cache_path,
        cache_dir,
        cache_ttl,
        source_hash: source_hash.to_owned(),
        expected_hash,
        observed_hash,
        status: status.to_owned(),
        line,
    })
}

fn options_for_owner(program: &SemanticProgram, owner_line: usize) -> Vec<WithOptionInfo> {
    program
        .with_blocks
        .iter()
        .filter(|block| block.owner_line == Some(owner_line))
        .flat_map(|block| block.options.iter().cloned())
        .filter(|option| option.status == "accepted")
        .collect()
}

fn process_cache_parts(program: &SemanticProgram, owner_line: usize, command: &str) -> Vec<String> {
    let options = options_for_owner(program, owner_line);
    let mut parts = vec!["process".to_owned(), command.to_owned()];
    for key in ["args", "cwd", "tool_version", "expected_outputs"] {
        if let Some(value) = option_value(&options, key) {
            parts.push(format!("{key}={value}"));
        }
    }
    parts
}

fn model_cache_parts(model: &MlInfo) -> Vec<String> {
    let mut parts = vec![
        "model".to_owned(),
        model.kind.clone(),
        model.binding.clone(),
        format!("expression={}", model.expression),
    ];
    if let Some(source) = &model.source {
        parts.push(format!("source={source}"));
    }
    if let Some(input) = &model.prediction_input {
        parts.push(format!("prediction_input={input}"));
    }
    if let Some(target) = &model.target {
        parts.push(format!("target={target}"));
    }
    if !model.features.is_empty() {
        parts.push(format!("features={}", model.features.join(",")));
    }
    if let Some(algorithm) = &model.algorithm {
        parts.push(format!("algorithm={algorithm}"));
    }
    if let Some(test_fraction) = &model.test_fraction {
        parts.push(format!("test={test_fraction}"));
    }
    if let Some(seed) = &model.seed {
        parts.push(format!("seed={seed}"));
    }
    if !model.hidden_layers.is_empty() {
        parts.push(format!("hidden={:?}", model.hidden_layers));
    }
    if let Some(epochs) = model.epochs {
        parts.push(format!("epochs={epochs}"));
    }
    parts
}

fn network_request_cache_parts(request: &crate::net::NetRequestInfo) -> Vec<String> {
    let mut parts = vec![
        "network_request".to_owned(),
        request.method.clone(),
        request.url_value.clone(),
    ];
    for query in &request.query {
        parts.push(format!("{}={}", query.key, query.value));
    }
    for header in &request.headers {
        parts.push(format!("header:{}={}", header.key, header.value));
    }
    if let Some(body) = &request.body {
        parts.push(format!(
            "body_sha256={}",
            crate::net::request_body_sha256(body)
        ));
    }
    if let Some(expected_hash) = &request.expected_sha256 {
        parts.push(format!("expected_sha256={expected_hash}"));
    }
    parts
}

fn network_download_cache_parts(download: &crate::net::NetDownloadInfo) -> Vec<String> {
    let mut parts = vec![
        "network_download".to_owned(),
        download.url_value.clone(),
        download.target_value.clone(),
    ];
    for query in &download.query {
        parts.push(format!("{}={}", query.key, query.value));
    }
    if let Some(expected_hash) = &download.expected_sha256 {
        parts.push(format!("expected_sha256={expected_hash}"));
    }
    parts
}

fn parse_cache_key_parts(raw: &str, arg_values: &[ArgValueInfo]) -> Vec<String> {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return split_top_level_commas(inner)
            .into_iter()
            .map(|part| resolve_cache_part(part, arg_values))
            .filter(|part| !part.is_empty())
            .collect();
    }
    vec![resolve_cache_part(trimmed, arg_values)]
}

fn resolve_cache_part(raw: &str, arg_values: &[ArgValueInfo]) -> String {
    let trimmed = raw.trim();
    if let Some(arg_name) = trimmed.strip_prefix("args.") {
        if let Some(arg) = arg_values.iter().find(|arg| arg.name == arg_name) {
            return arg.value.clone();
        }
    }
    strip_string_literal(trimmed)
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_escape = false;
    for (index, character) in value.char_indices() {
        if in_string {
            if character == '"' && !previous_escape {
                in_string = false;
            }
            previous_escape = character == '\\' && !previous_escape;
            if character != '\\' {
                previous_escape = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn validate_cache_key_parts(
    raw_cache_key: &str,
    parts: &[String],
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let normalized = raw_cache_key.to_ascii_lowercase();
    let nondeterministic = ["now(", "random(", "rand(", "uuid(", "env(", "secret "]
        .iter()
        .any(|pattern| normalized.contains(pattern));
    if nondeterministic {
        diagnostics.push(Diagnostic::error(
            "E-CACHE-KEY-NONDETERMINISTIC",
            line,
            "`cache_key` contains a nondeterministic or secret-dependent expression.",
            Some("Use stable literals, args values, source hashes, case IDs, or explicit version strings in cache keys."),
        ));
    }
    if parts.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E-CACHE-KEY-NONDETERMINISTIC",
            line,
            "`cache_key` must serialize to at least one deterministic part.",
            Some("Use a form such as `cache_key = [args.region, args.year]`."),
        ));
    }
}

fn serialize_cache_key(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

fn option_value<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a str> {
    option_for_key(options, key).map(|option| option.value.as_str())
}

fn option_for_key<'a>(options: &'a [WithOptionInfo], key: &str) -> Option<&'a WithOptionInfo> {
    options
        .iter()
        .find(|option| option.key == key && option.status == "accepted")
}

fn cache_ttl_policy(
    options: &[WithOptionInfo],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let option = option_for_key(options, "cache_ttl")?;
    match normalize_cache_ttl(&option.value) {
        Ok(ttl) => Some(ttl),
        Err(message) => {
            diagnostics.push(Diagnostic::error(
                "E-CACHE-TTL",
                option.line,
                &format!("Cache TTL `{}` is invalid.", option.value.trim()),
                Some(&message),
            ));
            None
        }
    }
}

fn normalize_cache_ttl(value: &str) -> Result<String, String> {
    let (amount, unit) = parse_number_with_suffix(value)
        .ok_or_else(|| "Use a TTL such as `30 s`, `10 min`, `1 h`, or `7 d`.".to_owned())?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err("Use a positive finite cache TTL duration.".to_owned());
    }
    let unit = unit.unwrap_or("s").to_ascii_lowercase();
    let seconds = match unit.as_str() {
        "ms" | "msec" | "millisecond" | "milliseconds" => amount / 1000.0,
        "s" | "sec" | "secs" | "second" | "seconds" => amount,
        "m" | "min" | "mins" | "minute" | "minutes" => amount * 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => amount * 3600.0,
        "d" | "day" | "days" => amount * 86_400.0,
        _ => {
            return Err("Supported cache TTL units are ms, s, min, h, and d.".to_owned());
        }
    };
    Ok(format!("{} s", format_duration_number(seconds)))
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

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
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

fn hash_text(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
