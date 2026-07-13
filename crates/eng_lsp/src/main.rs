use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use eng_compiler::{bundled_module_registry, format_source, parse_source, AstItem};
use eng_lsp::{
    completion_items_for_path_position, completion_items_for_source_position, completion_json,
    diagnostic_json, document_symbols_lsp_json, editor_metadata_json, folding_ranges_lsp_json,
    hover_json, semantic_legend, semantic_tokens_lsp_json, snapshot_for_path, snapshot_for_source,
    workflow_option_label_exists, LSP_SNAPSHOT_FORMAT,
};
use serde_json::{json, Value};

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--smoke") {
        return command_smoke();
    }
    if args.first().map(String::as_str) == Some("--editor-metadata") {
        return command_editor_metadata();
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
    if args.first().map(String::as_str) == Some("--format-stdin") {
        return command_format_stdin(args.get(1));
    }
    if args.first().map(String::as_str) == Some("--code-actions-stdin") {
        return command_code_actions_stdin(args.get(1));
    }
    if args.first().map(String::as_str) == Some("--completion") {
        return command_completion(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--completion-stdin") {
        return command_completion_stdin(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--definition-stdin") {
        return command_definition_stdin(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--workspace-symbols") {
        return command_workspace_symbols(args.get(1), args.get(2));
    }

    match run_lsp() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("eng-lsp failed: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn command_editor_metadata() -> std::process::ExitCode {
    println!("{}", editor_metadata_json());
    std::process::ExitCode::SUCCESS
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

fn command_format_stdin(path: Option<&String>) -> std::process::ExitCode {
    let Some(_path) = path else {
        eprintln!("usage: eng-lsp --format-stdin <file.eng>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let result = format_source(&source);
    println!(
        "{}",
        json!({
            "format": LSP_SNAPSHOT_FORMAT,
            "formatted": result.formatted,
            "changed": result.changed
        })
    );
    std::process::ExitCode::SUCCESS
}

fn command_code_actions_stdin(path: Option<&String>) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --code-actions-stdin <file.eng>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let path = Path::new(path);
    let uri = file_uri_from_path(path);
    let snapshot = snapshot_for_source(path, &source);
    let diagnostics = snapshot
        .diagnostics
        .iter()
        .map(diagnostic_json)
        .collect::<Vec<_>>();
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "context": { "diagnostics": diagnostics }
        }
    });
    let mut documents = Documents::new();
    documents.insert(uri.clone(), DocumentState::new(source, None));
    println!(
        "{}",
        json!({
            "format": LSP_SNAPSHOT_FORMAT,
            "uri": uri,
            "actions": code_actions_for_request(&request, &documents)
        })
    );
    std::process::ExitCode::SUCCESS
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

fn command_definition_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --definition-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --definition-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }

    let path = Path::new(path);
    let uri = file_uri_from_path(path);
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    let mut documents = Documents::new();
    documents.insert(uri, DocumentState::new(source, None));
    println!(
        "{}",
        definition_for_request(&request, &documents).unwrap_or(Value::Null)
    );
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

fn command_workspace_symbols(
    root: Option<&String>,
    query: Option<&String>,
) -> std::process::ExitCode {
    let Some(root) = root else {
        eprintln!("usage: eng-lsp --workspace-symbols <workspace-root> [query]");
        return std::process::ExitCode::from(2);
    };
    let root = Path::new(root)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(root));
    let request = json!({
        "params": {
            "query": query.map(String::as_str).unwrap_or("")
        }
    });
    let symbols = workspace_symbols_for_request(&request, &Documents::new(), &[root]);
    println!(
        "{}",
        json!({
            "format": LSP_SNAPSHOT_FORMAT,
            "symbols": symbols
        })
    );
    std::process::ExitCode::SUCCESS
}

#[derive(Clone, Debug)]
struct DocumentState {
    text: String,
    version: Option<i64>,
}

impl DocumentState {
    fn new(text: String, version: Option<i64>) -> Self {
        Self { text, version }
    }
}

type Documents = HashMap<String, DocumentState>;
fn run_lsp() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut documents = Documents::new();
    let mut workspace_roots = Vec::<PathBuf>::new();

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
                workspace_roots = workspace_roots_from_initialize(&request);
                let legend = semantic_legend();
                write_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": {
                                    "openClose": true,
                                    "change": 1,
                                    "save": { "includeText": true }
                                },
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "documentSymbolProvider": true,
                                "workspaceSymbolProvider": true,
                                "foldingRangeProvider": true,
                                "documentFormattingProvider": true,
                                "documentRangeFormattingProvider": true,
                                "completionProvider": {
                                    "triggerCharacters": [" ", ":", "[", "."]
                                },
                                "codeActionProvider": {
                                    "codeActionKinds": ["quickfix"],
                                    "resolveProvider": false
                                },
                                "semanticTokensProvider": {
                                    "legend": {
                                        "tokenTypes": legend.token_types,
                                        "tokenModifiers": legend.token_modifiers
                                    },
                                    "full": true,
                                    "range": true
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
                if let Some((uri, state)) = document_state_from_notification(&request, &documents) {
                    documents.insert(uri.clone(), state.clone());
                    publish_diagnostics(&mut output, &uri, &state)?;
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
            "textDocument/codeAction" => {
                let actions = code_actions_for_request(&request, &documents);
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": actions }),
                )?;
            }
            "textDocument/formatting" => {
                let edits = formatting_edits_for_request(&request, &documents);
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": edits }),
                )?;
            }
            "textDocument/rangeFormatting" => {
                let edits = range_formatting_edits_for_request(&request, &documents);
                write_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": edits }),
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
            "textDocument/semanticTokens/range" => {
                let tokens = semantic_tokens_range_for_request(&request, &documents)
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
            "workspace/symbol" => {
                let symbols = workspace_symbols_for_request(&request, &documents, &workspace_roots);
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

fn publish_diagnostics<W: Write>(
    output: &mut W,
    uri: &str,
    state: &DocumentState,
) -> io::Result<()> {
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let snapshot = snapshot_for_source(&path, &state.text);
    let diagnostics = snapshot
        .diagnostics
        .iter()
        .map(diagnostic_json)
        .collect::<Vec<_>>();
    let mut params = serde_json::Map::new();
    params.insert("uri".to_owned(), json!(uri));
    params.insert("diagnostics".to_owned(), json!(diagnostics));
    if let Some(version) = state.version {
        params.insert("version".to_owned(), json!(version));
    }
    write_response(
        output,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": Value::Object(params)
        }),
    )
}

fn semantic_tokens_for_request(
    request: &Value,
    documents: &Documents,
) -> Option<eng_lsp::LspSemanticTokens> {
    Some(snapshot_for_request(request, documents)?.semantic_tokens)
}

fn formatting_edits_for_request(request: &Value, documents: &Documents) -> Vec<Value> {
    let Some(uri) = request_uri(request) else {
        return Vec::new();
    };
    let Some(text) = document_text_for_uri(uri, documents) else {
        return Vec::new();
    };
    let result = format_source(&text);
    if !result.changed {
        return Vec::new();
    }
    vec![json!({
        "range": full_document_range(&text),
        "newText": result.formatted
    })]
}

fn range_formatting_edits_for_request(request: &Value, documents: &Documents) -> Vec<Value> {
    let Some(uri) = request_uri(request) else {
        return Vec::new();
    };
    let Some(text) = document_text_for_uri(uri, documents) else {
        return Vec::new();
    };
    let Some(((start_line, _start_character), (end_line, end_character))) = request_range(request)
    else {
        return Vec::new();
    };
    let result = format_source(&text);
    if !result.changed {
        return Vec::new();
    }
    let original_lines = split_lines_preserve_logical(&text);
    let formatted_lines = split_lines_preserve_logical(&result.formatted);
    if original_lines.len() != formatted_lines.len() {
        return Vec::new();
    }
    let Some((format_start_line, format_end_line)) =
        selected_line_range(start_line, end_line, end_character, original_lines.len())
    else {
        return Vec::new();
    };
    let newline = document_newline(&result.formatted);
    let formatted_selection = formatted_lines[format_start_line..=format_end_line].join(newline);
    let original_selection = original_lines[format_start_line..=format_end_line].join(newline);
    if formatted_selection == original_selection {
        return Vec::new();
    }
    vec![json!({
        "range": full_line_selection_range(&original_lines, format_start_line, format_end_line),
        "newText": formatted_selection
    })]
}

fn code_actions_for_request(request: &Value, documents: &Documents) -> Vec<Value> {
    let Some(uri) = request_uri(request) else {
        return Vec::new();
    };
    let Some(text) = document_text_for_uri(uri, documents) else {
        return Vec::new();
    };
    let Some(diagnostics) = request
        .pointer("/params/context/diagnostics")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    diagnostics
        .iter()
        .flat_map(|diagnostic| code_actions_for_diagnostic(uri, &text, diagnostic))
        .collect()
}

fn code_actions_for_diagnostic(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(code) = diagnostic_code(diagnostic) else {
        return Vec::new();
    };
    match code {
        "E-SYNTAX-DECL-001" => optional_code_action(lsp_replacement_code_action(
            uri,
            text,
            diagnostic,
            ":=",
            "=",
            "Replace := with =",
        )),
        "E-STRUCT-ARGS-001" => optional_code_action(lsp_replacement_code_action(
            uri,
            text,
            diagnostic,
            "struct Args",
            "args",
            "Replace struct Args with args",
        )),
        "E-EQ-BOOL-001" => optional_code_action(lsp_replacement_code_action(
            uri,
            text,
            diagnostic,
            "==",
            "eq",
            "Replace == with eq",
        )),
        "E-SCRIPT-001" => {
            optional_code_action(lsp_remove_script_wrapper_code_action(uri, text, diagnostic))
        }
        "W-QTY-AMBIG-001" => lsp_quantity_annotation_code_actions(uri, text, diagnostic),
        "W-STATS-SUM-001" => {
            optional_code_action(lsp_heat_rate_sum_code_action(uri, text, diagnostic))
        }
        code if code.starts_with("E-DIM-ADD-") => {
            lsp_missing_unit_code_actions(uri, text, diagnostic)
        }
        "E-PUBLIC-ANNOTATION-001" => {
            optional_code_action(lsp_schema_annotation_code_action(uri, text, diagnostic))
        }
        "E-FS-CONFIRM-001" => {
            optional_code_action(lsp_file_mutation_confirm_code_action(uri, text, diagnostic))
        }
        "E-FS-DELETE-001" => {
            optional_code_action(lsp_recursive_delete_code_action(uri, text, diagnostic))
        }
        "E-SAMPLING-SEED-MISSING" => {
            optional_code_action(lsp_sampling_seed_missing_code_action(uri, text, diagnostic))
        }
        "W-WITH-UNCERTAINTY-SEED-001" => optional_code_action(
            lsp_uncertainty_seed_missing_code_action(uri, text, diagnostic),
        ),
        "E-NET-INVALID-URL" => {
            optional_code_action(lsp_absolute_http_url_code_action(uri, text, diagnostic))
        }
        "E-NET-BODY-METHOD" => {
            optional_code_action(lsp_http_body_method_code_action(uri, text, diagnostic))
        }
        "E-NET-HASH-MISMATCH" => {
            optional_code_action(lsp_expected_sha256_code_action(uri, text, diagnostic))
        }
        "W-NET-FIXTURE-ALIAS" => optional_code_action(lsp_option_key_replacement_code_action(
            uri,
            text,
            diagnostic,
            "fixture",
            "offline_response",
            "Rename fixture to offline_response",
        )),
        "W-NET-RESPONSE-HASH-ALIAS" => {
            optional_code_action(lsp_diagnostic_range_replacement_code_action(
                uri,
                diagnostic,
                "response_hash",
                "Rename hash to response_hash",
            ))
        }
        "W-NET-RESPONSE-STATUS-ALIAS" => {
            optional_code_action(lsp_diagnostic_range_replacement_code_action(
                uri,
                diagnostic,
                "response_source",
                "Rename status to response_source",
            ))
        }
        "W-TABLE-LEGACY-SELECT-FIRST-ROW" => optional_code_action(
            lsp_select_first_row_migration_code_action(uri, text, diagnostic),
        ),
        "E-IO-JSON-FIELD-ACCESS-001" => {
            optional_code_action(lsp_json_read_promotion_code_action(uri, text, diagnostic))
        }
        "E-WITH-OPTION-001" => {
            optional_code_action(lsp_with_option_alias_code_action(uri, text, diagnostic))
        }
        "E-WITH-UNIT-001" => optional_code_action(
            lsp_remove_incompatible_display_unit_code_action(uri, text, diagnostic),
        ),
        "E-WRITE-002" => lsp_unsupported_write_format_code_actions(uri, text, diagnostic),
        "E-WRITE-STANDARD-TEXT-001" => optional_code_action(lsp_replacement_code_action(
            uri,
            text,
            diagnostic,
            "write standard_text",
            "write text",
            "Change writer to text",
        )),
        "E-WRITE-STANDARD-TEXT-OUTPUT" => optional_code_action(
            lsp_write_standard_text_output_code_action(uri, text, diagnostic),
        ),
        "E-PRINT-FMT-001" | "E-WRITE-FMT-001" => optional_code_action(
            lsp_close_unterminated_interpolation_code_action(uri, text, diagnostic),
        ),
        "E-PRINT-FMT-002" | "E-WRITE-FMT-002" => optional_code_action(
            lsp_remove_empty_interpolation_code_action(uri, text, diagnostic),
        ),
        "E-PRINT-FMT-003" | "E-WRITE-FMT-003" => optional_code_action(
            lsp_remove_interpolation_display_unit_code_action(uri, text, diagnostic),
        ),
        "E-PRINT-FMT-004" | "E-WRITE-FMT-004" => optional_code_action(
            lsp_convert_unresolved_interpolation_code_action(uri, text, diagnostic),
        ),
        "E-LOG-LEVEL-001" => {
            optional_code_action(lsp_log_level_info_code_action(uri, text, diagnostic))
        }
        "E-REPORT-BINDING-001"
        | "E-VALIDATE-BINDING-001"
        | "E-SIDE-EFFECT-BINDING-001"
        | "E-BLOCK-BINDING-001"
        | "E-STATEMENT-BINDING-001"
        | "E-OPTION-BINDING-001" => {
            optional_code_action(lsp_statement_only_unbind_code_action(uri, text, diagnostic))
        }
        "E-PROCESS-BINDING-001" => {
            optional_code_action(lsp_bind_process_result_code_action(uri, text, diagnostic))
        }
        "E-PROCESS-BINDING-002" => optional_code_action(lsp_unique_process_binding_code_action(
            uri, text, diagnostic,
        )),
        "E-PROCESS-CMD-001" => {
            optional_code_action(lsp_process_command_code_action(uri, text, diagnostic))
        }
        "E-ASSERT-001" => {
            optional_code_action(lsp_wrap_assertion_code_action(uri, text, diagnostic))
        }
        "E-GOLDEN-002" => {
            optional_code_action(lsp_golden_expected_file_code_action(uri, text, diagnostic))
        }
        "E-WHERE-FWD-001" => optional_code_action(lsp_reorder_where_local_definition_code_action(
            uri, text, diagnostic,
        )),
        "E-NAME-LOCAL-001" => {
            optional_code_action(lsp_promote_where_local_code_action(uri, text, diagnostic))
        }
        "E-ML-SOURCE-001" | "E-ML-SOURCE-002" => lsp_ml_source_code_actions(uri, text, diagnostic),
        "E-UNC-SOURCE-001" | "E-UNC-SOURCE-002" => {
            lsp_uncertainty_source_code_actions(uri, text, diagnostic)
        }
        code if code.starts_with("E-UNC-ARGS-") => {
            lsp_uncertainty_argument_code_actions(uri, text, diagnostic)
        }
        "E-UNC-DIRECT-COMPARE" => optional_code_action(lsp_uncertainty_direct_compare_code_action(
            uri, text, diagnostic,
        )),
        "E-CMD-AMBIG-001" => optional_code_action(lsp_parenthesize_command_target_code_action(
            uri, text, diagnostic,
        )),
        "E-STDLIB-MODULE-UNKNOWN" => optional_code_action(
            lsp_stdlib_module_replacement_code_action(uri, text, diagnostic),
        ),
        "W-STDLIB-MODULE-PLANNED" => optional_code_action(
            lsp_remove_stdlib_module_import_code_action(uri, text, diagnostic, "planned"),
        ),
        "W-STDLIB-MODULE-INTERNAL" => optional_code_action(
            lsp_remove_stdlib_module_import_code_action(uri, text, diagnostic, "internal"),
        ),
        code => optional_code_action(lsp_option_value_replacement_code_action(
            uri, text, diagnostic, code,
        )),
    }
}

fn optional_code_action(action: Option<Value>) -> Vec<Value> {
    action.into_iter().collect()
}

fn diagnostic_code(diagnostic: &Value) -> Option<&str> {
    diagnostic
        .get("code")
        .and_then(Value::as_str)
        .or_else(|| diagnostic.pointer("/code/value").and_then(Value::as_str))
}

fn diagnostic_message(diagnostic: &Value) -> &str {
    diagnostic
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn diagnostic_line(diagnostic: &Value) -> Option<usize> {
    diagnostic
        .pointer("/range/start/line")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn lsp_replacement_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    search: &str,
    replacement: &str,
    title: &str,
) -> Option<Value> {
    let line_number = diagnostic
        .pointer("/range/start/line")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())?;
    let line = text.lines().nth(line_number)?;
    let byte_start = line.find(search)?;
    let start_character = utf16_len(&line[..byte_start]);
    let end_character = start_character + utf16_len(search);
    let range = json!({
        "start": { "line": line_number, "character": start_character },
        "end": { "line": line_number, "character": end_character }
    });
    Some(json!({
        "title": title,
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, replacement)
    }))
}

fn lsp_unsupported_write_format_code_actions(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Vec<Value> {
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let Some((start_byte, end_byte, current_format)) = write_format_token_range(line) else {
        return Vec::new();
    };
    ["text", "json", "standard_text"]
        .into_iter()
        .filter(|replacement| *replacement != current_format)
        .map(|replacement| {
            json!({
                "title": format!("Change write format to {replacement}"),
                "kind": "quickfix",
                "isPreferred": replacement == "text",
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    line_byte_range(line_number, line, start_byte, end_byte),
                    replacement
                )
            })
        })
        .collect()
}

fn write_format_token_range(line: &str) -> Option<(usize, usize, &str)> {
    let code = strip_line_comment(line);
    let start = code.len() - code.trim_start().len();
    let rest = &code[start..];
    if !rest.starts_with("write") {
        return None;
    }
    let mut cursor = start + "write".len();
    if cursor < code.len()
        && (code.as_bytes()[cursor].is_ascii_alphanumeric() || code.as_bytes()[cursor] == b'_')
    {
        return None;
    }
    let whitespace_start = cursor;
    while cursor < code.len() && code.as_bytes()[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor == whitespace_start {
        return None;
    }
    let format_start = cursor;
    while cursor < code.len()
        && !code.as_bytes()[cursor].is_ascii_whitespace()
        && code.as_bytes()[cursor] != b','
    {
        cursor += 1;
    }
    (cursor > format_start).then_some((format_start, cursor, &code[format_start..cursor]))
}

fn lsp_diagnostic_range_replacement_code_action(
    uri: &str,
    diagnostic: &Value,
    replacement: &str,
    title: &str,
) -> Option<Value> {
    let range = diagnostic.get("range")?.clone();
    Some(json!({
        "title": title,
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, replacement)
    }))
}

fn lsp_close_unterminated_interpolation_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let insert_byte = unterminated_interpolation_close_byte(line, diagnostic)?;
    Some(json!({
        "title": "Close interpolation with }",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, insert_byte, insert_byte),
            "}"
        )
    }))
}

