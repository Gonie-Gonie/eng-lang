use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use eng_lsp::{
    completion_items_for_path_position, completion_items_for_source_position, completion_json,
    diagnostic_json, document_symbols_lsp_json, folding_ranges_lsp_json, hover_json,
    semantic_legend, semantic_tokens_lsp_json, snapshot_for_path, snapshot_for_source,
    LSP_SNAPSHOT_FORMAT,
};
use serde_json::{json, Value};

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--smoke") {
        return command_smoke();
    }
    if args.first().map(String::as_str) == Some("--snapshot") {
        return command_snapshot(args.get(1));
    }
    if args.first().map(String::as_str) == Some("--snapshot-stdin") {
        return command_snapshot_stdin(args.get(1));
    }
    if args.first().map(String::as_str) == Some("--snapshot-check") {
        return command_snapshot_check(args.get(1));
    }
    if args.first().map(String::as_str) == Some("--completion") {
        return command_completion(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--completion-stdin") {
        return command_completion_stdin(args.get(1), args.get(2), args.get(3));
    }

    match run_lsp() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("eng-lsp failed: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_smoke() -> std::process::ExitCode {
    let path = Path::new("examples/official/01_csv_plot/main.eng");
    match snapshot_for_path(path) {
        Ok(snapshot) => {
            let domain_path = Path::new("examples/internal/06_domain_port/main.eng");
            if !domain_path.exists() {
                println!(
                    "EngLang LSP public package smoke OK: {} diagnostic(s), {} completion(s), {} hover item(s)",
                    snapshot.diagnostics.len(),
                    snapshot.completions.len(),
                    snapshot.hovers.len()
                );
                return std::process::ExitCode::SUCCESS;
            }
            let domain_snapshot = match snapshot_for_path(domain_path) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    eprintln!("EngLang LSP smoke failed: {error}");
                    return std::process::ExitCode::from(1);
                }
            };
            let domain_hover_count = domain_snapshot
                .hovers
                .iter()
                .filter(|hover| {
                    matches!(
                        hover.kind.as_str(),
                        "domain"
                            | "domain_variable"
                            | "component"
                            | "component_port"
                            | "connection"
                    )
                })
                .count();
            if domain_hover_count == 0
                || !domain_snapshot
                    .completions
                    .iter()
                    .any(|completion| completion.label == "RoomBoundary.heat")
                || !domain_snapshot.hovers.iter().any(|hover| {
                    hover.kind == "component_port"
                        && hover.name == "SupplyPipe.inlet"
                        && hover.detail.contains("type Fluid[Water]")
                        && hover.detail.contains("domain Fluid")
                        && hover.detail.contains("medium Water")
                })
            {
                eprintln!(
                    "EngLang LSP smoke failed: {} produced incomplete domain/component LSP metadata",
                    domain_path.display()
                );
                return std::process::ExitCode::from(2);
            }
            println!(
                "EngLang LSP smoke OK: {} diagnostic(s), {} completion(s), {} hover item(s), {} domain hover item(s)",
                snapshot.diagnostics.len(),
                snapshot.completions.len(),
                snapshot.hovers.len(),
                domain_hover_count
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("EngLang LSP smoke failed: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_snapshot(path: Option<&String>) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --snapshot <file.eng>");
        return std::process::ExitCode::from(2);
    };
    match snapshot_for_path(Path::new(path)) {
        Ok(snapshot) => {
            println!("{}", eng_lsp::snapshot_json(&snapshot));
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_snapshot_stdin(path: Option<&String>) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --snapshot-stdin <file.eng>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let snapshot = snapshot_for_source(Path::new(path), &source);
    println!("{}", eng_lsp::snapshot_json(&snapshot));
    std::process::ExitCode::SUCCESS
}

fn command_snapshot_check(path: Option<&String>) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --snapshot-check <file.eng>");
        return std::process::ExitCode::from(2);
    };
    match snapshot_for_path(Path::new(path)) {
        Ok(snapshot) => {
            if snapshot.completions.is_empty() || snapshot.hovers.is_empty() {
                eprintln!("EngLang LSP snapshot check failed: expected completion and hover data");
                return std::process::ExitCode::from(2);
            }
            println!(
                "EngLang LSP snapshot OK: {} diagnostic(s), {} completion(s), {} hover item(s)",
                snapshot.diagnostics.len(),
                snapshot.completions.len(),
                snapshot.hovers.len()
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_completion(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --completion <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --completion <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    match completion_items_for_path_position(Path::new(path), line, character) {
        Ok(items) => {
            println!("{}", completion_payload_json(items));
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_completion_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --completion-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --completion-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let items = completion_items_for_source_position(Path::new(path), &source, line, character);
    println!("{}", completion_payload_json(items));
    std::process::ExitCode::SUCCESS
}

fn parse_position(line: Option<&String>, character: Option<&String>) -> Option<(usize, usize)> {
    Some((
        line?.parse::<usize>().ok()?,
        character?.parse::<usize>().ok()?,
    ))
}

fn completion_payload_json(items: Vec<eng_lsp::LspCompletion>) -> Value {
    json!({
        "format": LSP_SNAPSHOT_FORMAT,
        "completions": items.iter().map(completion_json).collect::<Vec<_>>()
    })
}

fn run_lsp() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut documents = HashMap::<String, String>::new();

    while let Some(message) = read_lsp_message(&mut input)? {
        let request = match serde_json::from_str::<Value>(&message) {
            Ok(value) => value,
            Err(error) => {
                write_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": { "code": -32700, "message": error.to_string() }
                    }),
                )?;
                continue;
            }
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let id = request.get("id").cloned();

        match method {
            "initialize" => {
                let legend = semantic_legend();
                write_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": 1,
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "documentSymbolProvider": true,
                                "foldingRangeProvider": true,
                                "completionProvider": {
                                    "triggerCharacters": [" ", ":", "[", "."]
                                },
                                "semanticTokensProvider": {
                                    "legend": {
                                        "tokenTypes": legend.token_types,
                                        "tokenModifiers": legend.token_modifiers
                                    },
                                    "full": true,
                                    "range": false
                                }
                            },
                            "serverInfo": {
                                "name": "eng-lsp",
                                "version": env!("CARGO_PKG_VERSION")
                            }
                        }
                    }),
                )?;
            }
            "shutdown" => {
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null }),
                )?;
            }
            "exit" => break,
            "initialized" => {}
            "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
                if let Some((uri, text)) = document_text_from_notification(&request, &documents) {
                    documents.insert(uri.clone(), text.clone());
                    publish_diagnostics(&mut output, &uri, &text)?;
                }
            }
            "textDocument/completion" => {
                let items = completions_for_request(&request, &documents)
                    .iter()
                    .map(completion_json)
                    .collect::<Vec<_>>();
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": items }),
                )?;
            }
            "textDocument/hover" => {
                let hover = hover_for_request(&request, &documents)
                    .map(|hover| hover_json(&hover))
                    .unwrap_or(Value::Null);
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": hover }),
                )?;
            }
            "textDocument/definition" => {
                let definition =
                    definition_for_request(&request, &documents).unwrap_or(Value::Null);
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": definition }),
                )?;
            }
            "textDocument/semanticTokens/full" => {
                let tokens = semantic_tokens_for_request(&request, &documents)
                    .map(|tokens| semantic_tokens_lsp_json(&tokens))
                    .unwrap_or_else(|| json!({ "data": [] }));
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": tokens }),
                )?;
            }
            "textDocument/documentSymbol" => {
                let symbols = snapshot_for_request(&request, &documents)
                    .map(|snapshot| document_symbols_lsp_json(&snapshot.document_symbols))
                    .unwrap_or_else(|| json!([]));
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": symbols }),
                )?;
            }
            "textDocument/foldingRange" => {
                let ranges = snapshot_for_request(&request, &documents)
                    .map(|snapshot| folding_ranges_lsp_json(&snapshot.folding_ranges))
                    .unwrap_or_else(|| json!([]));
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": ranges }),
                )?;
            }
            _ if id.is_some() => {
                write_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("unsupported method {method}") }
                    }),
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn publish_diagnostics<W: Write>(output: &mut W, uri: &str, text: &str) -> io::Result<()> {
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let snapshot = snapshot_for_source(&path, text);
    let diagnostics = snapshot
        .diagnostics
        .iter()
        .map(diagnostic_json)
        .collect::<Vec<_>>();
    write_response(
        output,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": diagnostics
            }
        }),
    )
}

