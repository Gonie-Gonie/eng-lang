use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use eng_compiler::{bundled_module_registry, format_source, parse_source, AstItem};
use eng_lsp::{
    completion_items_for_path_position, completion_items_for_source_position, completion_json,
    diagnostic_json, document_symbols_lsp_json, editor_metadata_json, folding_ranges_lsp_json,
    hover_json, semantic_legend, semantic_tokens_lsp_json, snapshot_for_path, snapshot_for_source,
    LSP_SNAPSHOT_FORMAT,
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
    let mut documents = HashMap::new();
    documents.insert(uri.clone(), source);
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
    let mut documents = HashMap::new();
    documents.insert(uri, source);
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
    let symbols = workspace_symbols_for_request(&request, &HashMap::new(), &[root]);
    println!(
        "{}",
        json!({
            "format": LSP_SNAPSHOT_FORMAT,
            "symbols": symbols
        })
    );
    std::process::ExitCode::SUCCESS
}

fn run_lsp() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut documents = HashMap::<String, String>::new();
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
                                "textDocumentSync": 1,
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "documentSymbolProvider": true,
                                "workspaceSymbolProvider": true,
                                "foldingRangeProvider": true,
                                "documentFormattingProvider": true,
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

fn formatting_edits_for_request(
    request: &Value,
    documents: &HashMap<String, String>,
) -> Vec<Value> {
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

fn code_actions_for_request(request: &Value, documents: &HashMap<String, String>) -> Vec<Value> {
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
        "E-NET-HASH-MISMATCH" => {
            optional_code_action(lsp_expected_sha256_code_action(uri, text, diagnostic))
        }
        "E-WITH-OPTION-001" => {
            optional_code_action(lsp_with_option_alias_code_action(uri, text, diagnostic))
        }
        "E-WITH-UNIT-001" => optional_code_action(
            lsp_remove_incompatible_display_unit_code_action(uri, text, diagnostic),
        ),
        "E-LOG-LEVEL-001" => {
            optional_code_action(lsp_log_level_info_code_action(uri, text, diagnostic))
        }
        "E-PROCESS-BINDING-001" => {
            optional_code_action(lsp_bind_process_result_code_action(uri, text, diagnostic))
        }
        "E-ASSERT-001" => {
            optional_code_action(lsp_wrap_assertion_code_action(uri, text, diagnostic))
        }
        "E-WHERE-FWD-001" => optional_code_action(lsp_reorder_where_local_definition_code_action(
            uri, text, diagnostic,
        )),
        "E-NAME-LOCAL-001" => {
            optional_code_action(lsp_promote_where_local_code_action(uri, text, diagnostic))
        }
        "E-UNC-SOURCE-001" | "E-UNC-SOURCE-002" => {
            lsp_uncertainty_source_code_actions(uri, text, diagnostic)
        }
        code if code.starts_with("E-UNC-ARGS-") => {
            lsp_uncertainty_argument_code_actions(uri, text, diagnostic)
        }
        "E-CMD-AMBIG-001" => optional_code_action(lsp_parenthesize_command_target_code_action(
            uri, text, diagnostic,
        )),
        "E-STDLIB-MODULE-UNKNOWN" => optional_code_action(
            lsp_stdlib_module_replacement_code_action(uri, text, diagnostic),
        ),
        code => optional_code_action(lsp_option_value_replacement_code_action(
            uri,
            text,
            diagnostic,
            option_quick_fix(code),
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
        _ => None,
    }
}

fn lsp_option_value_replacement_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    fix: Option<OptionQuickFix>,
) -> Option<Value> {
    let fix = fix?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let assignment = option_assignment_range(line, fix.option_names)?;
    let option_label = if fix.option_names.len() == 1 {
        fix.option_names[0]
    } else {
        &assignment.option_name
    };
    Some(json!({
        "title": format!("{}: {} = {}", fix.label, option_label, fix.value),
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
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let name_start = line_indent(line).len();
    let rest = &line[name_start..];
    let after_name = rest.strip_prefix(fix.from)?;
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
        "title": fix.title,
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            line_byte_range(line_number, line, name_start, name_start + fix.from.len()),
            fix.to
        )
    }))
}

fn with_option_alias_fix(option_name: &str) -> Option<WithOptionAliasFix> {
    match option_name {
        "unit" => Some(WithOptionAliasFix {
            from: "unit",
            to: "unit y",
            title: "Use plot y-axis option: unit y =",
        }),
        "y_unit" => Some(WithOptionAliasFix {
            from: "y_unit",
            to: "unit y",
            title: "Use plot y-axis option: unit y =",
        }),
        "x_unit" => Some(WithOptionAliasFix {
            from: "x_unit",
            to: "unit x",
            title: "Use plot x-axis option: unit x =",
        }),
        "confidence" => Some(WithOptionAliasFix {
            from: "confidence",
            to: "confidence_band",
            title: "Use confidence band option: confidence_band =",
        }),
        _ => None,
    }
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
            let placeholder =
                format!("{indent}{source} = normal(mean=5 kW, std=0.8 kW, samples=31)");
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
            let placeholder =
                format!("{indent}{source} = normal(mean=5 kW, std=0.8 kW, samples=31)");
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
        let comment_start = line[value_start..]
            .find('#')
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
    line.split_once('#')
        .map(|(head, _comment)| head)
        .unwrap_or(line)
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

fn document_text_for_uri(uri: &str, documents: &HashMap<String, String>) -> Option<String> {
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    documents
        .get(uri)
        .cloned()
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
    documents: &HashMap<String, String>,
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
    documents: &HashMap<String, String>,
    workspace_roots: &[PathBuf],
) -> Vec<Value> {
    let query = request
        .pointer("/params/query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut results = Vec::new();
    let mut seen = HashSet::<(String, usize, String)>::new();

    for (uri, source) in documents {
        if results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            break;
        }
        let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
        push_workspace_symbols_from_source(uri, &path, source, &query, &mut results, &mut seen);
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