fn unterminated_interpolation_close_byte(line: &str, diagnostic: &Value) -> Option<usize> {
    let open = interpolation_open_byte_index(line, diagnostic)?;
    let quote_end = unescaped_quote_byte_index_after(line, open + 1)?;
    if line[open + 1..quote_end].contains('}') {
        return None;
    }
    Some(quote_end)
}

fn interpolation_open_byte_index(line: &str, diagnostic: &Value) -> Option<usize> {
    let diagnostic_start = diagnostic
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())?;
    let start_byte = utf16_character_to_byte(line, diagnostic_start);
    if line.as_bytes().get(start_byte) == Some(&b'{') {
        return Some(start_byte);
    }
    strip_line_comment(line).find('{')
}

fn unescaped_quote_byte_index_after(line: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    for (relative_index, character) in line[start..].char_indices() {
        let index = start + relative_index;
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(index),
            _ => {}
        }
    }
    None
}

fn lsp_remove_empty_interpolation_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let (start_byte, end_byte) = empty_interpolation_range(line, diagnostic)?;
    Some(json!({
        "title": "Remove empty interpolation",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            ""
        )
    }))
}

fn empty_interpolation_range(line: &str, diagnostic: &Value) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let ranges = empty_interpolation_ranges(code);
    if ranges.is_empty() {
        return None;
    }
    let diagnostic_start = diagnostic
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    if let Some(diagnostic_start) = diagnostic_start {
        if let Some(range) = ranges.iter().copied().find(|(start, end)| {
            let start_character = utf16_len(&line[..*start]);
            let end_character = utf16_len(&line[..*end]);
            diagnostic_start >= start_character && diagnostic_start <= end_character
        }) {
            return Some(range);
        }
    }
    ranges.first().copied()
}

fn empty_interpolation_ranges(code: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor < code.len() {
        let Some(relative_open) = code[cursor..].find('{') else {
            break;
        };
        let open = cursor + relative_open;
        let Some(relative_close) = code[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + relative_close;
        if code[open + 1..close].trim().is_empty() {
            ranges.push((open, close + 1));
        }
        cursor = close + 1;
    }
    ranges
}

fn lsp_remove_interpolation_display_unit_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let unit = last_backtick_payload(diagnostic_message(diagnostic))?.trim();
    let (start_byte, end_byte) = interpolation_unit_removal_range(line, unit, diagnostic)?;
    Some(json!({
        "title": "Remove incompatible interpolation unit",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            ""
        )
    }))
}

fn interpolation_unit_removal_range(
    line: &str,
    unit: &str,
    diagnostic: &Value,
) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let ranges = interpolation_unit_removal_ranges(code, unit);
    if ranges.is_empty() {
        return None;
    }
    let diagnostic_start = diagnostic
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    if let Some(diagnostic_start) = diagnostic_start {
        if let Some(range) = ranges.iter().copied().find(|(start, end)| {
            let start_character = utf16_len(&line[..*start]);
            let end_character = utf16_len(&line[..*end]);
            diagnostic_start >= start_character && diagnostic_start <= end_character
        }) {
            return Some(range);
        }
    }
    ranges.first().copied()
}

fn interpolation_unit_removal_ranges(code: &str, unit: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor < code.len() {
        let Some(relative_open) = code[cursor..].find('{') else {
            break;
        };
        let open = cursor + relative_open;
        let Some(relative_close) = code[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + relative_close;
        let inside = &code[open + 1..close];
        if let Some(colon) = inside.find(':') {
            let colon_index = open + 1 + colon;
            let spec_start = colon_index + 1;
            let spec = &code[spec_start..close];
            if spec.trim() == unit {
                ranges.push((colon_index, close));
            } else if let Some((unit_start, unit_end)) = trailing_unit_in_format_spec(spec, unit) {
                ranges.push((spec_start + unit_start, spec_start + unit_end));
            }
        }
        cursor = close + 1;
    }
    ranges
}

fn trailing_unit_in_format_spec(spec: &str, unit: &str) -> Option<(usize, usize)> {
    let trimmed_end = spec.trim_end();
    if !trimmed_end.ends_with(unit) {
        return None;
    }
    let unit_start = trimmed_end.len().checked_sub(unit.len())?;
    if !format_spec_prefix_can_stand_without_unit(&spec[..unit_start]) {
        return None;
    }
    Some((unit_start, spec.len()))
}

fn format_spec_prefix_can_stand_without_unit(prefix: &str) -> bool {
    let trimmed = prefix.trim();
    let Some(rest) = trimmed.strip_prefix('.') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|value| value.is_ascii_digit())
}

fn lsp_convert_unresolved_interpolation_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let expression = first_backtick_payload(diagnostic_message(diagnostic))?.trim();
    let edit = unresolved_interpolation_literal_edit(line, expression, diagnostic)?;
    Some(json!({
        "title": "Convert unresolved interpolation to literal text",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, edit.start_byte, edit.end_byte),
            &edit.new_text
        )
    }))
}

struct InterpolationLiteralEdit {
    start_byte: usize,
    end_byte: usize,
    new_text: String,
}

fn unresolved_interpolation_literal_edit(
    line: &str,
    expression: &str,
    diagnostic: &Value,
) -> Option<InterpolationLiteralEdit> {
    let code = strip_line_comment(line);
    let ranges = interpolation_literal_edits(code, expression);
    if ranges.is_empty() {
        return None;
    }
    let diagnostic_start = diagnostic
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    if let Some(diagnostic_start) = diagnostic_start {
        if let Some(index) = ranges.iter().position(|edit| {
            let start_character = utf16_len(&line[..edit.start_byte]);
            let end_character = utf16_len(&line[..edit.end_byte]);
            diagnostic_start >= start_character && diagnostic_start <= end_character
        }) {
            return ranges.into_iter().nth(index);
        }
    }
    ranges.into_iter().next()
}

fn interpolation_literal_edits(code: &str, expression: &str) -> Vec<InterpolationLiteralEdit> {
    let mut edits = Vec::new();
    let mut cursor = 0usize;
    while cursor < code.len() {
        let Some(relative_open) = code[cursor..].find('{') else {
            break;
        };
        let open = cursor + relative_open;
        let Some(relative_close) = code[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + relative_close;
        let inside = code[open + 1..close].trim();
        let expression_part = inside
            .split_once(':')
            .map_or(inside, |(candidate, _)| candidate)
            .trim();
        if !inside.is_empty() && expression_part == expression.trim() {
            edits.push(InterpolationLiteralEdit {
                start_byte: open,
                end_byte: close + 1,
                new_text: inside.to_owned(),
            });
        }
        cursor = close + 1;
    }
    edits
}

fn lsp_heat_rate_sum_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let (start_byte, end_byte) = sum_function_name_range(line)?;
    Some(json!({
        "title": "Replace sum with integrate",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            "integrate"
        )
    }))
}

fn sum_function_name_range(line: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find("sum") else {
            break;
        };
        let start = search_start + relative_start;
        let after_name = start + "sum".len();
        if identifier_boundary(code, start, after_name) {
            let whitespace = code[after_name..]
                .chars()
                .take_while(|character| character.is_whitespace())
                .map(char::len_utf8)
                .sum::<usize>();
            if code.as_bytes().get(after_name + whitespace) == Some(&b'(') {
                return Some((start, after_name));
            }
        }
        search_start = after_name;
    }
    None
}

fn lsp_quantity_annotation_code_actions(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(details) = ambiguous_quantity_details(diagnostic_message(diagnostic)) else {
        return Vec::new();
    };
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let Some((start_byte, end_byte)) = assignment_head_byte_range(line, &details.name) else {
        return Vec::new();
    };
    let range = line_byte_range(line_number, line, start_byte, end_byte);
    details
        .candidates
        .into_iter()
        .map(|candidate| {
            json!({
                "title": format!("Annotate {} as {} [{}]", details.name, candidate, details.unit),
                "kind": "quickfix",
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    range.clone(),
                    &format!("{}: {} [{}] =", details.name, candidate, details.unit)
                )
            })
        })
        .collect()
}

struct AmbiguousQuantityDetails {
    name: String,
    unit: String,
    candidates: Vec<String>,
}

fn ambiguous_quantity_details(message: &str) -> Option<AmbiguousQuantityDetails> {
    let name = first_backtick_payload(message)?.to_owned();
    let after_unit = message.split_once(" has unit ")?.1;
    let unit = after_unit
        .split(|character: char| character == ',' || character.is_whitespace())
        .next()
        .filter(|value| is_unit_hint(value))?
        .to_owned();
    let after_candidates = message.split_once("Candidate quantity kinds:")?.1;
    let candidates_text = after_candidates
        .split('.')
        .next()
        .unwrap_or(after_candidates);
    let candidates = candidates_text
        .split(',')
        .map(str::trim)
        .filter(|candidate| is_identifier(candidate))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    Some(AmbiguousQuantityDetails {
        name,
        unit,
        candidates,
    })
}

fn lsp_missing_unit_code_actions(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let Some(unit) = missing_unit_hint(diagnostic_message(diagnostic), line) else {
        return Vec::new();
    };
    bare_numeric_ranges(line)
        .into_iter()
        .map(|(start_byte, end_byte)| {
            let literal = &line[start_byte..end_byte];
            let character = utf16_len(&line[..end_byte]);
            let range = json!({
                "start": { "line": line_number, "character": character },
                "end": { "line": line_number, "character": character }
            });
            json!({
                "title": format!("Add unit {unit} to {literal}"),
                "kind": "quickfix",
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(uri, range, &format!(" {unit}"))
            })
        })
        .collect()
}

fn missing_unit_hint(message: &str, line: &str) -> Option<String> {
    for payload in backtick_payloads(message) {
        if is_unit_hint(payload) {
            return Some(payload.to_owned());
        }
    }
    first_unit_on_line(line)
}

fn first_unit_on_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                index += 1;
            }
            let mut unit_start = index;
            while unit_start < bytes.len() && bytes[unit_start].is_ascii_whitespace() {
                unit_start += 1;
            }
            let mut unit_end = unit_start;
            while unit_end < bytes.len()
                && (bytes[unit_end].is_ascii_alphanumeric()
                    || matches!(bytes[unit_end], b'%' | b'/' | b'_'))
            {
                unit_end += 1;
            }
            if unit_end > unit_start {
                let unit = &line[unit_start..unit_end];
                if is_unit_hint(unit) {
                    return Some(unit.to_owned());
                }
            }
        } else {
            index += 1;
        }
    }
    if let Some(open) = line.find('[') {
        if let Some(close) = line[open + 1..].find(']') {
            let unit = line[open + 1..open + 1 + close].trim();
            if is_unit_hint(unit) {
                return Some(unit.to_owned());
            }
        }
    }
    None
}

fn is_unit_hint(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '%')
        && chars.all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '%' | '/' | '_' | '^' | '(' | ')' | '*')
        })
}

fn bare_numeric_ranges(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut ranges = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        if index > 0 {
            let previous = bytes[index - 1];
            if previous.is_ascii_alphanumeric() || previous == b'_' || previous == b'.' {
                index += 1;
                continue;
            }
        }
        let previous_non_space = line[..index]
            .chars()
            .rev()
            .find(|character| !character.is_whitespace());
        if !matches!(
            previous_non_space,
            None | Some('=')
                | Some('+')
                | Some('-')
                | Some('*')
                | Some('/')
                | Some('(')
                | Some(',')
        ) {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            let decimal = index + 1;
            if decimal < bytes.len() && bytes[decimal].is_ascii_digit() {
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
        }
        let end = index;
        let mut lookahead = end;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead < bytes.len()
            && (bytes[lookahead].is_ascii_alphabetic()
                || bytes[lookahead] == b'_'
                || bytes[lookahead] == b'%')
        {
            continue;
        }
        ranges.push((start, end));
    }
    ranges
}