fn semantic_tokens_for_request(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Option<eng_lsp::LspSemanticTokens> {
    Some(snapshot_for_request(request, documents)?.semantic_tokens)
}

fn snapshot_for_request(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Option<eng_lsp::LspSnapshot> {
    let uri = request_uri(request)?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let text = documents
        .get(uri)
        .cloned()
        .or_else(|| std::fs::read_to_string(&path).ok())?;
    Some(snapshot_for_source(&path, &text))
}

fn completions_for_request(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Vec<eng_lsp::LspCompletion> {
    let Some(uri) = request_uri(request) else {
        return Vec::new();
    };
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let line = request
        .pointer("/params/position/line")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let character = request
        .pointer("/params/position/character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if let Some(text) = documents.get(uri) {
        return completion_items_for_source_position(&path, text, line, character);
    }
    completion_items_for_path_position(&path, line, character).unwrap_or_default()
}

fn hover_for_request(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Option<eng_lsp::LspHover> {
    let uri = request_uri(request)?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let line_zero_based = request
        .pointer("/params/position/line")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let character = request
        .pointer("/params/position/character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let text = documents
        .get(uri)
        .cloned()
        .or_else(|| std::fs::read_to_string(&path).ok());
    let snapshot = text
        .as_deref()
        .map(|text| snapshot_for_source(&path, text))
        .or_else(|| snapshot_for_path(&path).ok())?;
    if let Some(symbol) = text
        .as_deref()
        .and_then(|text| symbol_at_position(text, line_zero_based, character))
    {
        if let Some(hover) = hover_for_symbol(&snapshot.hovers, &symbol).cloned() {
            return Some(hover);
        }
    }
    let line = line_zero_based + 1;
    snapshot.hovers.into_iter().find(|hover| hover.line == line)
}

fn definition_for_request(request: &Value, documents: &HashMap<String, String>) -> Option<Value> {
    let uri = request_uri(request)?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let line_zero_based = request
        .pointer("/params/position/line")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let character = request
        .pointer("/params/position/character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let text = documents
        .get(uri)
        .cloned()
        .or_else(|| std::fs::read_to_string(&path).ok())?;
    let snapshot = snapshot_for_source(&path, &text);
    let symbol = symbol_at_position(&text, line_zero_based, character)?;
    let hover = hover_for_symbol(&snapshot.hovers, &symbol)?;
    let definition_line = hover.line.saturating_sub(1);
    let line_text = text.lines().nth(definition_line)?;
    let label = hover.name.rsplit('.').next().unwrap_or(&hover.name);
    let (start_character, end_character) = definition_character_range(line_text, label)?;
    Some(json!({
        "uri": uri,
        "range": {
            "start": { "line": definition_line, "character": start_character },
            "end": { "line": definition_line, "character": end_character }
        }
    }))
}

fn hover_for_symbol<'a>(
    hovers: &'a [eng_lsp::LspHover],
    symbol: &str,
) -> Option<&'a eng_lsp::LspHover> {
    let symbol_label = symbol.rsplit('.').next().unwrap_or(symbol);
    hovers
        .iter()
        .find(|hover| hover.name == symbol)
        .or_else(|| {
            hovers.iter().find(|hover| {
                hover
                    .name
                    .rsplit('.')
                    .next()
                    .is_some_and(|hover_label| hover_label == symbol_label)
            })
        })
}

fn symbol_at_position(source: &str, line: usize, character: usize) -> Option<String> {
    let line_text = source.lines().nth(line)?;
    let bytes = line_text.as_bytes();
    let mut cursor = utf16_character_to_byte(line_text, character);
    while cursor > 0 && !is_symbol_byte(bytes[cursor.saturating_sub(1)]) {
        cursor -= 1;
    }
    let mut start = cursor;
    while start > 0 && is_symbol_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = cursor;
    while end < bytes.len() && is_symbol_byte(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(line_text[start..end].trim_matches('.').to_owned())
}

fn is_symbol_byte(byte: u8) -> bool {
    byte == b'.' || byte == b'_' || byte.is_ascii_alphanumeric()
}

fn definition_character_range(line_text: &str, label: &str) -> Option<(usize, usize)> {
    let (start_byte, end_byte) = find_identifier_byte_range(line_text, label)?;
    Some((
        utf16_len(&line_text[..start_byte]),
        utf16_len(&line_text[..end_byte]),
    ))
}

fn find_identifier_byte_range(line_text: &str, label: &str) -> Option<(usize, usize)> {
    if label.is_empty() {
        return None;
    }
    let mut search_start = 0;
    while search_start <= line_text.len() {
        let offset = line_text[search_start..].find(label)?;
        let start = search_start + offset;
        let end = start + label.len();
        if has_identifier_boundaries(line_text, start, end) {
            return Some((start, end));
        }
        search_start = end;
    }
    None
}

fn has_identifier_boundaries(line_text: &str, start: usize, end: usize) -> bool {
    let bytes = line_text.as_bytes();
    let before_ok = start == 0
        || bytes
            .get(start.saturating_sub(1))
            .is_none_or(|byte| !is_identifier_byte(*byte));
    let after_ok = bytes.get(end).is_none_or(|byte| !is_identifier_byte(*byte));
    before_ok && after_ok
}

fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn utf16_character_to_byte(line_text: &str, character: usize) -> usize {
    let mut units = 0;
    for (byte_index, ch) in line_text.char_indices() {
        if units >= character {
            return byte_index;
        }
        units += ch.len_utf16();
    }
    line_text.len()
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn document_text_from_notification(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Option<(String, String)> {
    let uri = request_uri(request)?.to_owned();
    if let Some(text) = request
        .pointer("/params/textDocument/text")
        .and_then(Value::as_str)
    {
        return Some((uri, text.to_owned()));
    }
    if let Some(text) = request
        .pointer("/params/contentChanges/0/text")
        .and_then(Value::as_str)
    {
        return Some((uri, text.to_owned()));
    }
    if let Some(text) = documents.get(&uri) {
        return Some((uri, text.clone()));
    }
    let path = path_from_uri(&uri)?;
    std::fs::read_to_string(path).ok().map(|text| (uri, text))
}

fn request_uri(request: &Value) -> Option<&str> {
    request
        .pointer("/params/textDocument/uri")
        .and_then(Value::as_str)
}

fn path_from_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let decoded = percent_decode(rest);
    let path = if decoded.starts_with('/') && decoded.as_bytes().get(2) == Some(&b':') {
        decoded.trim_start_matches('/').replace('/', "\\")
    } else {
        decoded.replace('/', "\\")
    };
    Some(PathBuf::from(path))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn read_lsp_message<R: Read>(input: &mut R) -> io::Result<Option<String>> {
    let mut headers = Vec::new();
    let mut byte = [0u8; 1];
    while input.read(&mut byte)? == 1 {
        headers.push(byte[0]);
        if headers.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    if headers.is_empty() {
        return Ok(None);
    }
    let headers = String::from_utf8_lossy(&headers);
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0u8; content_length];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn write_response<W: Write>(output: &mut W, value: Value) -> io::Result<()> {
    let body = value.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    output.flush()
}