fn lsp_schema_annotation_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let annotation = schema_annotation_from_message(diagnostic_message(diagnostic))?;
    let name = annotation.split_once(':')?.0.trim();
    if !is_identifier(name) {
        return None;
    }
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let indent = line_indent(line);
    let rest = &line[indent.len()..];
    if !rest.starts_with(name) || !rest[name.len()..].trim_start().starts_with('=') {
        return None;
    }
    if let Some(unit) = bracket_payload(&annotation) {
        if !line.contains(unit) {
            return None;
        }
    }
    Some(json!({
        "title": format!("Convert {name} to schema column annotation"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            full_line_same_line_range(line_number, line),
            &format!("{indent}{annotation}")
        )
    }))
}

fn schema_annotation_from_message(message: &str) -> Option<String> {
    let marker = "Write `";
    let start = message.find(marker)? + marker.len();
    let end = message[start..].find('`')? + start;
    Some(
        message[start..end]
            .replace(char::is_whitespace, " ")
            .trim()
            .to_owned(),
    )
}

fn lsp_file_mutation_confirm_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let trimmed = line.trim_start();
    if !trimmed.starts_with("move ") && !trimmed.starts_with("delete ") {
        return None;
    }
    lsp_boolean_with_options_code_action(uri, text, diagnostic, &["confirm"])
}

fn lsp_recursive_delete_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    if !line.trim_start().starts_with("delete dir(") {
        return None;
    }
    lsp_boolean_with_options_code_action(uri, text, diagnostic, &["recursive", "confirm"])
}

fn lsp_boolean_with_options_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    option_names: &[&str],
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let attached_block = attached_with_block(&lines, line_number);
    let missing_options = option_names
        .iter()
        .copied()
        .filter(|option_name| {
            attached_block
                .as_ref()
                .is_none_or(|block| !with_block_contains_option(&lines, block, option_name))
        })
        .collect::<Vec<_>>();
    if missing_options.is_empty() {
        return None;
    }
    let title = if missing_options.len() == 1 {
        format!("Add {} = true", missing_options[0])
    } else {
        format!("Add {} = true", missing_options.join(" = true and "))
    };
    let newline = document_newline(text);
    let (range, new_text) = if let Some(block) = attached_block {
        let insertion = missing_options
            .iter()
            .map(|option_name| format!("{}    {} = true", block.indent, option_name))
            .collect::<Vec<_>>()
            .join(newline);
        (
            zero_width_range(block.end_line, 0),
            format!("{insertion}{newline}"),
        )
    } else {
        let line = lines.get(line_number).copied().unwrap_or("");
        let indent = line_indent(line);
        let option_lines = missing_options
            .iter()
            .map(|option_name| format!("{indent}    {option_name} = true"))
            .collect::<Vec<_>>()
            .join(newline);
        let character = utf16_len(line);
        (
            zero_width_range(line_number, character),
            format!("{newline}{indent}with {{{newline}{option_lines}{newline}{indent}}}"),
        )
    };
    Some(json!({
        "title": title,
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, &new_text)
    }))
}

fn lsp_write_standard_text_output_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let owner_line = lines.get(line_number).copied().unwrap_or("");
    if !is_write_standard_text_owner_line(owner_line) {
        return None;
    }
    let attached_block = attached_with_block(&lines, line_number);
    if attached_block
        .as_ref()
        .is_some_and(|block| with_block_contains_option(&lines, block, "output"))
    {
        return None;
    }

    let newline = document_newline(text);
    let option_text = r#"output = join(args.output, "standard_weather_file.txt")"#;
    let (range, new_text) = if let Some(block) = attached_block {
        (
            zero_width_range(block.end_line, 0),
            format!("{}    {}{}", block.indent, option_text, newline),
        )
    } else {
        let indent = line_indent(owner_line);
        let character = utf16_len(owner_line);
        (
            zero_width_range(line_number, character),
            format!(
                "{newline}{indent}with {{{newline}{indent}    {option_text}{newline}{indent}}}"
            ),
        )
    };

    Some(json!({
        "title": "Add standard_text output path",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, &new_text)
    }))
}

fn is_write_standard_text_owner_line(line: &str) -> bool {
    strip_line_comment(line)
        .trim_start()
        .starts_with("write standard_text")
}

fn lsp_sampling_seed_missing_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let owner_line = lines.get(line_number).copied().unwrap_or("");
    if !is_sample_generation_owner_line(owner_line) {
        return None;
    }
    let attached_block = attached_with_block(&lines, line_number);
    if attached_block
        .as_ref()
        .is_some_and(|block| with_block_contains_option(&lines, block, "seed"))
    {
        return None;
    }

    let newline = document_newline(text);
    let (range, new_text) = if let Some(block) = attached_block {
        (
            zero_width_range(block.end_line, 0),
            format!("{}    seed = 42{}", block.indent, newline),
        )
    } else {
        let indent = line_indent(owner_line);
        let character = utf16_len(owner_line);
        (
            zero_width_range(line_number, character),
            format!("{newline}{indent}with {{{newline}{indent}    seed = 42{newline}{indent}}}"),
        )
    };

    Some(json!({
        "title": "Add sample seed: seed = 42",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, &new_text)
    }))
}

fn is_sample_generation_owner_line(line: &str) -> bool {
    let code = strip_line_comment(line).trim();
    let Some((_name, expression)) = code.split_once('=') else {
        return false;
    };
    matches!(
        expression.trim().to_ascii_lowercase().as_str(),
        "sample grid"
            | "sample random"
            | "sample uniform"
            | "sample lhs"
            | "sample latin_hypercube"
            | "sample latin-hypercube"
    )
}

fn lsp_uncertainty_seed_missing_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let block = with_block_containing_line(&lines, line_number)?;
    if with_block_contains_option(&lines, &block, "seed") {
        return None;
    }
    let newline = document_newline(text);
    Some(json!({
        "title": "Add uncertainty seed: seed = 7",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            zero_width_range(block.end_line, 0),
            &format!("{}    seed = 7{}", block.indent, newline)
        )
    }))
}

fn with_block_containing_line(lines: &[&str], line_number: usize) -> Option<AttachedBlock> {
    let mut cursor = line_number;
    loop {
        let line = *lines.get(cursor)?;
        if strip_line_comment(line).trim() == "with {" {
            let end_line = matching_block_end_line(lines, cursor)?;
            if end_line > line_number {
                return Some(AttachedBlock {
                    start_line: cursor,
                    end_line,
                    indent: line_indent(line).to_owned(),
                });
            }
        }
        if cursor == 0 {
            break;
        }
        cursor -= 1;
    }
    None
}

struct AttachedBlock {
    start_line: usize,
    end_line: usize,
    indent: String,
}

fn attached_with_block(lines: &[&str], owner_line_number: usize) -> Option<AttachedBlock> {
    let mut line_number = owner_line_number + 1;
    while line_number < lines.len() && lines[line_number].trim().is_empty() {
        line_number += 1;
    }
    let line = *lines.get(line_number)?;
    if line.trim() != "with {" {
        return None;
    }
    let end_line = matching_block_end_line(lines, line_number)?;
    if end_line <= line_number {
        return None;
    }
    Some(AttachedBlock {
        start_line: line_number,
        end_line,
        indent: line_indent(line).to_owned(),
    })
}

fn with_block_contains_option(lines: &[&str], block: &AttachedBlock, option_name: &str) -> bool {
    for line in lines.iter().take(block.end_line).skip(block.start_line + 1) {
        let trimmed = strip_line_comment(line).trim_start();
        if let Some(rest) = trimmed.strip_prefix(option_name) {
            if rest.trim_start().starts_with('=') {
                return true;
            }
        }
    }
    false
}

struct OptionQuickFix {
    option_names: &'static [&'static str],
    value: &'static str,
    label: &'static str,
}

struct OptionValueQuickFix {
    value: &'static str,
    label: &'static str,
}

fn option_quick_fix(code: &str) -> Option<OptionQuickFix> {
    match code {
        "E-NET-RETRY-POLICY" | "E-PROCESS-RETRY-POLICY" => Some(OptionQuickFix {
            option_names: &["retry"],
            value: "0",
            label: "Disable retries",
        }),
        "E-NET-TIMEOUT" => Some(OptionQuickFix {
            option_names: &["timeout"],
            value: "30 s",
            label: "Set timeout to 30 s",
        }),
        "E-PROCESS-TIMEOUT" => Some(OptionQuickFix {
            option_names: &["timeout"],
            value: "10 s",
            label: "Set timeout to 10 s",
        }),
        "E-NET-BODY-SIZE-LIMIT" => Some(OptionQuickFix {
            option_names: &["body_size_limit", "response_body_limit"],
            value: "10 MB",
            label: "Set response body limit to 10 MB",
        }),
        "E-NET-BODY-POLICY" => Some(OptionQuickFix {
            option_names: &["body"],
            value: "\"{}\"",
            label: "Replace request body with string literal",
        }),
        "E-PROCESS-ALLOW-FAILURE" => Some(OptionQuickFix {
            option_names: &["allow_failure"],
            value: "true",
            label: "Allow process failure",
        }),
        "E-PROCESS-CWD-001" => Some(OptionQuickFix {
            option_names: &["cwd"],
            value: "dir(\".\")",
            label: "Set process cwd",
        }),
        "E-PROCESS-ENV-001" => Some(OptionQuickFix {
            option_names: &["env"],
            value: "{ NAME = \"value\" }",
            label: "Set process env",
        }),
        "E-SAMPLING-COUNT-INVALID" => Some(OptionQuickFix {
            option_names: &["count"],
            value: "1",
            label: "Set sample count",
        }),
        "E-SAMPLING-SEED-INVALID" => Some(OptionQuickFix {
            option_names: &["seed"],
            value: "42",
            label: "Set sample seed",
        }),
        "E-WITH-UNCERTAINTY-POLICY-001" => Some(OptionQuickFix {
            option_names: &["uncertainty"],
            value: "linear",
            label: "Set uncertainty policy",
        }),
        "E-WITH-UNCERTAINTY-SAMPLES-001" => Some(OptionQuickFix {
            option_names: &["samples"],
            value: "64",
            label: "Set uncertainty samples",
        }),
        "E-WITH-UNCERTAINTY-SEED-001" => Some(OptionQuickFix {
            option_names: &["seed"],
            value: "7",
            label: "Set uncertainty seed",
        }),
        "E-CACHE-KEY-NONDETERMINISTIC" => Some(OptionQuickFix {
            option_names: &["cache_key"],
            value: "[\"stable\", \"v1\"]",
            label: "Set deterministic cache key",
        }),
        "E-CACHE-DIR" => Some(OptionQuickFix {
            option_names: &["cache_dir"],
            value: "dir(\"cache\")",
            label: "Set cache directory",
        }),
        "E-CACHE-TTL" => Some(OptionQuickFix {
            option_names: &["cache_ttl"],
            value: "1 h",
            label: "Set cache TTL to 1 h",
        }),
        "E-SIM-TIMESTEP-INVALID" => Some(OptionQuickFix {
            option_names: &["timestep"],
            value: "10 min",
            label: "Set simulation timestep",
        }),
        "E-SOLVE-TIMESTEP-INVALID" => Some(OptionQuickFix {
            option_names: &["timestep"],
            value: "1 s",
            label: "Set solver timestep",
        }),
        "E-SIM-DURATION-INVALID" => Some(OptionQuickFix {
            option_names: &["duration"],
            value: "30 min",
            label: "Set simulation duration",
        }),
        "E-SOLVE-DURATION-INVALID" => Some(OptionQuickFix {
            option_names: &["duration"],
            value: "10 s",
            label: "Set solver duration",
        }),
        "E-SIM-TOLERANCE-INVALID" => Some(OptionQuickFix {
            option_names: &["tolerance"],
            value: "0.0001",
            label: "Set simulation tolerance",
        }),
        "E-SOLVE-TOLERANCE-INVALID" => Some(OptionQuickFix {
            option_names: &["tolerance"],
            value: "0.0001",
            label: "Set solver tolerance",
        }),
        "E-SIM-SOLVER-UNSUPPORTED" => Some(OptionQuickFix {
            option_names: &["solver"],
            value: "fixed_step",
            label: "Set simulation solver",
        }),
        "E-SOLVE-SOLVER-UNSUPPORTED" => Some(OptionQuickFix {
            option_names: &["solver"],
            value: "fixed_point",
            label: "Set solve solver",
        }),
        "E-SOLVE-RELAXATION-INVALID" => Some(OptionQuickFix {
            option_names: &["relaxation"],
            value: "0.5",
            label: "Set solver relaxation",
        }),
        "E-SOLVE-FD-STEP-INVALID" => Some(OptionQuickFix {
            option_names: &["finite_difference_step"],
            value: "0.000001",
            label: "Set finite-difference step",
        }),
        "E-SOLVE-DAMPING-INVALID" => Some(OptionQuickFix {
            option_names: &["damping"],
            value: "1",
            label: "Set solver damping",
        }),
        "E-SOLVE-CONSISTENCY-TOLERANCE-INVALID" => Some(OptionQuickFix {
            option_names: &["consistency_tolerance"],
            value: "0.000001",
            label: "Set consistency tolerance",
        }),
        "E-SOLVE-MAX-ITER-INVALID" => Some(OptionQuickFix {
            option_names: &["max_iter"],
            value: "50",
            label: "Set solver max iterations",
        }),
        "E-SOLVE-LINE-SEARCH-STEPS-INVALID" => Some(OptionQuickFix {
            option_names: &["line_search_steps"],
            value: "8",
            label: "Set line-search steps",
        }),
        "E-SOLVE-INITIAL-INVALID" => Some(OptionQuickFix {
            option_names: &["initial", "initial_derivative", "initial_algebraic"],
            value: "1",
            label: "Set solver initial value",
        }),
        "E-SOLVE-VARIABLE-SCALE-INVALID" => Some(OptionQuickFix {
            option_names: &["variable_scale", "variable_scales"],
            value: "1",
            label: "Set solver variable scale",
        }),
        "E-SOLVE-MASS-MATRIX-INVALID" => Some(OptionQuickFix {
            option_names: &["mass_matrix"],
            value: "identity",
            label: "Set mass matrix",
        }),
        "E-SOLVE-JACOBIAN-UNSUPPORTED" => Some(OptionQuickFix {
            option_names: &["jacobian"],
            value: "finite_difference",
            label: "Set solver Jacobian policy",
        }),
        "E-SOLVE-ALGEBRAIC-INITIALIZATION-UNSUPPORTED" => Some(OptionQuickFix {
            option_names: &["algebraic_initialization"],
            value: "newton",
            label: "Set algebraic initialization",
        }),
        "E-ML-ARGS-003" => Some(OptionQuickFix {
            option_names: &["algorithm"],
            value: "linear",
            label: "Set regression algorithm",
        }),
        _ => None,
    }
}

fn option_quick_fix_option_names(code: &str) -> Option<Vec<&'static str>> {
    let option_names = option_quick_fix(code)
        .map(|fix| fix.option_names)
        .or_else(|| model_option_quick_fix_option_names(code))?;
    let known_names = option_names
        .iter()
        .copied()
        .filter(|option_name| workflow_option_label_exists(option_name))
        .collect::<Vec<_>>();
    (!known_names.is_empty()).then_some(known_names)
}

fn option_quick_fix_for_option(code: &str, option_name: &str) -> Option<OptionValueQuickFix> {
    if !workflow_option_label_exists(option_name) {
        return None;
    }
    if let Some(fix) = option_quick_fix(code) {
        if fix.option_names.contains(&option_name) {
            return Some(OptionValueQuickFix {
                value: fix.value,
                label: fix.label,
            });
        }
    }
    model_option_quick_fix(code, option_name)
}

fn model_option_quick_fix_option_names(code: &str) -> Option<&'static [&'static str]> {
    match code {
        "E-ML-ARGS-001" => Some(&["test", "hidden", "epochs"]),
        "E-ML-ARGS-002" => Some(&["test", "seed", "hidden", "epochs"]),
        _ => None,
    }
}

fn model_option_quick_fix(code: &str, option_name: &str) -> Option<OptionValueQuickFix> {
    match (code, option_name) {
        ("E-ML-ARGS-001" | "E-ML-ARGS-002", "test") => Some(OptionValueQuickFix {
            value: "0.25",
            label: "Set model test split",
        }),
        ("E-ML-ARGS-002", "seed") => Some(OptionValueQuickFix {
            value: "7",
            label: "Set model seed",
        }),
        ("E-ML-ARGS-001" | "E-ML-ARGS-002", "hidden") => Some(OptionValueQuickFix {
            value: "[8]",
            label: "Set model hidden layers",
        }),
        ("E-ML-ARGS-001" | "E-ML-ARGS-002", "epochs") => Some(OptionValueQuickFix {
            value: "20",
            label: "Set model epochs",
        }),
        _ => None,
    }
}

fn lsp_option_value_replacement_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    code: &str,
) -> Option<Value> {
    let option_names = option_quick_fix_option_names(code)?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let assignment = option_assignment_range(line, &option_names)?;
    let fix = option_quick_fix_for_option(code, &assignment.option_name)?;
    Some(json!({
        "title": format!("{}: {} = {}", fix.label, assignment.option_name, fix.value),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, assignment.value_start, assignment.value_end),
            fix.value
        )
    }))
}

fn lsp_absolute_http_url_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let (start_byte, end_byte) = net_url_literal_range(line)?;
    Some(json!({
        "title": "Replace URL with https://example.org",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            "\"https://example.org\""
        )
    }))
}

fn lsp_http_body_method_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let owner_line_number = owner_line_for_enclosing_with_block(&lines, line_number)?;
    let owner_line = *lines.get(owner_line_number)?;
    let (start_byte, end_byte) = http_method_token_range(owner_line)?;
    Some(json!({
        "title": "Change HTTP method to post",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(owner_line_number, owner_line, start_byte, end_byte),
            "post"
        )
    }))
}

fn lsp_expected_sha256_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let hash = expected_sha256_from_diagnostic(diagnostic)?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let assignment = option_assignment_range(line, &["expected_sha256"])?;
    let replacement = format!("\"{hash}\"");
    Some(json!({
        "title": "Update expected_sha256 to pinned response SHA-256",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, assignment.value_start, assignment.value_end),
            &replacement
        )
    }))
}

struct WithOptionAliasFix {
    from: &'static str,
    to: &'static str,
    title: &'static str,
}

fn lsp_with_option_alias_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let unknown_option = unknown_with_option_name(diagnostic_message(diagnostic))?;
    let fix = with_option_alias_fix(unknown_option)?;
    lsp_option_key_replacement_code_action(uri, text, diagnostic, fix.from, fix.to, fix.title)
}

fn lsp_option_key_replacement_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    from: &str,
    to: &str,
    title: &str,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let name_start = line_indent(line).len();
    let rest = &line[name_start..];
    let after_name = rest.strip_prefix(from)?;
    if !after_name
        .chars()
        .next()
        .is_some_and(|character| character.is_whitespace() || character == '=')
    {
        return None;
    }
    let equals_offset = after_name.find('=')?;
    if !after_name[..equals_offset].trim().is_empty() {
        return None;
    }
    Some(json!({
        "title": title,
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, name_start, name_start + from.len()),
            to
        )
    }))
}

fn with_option_alias_fix(option_name: &str) -> Option<WithOptionAliasFix> {
    let fix = match option_name {
        "unit" => WithOptionAliasFix {
            from: "unit",
            to: "unit y",
            title: "Use plot y-axis option: unit y =",
        },
        "y_unit" => WithOptionAliasFix {
            from: "y_unit",
            to: "unit y",
            title: "Use plot y-axis option: unit y =",
        },
        "x_unit" => WithOptionAliasFix {
            from: "x_unit",
            to: "unit x",
            title: "Use plot x-axis option: unit x =",
        },
        "confidence" => WithOptionAliasFix {
            from: "confidence",
            to: "confidence_band",
            title: "Use confidence band option: confidence_band =",
        },
        _ => return None,
    };
    workflow_option_label_exists(fix.to).then_some(fix)
}

fn unknown_with_option_name(message: &str) -> Option<&str> {
    let (_, after_marker) = message.split_once("Unknown with option `")?;
    let (option, _) = after_marker.split_once('`')?;
    Some(option.trim())
}

fn lsp_remove_incompatible_display_unit_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    option_assignment_range(line, &["unit y", "unit x", "display_unit", "unit"])?;
    Some(json!({
        "title": "Remove incompatible display unit option",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, full_line_range(text, line_number), "")
    }))
}

fn lsp_log_level_info_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let edit = log_level_info_edit(line_number, line)?;
    Some(json!({
        "title": "Set log level to info",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, edit.range, edit.new_text)
    }))
}

struct LogLevelEdit {
    range: Value,
    new_text: &'static str,
}

fn log_level_info_edit(line_number: usize, line: &str) -> Option<LogLevelEdit> {
    let code = strip_line_comment(line);
    let indent_len = line_indent(code).len();
    let rest = &code[indent_len..];
    let after_log = rest.strip_prefix("log")?;
    if after_log
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace() && character != '"')
    {
        return None;
    }
    let after_log_start = indent_len + "log".len();
    let whitespace_len = code[after_log_start..]
        .chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let token_start = after_log_start + whitespace_len;
    let first = code[token_start..].chars().next()?;
    if first == '"' {
        return Some(LogLevelEdit {
            range: line_byte_range(line_number, line, token_start, token_start),
            new_text: "info ",
        });
    }
    let token_end = token_start
        + code[token_start..]
            .chars()
            .take_while(|character| !character.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    let level = &code[token_start..token_end];
    if matches!(level, "debug" | "info" | "warn" | "error") {
        return None;
    }
    Some(LogLevelEdit {
        range: line_byte_range(line_number, line, token_start, token_end),
        new_text: "info",
    })
}

fn lsp_statement_only_unbind_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let (start_byte, end_byte) = statement_binding_prefix_range(line)?;
    Some(json!({
        "title": "Remove invalid binding prefix",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            ""
        )
    }))
}

fn statement_binding_prefix_range(line: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let start = line_indent(code).len();
    let rest = &code[start..];
    let name_len = statement_identifier_prefix_len(rest)?;
    let mut cursor = skip_binding_prefix_whitespace(code, start + name_len);
    if code[cursor..].starts_with(':') {
        let annotation_start = cursor + ':'.len_utf8();
        let equals_byte = annotation_start + code[annotation_start..].find('=')?;
        if code[annotation_start..equals_byte].trim().is_empty() {
            return None;
        }
        cursor = equals_byte;
    }
    cursor = skip_binding_prefix_whitespace(code, cursor);
    if !code[cursor..].starts_with('=') {
        return None;
    }
    cursor += '='.len_utf8();
    cursor = skip_binding_prefix_whitespace(code, cursor);
    if code[cursor..].trim().is_empty() {
        return None;
    }
    Some((start, cursor))
}

fn statement_identifier_prefix_len(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let first = *bytes.first()?;
    if first != b'_' && !first.is_ascii_alphabetic() {
        return None;
    }
    let mut end = 1usize;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }
    Some(end)
}

fn skip_binding_prefix_whitespace(text: &str, mut cursor: usize) -> usize {
    while cursor < text.len() {
        let Some(character) = text[cursor..].chars().next() else {
            break;
        };
        if !character.is_whitespace() {
            break;
        }
        cursor += character.len_utf8();
    }
    cursor
}

fn lsp_bind_process_result_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let indent_len = line_indent(code).len();
    let rest = &code[indent_len..];
    if !rest.starts_with("run command") {
        return None;
    }
    Some(json!({
        "title": "Bind process result",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, indent_len, indent_len),
            "result = "
        )
    }))
}

fn lsp_unique_process_binding_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let name = first_backtick_payload(diagnostic_message(diagnostic))?.trim();
    if !is_identifier(name) {
        return None;
    }
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let indent_len = line_indent(code).len();
    let rest = &code[indent_len..];
    let after_name = rest.strip_prefix(name)?;
    if after_name
        .chars()
        .next()
        .is_some_and(is_identifier_character)
        || !after_name.trim_start().starts_with('=')
    {
        return None;
    }
    let replacement = unique_binding_name(text, name);
    Some(json!({
        "title": format!("Rename process result to {replacement}"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, indent_len, indent_len + name.len()),
            &replacement
        )
    }))
}

fn unique_binding_name(text: &str, base: &str) -> String {
    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if !binding_name_exists(text, &candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search should return a unique binding name")
}

fn available_binding_name(text: &str, base: &str) -> String {
    if !binding_name_exists(text, base) {
        return base.to_owned();
    }
    unique_binding_name(text, base)
}

fn binding_name_exists(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let code = strip_line_comment(line);
        let trimmed = code.trim_start();
        let Some(rest) = trimmed.strip_prefix(name) else {
            return false;
        };
        !rest.chars().next().is_some_and(is_identifier_character)
            && rest.trim_start().starts_with('=')
    })
}

fn lsp_process_command_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let edit = process_command_edit(line_number, line)?;
    Some(json!({
        "title": "Add process command string",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, edit.range, edit.new_text)
    }))
}

fn lsp_json_read_promotion_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let access = json_read_field_access_from_diagnostic(diagnostic_message(diagnostic))?;
    let access_line_number = diagnostic_line(diagnostic)?;
    let access_line = text.lines().nth(access_line_number)?;
    let (access_start, access_end) =
        json_field_access_byte_range(access_line, &access.binding, &access.field)?;
    let schema_name = unique_schema_name(text, &schema_name_from_binding(&access.binding));
    let typed_binding = available_binding_name(text, &format!("{}_typed", access.binding));
    let newline = document_newline(text);
    let read_line = read_json_binding_line(text, &access.binding);
    let (schema_insert_line, promotion_insert_range, indent) =
        if let Some((line_number, line)) = read_line {
            (
                line_number,
                line_byte_range(line_number, line, line.len(), line.len()),
                line_indent(line).to_owned(),
            )
        } else {
            (
                access_line_number,
                zero_width_range(access_line_number, 0),
                line_indent(access_line).to_owned(),
            )
        };
    let schema_text = format!(
        "{indent}schema {schema_name} {{{newline}{indent}    {}: String{newline}{indent}}}{newline}{newline}",
        access.field
    );
    let promotion_text = if read_line.is_some() {
        format!(
            "{newline}{indent}{typed_binding} = promote json {} as {schema_name}",
            access.binding
        )
    } else {
        format!(
            "{indent}{typed_binding} = promote json {} as {schema_name}{newline}",
            access.binding
        )
    };
    let edits = if read_line.is_some() {
        json!([
            {
                "range": zero_width_range(schema_insert_line, 0),
                "newText": schema_text
            },
            {
                "range": promotion_insert_range,
                "newText": promotion_text
            },
            {
                "range": line_byte_range(access_line_number, access_line, access_start, access_end),
                "newText": format!("{typed_binding}.{}", access.field)
            }
        ])
    } else {
        json!([
            {
                "range": zero_width_range(schema_insert_line, 0),
                "newText": format!("{schema_text}{promotion_text}{newline}")
            },
            {
                "range": line_byte_range(access_line_number, access_line, access_start, access_end),
                "newText": format!("{typed_binding}.{}", access.field)
            }
        ])
    };
    Some(json!({
        "title": format!("Promote {} before field access", access.binding),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": workspace_edit_for_edits(uri, edits)
    }))
}

struct JsonReadFieldAccess {
    binding: String,
    field: String,
}

fn json_read_field_access_from_diagnostic(message: &str) -> Option<JsonReadFieldAccess> {
    for payload in backtick_payloads(message) {
        let Some((binding, field)) = payload.trim().split_once('.') else {
            continue;
        };
        let binding = binding.trim();
        let field = field.trim();
        if is_identifier(binding) && is_identifier(field) {
            return Some(JsonReadFieldAccess {
                binding: binding.to_owned(),
                field: field.to_owned(),
            });
        }
    }
    None
}

fn read_json_binding_line<'a>(text: &'a str, binding: &str) -> Option<(usize, &'a str)> {
    for (line_number, line) in text.lines().enumerate() {
        let code = strip_line_comment(line);
        let indent_len = line_indent(code).len();
        let rest = &code[indent_len..];
        let Some(after_binding) = rest.strip_prefix(binding) else {
            continue;
        };
        if after_binding
            .chars()
            .next()
            .is_some_and(is_identifier_character)
        {
            continue;
        }
        let Some(equals_offset) = after_binding.find('=') else {
            continue;
        };
        if !after_binding[..equals_offset].trim().is_empty() {
            continue;
        }
        let expression = after_binding[equals_offset + '='.len_utf8()..].trim_start();
        if expression.starts_with("read json ") {
            return Some((line_number, line));
        }
    }
    None
}

fn json_field_access_byte_range(line: &str, binding: &str, field: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let access = format!("{binding}.{field}");
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find(&access) else {
            break;
        };
        let start = search_start + relative_start;
        let end = start + access.len();
        if member_access_boundary(code, start, end) {
            return Some((start, end));
        }
        search_start = end;
    }
    None
}

fn member_access_boundary(line: &str, start: usize, end: usize) -> bool {
    let before = line[..start].chars().next_back();
    let after = line[end..].chars().next();
    !before.is_some_and(is_identifier_character) && !after.is_some_and(is_identifier_character)
}

fn schema_name_from_binding(binding: &str) -> String {
    let mut result = String::new();
    for segment in binding
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
    {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.extend(chars.map(|character| character.to_ascii_lowercase()));
        }
    }
    if result.is_empty() {
        result.push_str("JsonPayload");
    }
    result.push_str("Schema");
    result
}

fn unique_schema_name(text: &str, base: &str) -> String {
    if !schema_name_exists(text, base) {
        return base.to_owned();
    }
    for index in 2.. {
        let candidate = format!("{base}{index}");
        if !schema_name_exists(text, &candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search should return a unique schema name")
}

fn schema_name_exists(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let code = strip_line_comment(line);
        let trimmed = code.trim_start();
        let Some(rest) = trimmed.strip_prefix("schema") else {
            return false;
        };
        if !rest.chars().next().is_some_and(char::is_whitespace) {
            return false;
        }
        let candidate = rest.trim_start();
        let Some(after_name) = candidate.strip_prefix(name) else {
            return false;
        };
        !after_name
            .chars()
            .next()
            .is_some_and(is_identifier_character)
    })
}

struct ProcessCommandEdit {
    range: Value,
    new_text: &'static str,
}

fn process_command_edit(line_number: usize, line: &str) -> Option<ProcessCommandEdit> {
    let code = strip_line_comment(line);
    let command_start = code.find("run command")?;
    let after_command = command_start + "run command".len();
    let whitespace_len = code[after_command..]
        .chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let argument_start = after_command + whitespace_len;
    let argument = &code[argument_start..];
    if argument.starts_with("\"\"") {
        return Some(ProcessCommandEdit {
            range: line_byte_range(line_number, line, argument_start, argument_start + 2),
            new_text: "\"tool\"",
        });
    }
    if argument.trim().is_empty() {
        let insert_at = code.trim_end().len();
        return Some(ProcessCommandEdit {
            range: line_byte_range(line_number, line, insert_at, insert_at),
            new_text: " \"tool\"",
        });
    }
    None
}

fn lsp_wrap_assertion_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let indent = line_indent(code);
    let assertion = code[indent.len()..].trim_end();
    if !assertion.starts_with("assert ") {
        return None;
    }
    let newline = document_newline(text);
    let replacement = format!(
        "{indent}test \"assertion\" {{{newline}{indent}    {assertion}{newline}{indent}}}{newline}"
    );
    Some(json!({
        "title": "Wrap assertion in test block",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, full_line_range(text, line_number), &replacement)
    }))
}

fn lsp_golden_expected_file_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let (start_byte, end_byte) = golden_bare_expected_string_range(code)?;
    let expected = &code[start_byte..end_byte];
    Some(json!({
        "title": "Wrap golden expected path with file(...)",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            &format!("file({expected})")
        )
    }))
}

fn golden_bare_expected_string_range(line: &str) -> Option<(usize, usize)> {
    let trimmed_start = line_indent(line).len();
    if !line[trimmed_start..].starts_with("golden ") {
        return None;
    }
    let matches_index = line.find(" matches ")?;
    let mut cursor = matches_index + " matches ".len();
    while cursor < line.len() && line.as_bytes()[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if line[cursor..].starts_with("file(") {
        return None;
    }
    let range = string_literal_range_at(line, cursor)?;
    if !line[range.1..].trim().is_empty() {
        return None;
    }
    Some(range)
}

fn lsp_uncertainty_argument_code_actions(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let message = diagnostic_message(diagnostic);
    let mut actions = Vec::new();

    if let Some(example) = uncertainty_call_example_from_diagnostic(message) {
        if let Some((start_byte, end_byte)) = uncertainty_call_range_on_line(line) {
            actions.push(json!({
                "title": format!("Replace uncertainty call with {example}"),
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    line_byte_range(line_number, line, start_byte, end_byte),
                    example
                )
            }));
        }
    }

    if message.contains("method=linear") {
        if let Some(range) = named_argument_value_range(line, &["method"]) {
            actions.push(json!({
                "title": "Set uncertainty method to linear",
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    line_byte_range(line_number, line, range.value_start, range.value_end),
                    "linear"
                )
            }));
        }
    }

    if message.contains("supports `normal` and `uniform`")
        && strip_line_comment(line).contains("distribution(")
    {
        if let Some(range) = named_argument_value_range(line, &["kind"]) {
            actions.push(json!({
                "title": "Set distribution kind to normal",
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    line_byte_range(line_number, line, range.value_start, range.value_end),
                    "normal"
                )
            }));
        }
    }

    if message.contains("between 1 and 256") {
        if let Some(range) = named_argument_value_range(line, &["samples", "n"]) {
            actions.push(json!({
                "title": "Set uncertainty samples to 31",
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    line_byte_range(line_number, line, range.value_start, range.value_end),
                    "31"
                )
            }));
        }
    }

    actions
}

struct SelectFirstRowMigration<'a> {
    lhs: &'a str,
    binding: &'a str,
    table: &'a str,
    return_column: &'a str,
    filters: Vec<(&'a str, &'a str)>,
}

fn lsp_select_first_row_migration_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let migration = select_first_row_migration_from_line(line)?;
    let replacement = select_first_row_migration_replacement(
        &migration,
        line_indent(line),
        document_newline(text),
    );
    Some(json!({
        "title": "Replace select_first_row with filter + require_one",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, full_line_range(text, line_number), &replacement)
    }))
}

fn select_first_row_migration_from_line(line: &str) -> Option<SelectFirstRowMigration<'_>> {
    let code = strip_line_comment(line);
    let call_start = code.find("select_first_row(")?;
    let before_call = &code[..call_start];
    let equals = before_call.rfind('=')?;
    let lhs_start = line_indent(before_call).len();
    let lhs = before_call[lhs_start..equals].trim();
    let binding = lhs
        .split_once(':')
        .map(|(name, _annotation)| name.trim())
        .unwrap_or(lhs);
    if !is_identifier(binding) {
        return None;
    }

    let open = call_start + "select_first_row".len();
    if code.as_bytes().get(open) != Some(&b'(') {
        return None;
    }
    let close = matching_close_paren_byte(code, open)?;
    if !code[close + 1..].trim().is_empty() {
        return None;
    }

    let parts = split_top_level_commas(&code[open + 1..close]);
    let table = parts.first()?.trim();
    if !is_simple_path_expression(table) {
        return None;
    }

    let mut return_column = None;
    let mut filters = Vec::new();
    for part in parts.iter().skip(1) {
        let (name, value) = split_top_level_assignment(part)?;
        let name = name.trim();
        let value = value.trim();
        if name == "return_column" {
            return_column = Some(select_first_row_return_column(value)?);
            continue;
        }
        if !is_identifier(name)
            || value.is_empty()
            || value.contains('{')
            || value.contains('}')
            || value.contains('\n')
            || value.contains('\r')
        {
            return None;
        }
        filters.push((name, value));
    }

    let return_column = return_column?;
    if filters.is_empty() {
        return None;
    }
    Some(SelectFirstRowMigration {
        lhs,
        binding,
        table,
        return_column,
        filters,
    })
}

fn select_first_row_return_column(value: &str) -> Option<&str> {
    let candidate = unquoted_simple_string(value).unwrap_or(value.trim());
    is_identifier(candidate).then_some(candidate)
}

fn unquoted_simple_string(value: &str) -> Option<&str> {
    let inner = value.trim().strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('\\') && !inner.contains('"')).then_some(inner)
}

fn select_first_row_migration_replacement(
    migration: &SelectFirstRowMigration<'_>,
    indent: &str,
    newline: &str,
) -> String {
    let rows_binding = format!("{}_rows", migration.binding);
    let row_binding = format!("{}_row", migration.binding);
    let mut replacement = format!(
        "{indent}{rows_binding} = filter {}{newline}{indent}where {{{newline}",
        migration.table
    );
    for (name, value) in &migration.filters {
        replacement.push_str(&format!("{indent}    {name} == {value}{newline}"));
    }
    replacement.push_str(&format!(
        "{indent}}}{newline}{indent}{row_binding} = require_one {rows_binding}{newline}{indent}{} = {row_binding}.{}{newline}",
        migration.lhs, migration.return_column
    ));
    replacement
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts
}

fn split_top_level_assignment(input: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '=' if depth == 0 => return Some((&input[..index], &input[index + 1..])),
            _ => {}
        }
    }
    None
}

fn is_simple_path_expression(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.split('.').all(is_identifier)
}

fn lsp_uncertainty_direct_compare_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let expression = direct_uncertainty_expression_from_diagnostic(diagnostic_message(diagnostic))?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let (start_byte, end_byte) = direct_uncertainty_expression_range(line, expression)?;
    let replacement = format!("mean({expression})");
    Some(json!({
        "title": format!("Compare mean({expression}) instead"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            &replacement
        )
    }))
}

fn direct_uncertainty_expression_from_diagnostic(message: &str) -> Option<&str> {
    let expression = first_backtick_payload(message)?.trim();
    if expression.is_empty()
        || expression.contains('\n')
        || expression.contains('\r')
        || expression.starts_with("mean(")
    {
        return None;
    }
    Some(expression)
}

fn direct_uncertainty_expression_range(line: &str, expression: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let mut search_start = 0usize;
    while search_start <= code.len() {
        let Some(relative_start) = code[search_start..].find(expression) else {
            break;
        };
        let start = search_start + relative_start;
        let end = start + expression.len();
        if expression_boundary(code, start, end) {
            return Some((start, end));
        }
        search_start = end;
    }
    None
}

fn expression_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        text[..start].chars().next_back()
    };
    let after = text[end..].chars().next();
    before.is_none_or(|character| !is_expression_edge_character(character))
        && after.is_none_or(|character| !is_expression_edge_character(character))
}

fn is_expression_edge_character(character: char) -> bool {
    is_identifier_character(character) || character == '.'
}

fn lsp_ml_source_code_actions(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let message = diagnostic_message(diagnostic);
    let Some(expected) = ml_expected_source_kind(message) else {
        return Vec::new();
    };
    let mut actions = Vec::new();

    if diagnostic_code(diagnostic) == Some("E-ML-SOURCE-001") {
        if let Some(source) = ml_source_name_from_diagnostic(message) {
            let indent = line_indent(line);
            let skeleton = ml_source_skeleton(text, line_number, source, expected, indent);
            actions.push(json!({
                "title": format!("Define ML {} {source}", ml_expected_source_label(expected)),
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    zero_width_range(line_number, 0),
                    &skeleton
                )
            }));
        }
    }

    if diagnostic_code(diagnostic) == Some("E-ML-SOURCE-002") {
        if let Some(source) = ml_source_name_from_diagnostic(message) {
            if let Some(range) = ml_source_token_range(line, source) {
                if let Some((binding, skeleton)) = ml_source_adapter_skeleton(
                    text,
                    line_number,
                    source,
                    expected,
                    line_indent(line),
                ) {
                    actions.push(json!({
                        "title": ml_source_adapter_title(expected, source, &binding),
                        "kind": "quickfix",
                        "isPreferred": true,
                        "diagnostics": [diagnostic.clone()],
                        "edit": workspace_edit_for_edits(
                            uri,
                            json!([
                                {
                                    "range": zero_width_range(line_number, 0),
                                    "newText": skeleton
                                },
                                {
                                    "range": line_byte_range(line_number, line, range.0, range.1),
                                    "newText": binding
                                }
                            ])
                        )
                    }));
                }
            }
        }
    }

    actions
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MlExpectedSourceKind {
    TimeSeries,
    TrainTestSplit,
    Model,
    Table,
}

fn ml_expected_source_kind(message: &str) -> Option<MlExpectedSourceKind> {
    if let Some((_, rest)) = message.split_once("requires a prior ") {
        if let Some((label, _)) = rest.split_once(" binding") {
            return ml_expected_source_kind_from_label(label);
        }
    }
    if let Some((_, rest)) = message.split_once(" expects ") {
        if let Some((label, _)) = rest.split_once(" for its") {
            return ml_expected_source_kind_from_label(label);
        }
    }
    unknown_ml_role_from_diagnostic(message).and_then(ml_expected_source_kind_from_role)
}

fn ml_expected_source_kind_from_label(label: &str) -> Option<MlExpectedSourceKind> {
    if label.contains("TimeSeries") {
        Some(MlExpectedSourceKind::TimeSeries)
    } else if label.contains("TrainTestSplit") {
        Some(MlExpectedSourceKind::TrainTestSplit)
    } else if label.contains("Model[") {
        Some(MlExpectedSourceKind::Model)
    } else if label.contains("Table[") || label.contains("materialized derive table") {
        Some(MlExpectedSourceKind::Table)
    } else {
        None
    }
}

fn ml_expected_source_kind_from_role(role: &str) -> Option<MlExpectedSourceKind> {
    match role {
        "source" | "target" => Some(MlExpectedSourceKind::TimeSeries),
        "split" => Some(MlExpectedSourceKind::TrainTestSplit),
        "model" => Some(MlExpectedSourceKind::Model),
        "table" | "input" => Some(MlExpectedSourceKind::Table),
        _ => None,
    }
}

fn ml_expected_source_label(expected: MlExpectedSourceKind) -> &'static str {
    match expected {
        MlExpectedSourceKind::TimeSeries => "TimeSeries source",
        MlExpectedSourceKind::TrainTestSplit => "split source",
        MlExpectedSourceKind::Model => "model source",
        MlExpectedSourceKind::Table => "table source",
    }
}

fn unknown_ml_role_from_diagnostic(message: &str) -> Option<&str> {
    let (_, rest) = message.split_once("Unknown ML ")?;
    let (role, after_role) = rest.split_once(' ')?;
    after_role.trim_start().starts_with('`').then_some(role)
}

fn ml_source_name_from_diagnostic(message: &str) -> Option<&str> {
    if message.contains("Unknown ML ") {
        let (_, rest) = message.split_once('`')?;
        let (source, _) = rest.split_once('`')?;
        let source = source.trim();
        return is_identifier(source).then_some(source);
    }
    let source = first_backtick_payload(message)?.trim();
    is_identifier(source).then_some(source)
}

fn ml_source_skeleton(
    text: &str,
    line_number: usize,
    name: &str,
    expected: MlExpectedSourceKind,
    indent: &str,
) -> String {
    let newline = document_newline(text);
    match expected {
        MlExpectedSourceKind::TimeSeries => {
            format!("{indent}{name}: TimeSeries[Time] of HeatRate [kW] = 0 kW{newline}")
        }
        MlExpectedSourceKind::TrainTestSplit => {
            let series = ml_existing_or_default_binding(text, line_number, "Q_ml_series");
            let mut lines = Vec::new();
            if !binding_defined_before(text, &series, line_number) {
                lines.push(format!(
                    "{indent}{series}: TimeSeries[Time] of HeatRate [kW] = 0 kW"
                ));
            }
            lines.push(format!(
                "{indent}{name} = train_test_split({series}, target={series}, features=[feature_1], test=0.25, seed=7)"
            ));
            format!("{}{}", lines.join(newline), newline)
        }
        MlExpectedSourceKind::Model => {
            let split = ml_existing_or_default_binding(text, line_number, "split");
            let mut lines = Vec::new();
            if !binding_defined_before(text, &split, line_number) {
                let series = ml_existing_or_default_binding(text, line_number, "Q_ml_series");
                if !binding_defined_before(text, &series, line_number) {
                    lines.push(format!(
                        "{indent}{series}: TimeSeries[Time] of HeatRate [kW] = 0 kW"
                    ));
                }
                lines.push(format!(
                    "{indent}{split} = train_test_split({series}, target={series}, features=[feature_1], test=0.25, seed=7)"
                ));
            }
            lines.push(format!(
                "{indent}{name} = regression({split}, algorithm=linear)"
            ));
            format!("{}{}", lines.join(newline), newline)
        }
        MlExpectedSourceKind::Table => ml_table_skeleton(text, name, indent),
    }
}

fn ml_source_adapter_skeleton(
    text: &str,
    line_number: usize,
    source: &str,
    expected: MlExpectedSourceKind,
    indent: &str,
) -> Option<(String, String)> {
    let newline = document_newline(text);
    match expected {
        MlExpectedSourceKind::TimeSeries => {
            let binding = ml_unique_binding_name(text, line_number, "Q_ml_series");
            Some((
                binding.clone(),
                format!("{indent}{binding}: TimeSeries[Time] of HeatRate [kW] = 0 kW{newline}"),
            ))
        }
        MlExpectedSourceKind::TrainTestSplit => {
            let binding = ml_unique_binding_name(text, line_number, "split");
            Some((
                binding.clone(),
                format!(
                    "{indent}{binding} = train_test_split({source}, target={source}, features=[feature_1], test=0.25, seed=7){newline}"
                ),
            ))
        }
        MlExpectedSourceKind::Model => {
            let binding = ml_unique_binding_name(text, line_number, "reg_model");
            Some((
                binding.clone(),
                format!("{indent}{binding} = regression({source}, algorithm=linear){newline}"),
            ))
        }
        MlExpectedSourceKind::Table => {
            let binding = ml_unique_binding_name(text, line_number, "samples");
            Some((binding.clone(), ml_table_skeleton(text, &binding, indent)))
        }
    }
}

fn ml_source_adapter_title(expected: MlExpectedSourceKind, source: &str, binding: &str) -> String {
    match expected {
        MlExpectedSourceKind::TimeSeries => format!("Create ML TimeSeries {binding}"),
        MlExpectedSourceKind::TrainTestSplit => format!("Create ML split from {source}"),
        MlExpectedSourceKind::Model => format!("Create ML model from {source}"),
        MlExpectedSourceKind::Table => format!("Create ML table {binding}"),
    }
}

fn ml_table_skeleton(text: &str, name: &str, indent: &str) -> String {
    let newline = document_newline(text);
    [
        format!("{indent}{name} = sample lhs"),
        format!("{indent}with {{"),
        format!("{indent}    count = 1"),
        format!("{indent}    seed = 42"),
        format!("{indent}    feature_1 = uniform(0, 1)"),
        format!("{indent}}}"),
    ]
    .join(newline)
        + newline
}

fn ml_existing_or_default_binding(text: &str, line_number: usize, name: &str) -> String {
    if binding_defined_before(text, name, line_number) {
        name.to_owned()
    } else {
        ml_unique_binding_name(text, line_number, name)
    }
}

fn ml_unique_binding_name(text: &str, line_number: usize, base: &str) -> String {
    if !binding_defined_before(text, base, line_number)
        && !binding_defined_after(text, base, line_number)
    {
        return base.to_owned();
    }
    for suffix in 2..100 {
        let candidate = format!("{base}_{suffix}");
        if !binding_defined_before(text, &candidate, line_number)
            && !binding_defined_after(text, &candidate, line_number)
        {
            return candidate;
        }
    }
    format!("{base}_new")
}

fn binding_defined_before(text: &str, name: &str, line_limit: usize) -> bool {
    text.lines()
        .take(line_limit)
        .any(|line| binding_line_defines_name(line, name))
}

fn binding_defined_after(text: &str, name: &str, line_number: usize) -> bool {
    text.lines()
        .skip(line_number + 1)
        .any(|line| binding_line_defines_name(line, name))
}

fn binding_line_defines_name(line: &str, name: &str) -> bool {
    let code = strip_line_comment(line);
    let start = line_indent(code).len();
    let rest = &code[start..];
    let Some(after_name) = rest.strip_prefix(name) else {
        return false;
    };
    if after_name
        .chars()
        .next()
        .is_some_and(is_identifier_character)
    {
        return false;
    }
    let Some(separator) = after_name.trim_start().chars().next() else {
        return false;
    };
    separator == '=' || separator == ':'
}

fn ml_source_token_range(line: &str, source: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find(source) else {
            break;
        };
        let start = search_start + relative_start;
        let end = start + source.len();
        if identifier_boundary(code, start, end) {
            return Some((start, end));
        }
        search_start = end;
    }
    None
}

fn lsp_uncertainty_source_code_actions(uri: &str, text: &str, diagnostic: &Value) -> Vec<Value> {
    let Some(line_number) = diagnostic_line(diagnostic) else {
        return Vec::new();
    };
    let Some(line) = text.lines().nth(line_number) else {
        return Vec::new();
    };
    let message = diagnostic_message(diagnostic);
    let mut actions = Vec::new();

    if message.contains("Unknown uncertainty source") {
        if let Some(source) = uncertainty_source_name_from_diagnostic(message) {
            let indent = line_indent(line);
            let placeholder = format!("{indent}{}", uncertainty_source_definition(source, line));
            actions.push(json!({
                "title": format!("Define uncertainty source {source}"),
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": single_change_workspace_edit(
                    uri,
                    zero_width_range(line_number, 0),
                    &format!("{placeholder}{}", document_newline(text))
                )
            }));
        }
    }

    if message.contains("requires a prior uncertainty binding as its first argument") {
        if let Some(open_paren_byte) = uncertainty_source_call_open_paren(line) {
            let indent = line_indent(line);
            let source = "Q_source_unc";
            let placeholder = format!("{indent}{}", uncertainty_source_definition(source, line));
            actions.push(json!({
                "title": "Add uncertainty source Q_source_unc",
                "kind": "quickfix",
                "isPreferred": true,
                "diagnostics": [diagnostic.clone()],
                "edit": workspace_edit_for_edits(
                    uri,
                    json!([
                        {
                            "range": zero_width_range(line_number, 0),
                            "newText": format!("{placeholder}{}", document_newline(text))
                        },
                        {
                            "range": line_byte_range(
                                line_number,
                                line,
                                open_paren_byte + 1,
                                open_paren_byte + 1
                            ),
                            "newText": format!("{source}, ")
                        }
                    ])
                )
            }));
        }
    }

    if message.contains("not an uncertainty source") {
        if let Some(source) = uncertainty_source_name_from_diagnostic(message) {
            if let Some(range) = binding_expression_range_for_name(text, source, line_number) {
                if expression_starts_numeric(range.expression)
                    && !is_uncertainty_call_expression(range.expression)
                {
                    if let Some(unit) = first_unit_on_line(range.expression) {
                        let std_unit = if unit == "degC" { "K" } else { &unit };
                        let replacement =
                            format!("measured({}, std=0.8 {std_unit})", range.expression.trim());
                        actions.push(json!({
                            "title": format!("Convert {source} to measured uncertainty source"),
                            "kind": "quickfix",
                            "isPreferred": true,
                            "diagnostics": [diagnostic.clone()],
                            "edit": single_change_workspace_edit(
                                uri,
                                line_byte_range(
                                    range.line_number,
                                    range.line,
                                    range.expression_start,
                                    range.expression_end
                                ),
                                &replacement
                            )
                        }));
                    }
                }
            }
        }
    }

    actions
}

fn uncertainty_source_definition(source: &str, line: &str) -> String {
    let unit = first_unit_on_line(line).unwrap_or_else(|| "kW".to_owned());
    let (mean, std) = uncertainty_normal_literals(&unit);
    format!("{source} = normal(mean={mean}, std={std}, samples=31)")
}

fn uncertainty_normal_literals(unit: &str) -> (String, String) {
    if unit == "degC" {
        return ("20 degC".to_owned(), "0.8 K".to_owned());
    }
    if unit == "%" {
        return ("50 %".to_owned(), "5 %".to_owned());
    }
    (format!("5 {unit}"), format!("0.8 {unit}"))
}

fn uncertainty_source_name_from_diagnostic(message: &str) -> Option<&str> {
    if message.contains("Unknown uncertainty source") {
        let (_, after_marker) = message.split_once("Unknown uncertainty source `")?;
        let (source, _) = after_marker.split_once('`')?;
        let source = source.trim();
        return is_identifier(source).then_some(source);
    }
    let payload = first_backtick_payload(message)?.trim();
    is_identifier(payload).then_some(payload)
}

fn uncertainty_source_call_open_paren(line: &str) -> Option<usize> {
    let code = strip_line_comment(line);
    for call in ["propagate", "ensemble", "probability"] {
        let mut search_start = 0usize;
        while search_start < code.len() {
            let Some(relative_start) = code[search_start..].find(call) else {
                break;
            };
            let start = search_start + relative_start;
            let after_name = start + call.len();
            if identifier_boundary(code, start, after_name) {
                let whitespace = code[after_name..]
                    .chars()
                    .take_while(|character| character.is_whitespace())
                    .map(char::len_utf8)
                    .sum::<usize>();
                let open = after_name + whitespace;
                if code.as_bytes().get(open) == Some(&b'(') {
                    return Some(open);
                }
            }
            search_start = after_name;
        }
    }
    None
}

struct BindingExpressionRange<'a> {
    line_number: usize,
    line: &'a str,
    expression: &'a str,
    expression_start: usize,
    expression_end: usize,
}

fn binding_expression_range_for_name<'a>(
    text: &'a str,
    name: &str,
    line_limit: usize,
) -> Option<BindingExpressionRange<'a>> {
    for (line_number, line) in text.lines().enumerate().take(line_limit) {
        let code = strip_line_comment(line);
        let start = line_indent(code).len();
        let rest = &code[start..];
        let Some(after_name) = rest.strip_prefix(name) else {
            continue;
        };
        if after_name
            .chars()
            .next()
            .is_some_and(is_identifier_character)
        {
            continue;
        }
        let Some(equals_offset) = after_name.find('=') else {
            continue;
        };
        if !after_name[..equals_offset].trim().is_empty() {
            continue;
        }
        let expression_start = start + name.len() + equals_offset + 1;
        let expression_end = code.trim_end().len();
        let expression = code[expression_start..expression_end].trim();
        if expression.is_empty() {
            continue;
        }
        let leading = code[expression_start..expression_end]
            .len()
            .saturating_sub(code[expression_start..expression_end].trim_start().len());
        let trailing = code[expression_start..expression_end]
            .len()
            .saturating_sub(code[expression_start..expression_end].trim_end().len());
        return Some(BindingExpressionRange {
            line_number,
            line,
            expression,
            expression_start: expression_start + leading,
            expression_end: expression_end.saturating_sub(trailing),
        });
    }
    None
}

fn expression_starts_numeric(expression: &str) -> bool {
    let trimmed = expression.trim_start();
    let Some(first) = trimmed.as_bytes().first() else {
        return false;
    };
    first.is_ascii_digit()
}

fn is_uncertainty_call_expression(expression: &str) -> bool {
    let trimmed = expression.trim_start();
    [
        "measured(",
        "interval(",
        "normal(",
        "uniform(",
        "distribution(",
        "ensemble(",
        "propagate(",
        "probability(",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn lsp_reorder_where_local_definition_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let name = first_backtick_payload(diagnostic_message(diagnostic))?.trim();
    if !is_identifier(name) {
        return None;
    }
    let use_line = diagnostic_line(diagnostic)?;
    let block = where_block_range(text, use_line)?;
    let definition_line = where_local_definition_line(text, name, use_line + 1, block.end)?;
    let definition_text = text.lines().nth(definition_line)?;
    let definition_code = strip_line_comment(definition_text);
    if definition_code.contains('{') || definition_code.contains('}') {
        return None;
    }

    Some(json!({
        "title": format!("Move {name} definition before first use"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": workspace_edit_for_edits(
            uri,
            json!([
                {
                    "range": zero_width_range(use_line, 0),
                    "newText": format!("{}{}", definition_text, document_newline(text))
                },
                {
                    "range": full_line_range(text, definition_line),
                    "newText": ""
                }
            ])
        )
    }))
}

struct LineBlock {
    start: usize,
    end: usize,
}

fn where_block_range(text: &str, line_number: usize) -> Option<LineBlock> {
    let lines = text.lines().collect::<Vec<_>>();
    let start = (0..=line_number)
        .rev()
        .find(|index| is_where_block_start(lines.get(*index).copied().unwrap_or("")))?;
    let end = matching_block_end_line(&lines, start)?;
    (end > line_number).then_some(LineBlock { start, end })
}

fn lsp_promote_where_local_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let name = first_backtick_payload(diagnostic_message(diagnostic))?.trim();
    if !is_identifier(name) {
        return None;
    }
    let escape_line = diagnostic_line(diagnostic)?;
    let (block, definition_line) = where_block_defining_before(text, name, escape_line)?;
    let definition_text = text.lines().nth(definition_line)?;
    let definition_code = strip_line_comment(definition_text);
    if definition_code.contains('{') || definition_code.contains('}') {
        return None;
    }
    let owner_line = block.start.checked_sub(1)?;
    let promoted_definition = definition_text.trim_start();
    if promoted_definition.is_empty() {
        return None;
    }
    let removal_range = if where_block_meaningful_line_count(text, block.start, block.end) == 1 {
        full_line_block_range(text, block.start, block.end)
    } else {
        full_line_range(text, definition_line)
    };

    Some(json!({
        "title": format!("Promote {name} to top-level binding"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": workspace_edit_for_edits(
            uri,
            json!([
                {
                    "range": zero_width_range(owner_line, 0),
                    "newText": format!("{}{}", promoted_definition, document_newline(text))
                },
                {
                    "range": removal_range,
                    "newText": ""
                }
            ])
        )
    }))
}

fn where_block_defining_before(
    text: &str,
    name: &str,
    line_limit: usize,
) -> Option<(LineBlock, usize)> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut selected = None;
    for start in 0..line_limit {
        if !is_where_block_start(lines.get(start).copied().unwrap_or("")) {
            continue;
        }
        let Some(end) = matching_block_end_line(&lines, start) else {
            continue;
        };
        if end >= line_limit {
            continue;
        }
        if let Some(definition_line) = where_local_definition_line(text, name, start + 1, end) {
            selected = Some((LineBlock { start, end }, definition_line));
        }
    }
    selected
}

fn where_block_meaningful_line_count(text: &str, start_line: usize, end_line: usize) -> usize {
    text.lines()
        .enumerate()
        .skip(start_line + 1)
        .take(end_line.saturating_sub(start_line + 1))
        .filter(|(_line_number, line)| !strip_line_comment(line).trim().is_empty())
        .count()
}

fn is_where_block_start(line: &str) -> bool {
    let trimmed = strip_line_comment(line).trim();
    let Some(rest) = trimmed.strip_prefix("where") else {
        return false;
    };
    rest.trim() == "{"
}

fn where_local_definition_line(
    text: &str,
    name: &str,
    start_line: usize,
    end_line: usize,
) -> Option<usize> {
    text.lines()
        .enumerate()
        .skip(start_line)
        .take(end_line.saturating_sub(start_line))
        .find_map(|(line_number, line)| {
            where_local_definition_matches(strip_line_comment(line), name).then_some(line_number)
        })
}

fn where_local_definition_matches(line: &str, name: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix(name) else {
        return false;
    };
    if rest.chars().next().is_some_and(is_identifier_character) {
        return false;
    }
    rest.trim_start().starts_with('=')
}

fn uncertainty_call_example_from_diagnostic(message: &str) -> Option<&str> {
    for payload in backtick_payloads(message) {
        let candidate = payload.trim();
        if candidate.ends_with(')')
            && [
                "measured(",
                "interval(",
                "normal(",
                "uniform(",
                "distribution(",
                "propagate(",
                "ensemble(",
                "probability(",
            ]
            .iter()
            .any(|prefix| candidate.starts_with(prefix))
        {
            return Some(candidate);
        }
    }
    None
}

fn uncertainty_call_range_on_line(line: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    for call in [
        "measured",
        "interval",
        "normal",
        "uniform",
        "distribution",
        "propagate",
        "ensemble",
        "probability",
    ] {
        let mut search_start = 0usize;
        while search_start < code.len() {
            let Some(relative_start) = code[search_start..].find(call) else {
                break;
            };
            let start = search_start + relative_start;
            let after_name = start + call.len();
            if identifier_boundary(code, start, after_name) {
                let whitespace = code[after_name..]
                    .chars()
                    .take_while(|character| character.is_whitespace())
                    .map(char::len_utf8)
                    .sum::<usize>();
                let open = after_name + whitespace;
                if code.as_bytes().get(open) == Some(&b'(') {
                    let close = matching_close_paren_byte(code, open)?;
                    return Some((start, close + 1));
                }
            }
            search_start = after_name;
        }
    }
    None
}

fn named_argument_value_range(line: &str, names: &[&str]) -> Option<OptionAssignmentRange> {
    let code = strip_line_comment(line);
    let mut index = 0usize;
    while index < code.len() {
        for name in names {
            let end_name = index + name.len();
            if end_name > code.len()
                || !code[index..].starts_with(name)
                || !identifier_boundary(code, index, end_name)
            {
                continue;
            }
            let mut cursor = end_name;
            while cursor < code.len() && code.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if code.as_bytes().get(cursor) != Some(&b'=') {
                continue;
            }
            cursor += 1;
            while cursor < code.len() && code.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            let value_start = cursor;
            while cursor < code.len() && !matches!(code.as_bytes()[cursor], b',' | b')') {
                cursor += 1;
            }
            let value_end = value_start + code[value_start..cursor].trim_end().len();
            if value_end > value_start {
                return Some(OptionAssignmentRange {
                    option_name: (*name).to_owned(),
                    value_start,
                    value_end,
                });
            }
        }
        let Some((next_index, _)) = code[index..].char_indices().nth(1) else {
            break;
        };
        index += next_index;
    }
    None
}

fn matching_close_paren_byte(text: &str, open_byte: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in text[open_byte..].char_indices() {
        let byte_index = open_byte + index;
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(byte_index);
                }
            }
            _ => {}
        }
    }
    None
}

fn identifier_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        text[..start].chars().next_back()
    };
    let after = text[end..].chars().next();
    before.is_none_or(|character| !is_identifier_character(character))
        && after.is_none_or(|character| !is_identifier_character(character))
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn lsp_parenthesize_command_target_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let message = diagnostic_message(diagnostic);
    if !message.contains("ambiguous without parentheses") {
        return None;
    }
    let target = first_backtick_payload(message)?.trim();
    if target.is_empty() || target.starts_with('(') {
        return None;
    }
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let start_byte = code.find(target)?;
    let end_byte = start_byte + target.len();
    let before = code[..start_byte]
        .chars()
        .rev()
        .find(|character| !character.is_whitespace());
    let after = code[end_byte..]
        .chars()
        .find(|character| !character.is_whitespace());
    if before == Some('(') && after == Some(')') {
        return None;
    }
    let replacement = format!("({target})");
    Some(json!({
        "title": "Parenthesize command target",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            &replacement
        )
    }))
}

fn lsp_stdlib_module_replacement_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let unknown = stdlib_module_name_from_diagnostic(diagnostic_message(diagnostic))?;
    let replacement = closest_stdlib_module_name(unknown)?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let start_byte = line.find(unknown)?;
    let end_byte = start_byte + unknown.len();
    Some(json!({
        "title": format!("Replace {unknown} with {replacement}"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, start_byte, end_byte),
            &replacement
        )
    }))
}

fn lsp_remove_stdlib_module_import_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    status: &str,
) -> Option<Value> {
    let module = stdlib_module_name_from_diagnostic(diagnostic_message(diagnostic))?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    if !strip_line_comment(line).contains(module) {
        return None;
    }
    Some(json!({
        "title": format!("Remove {status} stdlib module import"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, full_line_range(text, line_number), "")
    }))
}

fn stdlib_module_name_from_diagnostic(message: &str) -> Option<&str> {
    backtick_payloads(message)
        .into_iter()
        .find(|payload| payload.starts_with("eng."))
}

fn closest_stdlib_module_name(unknown: &str) -> Option<String> {
    let registry = bundled_module_registry().ok()?;
    let (distance, name) = registry
        .modules
        .iter()
        .map(|module| (edit_distance(unknown, &module.name), module.name.as_str()))
        .filter(|(_distance, name)| *name != unknown)
        .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))?;
    if distance <= 2 || (distance <= 3 && unknown.len() >= 8) {
        Some(name.to_owned())
    } else {
        None
    }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_byte) in left.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_byte) in right.iter().enumerate() {
            let substitution = usize::from(left_byte != right_byte);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

fn expected_sha256_from_diagnostic(diagnostic: &Value) -> Option<String> {
    let message = diagnostic.get("message")?.as_str()?;
    expected_sha256_after(message, "offline response SHA256 was `")
        .or_else(|| expected_sha256_after(message, "fixture SHA256 was `"))
        .or_else(|| expected_sha256_after(message, "observed `"))
}

fn expected_sha256_after(message: &str, marker: &str) -> Option<String> {
    let (_, rest) = message.split_once(marker)?;
    let (hash, _) = rest.split_once('`')?;
    if hash.len() == 64 && hash.chars().all(|character| character.is_ascii_hexdigit()) {
        Some(hash.to_ascii_lowercase())
    } else {
        None
    }
}

fn net_url_literal_range(line: &str) -> Option<(usize, usize)> {
    call_string_argument_range(line, "url").or_else(|| first_string_literal_range(line))
}

fn call_string_argument_range(line: &str, function_name: &str) -> Option<(usize, usize)> {
    let mut search_start = 0usize;
    while search_start < line.len() {
        let Some(relative_start) = line[search_start..].find(function_name) else {
            break;
        };
        let start = search_start + relative_start;
        let after_name = start + function_name.len();
        if identifier_boundary(line, start, after_name) {
            let mut cursor = after_name;
            while cursor < line.len() && line.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if line.as_bytes().get(cursor) == Some(&b'(') {
                cursor += 1;
                while cursor < line.len() && line.as_bytes()[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if let Some(range) = string_literal_range_at(line, cursor) {
                    return Some(range);
                }
            }
        }
        search_start = after_name;
    }
    None
}

fn first_string_literal_range(line: &str) -> Option<(usize, usize)> {
    let quote = line.find('"')?;
    string_literal_range_at(line, quote)
}

fn string_literal_range_at(line: &str, quote_start: usize) -> Option<(usize, usize)> {
    if line.as_bytes().get(quote_start) != Some(&b'"') {
        return None;
    }
    let mut escaped = false;
    for (relative_index, character) in line[quote_start + 1..].char_indices() {
        let index = quote_start + 1 + relative_index;
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some((quote_start, index + '"'.len_utf8())),
            _ => {}
        }
    }
    None
}

fn owner_line_for_enclosing_with_block(lines: &[&str], line_number: usize) -> Option<usize> {
    let mut cursor = line_number;
    while cursor > 0 {
        cursor -= 1;
        if strip_line_comment(lines.get(cursor).copied().unwrap_or("")).trim() != "with {" {
            continue;
        }
        let mut owner = cursor;
        while owner > 0 {
            owner -= 1;
            if !strip_line_comment(lines.get(owner).copied().unwrap_or(""))
                .trim()
                .is_empty()
            {
                return Some(owner);
            }
        }
        return None;
    }
    None
}

fn http_method_token_range(line: &str) -> Option<(usize, usize)> {
    let code = strip_line_comment(line);
    let mut search_start = 0usize;
    while search_start < code.len() {
        let Some(relative_start) = code[search_start..].find("http") else {
            break;
        };
        let http_start = search_start + relative_start;
        let after_http = http_start + "http".len();
        if identifier_boundary(code, http_start, after_http) {
            let mut cursor = after_http;
            while cursor < code.len() && code.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            let method_start = cursor;
            while cursor < code.len() && code.as_bytes()[cursor].is_ascii_alphabetic() {
                cursor += 1;
            }
            let method = &code[method_start..cursor];
            if ["get", "head", "request", "fetch"]
                .iter()
                .any(|candidate| method.eq_ignore_ascii_case(candidate))
            {
                return Some((method_start, cursor));
            }
        }
        search_start = after_http;
    }
    None
}

struct OptionAssignmentRange {
    option_name: String,
    value_start: usize,
    value_end: usize,
}

fn option_assignment_range(line: &str, option_names: &[&str]) -> Option<OptionAssignmentRange> {
    let indent_len = line_indent(line).len();
    let rest = &line[indent_len..];
    for option_name in option_names {
        let Some(after_name) = rest.strip_prefix(option_name) else {
            continue;
        };
        if !after_name
            .chars()
            .next()
            .is_some_and(|character| character.is_whitespace() || character == '=')
        {
            continue;
        }
        let equals_offset = after_name.find('=')?;
        if !after_name[..equals_offset].trim().is_empty() {
            continue;
        }
        let raw_value_start = indent_len + option_name.len() + equals_offset + 1;
        let value_start = raw_value_start
            + line[raw_value_start..]
                .chars()
                .take_while(|character| character.is_whitespace())
                .map(char::len_utf8)
                .sum::<usize>();
        let comment_start = line_comment_start(&line[value_start..])
            .map(|offset| value_start + offset)
            .unwrap_or(line.len());
        let value_end = value_start + line[value_start..comment_start].trim_end().len();
        return Some(OptionAssignmentRange {
            option_name: (*option_name).to_owned(),
            value_start,
            value_end,
        });
    }
    None
}

fn assignment_head_byte_range(line: &str, name: &str) -> Option<(usize, usize)> {
    let start = line_indent(line).len();
    let rest = &line[start..];
    let after_name = rest.strip_prefix(name)?;
    let equals_offset = after_name.find('=')?;
    if !after_name[..equals_offset].trim().is_empty() {
        return None;
    }
    Some((start, start + name.len() + equals_offset + 1))
}

fn line_byte_range(line_number: usize, line: &str, start_byte: usize, end_byte: usize) -> Value {
    json!({
        "start": {
            "line": line_number,
            "character": utf16_len(&line[..start_byte])
        },
        "end": {
            "line": line_number,
            "character": utf16_len(&line[..end_byte])
        }
    })
}

fn full_line_same_line_range(line_number: usize, line: &str) -> Value {
    json!({
        "start": { "line": line_number, "character": 0 },
        "end": { "line": line_number, "character": utf16_len(line) }
    })
}

fn selected_line_range(
    start_line: usize,
    end_line: usize,
    end_character: usize,
    line_count: usize,
) -> Option<(usize, usize)> {
    if line_count == 0 || start_line >= line_count || end_line >= line_count {
        return None;
    }
    let format_end_line = if end_character == 0 && end_line > start_line {
        end_line - 1
    } else {
        end_line
    };
    (start_line <= format_end_line && format_end_line < line_count)
        .then_some((start_line, format_end_line))
}

fn full_line_selection_range(lines: &[&str], start_line: usize, end_line: usize) -> Value {
    let end_text = lines.get(end_line).copied().unwrap_or("");
    json!({
        "start": { "line": start_line, "character": 0 },
        "end": { "line": end_line, "character": utf16_len(end_text) }
    })
}

fn zero_width_range(line_number: usize, character: usize) -> Value {
    json!({
        "start": { "line": line_number, "character": character },
        "end": { "line": line_number, "character": character }
    })
}

fn line_indent(line: &str) -> &str {
    let end = line
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(line.len());
    &line[..end]
}

fn document_newline(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn split_lines_preserve_logical(text: &str) -> Vec<&str> {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect()
}

fn first_backtick_payload(text: &str) -> Option<&str> {
    backtick_payloads(text).into_iter().next()
}

fn last_backtick_payload(text: &str) -> Option<&str> {
    backtick_payloads(text).into_iter().last()
}

fn backtick_payloads(text: &str) -> Vec<&str> {
    let mut payloads = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('`') else {
            break;
        };
        payloads.push(&after_open[..close]);
        rest = &after_open[close + 1..];
    }
    payloads
}

fn bracket_payload(text: &str) -> Option<&str> {
    let open = text.find('[')?;
    let after_open = &text[open + 1..];
    let close = after_open.find(']')?;
    Some(after_open[..close].trim())
}

fn lsp_remove_script_wrapper_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let start_line = diagnostic
        .pointer("/range/start/line")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())?;
    let lines = text.lines().collect::<Vec<_>>();
    let start_text = *lines.get(start_line)?;
    if !is_script_wrapper_start(start_text) {
        return None;
    }
    let end_line = matching_block_end_line(&lines, start_line)?;
    if end_line <= start_line || lines.get(end_line)?.trim() != "}" {
        return None;
    }
    let edits = json!([
        {
            "range": full_line_range(text, end_line),
            "newText": ""
        },
        {
            "range": full_line_range(text, start_line),
            "newText": ""
        }
    ]);
    Some(json!({
        "title": "Promote script body to top-level workflow",
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": workspace_edit_for_edits(uri, edits)
    }))
}

fn is_script_wrapper_start(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.ends_with('{') {
        return false;
    }
    let head = trimmed.trim_end_matches('{').trim();
    if head == "script" {
        return true;
    }
    let Some(name) = head.strip_prefix("script ") else {
        return false;
    };
    is_identifier(name.trim())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn matching_block_end_line(lines: &[&str], start_line: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (line_number, line) in lines.iter().enumerate().skip(start_line) {
        for character in strip_line_comment(line).chars() {
            match character {
                '{' => depth = depth.saturating_add(1),
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(line_number);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn strip_line_comment(line: &str) -> &str {
    line_comment_start(line)
        .map(|comment_start| &line[..comment_start])
        .unwrap_or(line)
}

fn line_comment_start(line: &str) -> Option<usize> {
    let mut in_string = false;
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'\\' && in_string {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if !in_string && bytes[index..].starts_with(b"//") {
            return Some(index);
        }
        if !in_string && bytes[index] == b'#' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn full_line_range(text: &str, line_number: usize) -> Value {
    let lines = text.split('\n').collect::<Vec<_>>();
    if line_number + 1 < lines.len() {
        return json!({
            "start": { "line": line_number, "character": 0 },
            "end": { "line": line_number + 1, "character": 0 }
        });
    }
    let line = lines
        .get(line_number)
        .copied()
        .unwrap_or("")
        .trim_end_matches('\r');
    json!({
        "start": { "line": line_number, "character": 0 },
        "end": { "line": line_number, "character": utf16_len(line) }
    })
}

fn full_line_block_range(text: &str, start_line: usize, end_line: usize) -> Value {
    let lines = text.split('\n').collect::<Vec<_>>();
    if end_line + 1 < lines.len() {
        return json!({
            "start": { "line": start_line, "character": 0 },
            "end": { "line": end_line + 1, "character": 0 }
        });
    }
    let line = lines
        .get(end_line)
        .copied()
        .unwrap_or("")
        .trim_end_matches('\r');
    json!({
        "start": { "line": start_line, "character": 0 },
        "end": { "line": end_line, "character": utf16_len(line) }
    })
}

fn single_change_workspace_edit(uri: &str, range: Value, new_text: &str) -> Value {
    workspace_edit_for_edits(
        uri,
        json!([{
            "range": range,
            "newText": new_text
        }]),
    )
}

fn workspace_edit_for_edits(uri: &str, edits: Value) -> Value {
    let mut changes = serde_json::Map::new();
    changes.insert(uri.to_owned(), edits);
    json!({ "changes": changes })
}

fn document_text_for_uri(uri: &str, documents: &Documents) -> Option<String> {
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    documents
        .get(uri)
        .map(|state| state.text.clone())
        .or_else(|| std::fs::read_to_string(&path).ok())
}

fn full_document_range(text: &str) -> Value {
    let mut lines = text.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let end_line = lines.len().saturating_sub(1);
    let end_text = lines.last().copied().unwrap_or("").trim_end_matches('\r');
    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": end_line, "character": utf16_len(end_text) }
    })
}

fn semantic_tokens_range_for_request(
    request: &Value,
    documents: &Documents,
) -> Option<eng_lsp::LspSemanticTokens> {
    let mut tokens = semantic_tokens_for_request(request, documents)?;
    let ((start_line, start_character), (end_line, end_character)) = request_range(request)?;
    tokens.tokens.retain(|token| {
        semantic_token_intersects_range(token, start_line, start_character, end_line, end_character)
    });
    Some(tokens)
}

fn semantic_token_intersects_range(
    token: &eng_lsp::LspSemanticToken,
    start_line: usize,
    start_character: usize,
    end_line: usize,
    end_character: usize,
) -> bool {
    if token.line < start_line || token.line > end_line {
        return false;
    }
    let token_end = token.start.saturating_add(token.length);
    if token.line == start_line && token_end <= start_character {
        return false;
    }
    if token.line == end_line && token.start >= end_character {
        return false;
    }
    true
}

fn snapshot_for_request(request: &Value, documents: &Documents) -> Option<eng_lsp::LspSnapshot> {
    let uri = request_uri(request)?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let text = document_text_for_uri(uri, documents)?;
    Some(snapshot_for_source(&path, &text))
}

const MAX_WORKSPACE_SYMBOL_FILES: usize = 500;
const MAX_WORKSPACE_SYMBOL_RESULTS: usize = 200;

fn workspace_roots_from_initialize(request: &Value) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(uri) = request.pointer("/params/rootUri").and_then(Value::as_str) {
        if let Some(path) = path_from_uri(uri) {
            roots.push(path);
        }
    }
    if let Some(folders) = request
        .pointer("/params/workspaceFolders")
        .and_then(Value::as_array)
    {
        for folder in folders {
            let Some(uri) = folder.get("uri").and_then(Value::as_str) else {
                continue;
            };
            let Some(path) = path_from_uri(uri) else {
                continue;
            };
            if !roots.contains(&path) {
                roots.push(path);
            }
        }
    }
    roots
}

fn workspace_symbols_for_request(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
) -> Vec<Value> {
    let query = request
        .pointer("/params/query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut results = Vec::new();
    let mut seen = HashSet::<(String, usize, String)>::new();

    for (uri, state) in documents {
        if results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            break;
        }
        let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
        push_workspace_symbols_from_source(
            uri,
            &path,
            &state.text,
            &query,
            &mut results,
            &mut seen,
        );
    }

    let mut files = Vec::new();
    for root in workspace_roots {
        collect_workspace_eng_files(root, &mut files, MAX_WORKSPACE_SYMBOL_FILES);
        if files.len() >= MAX_WORKSPACE_SYMBOL_FILES {
            break;
        }
    }
    for path in files {
        if results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            break;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        let uri = file_uri_from_path(&canonical);
        if documents.contains_key(&uri) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&canonical) else {
            continue;
        };
        push_workspace_symbols_from_source(
            &uri,
            &canonical,
            &source,
            &query,
            &mut results,
            &mut seen,
        );
    }

    results
}

fn push_workspace_symbols_from_source(
    uri: &str,
    path: &Path,
    source: &str,
    query: &str,
    results: &mut Vec<Value>,
    seen: &mut HashSet<(String, usize, String)>,
) {
    let snapshot = snapshot_for_source(path, source);
    push_workspace_symbols_from_document_symbols(
        uri,
        &snapshot.document_symbols,
        query,
        results,
        seen,
    );
}

fn push_workspace_symbols_from_document_symbols(
    uri: &str,
    symbols: &[eng_lsp::LspDocumentSymbol],
    query: &str,
    results: &mut Vec<Value>,
    seen: &mut HashSet<(String, usize, String)>,
) {
    for symbol in symbols {
        if results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            return;
        }
        if workspace_symbol_matches(symbol, query)
            && seen.insert((uri.to_owned(), symbol.line, symbol.name.clone()))
        {
            results.push(workspace_symbol_json(uri, symbol));
        }
        push_workspace_symbols_from_document_symbols(uri, &symbol.children, query, results, seen);
    }
}

fn workspace_symbol_matches(symbol: &eng_lsp::LspDocumentSymbol, query: &str) -> bool {
    query.is_empty()
        || symbol.name.to_ascii_lowercase().contains(query)
        || symbol.detail.to_ascii_lowercase().contains(query)
}

fn workspace_symbol_json(uri: &str, symbol: &eng_lsp::LspDocumentSymbol) -> Value {
    json!({
        "name": symbol.name,
        "kind": symbol.kind,
        "location": {
            "uri": uri,
            "range": {
                "start": { "line": symbol.line, "character": symbol.character },
                "end": { "line": symbol.end_line, "character": symbol.end_character }
            }
        },
        "containerName": symbol.detail
    })
}

fn collect_workspace_eng_files(root: &Path, files: &mut Vec<PathBuf>, limit: usize) {
    if files.len() >= limit {
        return;
    }
    let Ok(metadata) = std::fs::metadata(root) else {
        return;
    };
    if metadata.is_file() {
        if root.extension().is_some_and(|extension| extension == "eng") {
            files.push(root.to_path_buf());
        }
        return;
    }
    if !metadata.is_dir() || skip_workspace_symbol_dir(root) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= limit {
            break;
        }
        collect_workspace_eng_files(&entry.path(), files, limit);
    }
}

fn skip_workspace_symbol_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git" | ".vscode" | "target" | "dist" | "node_modules" | "__pycache__"
    )
}

fn completions_for_request(request: &Value, documents: &Documents) -> Vec<eng_lsp::LspCompletion> {
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
    if let Some(text) = document_text_for_uri(uri, documents) {
        return completion_items_for_source_position(&path, &text, line, character);
    }
    completion_items_for_path_position(&path, line, character).unwrap_or_default()
}

fn hover_for_request(request: &Value, documents: &Documents) -> Option<eng_lsp::LspHover> {
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
    let text = document_text_for_uri(uri, documents);
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

fn definition_for_request(request: &Value, documents: &Documents) -> Option<Value> {
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
    let text = document_text_for_uri(uri, documents)?;
    let snapshot = snapshot_for_source(&path, &text);
    let symbol = symbol_at_position(&text, line_zero_based, character)?;
    if let Some(target) = stdlib_module_definition_target(&symbol) {
        return Some(definition_location_json(&target));
    }
    let hover = hover_for_symbol(&snapshot.hovers, &symbol)?;
    let label = definition_label_for_hover_name(&hover.name);
    let target = definition_target_in_source(uri, &text, &label, hover.line)
        .or_else(|| imported_definition_target(&path, &text, &label, hover.line))?;
    Some(definition_location_json(&target))
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionTarget {
    uri: String,
    line: usize,
    start_character: usize,
    end_character: usize,
}

fn definition_label_for_hover_name(name: &str) -> String {
    let label = name.rsplit('.').next().unwrap_or(name);
    label.strip_suffix("()").unwrap_or(label).to_owned()
}

fn definition_location_json(target: &DefinitionTarget) -> Value {
    json!({
        "uri": target.uri,
        "range": {
            "start": { "line": target.line, "character": target.start_character },
            "end": { "line": target.line, "character": target.end_character }
        }
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

fn definition_target_in_source(
    uri: &str,
    source: &str,
    label: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let parsed = parse_source(source);
    let mut first_line = None;
    for item in &parsed.items {
        let Some(line) = ast_definition_line_for_label(item, label) else {
            continue;
        };
        first_line.get_or_insert(line);
        if line == preferred_line {
            return definition_target_on_line(uri, source, line, label);
        }
    }
    first_line.and_then(|line| definition_target_on_line(uri, source, line, label))
}

fn imported_definition_target(
    source_path: &Path,
    source: &str,
    label: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let base_dir = source_path.parent()?;
    let parsed = parse_source(source);
    let mut visited = HashSet::new();
    imported_definition_target_from_program(&parsed, base_dir, label, preferred_line, &mut visited)
}

fn imported_definition_target_from_program(
    parsed: &eng_compiler::ParsedProgram,
    base_dir: &Path,
    label: &str,
    preferred_line: usize,
    visited: &mut HashSet<PathBuf>,
) -> Option<DefinitionTarget> {
    for item in &parsed.items {
        let AstItem::Import(import) = item else {
            continue;
        };
        if import.kind != "file" {
            continue;
        }
        let Some(import_path) = resolve_static_import_path(base_dir, &import.target) else {
            continue;
        };
        if !visited.insert(import_path.clone()) {
            continue;
        }
        let Ok(imported_source) = std::fs::read_to_string(&import_path) else {
            visited.remove(&import_path);
            continue;
        };
        let imported_uri = file_uri_from_path(&import_path);
        if let Some(target) =
            definition_target_in_source(&imported_uri, &imported_source, label, preferred_line)
        {
            return Some(target);
        }
        let imported = parse_source(&imported_source);
        if let Some(import_base_dir) = import_path.parent() {
            if let Some(target) = imported_definition_target_from_program(
                &imported,
                import_base_dir,
                label,
                preferred_line,
                visited,
            ) {
                return Some(target);
            }
        }
        visited.remove(&import_path);
    }
    None
}

fn stdlib_module_definition_target(symbol: &str) -> Option<DefinitionTarget> {
    if !symbol
        .strip_prefix("eng.")
        .is_some_and(|name| !name.is_empty() && !name.contains('.'))
    {
        return None;
    }
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)?;
    let module_name = symbol.strip_prefix("eng.")?;
    let module_path = repo_root
        .join("stdlib")
        .join("eng")
        .join(format!("{module_name}.eng"));
    if module_path.exists() {
        return stdlib_module_file_definition_target(&module_path, symbol);
    }
    let registry_path = repo_root.join("stdlib").join("eng").join("modules.toml");
    stdlib_module_registry_definition_target(&registry_path, symbol)
}

fn stdlib_module_file_definition_target(
    path: &Path,
    module_name: &str,
) -> Option<DefinitionTarget> {
    let source = std::fs::read_to_string(path).ok()?;
    let uri = file_uri_from_path(&path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    let line = source
        .lines()
        .position(|line| line.contains(&format!("module: {module_name}")))
        .map(|line| line + 1)
        .unwrap_or(1);
    definition_target_on_line(&uri, &source, line, module_name)
}

fn stdlib_module_registry_definition_target(
    path: &Path,
    module_name: &str,
) -> Option<DefinitionTarget> {
    let source = std::fs::read_to_string(path).ok()?;
    let uri = file_uri_from_path(&path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    let header = format!("[module.\"{module_name}\"]");
    let line = source
        .lines()
        .position(|line| line.trim() == header)
        .map(|line| line + 1)?;
    definition_target_on_line(&uri, &source, line, module_name)
}

fn resolve_static_import_path(base_dir: &Path, target: &str) -> Option<PathBuf> {
    let raw = Path::new(target);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    path.canonicalize()
        .ok()
        .or_else(|| path.exists().then_some(path))
}

fn file_uri_from_path(path: &Path) -> String {
    let mut path = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = path.strip_prefix("//?/UNC/") {
        path = format!("//{stripped}");
    } else if let Some(stripped) = path.strip_prefix("//?/") {
        path = stripped.to_owned();
    }
    if path.as_bytes().get(1) == Some(&b':') {
        path = format!("/{path}");
    }
    format!("file://{}", path.replace(' ', "%20"))
}

fn ast_definition_line_for_label(item: &AstItem, label: &str) -> Option<usize> {
    match item {
        AstItem::Function(function) if function.name == label => Some(function.span.line),
        AstItem::Const(declaration) if declaration.name == label => Some(declaration.line),
        AstItem::FastBinding(binding) if binding.name == label => Some(binding.line),
        AstItem::ExplicitDecl(declaration) if declaration.name == label => Some(declaration.line),
        AstItem::Schema(schema) if schema.name == label => Some(schema.span.line),
        AstItem::Struct(structure) if structure.name == label => Some(structure.span.line),
        AstItem::Class(class_info) if class_info.name == label => Some(class_info.span.line),
        AstItem::ClassField(field) if field.name == label => Some(field.line),
        AstItem::ClassMethod(method) if method.name == label => Some(method.line),
        AstItem::ClassObject(object) if object.name == label => Some(object.line),
        AstItem::ClassObjectCopy(object) if object.name == label => Some(object.line),
        AstItem::ClassObjectField(field) if field.name == label => Some(field.line),
        AstItem::Args(args) if args.name == label => Some(args.span.line),
        AstItem::ArgsField(field) if field.name == label => Some(field.line),
        AstItem::System(system) if system.name == label => Some(system.span.line),
        AstItem::SystemVariable(variable) if variable.name == label => Some(variable.line),
        AstItem::StateSpaceTypeBlock(block) if block.name == label => Some(block.line),
        AstItem::StateSpaceTypeMember(member) if member.name == label => Some(member.line),
        AstItem::StateSpaceVector(vector) if vector.name == label => Some(vector.line),
        AstItem::Domain(domain) if domain.name == label => Some(domain.span.line),
        AstItem::DomainVariable(variable) if variable.name == label => Some(variable.line),
        AstItem::Component(component) if component.name == label => Some(component.span.line),
        AstItem::Port(port) if port.name == label => Some(port.line),
        AstItem::WhereBinding(binding) if binding.name == label => Some(binding.line),
        AstItem::WithOption(option) if option.key == label => Some(option.line),
        AstItem::Test(test) if test.name == label => Some(test.line),
        _ => None,
    }
}

fn definition_target_on_line(
    uri: &str,
    source: &str,
    line_one_based: usize,
    label: &str,
) -> Option<DefinitionTarget> {
    let line = line_one_based.saturating_sub(1);
    let line_text = source.lines().nth(line)?;
    let (start_character, end_character) = definition_character_range(line_text, label)?;
    Some(DefinitionTarget {
        uri: uri.to_owned(),
        line,
        start_character,
        end_character,
    })
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

fn document_state_from_notification(
    request: &Value,
    documents: &Documents,
) -> Option<(String, DocumentState)> {
    let uri = request_uri(request)?.to_owned();
    let version = document_version_from_request(request)
        .or_else(|| documents.get(&uri).and_then(|state| state.version));
    if let Some(text) = request
        .pointer("/params/textDocument/text")
        .and_then(Value::as_str)
    {
        return Some((uri, DocumentState::new(text.to_owned(), version)));
    }
    if let Some(text) = request
        .pointer("/params/contentChanges")
        .and_then(Value::as_array)
        .and_then(|changes| {
            changes
                .iter()
                .rev()
                .find_map(|change| change.get("text").and_then(Value::as_str))
        })
    {
        return Some((uri, DocumentState::new(text.to_owned(), version)));
    }
    if let Some(state) = documents.get(&uri) {
        return Some((uri, DocumentState::new(state.text.clone(), version)));
    }
    let path = path_from_uri(&uri)?;
    std::fs::read_to_string(path)
        .ok()
        .map(|text| (uri, DocumentState::new(text, version)))
}

fn document_version_from_request(request: &Value) -> Option<i64> {
    request
        .pointer("/params/textDocument/version")
        .and_then(Value::as_i64)
}

fn request_uri(request: &Value) -> Option<&str> {
    request
        .pointer("/params/textDocument/uri")
        .and_then(Value::as_str)
}

fn request_range(request: &Value) -> Option<((usize, usize), (usize, usize))> {
    let start_line = request
        .pointer("/params/range/start/line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let start_character = request
        .pointer("/params/range/start/character")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end_line = request
        .pointer("/params/range/end/line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end_character = request
        .pointer("/params/range/end/character")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    Some(((start_line, start_character), (end_line, end_character)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_assignment_range_preserves_trailing_line_comments() {
        let slash_line = "    timeout = 90 s // keep this note";
        let slash_assignment = option_assignment_range(slash_line, &["timeout"]).unwrap();
        assert_eq!(slash_assignment.option_name, "timeout");
        assert_eq!(
            &slash_line[slash_assignment.value_start..slash_assignment.value_end],
            "90 s"
        );

        let hash_line = "    expected_sha256 = \"old\" # observed mismatch";
        let hash_assignment = option_assignment_range(hash_line, &["expected_sha256"]).unwrap();
        assert_eq!(
            &hash_line[hash_assignment.value_start..hash_assignment.value_end],
            "\"old\""
        );
    }

    #[test]
    fn option_assignment_range_keeps_comment_markers_inside_strings() {
        let line = "    cache_key = [\"https://example.org/#fragment\"] // note";
        let assignment = option_assignment_range(line, &["cache_key"]).unwrap();
        assert_eq!(
            &line[assignment.value_start..assignment.value_end],
            "[\"https://example.org/#fragment\"]"
        );
    }
}
