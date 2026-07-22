use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eng_compiler::{
    bundled_module_registry, check_source_with_import_overrides, format_source, parse_source,
    recheck_scalar_declaration_suffix_incrementally, retarget_check_report_for_token_stable_trivia,
    AstItem, CheckOptions, CheckReport, ImportSourceOverrides, ParseContext,
    UNCERTAINTY_ARGUMENT_ALIASES,
};
use eng_lsp::{
    completion_items_at, completion_items_for_path_position, completion_items_for_source_position,
    completion_items_for_source_position_with_import_overrides, completion_json, diagnostic_json,
    document_symbols_lsp_json, editor_metadata_json, editor_syntax_catalog_json,
    folding_ranges_lsp_json, hover_json, semantic_legend, semantic_tokens_lsp_data,
    semantic_tokens_lsp_json, snapshot_for_path, snapshot_for_source,
    snapshot_for_source_with_import_overrides, snapshot_from_report_with_source,
    workflow_option_label_exists, LSP_SNAPSHOT_FORMAT,
};
use serde_json::{json, Value};

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--stdio") {
        if args.len() != 1 {
            eprintln!("usage: eng-lsp --stdio");
            return std::process::ExitCode::from(2);
        }
        return command_stdio();
    }
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
    if args.first().map(String::as_str) == Some("--workspace-snapshot-stdin") {
        return command_workspace_snapshot_stdin(args.get(1), args.get(2));
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
    if args.first().map(String::as_str) == Some("--workspace-completion-stdin") {
        return command_workspace_completion_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
        );
    }
    if args.first().map(String::as_str) == Some("--definition-stdin") {
        return command_definition_stdin(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--workspace-definition-stdin") {
        return command_workspace_definition_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
        );
    }
    if args.first().map(String::as_str) == Some("--document-highlights-stdin") {
        return command_document_highlights_stdin(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--references-stdin") {
        return command_references_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
            args.get(5),
        );
    }
    if args.first().map(String::as_str) == Some("--workspace-references-stdin") {
        return command_workspace_references_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
            args.get(5),
        );
    }
    if args.first().map(String::as_str) == Some("--prepare-rename-stdin") {
        return command_prepare_rename_stdin(args.get(1), args.get(2), args.get(3));
    }
    if args.first().map(String::as_str) == Some("--workspace-prepare-rename-stdin") {
        return command_workspace_prepare_rename_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
        );
    }
    if args.first().map(String::as_str) == Some("--rename-stdin") {
        return command_rename_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
            args.get(5),
        );
    }
    if args.first().map(String::as_str) == Some("--workspace-rename-stdin") {
        return command_workspace_rename_stdin(
            args.get(1),
            args.get(2),
            args.get(3),
            args.get(4),
            args.get(5),
        );
    }
    if args.first().map(String::as_str) == Some("--workspace-symbols-stdin") {
        return command_workspace_symbols_stdin(args.get(1), args.get(2));
    }
    if args.first().map(String::as_str) == Some("--workspace-symbols") {
        return command_workspace_symbols(args.get(1), args.get(2));
    }

    command_stdio()
}

fn command_stdio() -> std::process::ExitCode {
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

fn command_workspace_snapshot_stdin(
    root: Option<&String>,
    path: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-snapshot-stdin <workspace-root> <file.eng>";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (path, source) = match selected_workspace_document(&root, path, &documents) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("invalid workspace snapshot request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let import_overrides = import_source_overrides_from_documents(&documents);
    let snapshot = snapshot_for_source_with_import_overrides(&path, &source, &import_overrides);
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

fn command_workspace_completion_stdin(
    root: Option<&String>,
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-completion-stdin <workspace-root> <file.eng> <line> <character>";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (path, source) = match selected_workspace_document(&root, path, &documents) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("invalid workspace completion request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let import_overrides = import_source_overrides_from_documents(&documents);
    let items = completion_items_for_source_position_with_import_overrides(
        &path,
        &source,
        line,
        character,
        &import_overrides,
    );
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

fn command_workspace_definition_stdin(
    root: Option<&String>,
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-definition-stdin <workspace-root> <file.eng> <line> <character>";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(value) => value,
        Err(exit_code) => return exit_code,
    };
    let uri = match workspace_request_uri(&root, path, &documents) {
        Ok(uri) => uri,
        Err(error) => {
            eprintln!("invalid workspace definition request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    println!(
        "{}",
        definition_for_request(&request, &documents).unwrap_or(Value::Null)
    );
    std::process::ExitCode::SUCCESS
}

fn command_document_highlights_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --document-highlights-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --document-highlights-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let uri = file_uri_from_path(Path::new(path));
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    let mut documents = Documents::new();
    documents.insert(uri, DocumentState::new(source, None));
    println!("{}", document_highlights_for_request(&request, &documents));
    std::process::ExitCode::SUCCESS
}

fn command_references_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
    include_declaration: Option<&String>,
    workspace_root: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --references-stdin <file.eng> <line> <character> [true|false] [workspace-root]");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --references-stdin <file.eng> <line> <character> [true|false] [workspace-root]");
        return std::process::ExitCode::from(2);
    };
    let include_declaration = match include_declaration.map(String::as_str) {
        None | Some("true") => true,
        Some("false") => false,
        Some(_) => {
            eprintln!(
                "usage: eng-lsp --references-stdin <file.eng> <line> <character> [true|false] [workspace-root]"
            );
            return std::process::ExitCode::from(2);
        }
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let uri = file_uri_from_path(Path::new(path));
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": include_declaration }
        }
    });
    let mut documents = Documents::new();
    documents.insert(uri, DocumentState::new(source, None));
    let workspace_roots = workspace_root
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    println!(
        "{}",
        references_for_request(&request, &documents, &workspace_roots)
    );
    std::process::ExitCode::SUCCESS
}

fn command_workspace_references_stdin(
    root: Option<&String>,
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
    include_declaration: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-references-stdin <workspace-root> <file.eng> <line> <character> [true|false]";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let include_declaration = match include_declaration.map(String::as_str) {
        None | Some("true") => true,
        Some("false") => false,
        Some(_) => {
            eprintln!("{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(value) => value,
        Err(exit_code) => return exit_code,
    };
    let uri = match workspace_request_uri(&root, path, &documents) {
        Ok(uri) => uri,
        Err(error) => {
            eprintln!("invalid workspace reference request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": include_declaration }
        }
    });
    println!(
        "{}",
        references_for_request(&request, &documents, std::slice::from_ref(&root))
    );
    std::process::ExitCode::SUCCESS
}

fn command_prepare_rename_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --prepare-rename-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --prepare-rename-stdin <file.eng> <line> <character>");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let uri = file_uri_from_path(Path::new(path));
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
        prepare_rename_for_request(&request, &documents).unwrap_or(Value::Null)
    );
    std::process::ExitCode::SUCCESS
}

fn command_workspace_prepare_rename_stdin(
    root: Option<&String>,
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-prepare-rename-stdin <workspace-root> <file.eng> <line> <character>";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(value) => value,
        Err(exit_code) => return exit_code,
    };
    let uri = match workspace_request_uri(&root, path, &documents) {
        Ok(uri) => uri,
        Err(error) => {
            eprintln!("invalid workspace rename preparation request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }
    });
    println!(
        "{}",
        prepare_rename_for_request(&request, &documents).unwrap_or(Value::Null)
    );
    std::process::ExitCode::SUCCESS
}

fn command_rename_stdin(
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
    new_name: Option<&String>,
    workspace_root: Option<&String>,
) -> std::process::ExitCode {
    let Some(path) = path else {
        eprintln!("usage: eng-lsp --rename-stdin <file.eng> <line> <character> <new-name> [workspace-root]");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("usage: eng-lsp --rename-stdin <file.eng> <line> <character> <new-name> [workspace-root]");
        return std::process::ExitCode::from(2);
    };
    let Some(new_name) = new_name else {
        eprintln!("usage: eng-lsp --rename-stdin <file.eng> <line> <character> <new-name> [workspace-root]");
        return std::process::ExitCode::from(2);
    };
    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("failed to read EngLang source from stdin: {error}");
        return std::process::ExitCode::from(1);
    }
    let uri = file_uri_from_path(Path::new(path));
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "newName": new_name
        }
    });
    let mut documents = Documents::new();
    documents.insert(uri, DocumentState::new(source, None));
    let workspace_roots = workspace_root
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    match rename_for_request(&request, &documents, &workspace_roots) {
        Ok(edit) => println!("{edit}"),
        Err(message) => println!("{}", json!({ "error": message })),
    }
    std::process::ExitCode::SUCCESS
}

fn command_workspace_rename_stdin(
    root: Option<&String>,
    path: Option<&String>,
    line: Option<&String>,
    character: Option<&String>,
    new_name: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-rename-stdin <workspace-root> <file.eng> <line> <character> <new-name>";
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some((line, character)) = parse_position(line, character) else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let Some(new_name) = new_name else {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    };
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(value) => value,
        Err(exit_code) => return exit_code,
    };
    let uri = match workspace_request_uri(&root, path, &documents) {
        Ok(uri) => uri,
        Err(error) => {
            eprintln!("invalid workspace rename request: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let request = json!({
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "newName": new_name
        }
    });
    match rename_for_request(&request, &documents, std::slice::from_ref(&root)) {
        Ok(edit) => println!("{edit}"),
        Err(message) => println!("{}", json!({ "error": message })),
    }
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

fn command_workspace_symbols_stdin(
    root: Option<&String>,
    query: Option<&String>,
) -> std::process::ExitCode {
    const USAGE: &str = "usage: eng-lsp --workspace-symbols-stdin <workspace-root> [query]";
    let (root, documents) = match read_workspace_documents_stdin(root, USAGE) {
        Ok(value) => value,
        Err(exit_code) => return exit_code,
    };
    let request = json!({
        "params": {
            "query": query.map(String::as_str).unwrap_or("")
        }
    });
    let symbols = workspace_symbols_for_request(&request, &documents, &[root]);
    println!(
        "{}",
        json!({
            "format": LSP_SNAPSHOT_FORMAT,
            "symbols": symbols
        })
    );
    std::process::ExitCode::SUCCESS
}

fn read_workspace_documents_stdin(
    root: Option<&String>,
    usage: &str,
) -> Result<(PathBuf, Documents), std::process::ExitCode> {
    let Some(root) = root else {
        eprintln!("{usage}");
        return Err(std::process::ExitCode::from(2));
    };
    let root = match Path::new(root).canonicalize() {
        Ok(root) if root.is_dir() => root,
        Ok(_) => {
            eprintln!("workspace root is not a directory: {root}");
            return Err(std::process::ExitCode::from(2));
        }
        Err(error) => {
            eprintln!("could not resolve workspace root {root}: {error}");
            return Err(std::process::ExitCode::from(2));
        }
    };
    let mut input = Vec::new();
    if let Err(error) = std::io::stdin()
        .take((MAX_WORKSPACE_OPEN_DOCUMENT_PAYLOAD_BYTES + 1) as u64)
        .read_to_end(&mut input)
    {
        eprintln!("failed to read workspace documents from stdin: {error}");
        return Err(std::process::ExitCode::from(1));
    }
    if input.len() > MAX_WORKSPACE_OPEN_DOCUMENT_PAYLOAD_BYTES {
        eprintln!(
            "workspace document payload exceeded the {}-byte limit",
            MAX_WORKSPACE_OPEN_DOCUMENT_PAYLOAD_BYTES
        );
        return Err(std::process::ExitCode::from(2));
    }
    let payload = match serde_json::from_slice::<Value>(&input) {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("could not parse workspace document payload: {error}");
            return Err(std::process::ExitCode::from(2));
        }
    };
    let documents = match workspace_documents_from_payload(&payload, &root) {
        Ok(documents) => documents,
        Err(error) => {
            eprintln!("invalid workspace document payload: {error}");
            return Err(std::process::ExitCode::from(2));
        }
    };
    Ok((root, documents))
}

fn workspace_documents_from_payload(payload: &Value, root: &Path) -> Result<Documents, String> {
    if payload.get("format").and_then(Value::as_str) != Some(WORKSPACE_OPEN_DOCUMENT_FORMAT) {
        return Err(format!("format must be {WORKSPACE_OPEN_DOCUMENT_FORMAT}"));
    }
    let document_values = payload
        .get("documents")
        .and_then(Value::as_array)
        .ok_or_else(|| "documents must be an array".to_owned())?;
    if document_values.len() > MAX_WORKSPACE_OPEN_DOCUMENTS {
        return Err(format!(
            "documents exceeded the {MAX_WORKSPACE_OPEN_DOCUMENTS}-document limit"
        ));
    }

    let mut documents = Documents::new();
    let mut total_source_bytes = 0usize;
    for document in document_values {
        let path = document
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| "each document must have a non-empty path".to_owned())?;
        let source = document
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("workspace document {path} must have string source"))?;
        if source.len() > MAX_WORKSPACE_OPEN_DOCUMENT_BYTES {
            return Err(format!(
                "workspace document {path} exceeded the {MAX_WORKSPACE_OPEN_DOCUMENT_BYTES}-byte limit"
            ));
        }
        total_source_bytes = total_source_bytes
            .checked_add(source.len())
            .ok_or_else(|| "workspace document source size overflowed".to_owned())?;
        if total_source_bytes > MAX_WORKSPACE_OPEN_DOCUMENT_TOTAL_BYTES {
            return Err(format!(
                "workspace document sources exceeded the {MAX_WORKSPACE_OPEN_DOCUMENT_TOTAL_BYTES}-byte total limit"
            ));
        }

        let canonical = canonical_workspace_document_path(root, path)?;
        let uri = file_uri_from_path(&canonical);
        if documents
            .insert(uri, DocumentState::new(source.to_owned(), None))
            .is_some()
        {
            return Err(format!(
                "workspace document was provided more than once: {path}"
            ));
        }
    }
    Ok(documents)
}

fn canonical_workspace_document_path(root: &Path, path: &str) -> Result<PathBuf, String> {
    let raw_path = PathBuf::from(path);
    let candidate = if raw_path.is_absolute() {
        raw_path
    } else {
        root.join(raw_path)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("could not resolve workspace document {path}: {error}"))?;
    if !canonical.is_file() || !canonical.starts_with(root) {
        return Err(format!(
            "workspace document is outside the workspace or is not a file: {path}"
        ));
    }
    if !canonical
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("eng"))
    {
        return Err(format!("workspace document is not an .eng file: {path}"));
    }
    Ok(canonical)
}

fn workspace_request_uri(root: &Path, path: &str, documents: &Documents) -> Result<String, String> {
    let canonical = canonical_workspace_document_path(root, path)?;
    let uri = file_uri_from_path(&canonical);
    if !documents.contains_key(&uri) {
        return Err(format!(
            "selected workspace document must be included in the open-document payload: {path}"
        ));
    }
    Ok(uri)
}

fn selected_workspace_document(
    root: &Path,
    path: &str,
    documents: &Documents,
) -> Result<(PathBuf, String), String> {
    let uri = workspace_request_uri(root, path, documents)?;
    let path = path_from_uri(&uri)
        .ok_or_else(|| format!("could not convert workspace document URI to a path: {uri}"))?;
    let source = document_text_for_uri(&uri, documents)
        .ok_or_else(|| format!("workspace document source was unavailable: {uri}"))?;
    Ok((path, source))
}

fn import_source_overrides_from_documents(documents: &Documents) -> ImportSourceOverrides {
    let mut overrides = ImportSourceOverrides::new();
    for (uri, state) in documents {
        let Some(path) = path_from_uri(uri) else {
            continue;
        };
        let _ = overrides.insert(path, state.text.clone());
    }
    overrides
}

#[derive(Clone, Debug)]
struct CachedDocumentAnalysis {
    source: String,
    report: Arc<CheckReport>,
    snapshot: Option<Arc<eng_lsp::LspSnapshot>>,
}

#[derive(Debug, Default)]
struct DocumentAnalysisCache {
    analysis: Option<CachedDocumentAnalysis>,
    hits: usize,
    misses: usize,
    trivia_reuses: usize,
    scalar_declaration_reuses: usize,
}

#[derive(Clone, Debug)]
struct DocumentState {
    text: String,
    version: Option<i64>,
    analysis_cache: Arc<Mutex<DocumentAnalysisCache>>,
}

impl DocumentState {
    fn new(text: String, version: Option<i64>) -> Self {
        Self {
            text,
            version,
            analysis_cache: Arc::new(Mutex::new(DocumentAnalysisCache::default())),
        }
    }

    fn updated(text: String, version: Option<i64>, previous: Option<&Self>) -> Self {
        if let Some(previous) = previous {
            return Self {
                text,
                version,
                analysis_cache: Arc::clone(&previous.analysis_cache),
            };
        }
        Self::new(text, version)
    }

    fn cached_analysis(&self, source: &str) -> Option<CachedDocumentAnalysis> {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let analysis = cache
            .analysis
            .as_ref()
            .filter(|analysis| analysis.source == source)
            .cloned();
        if analysis.is_some() {
            cache.hits += 1;
        } else {
            cache.misses += 1;
        }
        analysis
    }

    fn reuse_analysis_for_token_stable_trivia(
        &self,
        source: &str,
    ) -> Option<CachedDocumentAnalysis> {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = cache.analysis.as_ref()?;
        let report = Arc::new(retarget_check_report_for_token_stable_trivia(
            &previous.report,
            &previous.source,
            source,
        )?);
        let analysis = CachedDocumentAnalysis {
            source: source.to_owned(),
            report,
            snapshot: None,
        };
        cache.analysis = Some(analysis.clone());
        cache.trivia_reuses += 1;
        Some(analysis)
    }

    fn recheck_scalar_declaration_suffix(&self, source: &str) -> Option<CachedDocumentAnalysis> {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = cache.analysis.as_ref()?;
        let report = Arc::new(recheck_scalar_declaration_suffix_incrementally(
            &previous.report,
            &previous.source,
            source,
        )?);
        let analysis = CachedDocumentAnalysis {
            source: source.to_owned(),
            report,
            snapshot: None,
        };
        cache.analysis = Some(analysis.clone());
        cache.scalar_declaration_reuses += 1;
        Some(analysis)
    }

    fn store_analysis(&self, analysis: &CachedDocumentAnalysis) {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.analysis = Some(analysis.clone());
    }

    fn store_snapshot(
        &self,
        source: &str,
        report: &Arc<CheckReport>,
        snapshot: &eng_lsp::LspSnapshot,
    ) {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(analysis) = cache
            .analysis
            .as_mut()
            .filter(|analysis| analysis.source == source && Arc::ptr_eq(&analysis.report, report))
        else {
            return;
        };
        analysis.snapshot = Some(Arc::new(snapshot.clone()));
    }

    fn invalidate_analysis(&self) {
        let mut cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.analysis = None;
    }

    #[cfg(test)]
    fn analysis_cache_stats(&self) -> (usize, usize, usize, bool, bool) {
        let cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (
            cache.hits,
            cache.misses,
            cache.trivia_reuses,
            cache.analysis.is_some(),
            cache
                .analysis
                .as_ref()
                .is_some_and(|analysis| analysis.snapshot.is_some()),
        )
    }

    #[cfg(test)]
    fn analysis_cache_snapshot(&self) -> Option<(String, Arc<eng_lsp::LspSnapshot>)> {
        let cache = self
            .analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let analysis = cache.analysis.as_ref()?;
        Some((
            analysis.source.clone(),
            Arc::clone(analysis.snapshot.as_ref()?),
        ))
    }

    #[cfg(test)]
    fn scalar_binding_reuse_count(&self) -> usize {
        self.analysis_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .scalar_declaration_reuses
    }
}

type Documents = HashMap<String, DocumentState>;

fn invalidate_document_analyses<'a>(states: impl IntoIterator<Item = &'a DocumentState>) {
    for state in states {
        state.invalidate_analysis();
    }
}

fn invalidate_dependent_document_analyses(changed_uri: &str, affected: &[(String, DocumentState)]) {
    invalidate_document_analyses(
        affected
            .iter()
            .filter(|(uri, _)| uri != changed_uri)
            .map(|(_, state)| state),
    );
}

const DEFAULT_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS: u64 = 150;
const MAX_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS: u64 = 5_000;
const MAX_QUEUED_LSP_MESSAGES: usize = 64;

#[derive(Debug)]
struct PendingDiagnostics {
    delay: Duration,
    due_at: Option<Instant>,
    uris: HashSet<String>,
}

impl Default for PendingDiagnostics {
    fn default() -> Self {
        Self::new(Duration::from_millis(
            DEFAULT_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS,
        ))
    }
}

impl PendingDiagnostics {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            due_at: None,
            uris: HashSet::new(),
        }
    }

    fn set_delay(&mut self, delay: Duration, now: Instant) {
        self.delay = delay;
        if !self.uris.is_empty() {
            self.due_at = Some(now + delay);
        }
    }

    fn schedule(&mut self, uris: impl IntoIterator<Item = String>, now: Instant) {
        self.uris.extend(uris);
        if !self.uris.is_empty() {
            self.due_at = Some(now + self.delay);
        }
    }

    fn cancel(&mut self, uri: &str) {
        self.uris.remove(uri);
        if self.uris.is_empty() {
            self.due_at = None;
        }
    }

    fn timeout(&self, now: Instant) -> Option<Duration> {
        self.due_at
            .map(|due_at| due_at.saturating_duration_since(now))
    }

    fn take_due(&mut self, now: Instant) -> Vec<String> {
        if self.due_at.is_none_or(|due_at| due_at > now) {
            return Vec::new();
        }
        self.due_at = None;
        let mut uris = self.uris.drain().collect::<Vec<_>>();
        uris.sort();
        uris
    }
}

#[derive(Clone, Debug, Default)]
struct RequestCancellationRegistry {
    requests: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl RequestCancellationRegistry {
    fn register(&self, id: &Value) -> Option<RequestCancellation> {
        let key = request_id_key(id)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.clone(), Arc::clone(&cancelled));
        Some(RequestCancellation {
            key,
            cancelled,
            requests: Arc::clone(&self.requests),
        })
    }

    fn cancel(&self, id: &Value) -> bool {
        let Some(key) = request_id_key(id) else {
            return false;
        };
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(cancelled) = requests.get(&key) else {
            return false;
        };
        cancelled.store(true, Ordering::Release);
        true
    }
}

#[derive(Debug)]
struct RequestCancellation {
    key: String,
    cancelled: Arc<AtomicBool>,
    requests: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl RequestCancellation {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

fn request_is_cancelled(cancellation: Option<&RequestCancellation>) -> bool {
    cancellation.is_some_and(RequestCancellation::is_cancelled)
}

impl Drop for RequestCancellation {
    fn drop(&mut self) {
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if requests
            .get(&self.key)
            .is_some_and(|cancelled| Arc::ptr_eq(cancelled, &self.cancelled))
        {
            requests.remove(&self.key);
        }
    }
}

fn request_id_key(id: &Value) -> Option<String> {
    match id {
        Value::Number(number) => Some(format!("number:{number}")),
        Value::String(value) => Some(format!("string:{value}")),
        _ => None,
    }
}

enum LspInputEvent {
    Message {
        request: Value,
        cancellation: Option<RequestCancellation>,
    },
    ParseError(String),
    End,
    Error(io::Error),
}

fn lsp_input_event(body: String, cancellations: &RequestCancellationRegistry) -> LspInputEvent {
    let request = match serde_json::from_str::<Value>(&body) {
        Ok(request) => request,
        Err(error) => return LspInputEvent::ParseError(error.to_string()),
    };
    let method = request.get("method").and_then(Value::as_str);
    let cancellation = if method == Some("$/cancelRequest") {
        if let Some(id) = request.pointer("/params/id") {
            cancellations.cancel(id);
        }
        None
    } else {
        method
            .is_some()
            .then(|| request.get("id").and_then(|id| cancellations.register(id)))
            .flatten()
    };
    LspInputEvent::Message {
        request,
        cancellation,
    }
}

fn spawn_lsp_input_reader() -> mpsc::Receiver<LspInputEvent> {
    let (sender, receiver) = mpsc::sync_channel(MAX_QUEUED_LSP_MESSAGES);
    let cancellations = RequestCancellationRegistry::default();
    thread::spawn(move || {
        let mut input = io::stdin().lock();
        loop {
            let event = match read_lsp_message(&mut input) {
                Ok(Some(message)) => lsp_input_event(message, &cancellations),
                Ok(None) => LspInputEvent::End,
                Err(error) => LspInputEvent::Error(error),
            };
            let terminal = matches!(&event, LspInputEvent::End | LspInputEvent::Error(_));
            if sender.send(event).is_err() || terminal {
                break;
            }
        }
    });
    receiver
}

const MAX_SEMANTIC_TOKEN_RESULTS_PER_DOCUMENT: usize = 4;

#[derive(Clone, Debug)]
struct CachedSemanticTokens {
    result_id: String,
    data: Vec<usize>,
}

#[derive(Debug, Default)]
struct SemanticTokenCache {
    next_result_id: u64,
    documents: HashMap<String, VecDeque<CachedSemanticTokens>>,
}

impl SemanticTokenCache {
    fn full_response(&mut self, uri: Option<&str>, data: Vec<usize>) -> Value {
        let Some(uri) = uri else {
            return json!({ "data": data });
        };
        let result_id = self.insert(uri, data.clone());
        json!({ "resultId": result_id, "data": data })
    }

    fn delta_response(
        &mut self,
        uri: Option<&str>,
        previous_result_id: Option<&str>,
        data: Vec<usize>,
    ) -> Value {
        let Some(uri) = uri else {
            return json!({ "data": data });
        };
        let edits = previous_result_id
            .and_then(|result_id| self.find(uri, result_id))
            .map(|previous| semantic_token_delta_edits(&previous.data, &data));
        let result_id = self.insert(uri, data.clone());
        match edits {
            Some(edits) => json!({ "resultId": result_id, "edits": edits }),
            None => json!({ "resultId": result_id, "data": data }),
        }
    }

    fn remove_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    fn find(&self, uri: &str, result_id: &str) -> Option<&CachedSemanticTokens> {
        self.documents
            .get(uri)?
            .iter()
            .find(|result| result.result_id == result_id)
    }

    fn insert(&mut self, uri: &str, data: Vec<usize>) -> String {
        self.next_result_id += 1;
        let result_id = self.next_result_id.to_string();
        let results = self.documents.entry(uri.to_owned()).or_default();
        results.push_back(CachedSemanticTokens {
            result_id: result_id.clone(),
            data,
        });
        while results.len() > MAX_SEMANTIC_TOKEN_RESULTS_PER_DOCUMENT {
            results.pop_front();
        }
        result_id
    }
}

fn persistent_diagnostics_debounce(request: &Value) -> Duration {
    let milliseconds = request
        .pointer("/params/initializationOptions/diagnosticsDebounceMs")
        .or_else(|| request.pointer("/params/initializationOptions/englang/liveDiagnosticsDelayMs"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS)
        .min(MAX_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS);
    Duration::from_millis(milliseconds)
}

fn run_lsp() -> io::Result<()> {
    let input = spawn_lsp_input_reader();
    let mut output = io::stdout().lock();
    let mut documents = Documents::new();
    let mut pending_diagnostics = PendingDiagnostics::default();
    let mut semantic_token_cache = SemanticTokenCache::default();
    let mut workspace_roots = Vec::<PathBuf>::new();

    loop {
        let queued_event = match input.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => break,
        };
        let event = match queued_event {
            Some(event) => event,
            None => {
                publish_due_diagnostics(&mut output, &mut pending_diagnostics, &documents)?;
                match pending_diagnostics.timeout(Instant::now()) {
                    Some(timeout) => match input.recv_timeout(timeout) {
                        Ok(event) => event,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    },
                    None => match input.recv() {
                        Ok(event) => event,
                        Err(_) => break,
                    },
                }
            }
        };
        let (request, cancellation) = match event {
            LspInputEvent::Message {
                request,
                cancellation,
            } => (request, cancellation),
            LspInputEvent::ParseError(error) => {
                write_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": { "code": -32700, "message": error }
                    }),
                )?;
                continue;
            }
            LspInputEvent::End => break,
            LspInputEvent::Error(error) => return Err(error),
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let id = request.get("id").cloned();
        if cancellation
            .as_ref()
            .is_some_and(RequestCancellation::is_cancelled)
        {
            write_request_cancelled(&mut output, id)?;
            continue;
        }

        match method {
            "initialize" => {
                workspace_roots = workspace_roots_from_initialize(&request);
                pending_diagnostics
                    .set_delay(persistent_diagnostics_debounce(&request), Instant::now());
                let legend = semantic_legend();
                write_request_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": {
                                    "openClose": true,
                                    "change": 2,
                                    "save": { "includeText": true }
                                },
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "documentHighlightProvider": true,
                                "referencesProvider": true,
                                "renameProvider": { "prepareProvider": true },
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
                                    "full": { "delta": true },
                                    "range": true
                                },
                                "experimental": {
                                    "englangSnapshotProvider": true
                                }
                            },
                            "serverInfo": {
                                "name": "eng-lsp",
                                "version": env!("CARGO_PKG_VERSION")
                            }
                        }
                    }),
                    cancellation.as_ref(),
                )?;
            }
            "shutdown" => {
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null }),
                    cancellation.as_ref(),
                )?;
            }
            "exit" => break,
            "initialized" => {}
            "$/cancelRequest" => {}
            "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
                if let Some((uri, state)) = document_state_from_notification(&request, &documents) {
                    let source_changed = documents
                        .get(&uri)
                        .is_none_or(|previous| previous.text != state.text);
                    documents.insert(uri.clone(), state.clone());
                    let affected = diagnostic_documents_after_change(&uri, &documents);
                    if source_changed {
                        invalidate_dependent_document_analyses(&uri, &affected);
                    }
                    if method == "textDocument/didChange" {
                        pending_diagnostics.schedule(
                            affected.into_iter().map(|(open_uri, _)| open_uri),
                            Instant::now(),
                        );
                    } else {
                        for (open_uri, open_state) in affected {
                            pending_diagnostics.cancel(&open_uri);
                            publish_diagnostics(&mut output, &open_uri, &open_state, &documents)?;
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = request_uri(&request).map(str::to_owned) {
                    let dependents = diagnostic_documents_after_change(&uri, &documents)
                        .into_iter()
                        .filter(|(open_uri, _)| open_uri != &uri)
                        .collect::<Vec<_>>();
                    invalidate_document_analyses(dependents.iter().map(|(_, state)| state));
                    pending_diagnostics.cancel(&uri);
                    documents.remove(&uri);
                    semantic_token_cache.remove_document(&uri);
                    clear_diagnostics(&mut output, &uri)?;
                    for (open_uri, open_state) in dependents {
                        pending_diagnostics.cancel(&open_uri);
                        publish_diagnostics(&mut output, &open_uri, &open_state, &documents)?;
                    }
                }
            }
            "workspace/didChangeWatchedFiles" => {
                invalidate_document_analyses(documents.values());
                semantic_token_cache = SemanticTokenCache::default();
                let open_documents = documents
                    .iter()
                    .map(|(uri, state)| (uri.clone(), state.clone()))
                    .collect::<Vec<_>>();
                for (uri, state) in open_documents {
                    pending_diagnostics.cancel(&uri);
                    publish_diagnostics(&mut output, &uri, &state, &documents)?;
                }
            }
            "textDocument/completion" => {
                let items = completions_for_request(&request, &documents)
                    .iter()
                    .map(completion_json)
                    .collect::<Vec<_>>();
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": items }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/hover" => {
                let hover = hover_for_request(&request, &documents)
                    .map(|hover| hover_json(&hover))
                    .unwrap_or(Value::Null);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": hover }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/definition" => {
                let definition =
                    definition_for_request(&request, &documents).unwrap_or(Value::Null);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": definition }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/documentHighlight" => {
                let highlights = document_highlights_for_request(&request, &documents);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": highlights }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/references" => {
                let references = references_for_request_with_cancellation(
                    &request,
                    &documents,
                    &workspace_roots,
                    cancellation.as_ref(),
                );
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": references }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/prepareRename" => {
                let prepared =
                    prepare_rename_for_request(&request, &documents).unwrap_or(Value::Null);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": prepared }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/rename" => {
                match rename_for_request_with_cancellation(
                    &request,
                    &documents,
                    &workspace_roots,
                    cancellation.as_ref(),
                ) {
                    Ok(edit) => write_request_response(
                        &mut output,
                        json!({ "jsonrpc": "2.0", "id": id, "result": edit }),
                        cancellation.as_ref(),
                    )?,
                    Err(message) => write_request_response(
                        &mut output,
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32602, "message": message }
                        }),
                        cancellation.as_ref(),
                    )?,
                }
            }
            "textDocument/codeAction" => {
                let actions = code_actions_for_request(&request, &documents);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": actions }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/formatting" => {
                let edits = formatting_edits_for_request(&request, &documents);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": edits }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/rangeFormatting" => {
                let edits = range_formatting_edits_for_request(&request, &documents);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": edits }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/semanticTokens/full" => {
                let tokens = semantic_tokens_full_response_for_request(
                    &request,
                    &documents,
                    &mut semantic_token_cache,
                );
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": tokens }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/semanticTokens/full/delta" => {
                let tokens = semantic_tokens_delta_response_for_request(
                    &request,
                    &documents,
                    &mut semantic_token_cache,
                );
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": tokens }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/semanticTokens/range" => {
                let tokens = semantic_tokens_range_for_request(&request, &documents)
                    .map(|tokens| semantic_tokens_lsp_json(&tokens))
                    .unwrap_or_else(|| json!({ "data": [] }));
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": tokens }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/documentSymbol" => {
                let symbols = snapshot_for_request(&request, &documents)
                    .map(|snapshot| document_symbols_lsp_json(&snapshot.document_symbols))
                    .unwrap_or_else(|| json!([]));
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": symbols }),
                    cancellation.as_ref(),
                )?;
            }
            "workspace/symbol" => {
                let symbols = workspace_symbols_for_request_with_cancellation(
                    &request,
                    &documents,
                    &workspace_roots,
                    cancellation.as_ref(),
                );
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": symbols }),
                    cancellation.as_ref(),
                )?;
            }
            "textDocument/foldingRange" => {
                let ranges = snapshot_for_request(&request, &documents)
                    .map(|snapshot| folding_ranges_lsp_json(&snapshot.folding_ranges))
                    .unwrap_or_else(|| json!([]));
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": ranges }),
                    cancellation.as_ref(),
                )?;
            }
            "englang/snapshot" => {
                let snapshot = snapshot_for_request(&request, &documents)
                    .map(|snapshot| eng_lsp::snapshot_json(&snapshot))
                    .unwrap_or(Value::Null);
                write_request_response(
                    &mut output,
                    json!({ "jsonrpc": "2.0", "id": id, "result": snapshot }),
                    cancellation.as_ref(),
                )?;
            }
            _ if id.is_some() => {
                write_request_response(
                    &mut output,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("unsupported method {method}") }
                    }),
                    cancellation.as_ref(),
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn publish_due_diagnostics<W: Write>(
    output: &mut W,
    pending: &mut PendingDiagnostics,
    documents: &Documents,
) -> io::Result<()> {
    for uri in pending.take_due(Instant::now()) {
        let Some(state) = documents.get(&uri) else {
            continue;
        };
        publish_diagnostics(output, &uri, state, documents)?;
    }
    Ok(())
}

fn publish_diagnostics<W: Write>(
    output: &mut W,
    uri: &str,
    state: &DocumentState,
    documents: &Documents,
) -> io::Result<()> {
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let snapshot = snapshot_for_open_documents(&path, &state.text, documents);
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

fn clear_diagnostics<W: Write>(output: &mut W, uri: &str) -> io::Result<()> {
    write_response(
        output,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": []
            }
        }),
    )
}

fn diagnostic_documents_after_change(
    changed_uri: &str,
    documents: &Documents,
) -> Vec<(String, DocumentState)> {
    let changed_path = path_from_uri(changed_uri).map(|path| path.canonicalize().unwrap_or(path));
    let mut dependents = documents
        .iter()
        .filter_map(|(uri, state)| {
            if uri == changed_uri {
                return None;
            }
            let changed_path = changed_path.as_deref()?;
            let source_path = path_from_uri(uri)?;
            source_depends_on_import_path(
                &source_path,
                &state.text,
                changed_path,
                documents,
                &mut HashSet::new(),
            )
            .then(|| (uri.clone(), state.clone()))
        })
        .collect::<Vec<_>>();
    dependents.sort_by(|left, right| left.0.cmp(&right.0));
    if let Some(state) = documents.get(changed_uri) {
        dependents.push((changed_uri.to_owned(), state.clone()));
    }
    dependents
}

fn source_depends_on_import_path(
    source_path: &Path,
    source: &str,
    target_path: &Path,
    documents: &Documents,
    visited: &mut HashSet<PathBuf>,
) -> bool {
    let Some(base_dir) = source_path.parent() else {
        return false;
    };
    for item in parse_source(source).items {
        let AstItem::Import(import) = item else {
            continue;
        };
        if import.kind != "file" {
            continue;
        }
        let Some(import_path) = resolve_static_import_path(base_dir, &import.target) else {
            continue;
        };
        if import_path == target_path {
            return true;
        }
        if !visited.insert(import_path.clone()) {
            continue;
        }
        let imported_source = workspace_document_for_path(documents, &import_path)
            .map(|(_, state)| Cow::Borrowed(state.text.as_str()))
            .or_else(|| std::fs::read_to_string(&import_path).ok().map(Cow::Owned));
        if imported_source.is_some_and(|source| {
            source_depends_on_import_path(
                &import_path,
                source.as_ref(),
                target_path,
                documents,
                visited,
            )
        }) {
            return true;
        }
    }
    false
}

fn semantic_tokens_for_request(
    request: &Value,
    documents: &Documents,
) -> Option<eng_lsp::LspSemanticTokens> {
    Some(snapshot_for_request(request, documents)?.semantic_tokens)
}

fn semantic_token_data_for_request(request: &Value, documents: &Documents) -> Vec<usize> {
    semantic_tokens_for_request(request, documents)
        .map(|tokens| semantic_tokens_lsp_data(&tokens))
        .unwrap_or_default()
}

fn semantic_tokens_full_response_for_request(
    request: &Value,
    documents: &Documents,
    cache: &mut SemanticTokenCache,
) -> Value {
    cache.full_response(
        request_uri(request),
        semantic_token_data_for_request(request, documents),
    )
}

fn semantic_tokens_delta_response_for_request(
    request: &Value,
    documents: &Documents,
    cache: &mut SemanticTokenCache,
) -> Value {
    cache.delta_response(
        request_uri(request),
        request
            .pointer("/params/previousResultId")
            .and_then(Value::as_str),
        semantic_token_data_for_request(request, documents),
    )
}

fn semantic_token_delta_edits(previous: &[usize], current: &[usize]) -> Vec<Value> {
    let prefix_len = previous
        .iter()
        .zip(current)
        .take_while(|(left, right)| left == right)
        .count();
    if prefix_len == previous.len() && prefix_len == current.len() {
        return Vec::new();
    }

    let max_suffix_len = previous
        .len()
        .saturating_sub(prefix_len)
        .min(current.len().saturating_sub(prefix_len));
    let suffix_len = previous[prefix_len..]
        .iter()
        .rev()
        .zip(current[prefix_len..].iter().rev())
        .take(max_suffix_len)
        .take_while(|(left, right)| left == right)
        .count();
    let delete_count = previous.len() - prefix_len - suffix_len;
    let inserted = current[prefix_len..current.len() - suffix_len].to_vec();
    if inserted.is_empty() {
        vec![json!({ "start": prefix_len, "deleteCount": delete_count })]
    } else {
        vec![json!({
            "start": prefix_len,
            "deleteCount": delete_count,
            "data": inserted
        })]
    }
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
        "E-SAMPLING-RANGE-UNIT" => {
            optional_code_action(lsp_sampling_range_unit_code_action(uri, text, diagnostic))
        }
        "E-TIMESERIES-FILL-METHOD" => {
            lsp_timeseries_fill_method_replacement_actions(uri, text, diagnostic)
        }
        "W-TIMESERIES-FILL-METHOD-IMPLICIT" => {
            lsp_timeseries_fill_method_missing_actions(uri, text, diagnostic)
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
        "W-ML-TRAIN-ALIAS" => optional_code_action(
            lsp_legacy_model_training_migration_code_action(uri, text, diagnostic),
        ),
        "W-ML-ANN-ALIAS" => optional_code_action(lsp_diagnostic_range_replacement_code_action(
            uri,
            diagnostic,
            "mlp",
            "Replace ann with mlp",
        )),
        "W-SAMPLING-UNIFORM-ALIAS" => {
            optional_code_action(lsp_diagnostic_range_replacement_code_action(
                uri,
                diagnostic,
                "random",
                "Replace sampling method with random",
            ))
        }
        "W-SAMPLING-LATIN-HYPERCUBE-ALIAS" => {
            optional_code_action(lsp_diagnostic_range_replacement_code_action(
                uri,
                diagnostic,
                "lhs",
                "Replace sampling method with lhs",
            ))
        }
        "W-UNC-ARG-ALIAS" => optional_code_action(lsp_uncertainty_argument_alias_code_action(
            uri, text, diagnostic,
        )),
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
        "E-GOLDEN-001" => optional_code_action(lsp_wrap_golden_code_action(uri, text, diagnostic)),
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
        "E-CMD-UNKNOWN-VERB" => optional_code_action(lsp_command_style_function_call_code_action(
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

fn lsp_uncertainty_argument_alias_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    for item in UNCERTAINTY_ARGUMENT_ALIASES {
        if diagnostic_range_for_exact_text(text, diagnostic, item.alias).is_some() {
            let title = format!("Replace uncertainty argument with {}", item.canonical);
            return lsp_diagnostic_range_replacement_code_action(
                uri,
                diagnostic,
                item.canonical,
                &title,
            );
        }
    }
    None
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
    if value == "1" {
        return true;
    }
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

fn lsp_timeseries_fill_method_replacement_actions(
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
    let Some(assignment) = option_assignment_range(line, &["method"]) else {
        return Vec::new();
    };
    [
        (
            "interpolate",
            "Fill missing values with native interpolation",
        ),
        ("record_only", "Record fill policy without changing values"),
    ]
    .into_iter()
    .map(|(value, title)| {
        json!({
            "title": title,
            "kind": "quickfix",
            "diagnostics": [diagnostic.clone()],
            "edit": single_change_workspace_edit(
                uri,
                line_byte_range(
                    line_number,
                    line,
                    assignment.value_start,
                    assignment.value_end,
                ),
                value,
            )
        })
    })
    .collect()
}

fn lsp_timeseries_fill_method_missing_actions(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Vec<Value> {
    [
        (
            "interpolate",
            "Fill missing values with native interpolation",
        ),
        ("record_only", "Record fill policy without changing values"),
    ]
    .into_iter()
    .filter_map(|(value, title)| {
        lsp_with_option_value_code_action(uri, text, diagnostic, "method", value, title)
    })
    .collect()
}

fn lsp_with_option_value_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
    option_name: &str,
    option_value: &str,
    title: &str,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    let attached_block = attached_with_block(&lines, line_number);
    if attached_block
        .as_ref()
        .is_some_and(|block| with_block_contains_option(&lines, block, option_name))
    {
        return None;
    }
    let newline = document_newline(text);
    let (range, new_text) = if let Some(block) = attached_block {
        (
            zero_width_range(block.end_line, 0),
            format!(
                "{}    {option_name} = {option_value}{newline}",
                block.indent
            ),
        )
    } else {
        let line = lines.get(line_number).copied().unwrap_or("");
        let indent = line_indent(line);
        (
            zero_width_range(line_number, utf16_len(line)),
            format!(
                "{newline}{indent}with {{{newline}{indent}    {option_name} = {option_value}{newline}{indent}}}"
            ),
        )
    };
    Some(json!({
        "title": title,
        "kind": "quickfix",
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, range, &new_text)
    }))
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

fn lsp_sampling_range_unit_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let fix = sample_uniform_endpoint_unit_fix(line)?;
    Some(json!({
        "title": format!("Add unit {} to sample {} endpoint", fix.unit, fix.endpoint),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            zero_width_range(line_number, utf16_len(&line[..fix.insert_byte])),
            &format!(" {}", fix.unit)
        )
    }))
}

struct SampleEndpointUnitFix {
    endpoint: &'static str,
    unit: String,
    insert_byte: usize,
}

fn sample_uniform_endpoint_unit_fix(line: &str) -> Option<SampleEndpointUnitFix> {
    let code = strip_line_comment(line);
    let uniform_start = code.find("uniform(")?;
    let open = uniform_start + "uniform".len();
    let close = matching_close_paren_byte(code, open)?;
    let inner_start = open + 1;
    let inner = &code[inner_start..close];
    let comma = top_level_comma_byte(inner)?;
    let lower = sample_endpoint_literal(&inner[..comma], inner_start)?;
    let upper = sample_endpoint_literal(&inner[comma + 1..], inner_start + comma + 1)?;
    match (lower.unit, upper.unit) {
        (None, Some(unit)) => Some(SampleEndpointUnitFix {
            endpoint: "lower",
            unit,
            insert_byte: lower.literal_end,
        }),
        (Some(unit), None) => Some(SampleEndpointUnitFix {
            endpoint: "upper",
            unit,
            insert_byte: upper.literal_end,
        }),
        _ => None,
    }
}

struct SampleEndpointLiteral {
    literal_end: usize,
    unit: Option<String>,
}

fn sample_endpoint_literal(segment: &str, absolute_start: usize) -> Option<SampleEndpointLiteral> {
    let leading = segment
        .chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let trimmed_end = segment.trim_end().len();
    if leading > trimmed_end {
        return None;
    }
    let text = &segment[leading..trimmed_end];
    let bytes = text.as_bytes();
    let mut index = 0usize;
    if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
        index += 1;
    }
    let digit_start = index;
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
    if index == digit_start {
        return None;
    }
    let literal_end = absolute_start + leading + index;
    let mut unit_start = index;
    while unit_start < bytes.len() && bytes[unit_start].is_ascii_whitespace() {
        unit_start += 1;
    }
    if unit_start == bytes.len() {
        return Some(SampleEndpointLiteral {
            literal_end,
            unit: None,
        });
    }
    let mut unit_end = unit_start;
    while unit_end < bytes.len()
        && (bytes[unit_end].is_ascii_alphanumeric()
            || matches!(
                bytes[unit_end],
                b'%' | b'/' | b'_' | b'^' | b'(' | b')' | b'*'
            ))
    {
        unit_end += 1;
    }
    if unit_end == unit_start || !text[unit_end..].trim().is_empty() {
        return None;
    }
    let unit = &text[unit_start..unit_end];
    is_unit_hint(unit).then(|| SampleEndpointLiteral {
        literal_end,
        unit: Some(unit.to_owned()),
    })
}

fn top_level_comma_byte(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
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
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
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
        "E-TIMESERIES-FILL-STEP" => Some(OptionQuickFix {
            option_names: &["expected_step", "step"],
            value: "1 h",
            label: "Set fill step to 1 h",
        }),
        "E-TIMESERIES-FILL-MAX-GAP" => Some(OptionQuickFix {
            option_names: &["max_gap"],
            value: "3 h",
            label: "Set maximum fill gap to 3 h",
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
    if let Some(assignment) = option_assignment_range(line, &option_names) {
        let fix = option_quick_fix_for_option(code, &assignment.option_name)?;
        let current_value = line.get(assignment.value_start..assignment.value_end)?;
        let range = diagnostic_range_for_exact_text(text, diagnostic, current_value)
            .unwrap_or_else(|| {
                line_byte_range(
                    line_number,
                    line,
                    assignment.value_start,
                    assignment.value_end,
                )
            });
        return Some(json!({
            "title": format!("{}: {} = {}", fix.label, assignment.option_name, fix.value),
            "kind": "quickfix",
            "isPreferred": true,
            "diagnostics": [diagnostic.clone()],
            "edit": single_change_workspace_edit(uri, range, fix.value)
        }));
    }

    let [option_name] = option_names.as_slice() else {
        return None;
    };
    if !is_simulate_or_solve_owner_line(line) {
        return None;
    }
    let fix = option_quick_fix_for_option(code, option_name)?;
    let title = format!("{}: {} = {}", fix.label, option_name, fix.value);
    let mut action =
        lsp_with_option_value_code_action(uri, text, diagnostic, option_name, fix.value, &title)?;
    action["isPreferred"] = json!(true);
    Some(action)
}

fn is_simulate_or_solve_owner_line(line: &str) -> bool {
    let code = strip_line_comment(line).trim();
    let Some((_binding, expression)) = code.split_once('=') else {
        return false;
    };
    matches!(
        expression.split_whitespace().next(),
        Some("simulate" | "solve")
    )
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

fn lsp_wrap_golden_code_action(uri: &str, text: &str, diagnostic: &Value) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let indent = line_indent(code);
    let golden = code[indent.len()..].trim_end();
    if !golden.starts_with("golden ") {
        return None;
    }
    let newline = document_newline(text);
    let replacement = format!(
        "{indent}test \"golden\" {{{newline}{indent}    {golden}{newline}{indent}}}{newline}"
    );
    Some(json!({
        "title": "Wrap golden check in test block",
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

struct LegacyModelTrainingMigration<'a> {
    alias: &'static str,
    lhs: &'a str,
    source: &'a str,
    options: Vec<(&'static str, &'a str)>,
    trailing_comment: &'a str,
}

fn lsp_legacy_model_training_migration_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let line_number = diagnostic_line(diagnostic)?;
    let lines = split_lines_preserve_logical(text);
    if has_following_with_block_after_trivia(&lines, line_number) {
        return None;
    }
    let line = *lines.get(line_number)?;
    let migration = legacy_model_training_migration_from_line(line)?;
    let replacement = legacy_model_training_migration_replacement(
        &migration,
        line_indent(line),
        document_newline(text),
    );
    Some(json!({
        "title": format!("Replace {} with train regression", migration.alias),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(uri, full_line_range(text, line_number), &replacement)
    }))
}

fn has_following_with_block_after_trivia(lines: &[&str], owner_line_number: usize) -> bool {
    let mut line_number = owner_line_number.saturating_add(1);
    while line_number < lines.len() && strip_line_comment(lines[line_number]).trim().is_empty() {
        line_number += 1;
    }
    lines
        .get(line_number)
        .is_some_and(|line| strip_line_comment(line).trim() == "with {")
}

fn legacy_model_training_migration_from_line(
    line: &str,
) -> Option<LegacyModelTrainingMigration<'_>> {
    let comment_start = line_comment_start(line).unwrap_or(line.len());
    let code = &line[..comment_start];
    let trailing_comment = line[comment_start..].trim();
    let (call_start, alias) = ["regression_table", "train_regression"]
        .into_iter()
        .filter_map(|alias| code.find(&format!("{alias}(")).map(|start| (start, alias)))
        .min_by_key(|(start, _)| *start)?;
    let before_call = &code[..call_start];
    let equals = before_call.rfind('=')?;
    if !before_call[equals + 1..].trim().is_empty() {
        return None;
    }
    let lhs = before_call[line_indent(before_call).len()..equals].trim();
    let binding = lhs
        .split_once(':')
        .map(|(name, _annotation)| name.trim())
        .unwrap_or(lhs);
    if !is_identifier(binding) {
        return None;
    }

    let open = call_start.checked_add(alias.len())?;
    let close = matching_close_paren_byte(code, open)?;
    if !code[close + 1..].trim().is_empty() {
        return None;
    }
    let parts = split_top_level_commas(&code[open + 1..close]);
    let source = parts.first()?.trim();
    if !is_simple_path_expression(source) {
        return None;
    }

    let mut seen = HashSet::new();
    let mut options = Vec::new();
    for part in parts.iter().skip(1) {
        let (name, value) = split_top_level_assignment(part)?;
        let name = match name.trim() {
            "target" | "y" => "target",
            "features" | "x" => "features",
            "test" | "test_fraction" => "test",
            "algorithm" => "algorithm",
            "seed" => "seed",
            _ => return None,
        };
        let value = value.trim();
        if value.is_empty() || value.contains('{') || value.contains('}') || !seen.insert(name) {
            return None;
        }
        options.push((name, value));
    }

    Some(LegacyModelTrainingMigration {
        alias,
        lhs,
        source,
        options,
        trailing_comment,
    })
}

fn legacy_model_training_migration_replacement(
    migration: &LegacyModelTrainingMigration<'_>,
    indent: &str,
    newline: &str,
) -> String {
    let comment = if migration.trailing_comment.is_empty() {
        String::new()
    } else {
        format!(" {}", migration.trailing_comment)
    };
    let mut replacement = format!(
        "{indent}{} = train regression {}{comment}{newline}",
        migration.lhs, migration.source
    );
    if migration.options.is_empty() {
        return replacement;
    }
    replacement.push_str(&format!("{indent}with {{{newline}"));
    for (name, value) in &migration.options {
        replacement.push_str(&format!("{indent}    {name} = {value}{newline}"));
    }
    replacement.push_str(&format!("{indent}}}{newline}"));
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
    let range = diagnostic_range_for_exact_text(text, diagnostic, expression).or_else(|| {
        let line_number = diagnostic_line(diagnostic)?;
        let line = text.lines().nth(line_number)?;
        let (start_byte, end_byte) = direct_uncertainty_expression_range(line, expression)?;
        Some(line_byte_range(line_number, line, start_byte, end_byte))
    })?;
    let replacement = format!("mean({expression})");
    Some(json!({
        "title": format!("Compare mean({expression}) instead"),
        "kind": "quickfix",
        "isPreferred": true,
        "diagnostics": [diagnostic.clone()],
        "edit": single_change_workspace_edit(
            uri,
            range,
            &replacement
        )
    }))
}

fn diagnostic_range_for_exact_text(
    text: &str,
    diagnostic: &Value,
    expected: &str,
) -> Option<Value> {
    let start_line = diagnostic
        .pointer("/range/start/line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end_line = diagnostic
        .pointer("/range/end/line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    if start_line != end_line {
        return None;
    }
    let start_character = diagnostic
        .pointer("/range/start/character")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let end_character = diagnostic
        .pointer("/range/end/character")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let line = text.lines().nth(start_line)?;
    let start_byte = utf16_character_to_byte(line, start_character);
    let end_byte = utf16_character_to_byte(line, end_character);
    if line.get(start_byte..end_byte) != Some(expected) {
        return None;
    }
    diagnostic.get("range").cloned()
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

fn lsp_command_style_function_call_code_action(
    uri: &str,
    text: &str,
    diagnostic: &Value,
) -> Option<Value> {
    let verb = command_style_verb_from_diagnostic(diagnostic_message(diagnostic))?;
    let line_number = diagnostic_line(diagnostic)?;
    let line = text.lines().nth(line_number)?;
    let code = strip_line_comment(line);
    let edit = command_style_function_call_edit(code, verb)?;
    Some(json!({
        "title": "Convert command-style call to function call",
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

struct CommandStyleFunctionCallEdit {
    start_byte: usize,
    end_byte: usize,
    new_text: String,
}

fn command_style_function_call_edit(
    code: &str,
    verb: &str,
) -> Option<CommandStyleFunctionCallEdit> {
    if !is_identifier(verb) {
        return None;
    }
    let search_start = code
        .find('=')
        .map(|index| index + 1)
        .unwrap_or_else(|| line_indent(code).len());
    let (verb_start, verb_end) = find_identifier_followed_by_whitespace(code, verb, search_start)?;
    let mut argument_start = verb_end;
    let bytes = code.as_bytes();
    if !bytes
        .get(argument_start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }
    while argument_start < bytes.len() && bytes[argument_start].is_ascii_whitespace() {
        argument_start += 1;
    }
    let end_byte = code.trim_end().len();
    if argument_start >= end_byte {
        return None;
    }
    let arguments = command_style_function_call_arguments(&code[argument_start..end_byte])?;
    Some(CommandStyleFunctionCallEdit {
        start_byte: verb_start,
        end_byte,
        new_text: format!("{verb}({arguments})"),
    })
}

fn command_style_function_call_arguments(rest: &str) -> Option<String> {
    let (target, clauses) = split_top_level_command_clauses(rest);
    let target = target.trim();
    if target.is_empty() || target.starts_with('(') || target.ends_with('{') {
        return None;
    }
    let mut arguments = vec![target.to_owned()];
    for (name, value) in clauses {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        arguments.push(format!("{name}={value}"));
    }
    Some(arguments.join(", "))
}

fn command_style_verb_from_diagnostic(message: &str) -> Option<&str> {
    let verb = first_backtick_payload(message)?.trim();
    is_identifier(verb).then_some(verb)
}

fn find_identifier_followed_by_whitespace(
    text: &str,
    name: &str,
    search_start: usize,
) -> Option<(usize, usize)> {
    let mut cursor = search_start.min(text.len());
    while let Some(relative_start) = text[cursor..].find(name) {
        let start = cursor + relative_start;
        let end = start + name.len();
        if identifier_boundary(text, start, end)
            && text
                .as_bytes()
                .get(end)
                .is_some_and(u8::is_ascii_whitespace)
        {
            return Some((start, end));
        }
        cursor = end;
    }
    None
}

fn split_top_level_command_clauses(rest: &str) -> (String, Vec<(&'static str, String)>) {
    let positions = top_level_command_clause_positions(rest);
    if positions.is_empty() {
        return (rest.trim().to_owned(), Vec::new());
    }
    let target = rest[..positions[0].0].trim().to_owned();
    let mut clauses = Vec::new();
    for (index, (start, name)) in positions.iter().enumerate() {
        let value_start = start + name.len();
        let value_end = positions
            .get(index + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(rest.len());
        clauses.push((*name, rest[value_start..value_end].trim().to_owned()));
    }
    (target, clauses)
}

fn top_level_command_clause_positions(text: &str) -> Vec<(usize, &'static str)> {
    const CLAUSES: &[&str] = &[
        "over", "by", "as", "above", "below", "between", "from", "to", "with",
    ];
    let mut positions = Vec::new();
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        match character {
            '"' => in_string = !in_string,
            '(' if !in_string => paren_depth += 1,
            ')' if !in_string => paren_depth -= 1,
            '[' if !in_string => bracket_depth += 1,
            ']' if !in_string => bracket_depth -= 1,
            _ => {}
        }
        if in_string || paren_depth != 0 || bracket_depth != 0 {
            continue;
        }
        for clause in CLAUSES {
            if starts_with_word_at(text, index, clause) {
                positions.push((index, *clause));
            }
        }
    }
    positions.sort_by_key(|(index, _)| *index);
    positions.dedup_by_key(|(index, _)| *index);
    positions
}

fn starts_with_word_at(text: &str, index: usize, word: &str) -> bool {
    if !text[index..].starts_with(word) {
        return false;
    }
    let before_ok = index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_some_and(|character| !is_identifier_character(character));
    let after_index = index + word.len();
    let after_ok = after_index >= text.len()
        || text[after_index..]
            .chars()
            .next()
            .is_some_and(|character| !is_identifier_character(character));
    before_ok && after_ok
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
    Some(snapshot_for_open_documents(&path, &text, documents))
}

fn snapshot_for_open_documents(
    path: &Path,
    source: &str,
    documents: &Documents,
) -> eng_lsp::LspSnapshot {
    let analysis = analysis_for_open_documents(path, source, documents);
    if let Some(snapshot) = analysis.snapshot {
        return (*snapshot).clone();
    }
    let snapshot = snapshot_from_report_with_source(&analysis.report, Some(source));
    if let Some((_, state)) =
        workspace_document_for_path(documents, path).filter(|(_, state)| state.text == source)
    {
        state.store_snapshot(source, &analysis.report, &snapshot);
    }
    snapshot
}

fn analysis_for_open_documents(
    path: &Path,
    source: &str,
    documents: &Documents,
) -> CachedDocumentAnalysis {
    let open_document = workspace_document_for_path(documents, path)
        .map(|(_, state)| state)
        .filter(|state| state.text == source);
    if let Some(state) = open_document {
        if let Some(analysis) = state.cached_analysis(source) {
            return analysis;
        }
        if let Some(analysis) = state.reuse_analysis_for_token_stable_trivia(source) {
            return analysis;
        }
        if let Some(analysis) = state.recheck_scalar_declaration_suffix(source) {
            return analysis;
        }
    }
    let import_overrides = import_source_overrides_from_documents(documents);
    let report = Arc::new(check_source_with_import_overrides(
        path,
        source,
        &import_overrides,
        &CheckOptions::default(),
    ));
    let analysis = CachedDocumentAnalysis {
        source: source.to_owned(),
        report,
        snapshot: None,
    };
    if let Some(state) = open_document {
        state.store_analysis(&analysis);
    }
    analysis
}

const MAX_WORKSPACE_INDEX_FILES: usize = 500;
const MAX_WORKSPACE_SYMBOL_RESULTS: usize = 200;
const MAX_WORKSPACE_REFERENCE_RESULTS: usize = 1_000;
const WORKSPACE_OPEN_DOCUMENT_FORMAT: &str = "eng-lsp-open-documents-v1";
const MAX_WORKSPACE_OPEN_DOCUMENTS: usize = 128;
const MAX_WORKSPACE_OPEN_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_WORKSPACE_OPEN_DOCUMENT_TOTAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_WORKSPACE_OPEN_DOCUMENT_PAYLOAD_BYTES: usize = 100 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WorkspaceWalkStatus {
    truncated: bool,
    unreadable: bool,
    cancelled: bool,
}

impl WorkspaceWalkStatus {
    fn merge(&mut self, other: Self) {
        self.truncated |= other.truncated;
        self.unreadable |= other.unreadable;
        self.cancelled |= other.cancelled;
    }
}

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
    workspace_symbols_for_request_with_cancellation(request, documents, workspace_roots, None)
}

fn workspace_symbols_for_request_with_cancellation(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    cancellation: Option<&RequestCancellation>,
) -> Vec<Value> {
    let query = request
        .pointer("/params/query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut results = Vec::new();
    let mut seen = HashSet::<(String, usize, String)>::new();
    let mut open_document_paths = HashSet::<PathBuf>::new();

    for (uri, state) in documents {
        if request_is_cancelled(cancellation) || results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            break;
        }
        let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !path_is_within_workspace_roots(&canonical, workspace_roots) {
            continue;
        }
        if !open_document_paths.insert(canonical.clone()) {
            continue;
        }
        push_workspace_symbols_from_source(
            uri,
            &canonical,
            &state.text,
            &query,
            &mut results,
            &mut seen,
        );
    }

    let mut files = Vec::new();
    for root in workspace_roots {
        let status = collect_workspace_eng_files_with_cancellation(
            root,
            &mut files,
            MAX_WORKSPACE_INDEX_FILES,
            cancellation,
        );
        if status.cancelled || files.len() >= MAX_WORKSPACE_INDEX_FILES {
            break;
        }
    }
    for path in files {
        if request_is_cancelled(cancellation) || results.len() >= MAX_WORKSPACE_SYMBOL_RESULTS {
            break;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        if !path_is_within_workspace_roots(&canonical, workspace_roots) {
            continue;
        }
        let uri = file_uri_from_path(&canonical);
        if open_document_paths.contains(&canonical) || documents.contains_key(&uri) {
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

fn path_is_within_workspace_roots(path: &Path, workspace_roots: &[PathBuf]) -> bool {
    workspace_roots.is_empty()
        || workspace_roots.iter().any(|root| {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            path.starts_with(canonical_root)
        })
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

fn collect_workspace_eng_files_with_cancellation(
    root: &Path,
    files: &mut Vec<PathBuf>,
    limit: usize,
    cancellation: Option<&RequestCancellation>,
) -> WorkspaceWalkStatus {
    let mut status = WorkspaceWalkStatus::default();
    if request_is_cancelled(cancellation) {
        status.cancelled = true;
        return status;
    }
    if files.len() >= limit {
        status.truncated = true;
        return status;
    }
    let Ok(metadata) = std::fs::metadata(root) else {
        status.unreadable = true;
        return status;
    };
    if metadata.is_file() {
        if root.extension().is_some_and(|extension| extension == "eng") {
            files.push(root.to_path_buf());
        }
        return status;
    }
    if !metadata.is_dir() || skip_workspace_index_dir(root) {
        return status;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        status.unreadable = true;
        return status;
    };
    for entry in entries {
        if request_is_cancelled(cancellation) {
            status.cancelled = true;
            break;
        }
        let Ok(entry) = entry else {
            status.unreadable = true;
            continue;
        };
        if files.len() >= limit {
            status.truncated = true;
            break;
        }
        status.merge(collect_workspace_eng_files_with_cancellation(
            &entry.path(),
            files,
            limit,
            cancellation,
        ));
        if status.cancelled {
            break;
        }
    }
    status
}

fn skip_workspace_index_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".dev" | ".git" | ".vscode" | "build" | "target" | "dist" | "node_modules" | "__pycache__"
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
        let analysis = analysis_for_open_documents(&path, &text, documents);
        return completion_items_at(&analysis.report, &text, line, character);
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
        .map(|text| snapshot_for_open_documents(&path, text, documents))
        .or_else(|| snapshot_for_path(&path).ok())?;
    if let Some(symbol) = text
        .as_deref()
        .and_then(|text| symbol_at_position(text, line_zero_based, character))
    {
        if let Some(hover) =
            hover_for_symbol_on_line(&snapshot.hovers, &symbol, line_zero_based + 1).cloned()
        {
            return Some(hover);
        }
    }
    if let Some(hover) = text.as_deref().and_then(|text| {
        hover_for_semantic_token_position(&snapshot, text, line_zero_based, character)
    }) {
        return Some(hover.clone());
    }
    let line = line_zero_based + 1;
    root_hover_on_line(&snapshot.hovers, line).cloned()
}

fn hover_for_semantic_token_position<'a>(
    snapshot: &'a eng_lsp::LspSnapshot,
    source: &str,
    line: usize,
    character: usize,
) -> Option<&'a eng_lsp::LspHover> {
    let mut tokens = snapshot
        .semantic_tokens
        .tokens
        .iter()
        .filter(|token| {
            token.line == line
                && character >= token.start
                && character <= token.start + token.length
        })
        .collect::<Vec<_>>();
    tokens.sort_by_key(|token| (character == token.start + token.length, token.length));
    for token in tokens {
        let Some(text) = semantic_token_text(source, token) else {
            continue;
        };
        if let Some(hover) = snapshot
            .hovers
            .iter()
            .filter(|hover| hover.line == line + 1 && hover.name == text)
            .min_by_key(|hover| !hover.is_root_source())
        {
            return Some(hover);
        }
    }
    None
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
    let snapshot = snapshot_for_open_documents(&path, &text, documents);
    let symbol = symbol_at_position(&text, line_zero_based, character)?;
    if let Some(target) = stdlib_module_definition_target(&symbol) {
        return Some(definition_location_json(&target));
    }
    if let Some(hover) = hover_for_symbol(&snapshot.hovers, &symbol) {
        let label = definition_label_for_hover_name(&hover.name);
        if let Some(target) = definition_target_in_source(uri, &text, &label, hover.line)
            .or_else(|| imported_definition_target(&path, &text, documents, &label, hover.line))
        {
            return Some(definition_location_json(&target));
        }
    }

    let semantic_symbol = workspace_semantic_symbol_occurrences(
        &path,
        &text,
        documents,
        &snapshot.semantic_tokens.tokens,
        &snapshot.hovers,
        line_zero_based,
        character,
    )?;
    let target = definition_target_for_family_in_source(
        uri,
        &text,
        &semantic_symbol.label,
        &semantic_symbol.family,
        line_zero_based + 1,
    )
    .or_else(|| {
        imported_definition_target_for_family(
            &path,
            &text,
            documents,
            &semantic_symbol.label,
            &semantic_symbol.family,
            line_zero_based + 1,
        )
    })?;
    Some(definition_location_json(&target))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DocumentHighlightTarget {
    line: usize,
    start_character: usize,
    end_character: usize,
    kind: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DocumentHighlightScope {
    start_line: usize,
    end_line: usize,
}

#[derive(Clone, Debug)]
struct SemanticSymbolOccurrences {
    selected: eng_lsp::LspSemanticToken,
    label: String,
    family: String,
    scope: Option<DocumentHighlightScope>,
    occurrences: Vec<eng_lsp::LspSemanticToken>,
}

fn document_highlights_for_request(request: &Value, documents: &Documents) -> Value {
    let Some(uri) = request_uri(request) else {
        return json!([]);
    };
    let Some(text) = document_text_for_uri(uri, documents) else {
        return json!([]);
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
    let snapshot = snapshot_for_open_documents(&path, &text, documents);
    let Some(symbol) = workspace_semantic_symbol_occurrences(
        &path,
        &text,
        documents,
        &snapshot.semantic_tokens.tokens,
        &snapshot.hovers,
        line,
        character,
    ) else {
        return json!([]);
    };
    let highlights = symbol
        .occurrences
        .iter()
        .map(|token| DocumentHighlightTarget {
            line: token.line,
            start_character: token.start,
            end_character: token.start + token.length,
            kind: document_highlight_kind(&text, token),
        })
        .collect::<Vec<_>>();
    json!(highlights
        .iter()
        .map(document_highlight_json)
        .collect::<Vec<_>>())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceReferenceIdentity {
    definition_path: PathBuf,
    definition_line: usize,
    label: String,
    family: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SemanticReferenceLocation {
    uri: String,
    line: usize,
    start: usize,
    length: usize,
    declaration: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceSource {
    uri: String,
    path: PathBuf,
    text: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct WorkspaceSourceCollection {
    sources: Vec<WorkspaceSource>,
    truncated: bool,
    unreadable: bool,
    cancelled: bool,
}

fn references_for_request(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
) -> Value {
    references_for_request_with_cancellation(request, documents, workspace_roots, None)
}

fn references_for_request_with_cancellation(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    cancellation: Option<&RequestCancellation>,
) -> Value {
    if request_is_cancelled(cancellation) {
        return json!([]);
    }
    let Some(uri) = request_uri(request) else {
        return json!([]);
    };
    let Some(text) = document_text_for_uri(uri, documents) else {
        return json!([]);
    };
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let Some((line, character)) = request_position(request) else {
        return json!([]);
    };
    let include_declaration = request
        .pointer("/params/context/includeDeclaration")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let snapshot = snapshot_for_open_documents(&path, &text, documents);
    if request_is_cancelled(cancellation) {
        return json!([]);
    }
    let Some(symbol) = workspace_semantic_symbol_occurrences(
        &path,
        &text,
        documents,
        &snapshot.semantic_tokens.tokens,
        &snapshot.hovers,
        line,
        character,
    ) else {
        return json!([]);
    };
    let mut locations = symbol
        .occurrences
        .iter()
        .map(|token| semantic_reference_location(uri, token))
        .collect::<Vec<_>>();

    if let Some(identity) =
        workspace_reference_identity(uri, &path, &text, documents, &snapshot.hovers, &symbol)
    {
        let mut workspace_locations = workspace_reference_locations(
            uri,
            &path,
            documents,
            workspace_roots,
            &identity,
            cancellation,
        );
        workspace_locations
            .truncate(MAX_WORKSPACE_REFERENCE_RESULTS.saturating_sub(locations.len()));
        locations.extend(workspace_locations);
    }

    locations.retain(|location| include_declaration || !location.declaration);
    locations.sort_by(|left, right| {
        (&left.uri, left.line, left.start, left.length).cmp(&(
            &right.uri,
            right.line,
            right.start,
            right.length,
        ))
    });
    locations.dedup_by(|left, right| {
        left.uri == right.uri
            && left.line == right.line
            && left.start == right.start
            && left.length == right.length
    });
    locations.truncate(MAX_WORKSPACE_REFERENCE_RESULTS);
    json!(locations
        .iter()
        .map(semantic_reference_location_json)
        .collect::<Vec<_>>())
}

fn semantic_reference_location(
    uri: &str,
    token: &eng_lsp::LspSemanticToken,
) -> SemanticReferenceLocation {
    SemanticReferenceLocation {
        uri: uri.to_owned(),
        line: token.line,
        start: token.start,
        length: token.length,
        declaration: semantic_token_is_declaration(token),
    }
}

fn semantic_reference_location_json(location: &SemanticReferenceLocation) -> Value {
    json!({
        "uri": location.uri,
        "range": {
            "start": { "line": location.line, "character": location.start },
            "end": {
                "line": location.line,
                "character": location.start + location.length
            }
        }
    })
}

fn workspace_reference_identity(
    uri: &str,
    source_path: &Path,
    source: &str,
    documents: &Documents,
    hovers: &[eng_lsp::LspHover],
    symbol: &SemanticSymbolOccurrences,
) -> Option<WorkspaceReferenceIdentity> {
    if symbol.scope.is_some()
        || has_semantic_modifier(&symbol.selected, "local")
        || has_semantic_modifier(&symbol.selected, "defaultLibrary")
        || !matches!(symbol.family.as_str(), "type" | "variable" | "function")
    {
        return None;
    }

    let preferred_line = hover_for_symbol(hovers, &symbol.label)
        .map(|hover| hover.line)
        .unwrap_or(symbol.selected.line + 1);
    let local_target = definition_target_for_family_in_source(
        uri,
        source,
        &symbol.label,
        &symbol.family,
        preferred_line,
    );
    let target = if let Some(local_target) = local_target {
        let importable_target = importable_definition_target_in_source(
            uri,
            source,
            &symbol.label,
            &symbol.family,
            preferred_line,
        )?;
        (local_target.line == importable_target.line).then_some(importable_target)?
    } else {
        imported_definition_target_for_family(
            source_path,
            source,
            documents,
            &symbol.label,
            &symbol.family,
            preferred_line,
        )?
    };
    let definition_path = path_from_uri(&target.uri)?;
    let definition_path = definition_path.canonicalize().unwrap_or(definition_path);
    let definition_line = if let Some((definition_uri, state)) =
        workspace_document_for_path(documents, &definition_path)
    {
        importable_definition_target_in_source(
            definition_uri,
            &state.text,
            &symbol.label,
            &symbol.family,
            target.line + 1,
        )?
        .line
    } else {
        target.line
    };
    Some(WorkspaceReferenceIdentity {
        definition_path,
        definition_line,
        label: symbol.label.clone(),
        family: symbol.family.clone(),
    })
}

fn workspace_reference_locations(
    selected_uri: &str,
    selected_path: &Path,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    identity: &WorkspaceReferenceIdentity,
    cancellation: Option<&RequestCancellation>,
) -> Vec<SemanticReferenceLocation> {
    let collection = workspace_sources_for_reference(
        selected_path,
        documents,
        workspace_roots,
        identity,
        cancellation,
    );
    let mut locations = Vec::new();
    for source in collection.sources {
        if request_is_cancelled(cancellation) || locations.len() >= MAX_WORKSPACE_REFERENCE_RESULTS
        {
            break;
        }
        if source.uri == selected_uri
            || !source.text.contains(&identity.label)
            || !source_resolves_workspace_reference(&source, documents, identity)
        {
            continue;
        }
        let snapshot = snapshot_for_open_documents(&source.path, &source.text, documents);
        let Some(symbol) = semantic_symbol_occurrences_for_workspace_identity(
            &source.text,
            &snapshot.semantic_tokens.tokens,
            identity,
        ) else {
            continue;
        };
        locations.extend(
            symbol
                .occurrences
                .iter()
                .map(|token| semantic_reference_location(&source.uri, token)),
        );
    }
    locations
}

fn workspace_sources_for_reference(
    selected_path: &Path,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    identity: &WorkspaceReferenceIdentity,
    cancellation: Option<&RequestCancellation>,
) -> WorkspaceSourceCollection {
    let workspace_roots = canonical_workspace_roots(workspace_roots);
    let selected_path = selected_path
        .canonicalize()
        .unwrap_or_else(|_| selected_path.to_path_buf());
    let mut seen_paths = HashSet::from([selected_path]);
    let mut collection = WorkspaceSourceCollection::default();

    if request_is_cancelled(cancellation) {
        collection.cancelled = true;
        return collection;
    }

    collection.unreadable |= !push_workspace_source_for_path(
        &identity.definition_path,
        documents,
        &mut seen_paths,
        &mut collection.sources,
    );
    for (uri, state) in documents {
        if request_is_cancelled(cancellation) {
            collection.cancelled = true;
            break;
        }
        if collection.sources.len() >= MAX_WORKSPACE_INDEX_FILES {
            collection.truncated = true;
            break;
        }
        let Some(path) = path_from_uri(uri) else {
            continue;
        };
        let path = path.canonicalize().unwrap_or(path);
        if !workspace_roots.is_empty() && !path_is_in_workspace(&path, &workspace_roots) {
            continue;
        }
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        collection.sources.push(WorkspaceSource {
            uri: uri.clone(),
            path,
            text: state.text.clone(),
        });
    }

    let mut files = Vec::new();
    for root in &workspace_roots {
        let status = collect_workspace_eng_files_with_cancellation(
            root,
            &mut files,
            MAX_WORKSPACE_INDEX_FILES,
            cancellation,
        );
        collection.truncated |= status.truncated;
        collection.unreadable |= status.unreadable;
        collection.cancelled |= status.cancelled;
        if status.truncated || status.cancelled {
            break;
        }
    }
    for path in files {
        if request_is_cancelled(cancellation) {
            collection.cancelled = true;
            break;
        }
        if collection.sources.len() >= MAX_WORKSPACE_INDEX_FILES {
            collection.truncated = true;
            break;
        }
        let path = path.canonicalize().unwrap_or(path);
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            collection.unreadable = true;
            continue;
        };
        collection.sources.push(WorkspaceSource {
            uri: file_uri_from_path(&path),
            path,
            text,
        });
    }
    collection
}

fn push_workspace_source_for_path(
    path: &Path,
    documents: &Documents,
    seen_paths: &mut HashSet<PathBuf>,
    sources: &mut Vec<WorkspaceSource>,
) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen_paths.insert(path.clone()) || sources.len() >= MAX_WORKSPACE_INDEX_FILES {
        return true;
    }
    if let Some((uri, state)) = workspace_document_for_path(documents, &path) {
        sources.push(WorkspaceSource {
            uri: uri.clone(),
            path,
            text: state.text.clone(),
        });
        return true;
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    sources.push(WorkspaceSource {
        uri: file_uri_from_path(&path),
        path,
        text,
    });
    true
}

fn workspace_document_for_path<'a>(
    documents: &'a Documents,
    path: &Path,
) -> Option<(&'a String, &'a DocumentState)> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    documents.iter().find(|(uri, _)| {
        path_from_uri(uri)
            .map(|candidate| candidate.canonicalize().unwrap_or(candidate) == path)
            .unwrap_or(false)
    })
}

fn source_resolves_workspace_reference(
    source: &WorkspaceSource,
    documents: &Documents,
    identity: &WorkspaceReferenceIdentity,
) -> bool {
    if source.path == identity.definition_path {
        return true;
    }
    if definition_target_for_family_in_source(
        &source.uri,
        &source.text,
        &identity.label,
        &identity.family,
        identity.definition_line + 1,
    )
    .is_some()
    {
        return false;
    }
    let Some(target) = imported_definition_target_for_family(
        &source.path,
        &source.text,
        documents,
        &identity.label,
        &identity.family,
        identity.definition_line + 1,
    ) else {
        return false;
    };
    path_from_uri(&target.uri)
        .map(|path| path.canonicalize().unwrap_or(path) == identity.definition_path)
        .unwrap_or(false)
}

fn semantic_token_is_declaration(token: &eng_lsp::LspSemanticToken) -> bool {
    token
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.as_str(), "declaration" | "definition"))
}

fn semantic_symbol_occurrences(
    source: &str,
    tokens: &[eng_lsp::LspSemanticToken],
    hovers: &[eng_lsp::LspHover],
    line: usize,
    character: usize,
) -> Option<SemanticSymbolOccurrences> {
    let selected = semantic_symbol_token_at_position(tokens, line, character)?;
    let label = semantic_token_text(source, selected)?;
    let family = semantic_symbol_family(&selected.token_type, &selected.modifiers)?.to_owned();
    let selected_is_local = has_semantic_modifier(selected, "local");
    let selected_member_receiver = semantic_member_receiver(source, selected);
    let scope = semantic_symbol_scope(source, hovers, selected, &label);
    let mut occurrences = tokens
        .iter()
        .filter(|token| {
            semantic_symbol_family(&token.token_type, &token.modifiers) == Some(family.as_str())
                && has_semantic_modifier(token, "local") == selected_is_local
                && (!matches!(family.as_str(), "property" | "method")
                    || semantic_member_receiver(source, token) == selected_member_receiver)
                && scope.is_none_or(|scope| {
                    token.line >= scope.start_line && token.line <= scope.end_line
                })
                && semantic_token_text(source, token).as_deref() == Some(label.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    occurrences.sort_by_key(|token| (token.line, token.start, token.length));
    occurrences.dedup_by_key(|token| (token.line, token.start, token.length));
    Some(SemanticSymbolOccurrences {
        selected: selected.clone(),
        label,
        family,
        scope,
        occurrences,
    })
}

fn workspace_semantic_symbol_occurrences(
    source_path: &Path,
    source: &str,
    documents: &Documents,
    tokens: &[eng_lsp::LspSemanticToken],
    hovers: &[eng_lsp::LspHover],
    line: usize,
    character: usize,
) -> Option<SemanticSymbolOccurrences> {
    if let Some(symbol) = semantic_symbol_occurrences(source, tokens, hovers, line, character) {
        return Some(symbol);
    }

    let (label, start, length) = identifier_range_at_position(source, line, character)?;
    let mut candidates = ["type", "variable", "function"]
        .into_iter()
        .filter_map(|family| {
            imported_definition_target_for_family(
                source_path,
                source,
                documents,
                &label,
                family,
                line + 1,
            )
            .map(|target| (family, target))
        })
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return None;
    }
    let (family, target) = candidates.pop()?;
    let definition_path = path_from_uri(&target.uri)?;
    let identity = WorkspaceReferenceIdentity {
        definition_path: definition_path.canonicalize().unwrap_or(definition_path),
        definition_line: target.line,
        label,
        family: family.to_owned(),
    };
    let mut symbol = semantic_symbol_occurrences_for_workspace_identity(source, tokens, &identity)?;
    symbol.selected = symbol
        .occurrences
        .iter()
        .find(|token| token.line == line && token.start == start && token.length == length)?
        .clone();
    Some(symbol)
}

fn prepare_rename_for_request(request: &Value, documents: &Documents) -> Option<Value> {
    let uri = request_uri(request)?;
    let text = document_text_for_uri(uri, documents)?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let (line, character) = request_position(request)?;
    let snapshot = snapshot_for_open_documents(&path, &text, documents);
    let symbol = workspace_semantic_symbol_occurrences(
        &path,
        &text,
        documents,
        &snapshot.semantic_tokens.tokens,
        &snapshot.hovers,
        line,
        character,
    )?;
    let workspace_identity =
        workspace_reference_identity(uri, &path, &text, documents, &snapshot.hovers, &symbol);
    (semantic_symbol_is_renameable(&text, &symbol) || workspace_identity.is_some()).then(|| {
        json!({
            "range": semantic_token_range_json(&symbol.selected),
            "placeholder": symbol.label
        })
    })
}

fn rename_for_request(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
) -> Result<Value, String> {
    rename_for_request_with_cancellation(request, documents, workspace_roots, None)
}

fn rename_for_request_with_cancellation(
    request: &Value,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    cancellation: Option<&RequestCancellation>,
) -> Result<Value, String> {
    if request_is_cancelled(cancellation) {
        return Err("Request cancelled.".to_owned());
    }
    let uri = request_uri(request)
        .ok_or_else(|| "Rename request is missing a document URI.".to_owned())?;
    let text = document_text_for_uri(uri, documents)
        .ok_or_else(|| "Rename request could not load the current document.".to_owned())?;
    let path = path_from_uri(uri).unwrap_or_else(|| PathBuf::from("buffer.eng"));
    let (line, character) = request_position(request)
        .ok_or_else(|| "Rename request is missing a valid cursor position.".to_owned())?;
    let new_name = request
        .pointer("/params/newName")
        .and_then(Value::as_str)
        .map(str::trim)
        .ok_or_else(|| "Rename request is missing the new symbol name.".to_owned())?;
    let snapshot = snapshot_for_open_documents(&path, &text, documents);
    if request_is_cancelled(cancellation) {
        return Err("Request cancelled.".to_owned());
    }
    let symbol = workspace_semantic_symbol_occurrences(
        &path,
        &text,
        documents,
        &snapshot.semantic_tokens.tokens,
        &snapshot.hovers,
        line,
        character,
    )
    .ok_or_else(|| "The selected token is not a renameable EngLang symbol.".to_owned())?;
    let current_file_renameable = semantic_symbol_is_renameable(&text, &symbol);
    let workspace_identity =
        workspace_reference_identity(uri, &path, &text, documents, &snapshot.hovers, &symbol);
    if new_name == symbol.label {
        return Err("The new symbol name is unchanged.".to_owned());
    }
    if !valid_rename_identifier(new_name) {
        return Err(format!("`{new_name}` is not a valid EngLang identifier."));
    }
    if reserved_rename_identifier(new_name) {
        return Err(format!("`{new_name}` is reserved by EngLang."));
    }

    if let Some(identity) = workspace_identity {
        if !workspace_roots.is_empty() {
            let selected = WorkspaceSource {
                uri: uri.to_owned(),
                path: path.clone(),
                text: text.clone(),
            };
            return workspace_rename_for_symbol(
                &selected,
                documents,
                workspace_roots,
                &identity,
                new_name,
                cancellation,
            );
        }
        if !current_file_renameable {
            return Err(
                "Static-import rename needs an initialized workspace root so every affected EngLang file can be verified."
                    .to_owned(),
            );
        }
    }
    if !current_file_renameable {
        return Err(
            "Rename supports current-file declarations and static file-import symbols; this selection has no editable declaration identity."
                .to_owned(),
        );
    }
    if semantic_rename_conflicts(&text, &snapshot.semantic_tokens.tokens, &symbol, new_name) {
        return Err(format!(
            "Rename would conflict with the existing `{new_name}` symbol in this scope."
        ));
    }

    let edits = symbol
        .occurrences
        .iter()
        .map(|token| {
            json!({
                "range": semantic_token_range_json(token),
                "newText": new_name
            })
        })
        .collect::<Vec<_>>();
    let mut changes = serde_json::Map::new();
    changes.insert(uri.to_owned(), Value::Array(edits));
    Ok(json!({ "changes": Value::Object(changes) }))
}

fn workspace_rename_for_symbol(
    selected: &WorkspaceSource,
    documents: &Documents,
    workspace_roots: &[PathBuf],
    identity: &WorkspaceReferenceIdentity,
    new_name: &str,
    cancellation: Option<&RequestCancellation>,
) -> Result<Value, String> {
    let workspace_roots = canonical_workspace_roots(workspace_roots);
    let selected_path = selected
        .path
        .canonicalize()
        .unwrap_or_else(|_| selected.path.clone());
    if !path_is_in_workspace(&selected_path, &workspace_roots) {
        return Err(
            "Workspace rename requires the selected EngLang file to be inside an initialized workspace root."
                .to_owned(),
        );
    }
    if !path_is_in_workspace(&identity.definition_path, &workspace_roots) {
        return Err(
            "Workspace rename cannot edit a static-import declaration outside the initialized workspace roots."
                .to_owned(),
        );
    }

    let collection = workspace_sources_for_reference(
        &selected_path,
        documents,
        &workspace_roots,
        identity,
        cancellation,
    );
    if collection.cancelled {
        return Err("Request cancelled.".to_owned());
    }
    if collection.truncated {
        return Err(format!(
            "Workspace rename stopped because the EngLang index reached its {MAX_WORKSPACE_INDEX_FILES}-file safety limit. Narrow the workspace and retry."
        ));
    }
    if collection.unreadable {
        return Err(
            "Workspace rename stopped because at least one required EngLang source could not be read."
                .to_owned(),
        );
    }

    let mut sources = Vec::with_capacity(collection.sources.len() + 1);
    sources.push(WorkspaceSource {
        uri: selected.uri.clone(),
        path: selected_path,
        text: selected.text.clone(),
    });
    sources.extend(collection.sources);

    let mut changes = serde_json::Map::new();
    let mut edit_count = 0usize;
    let mut declaration_edited = false;
    for source in sources {
        if request_is_cancelled(cancellation) {
            return Err("Request cancelled.".to_owned());
        }
        if !path_is_in_workspace(&source.path, &workspace_roots)
            || !source.text.contains(&identity.label)
            || (source.uri != selected.uri
                && !source_resolves_workspace_reference(&source, documents, identity))
        {
            continue;
        }
        let snapshot = snapshot_for_open_documents(&source.path, &source.text, documents);
        let Some(symbol) = semantic_symbol_occurrences_for_workspace_identity(
            &source.text,
            &snapshot.semantic_tokens.tokens,
            identity,
        ) else {
            if source
                .text
                .lines()
                .any(|line| !rename_identifier_ranges_on_line(line, &identity.label).is_empty())
            {
                return Err(format!(
                    "Workspace rename cannot prove every `{}` occurrence in {} is semantic.",
                    identity.label,
                    source.path.display()
                ));
            }
            continue;
        };
        if !semantic_rename_occurrences_are_complete(&source.text, &symbol) {
            return Err(format!(
                "Workspace rename cannot prove every `{}` occurrence in {} is semantic.",
                identity.label,
                source.path.display()
            ));
        }
        if semantic_rename_conflicts(
            &source.text,
            &snapshot.semantic_tokens.tokens,
            &symbol,
            new_name,
        ) {
            return Err(format!(
                "Workspace rename would conflict with the existing `{new_name}` symbol in {}.",
                source.path.display()
            ));
        }
        if edit_count + symbol.occurrences.len() > MAX_WORKSPACE_REFERENCE_RESULTS {
            return Err(format!(
                "Workspace rename stopped because it exceeded the {MAX_WORKSPACE_REFERENCE_RESULTS}-edit safety limit."
            ));
        }

        if source.path == identity.definition_path
            && symbol.occurrences.iter().any(|token| {
                token.line == identity.definition_line && semantic_token_is_declaration(token)
            })
        {
            declaration_edited = true;
        }
        let edits = symbol
            .occurrences
            .iter()
            .map(|token| {
                json!({
                    "range": semantic_token_range_json(token),
                    "newText": new_name
                })
            })
            .collect::<Vec<_>>();
        edit_count += edits.len();
        if !edits.is_empty() {
            changes.insert(source.uri, Value::Array(edits));
        }
    }

    if !declaration_edited {
        return Err(
            "Workspace rename could not verify an edit for the static-import declaration."
                .to_owned(),
        );
    }
    Ok(json!({ "changes": Value::Object(changes) }))
}

fn canonical_workspace_roots(workspace_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = workspace_roots
        .iter()
        .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

fn path_is_in_workspace(path: &Path, workspace_roots: &[PathBuf]) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    workspace_roots
        .iter()
        .any(|root| path == *root || path.starts_with(root))
}

fn semantic_symbol_occurrences_for_workspace_identity(
    source: &str,
    tokens: &[eng_lsp::LspSemanticToken],
    identity: &WorkspaceReferenceIdentity,
) -> Option<SemanticSymbolOccurrences> {
    let mut occurrences = tokens
        .iter()
        .filter(|token| {
            !has_semantic_modifier(token, "local")
                && semantic_symbol_family(&token.token_type, &token.modifiers)
                    == Some(identity.family.as_str())
                && semantic_token_text(source, token).as_deref() == Some(identity.label.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let token_type = match identity.family.as_str() {
        "type" => "type",
        "variable" => "variable",
        "function" => "function",
        _ => return None,
    };
    let mut known_ranges = occurrences
        .iter()
        .map(|token| (token.line, token.start, token.length))
        .collect::<HashSet<_>>();
    for (line_number, line) in source.lines().enumerate() {
        for (start, length) in rename_identifier_ranges_on_line(line, &identity.label) {
            if known_ranges.contains(&(line_number, start, length)) {
                continue;
            }
            let conflicts_with_semantic_symbol = tokens.iter().any(|token| {
                token.line == line_number
                    && token.start == start
                    && token.length == length
                    && semantic_symbol_family(&token.token_type, &token.modifiers).is_some()
            });
            if conflicts_with_semantic_symbol {
                continue;
            }
            known_ranges.insert((line_number, start, length));
            occurrences.push(eng_lsp::LspSemanticToken {
                line: line_number,
                start,
                length,
                token_type: token_type.to_owned(),
                modifiers: Vec::new(),
            });
        }
    }
    occurrences.sort_by_key(|token| (token.line, token.start, token.length));
    occurrences.dedup_by_key(|token| (token.line, token.start, token.length));
    let selected = occurrences
        .iter()
        .find(|token| {
            token.line == identity.definition_line && semantic_token_is_declaration(token)
        })
        .or_else(|| occurrences.first())?
        .clone();
    Some(SemanticSymbolOccurrences {
        selected,
        label: identity.label.clone(),
        family: identity.family.clone(),
        scope: None,
        occurrences,
    })
}

fn request_position(request: &Value) -> Option<(usize, usize)> {
    Some((
        request
            .pointer("/params/position/line")
            .and_then(Value::as_u64)? as usize,
        request
            .pointer("/params/position/character")
            .and_then(Value::as_u64)? as usize,
    ))
}

fn semantic_symbol_is_renameable(source: &str, symbol: &SemanticSymbolOccurrences) -> bool {
    if !matches!(
        symbol.family.as_str(),
        "namespace" | "type" | "parameter" | "variable" | "function"
    ) || has_semantic_modifier(&symbol.selected, "imported")
        || has_semantic_modifier(&symbol.selected, "defaultLibrary")
    {
        return false;
    }
    symbol.occurrences.iter().any(|token| {
        has_semantic_modifier(token, "declaration")
            || has_semantic_modifier(token, "definition")
            || document_highlight_kind(source, token) == 3
    }) && semantic_rename_occurrences_are_complete(source, symbol)
}

fn semantic_rename_occurrences_are_complete(
    source: &str,
    symbol: &SemanticSymbolOccurrences,
) -> bool {
    let semantic_ranges = symbol
        .occurrences
        .iter()
        .map(|token| (token.line, token.start, token.length))
        .collect::<HashSet<_>>();
    let lines = source.lines().collect::<Vec<_>>();
    let start_line = symbol.scope.map_or(0, |scope| scope.start_line);
    let end_line = symbol
        .scope
        .map_or_else(|| lines.len().saturating_sub(1), |scope| scope.end_line)
        .min(lines.len().saturating_sub(1));

    lines
        .iter()
        .enumerate()
        .skip(start_line)
        .take(end_line.saturating_sub(start_line) + 1)
        .flat_map(|(line_number, line)| {
            rename_identifier_ranges_on_line(line, &symbol.label)
                .into_iter()
                .map(move |(start, length)| (line_number, start, length))
        })
        .all(|range| semantic_ranges.contains(&range))
}

fn rename_identifier_ranges_on_line(line: &str, label: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = line.as_bytes();
    let end = line_comment_start(line).unwrap_or(line.len());
    let mut index = 0usize;
    let mut in_string = false;
    while index < end {
        if in_string {
            if bytes[index] == b'\\' {
                index = (index + 2).min(end);
                continue;
            }
            if bytes[index] == b'"' {
                in_string = false;
                index += 1;
                continue;
            }
            if bytes[index] == b'{' && bytes.get(index + 1) != Some(&b'{') {
                let expression_start = index + 1;
                let Some(close_offset) = line[expression_start..end].find('}') else {
                    break;
                };
                let close = expression_start + close_offset;
                let expression_end = line[expression_start..close]
                    .find(':')
                    .map_or(close, |colon| expression_start + colon);
                collect_rename_identifier_ranges(
                    line,
                    expression_start,
                    expression_end,
                    label,
                    &mut ranges,
                );
                index = close + 1;
                continue;
            }
            index += 1;
            continue;
        }
        if bytes[index] == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if is_identifier_byte(bytes[index]) && !bytes[index].is_ascii_digit() {
            let token_start = index;
            index += 1;
            while index < end && is_identifier_byte(bytes[index]) {
                index += 1;
            }
            push_rename_identifier_range(line, token_start, index, label, &mut ranges);
            continue;
        }
        index += 1;
    }
    ranges
}

fn collect_rename_identifier_ranges(
    line: &str,
    start: usize,
    end: usize,
    label: &str,
    ranges: &mut Vec<(usize, usize)>,
) {
    let bytes = line.as_bytes();
    let mut index = start;
    while index < end {
        if is_identifier_byte(bytes[index]) && !bytes[index].is_ascii_digit() {
            let token_start = index;
            index += 1;
            while index < end && is_identifier_byte(bytes[index]) {
                index += 1;
            }
            push_rename_identifier_range(line, token_start, index, label, ranges);
            continue;
        }
        index += 1;
    }
}

fn push_rename_identifier_range(
    line: &str,
    start: usize,
    end: usize,
    label: &str,
    ranges: &mut Vec<(usize, usize)>,
) {
    if line.get(start..end) != Some(label)
        || start
            .checked_sub(1)
            .and_then(|index| line.as_bytes().get(index))
            == Some(&b'.')
    {
        return;
    }
    ranges.push((utf16_len(&line[..start]), utf16_len(&line[start..end])));
}

fn semantic_rename_conflicts(
    source: &str,
    tokens: &[eng_lsp::LspSemanticToken],
    symbol: &SemanticSymbolOccurrences,
    new_name: &str,
) -> bool {
    let Some(selected_namespace) = semantic_rename_namespace(&symbol.family) else {
        return true;
    };
    let selected_ranges = symbol
        .occurrences
        .iter()
        .map(|token| (token.line, token.start, token.length))
        .collect::<HashSet<_>>();
    tokens.iter().any(|token| {
        let Some(candidate_family) = semantic_symbol_family(&token.token_type, &token.modifiers)
        else {
            return false;
        };
        if selected_ranges.contains(&(token.line, token.start, token.length))
            || symbol
                .scope
                .is_some_and(|scope| token.line < scope.start_line || token.line > scope.end_line)
            || semantic_token_text(source, token).as_deref() != Some(new_name)
        {
            return false;
        }
        semantic_rename_namespace(candidate_family) == Some(selected_namespace)
    })
}

fn semantic_rename_namespace(family: &str) -> Option<&'static str> {
    match family {
        "type" => Some("type"),
        "namespace" => Some("namespace"),
        "parameter" | "variable" | "function" => Some("value"),
        _ => None,
    }
}

fn valid_rename_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn reserved_rename_identifier(name: &str) -> bool {
    let catalog = editor_syntax_catalog_json();
    for key in [
        "keywords",
        "constants",
        "workflow_status_literals",
        "operator_words",
        "workflow_builtins",
    ] {
        if catalog
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(name)))
        {
            return true;
        }
    }
    for key in ["types", "quantities", "units"] {
        if catalog
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.get("label").and_then(Value::as_str) == Some(name))
            })
        {
            return true;
        }
    }
    false
}

fn semantic_token_range_json(token: &eng_lsp::LspSemanticToken) -> Value {
    json!({
        "start": { "line": token.line, "character": token.start },
        "end": { "line": token.line, "character": token.start + token.length }
    })
}

fn has_semantic_modifier(token: &eng_lsp::LspSemanticToken, modifier: &str) -> bool {
    token
        .modifiers
        .iter()
        .any(|candidate| candidate == modifier)
}

fn semantic_symbol_token_at_position(
    tokens: &[eng_lsp::LspSemanticToken],
    line: usize,
    character: usize,
) -> Option<&eng_lsp::LspSemanticToken> {
    tokens
        .iter()
        .filter(|token| {
            token.line == line
                && semantic_symbol_family(&token.token_type, &token.modifiers).is_some()
                && character >= token.start
                && character < token.start + token.length
        })
        .min_by_key(|token| token.length)
        .or_else(|| {
            tokens
                .iter()
                .filter(|token| {
                    token.line == line
                        && semantic_symbol_family(&token.token_type, &token.modifiers).is_some()
                        && character == token.start + token.length
                })
                .min_by_key(|token| token.length)
        })
}

fn semantic_symbol_family<'a>(token_type: &'a str, modifiers: &[String]) -> Option<&'a str> {
    if modifiers.iter().any(|modifier| modifier == "unit") {
        return None;
    }
    match token_type {
        "type" | "class" | "interface" => Some("type"),
        "namespace" | "parameter" | "variable" | "property" | "function" | "method" => {
            Some(token_type)
        }
        _ => None,
    }
}

fn semantic_token_text(source: &str, token: &eng_lsp::LspSemanticToken) -> Option<String> {
    let line = source.lines().nth(token.line)?;
    let start = utf16_character_to_byte(line, token.start);
    let end = utf16_character_to_byte(line, token.start + token.length);
    (start < end && end <= line.len()).then(|| line[start..end].to_owned())
}

fn semantic_member_receiver(source: &str, token: &eng_lsp::LspSemanticToken) -> Option<String> {
    if !matches!(token.token_type.as_str(), "property" | "method") {
        return None;
    }
    let line = source.lines().nth(token.line)?;
    let start = utf16_character_to_byte(line, token.start);
    let bytes = line.as_bytes();
    if start == 0 || bytes.get(start - 1) != Some(&b'.') {
        return None;
    }
    let mut receiver_start = start - 1;
    while receiver_start > 0 && is_symbol_byte(bytes[receiver_start - 1]) {
        receiver_start -= 1;
    }
    let receiver = line[receiver_start..start - 1].trim_matches('.');
    (!receiver.is_empty()).then(|| receiver.to_owned())
}

fn semantic_symbol_scope(
    source: &str,
    hovers: &[eng_lsp::LspHover],
    selected: &eng_lsp::LspSemanticToken,
    label: &str,
) -> Option<DocumentHighlightScope> {
    let is_local = has_semantic_modifier(selected, "local");
    if !is_local && selected.token_type != "parameter" {
        return None;
    }

    let lines = source.lines().collect::<Vec<_>>();
    let mut scopes = Vec::new();
    for hover in hovers
        .iter()
        .filter(|hover| matches!(hover.kind.as_str(), "function" | "component") && hover.line > 0)
    {
        let start_line = hover.line - 1;
        let Some(end_line) = matching_block_end_line(&lines, start_line) else {
            continue;
        };
        if selected.line >= start_line && selected.line <= end_line {
            scopes.push(DocumentHighlightScope {
                start_line,
                end_line,
            });
        }
    }

    if hovers.iter().any(|hover| {
        hover.kind == "where_local"
            && definition_label_for_hover_name(&hover.name) == label
            && hover.line > 0
    }) {
        for start_line in 0..lines.len() {
            if !is_where_block_start(lines[start_line]) {
                continue;
            }
            let Some(end_line) = matching_block_end_line(&lines, start_line) else {
                continue;
            };
            let scope_start = start_line.saturating_sub(1);
            if selected.line >= scope_start && selected.line <= end_line {
                scopes.push(DocumentHighlightScope {
                    start_line: scope_start,
                    end_line,
                });
            }
        }
    }

    scopes
        .into_iter()
        .min_by_key(|scope| scope.end_line.saturating_sub(scope.start_line))
}

fn document_highlight_kind(source: &str, token: &eng_lsp::LspSemanticToken) -> u8 {
    if token
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier.as_str(), "declaration" | "definition"))
    {
        return 3;
    }
    let Some(line) = source.lines().nth(token.line) else {
        return 2;
    };
    let end = utf16_character_to_byte(line, token.start + token.length);
    let suffix = line.get(end..).unwrap_or("").trim_start();
    if suffix.starts_with('=') && !suffix.starts_with("==") && !suffix.starts_with("=>") {
        3
    } else {
        2
    }
}

fn document_highlight_json(target: &DocumentHighlightTarget) -> Value {
    json!({
        "range": {
            "start": { "line": target.line, "character": target.start_character },
            "end": { "line": target.line, "character": target.end_character }
        },
        "kind": target.kind
    })
}

fn hover_for_symbol<'a>(
    hovers: &'a [eng_lsp::LspHover],
    symbol: &str,
) -> Option<&'a eng_lsp::LspHover> {
    hover_for_symbol_role(hovers, symbol, None, false, Some(true))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, false, Some(false)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, true, Some(true)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, true, Some(false)))
}

fn root_hover_on_line(hovers: &[eng_lsp::LspHover], line: usize) -> Option<&eng_lsp::LspHover> {
    hovers
        .iter()
        .find(|hover| hover.is_root_source() && hover.line == line)
}

fn hover_for_symbol_on_line<'a>(
    hovers: &'a [eng_lsp::LspHover],
    symbol: &str,
    line: usize,
) -> Option<&'a eng_lsp::LspHover> {
    hover_for_symbol_role(hovers, symbol, Some(line), false, Some(true))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, false, Some(true)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, Some(line), false, Some(false)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, false, Some(false)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, Some(line), true, Some(true)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, true, Some(true)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, Some(line), true, Some(false)))
        .or_else(|| hover_for_symbol_role(hovers, symbol, None, true, Some(false)))
}

fn hover_for_symbol_role<'a>(
    hovers: &'a [eng_lsp::LspHover],
    symbol: &str,
    line: Option<usize>,
    semantic_role_fallback: bool,
    root_source: Option<bool>,
) -> Option<&'a eng_lsp::LspHover> {
    let symbol_label = symbol.rsplit('.').next().unwrap_or(symbol);
    hovers
        .iter()
        .filter(|hover| line.is_none_or(|line| hover.line == line))
        .filter(|hover| root_source.is_none_or(|root| hover.is_root_source() == root))
        .filter(|hover| semantic_role_hover_kind(&hover.kind) == semantic_role_fallback)
        .find(|hover| hover.name == symbol)
        .or_else(|| {
            hovers
                .iter()
                .filter(|hover| line.is_none_or(|line| hover.line == line))
                .filter(|hover| root_source.is_none_or(|root| hover.is_root_source() == root))
                .filter(|hover| semantic_role_hover_kind(&hover.kind) == semantic_role_fallback)
                .find(|hover| {
                    hover
                        .name
                        .rsplit('.')
                        .next()
                        .map(|label| label.strip_suffix("()").unwrap_or(label))
                        .is_some_and(|hover_label| hover_label == symbol_label)
                })
        })
}

fn semantic_role_hover_kind(kind: &str) -> bool {
    matches!(
        kind,
        "unit"
            | "quantity"
            | "timeseries_axis"
            | "timeseries"
            | "side_effect"
            | "external_boundary"
            | "uncertainty"
            | "validation"
    )
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
    let cursor = utf16_character_to_byte(line_text, character);
    if !bytes.get(cursor).is_some_and(|byte| is_symbol_byte(*byte))
        && !cursor
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .is_some_and(|byte| is_symbol_byte(*byte))
    {
        return None;
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

fn identifier_range_at_position(
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, usize, usize)> {
    let line_text = source.lines().nth(line)?;
    let bytes = line_text.as_bytes();
    let cursor = utf16_character_to_byte(line_text, character);
    if !bytes
        .get(cursor)
        .is_some_and(|byte| is_identifier_byte(*byte))
        && !cursor
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .is_some_and(|byte| is_identifier_byte(*byte))
    {
        return None;
    }
    let mut start = cursor;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = cursor;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }
    (start < end).then(|| {
        (
            line_text[start..end].to_owned(),
            utf16_len(&line_text[..start]),
            utf16_len(&line_text[start..end]),
        )
    })
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

fn definition_target_for_family_in_source(
    uri: &str,
    source: &str,
    label: &str,
    family: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let parsed = parse_source(source);
    let mut first_line = None;
    for item in &parsed.items {
        let Some(line) = ast_definition_line_for_family(item, label, family) else {
            continue;
        };
        first_line.get_or_insert(line);
        if line == preferred_line {
            return definition_target_on_line(uri, source, line, label);
        }
    }
    first_line.and_then(|line| definition_target_on_line(uri, source, line, label))
}

fn importable_definition_target_in_source(
    uri: &str,
    source: &str,
    label: &str,
    family: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let parsed = parse_source(source);
    let mut first_line = None;
    for item in &parsed.items {
        let Some(line) = ast_importable_definition_line_for_family(item, label, family) else {
            continue;
        };
        first_line.get_or_insert(line);
        if line == preferred_line {
            return definition_target_on_line(uri, source, line, label);
        }
    }
    first_line.and_then(|line| definition_target_on_line(uri, source, line, label))
}

fn ast_definition_line_for_family(item: &AstItem, label: &str, family: &str) -> Option<usize> {
    let line = ast_definition_line_for_label(item, label)?;
    let matches_family = match family {
        "function" => matches!(item, AstItem::Function(_)),
        "type" => matches!(
            item,
            AstItem::Schema(_)
                | AstItem::Struct(_)
                | AstItem::Class(_)
                | AstItem::System(_)
                | AstItem::StateSpaceTypeBlock(_)
                | AstItem::Domain(_)
                | AstItem::Component(_)
        ),
        "variable" => matches!(
            item,
            AstItem::Const(_)
                | AstItem::FastBinding(_)
                | AstItem::ExplicitDecl(_)
                | AstItem::ClassObject(_)
                | AstItem::ClassObjectCopy(_)
                | AstItem::StateSpaceVector(_)
                | AstItem::Test(_)
        ),
        _ => false,
    };
    matches_family.then_some(line)
}

fn ast_importable_definition_line_for_family(
    item: &AstItem,
    label: &str,
    family: &str,
) -> Option<usize> {
    match (family, item) {
        ("function", AstItem::Function(function)) if function.name == label => {
            Some(function.span.line)
        }
        ("variable", AstItem::Const(declaration))
            if declaration.name == label && declaration.context == ParseContext::TopLevel =>
        {
            Some(declaration.line)
        }
        ("type", AstItem::Schema(schema)) if schema.name == label => Some(schema.span.line),
        ("type", AstItem::Class(class_info)) if class_info.name == label => {
            Some(class_info.span.line)
        }
        ("type", AstItem::System(system)) if system.name == label => Some(system.span.line),
        ("type", AstItem::StateSpaceTypeBlock(block)) if block.name == label => Some(block.line),
        ("type", AstItem::Domain(domain)) if domain.name == label => Some(domain.span.line),
        ("type", AstItem::Component(component)) if component.name == label => {
            Some(component.span.line)
        }
        _ => None,
    }
}

fn imported_definition_target_for_family(
    source_path: &Path,
    source: &str,
    documents: &Documents,
    label: &str,
    family: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let base_dir = source_path.parent()?;
    let parsed = parse_source(source);
    let mut visited = HashSet::new();
    imported_definition_target_for_family_from_program(
        &parsed,
        base_dir,
        documents,
        label,
        family,
        preferred_line,
        &mut visited,
    )
}

fn imported_definition_target_for_family_from_program(
    parsed: &eng_compiler::ParsedProgram,
    base_dir: &Path,
    documents: &Documents,
    label: &str,
    family: &str,
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
        let (imported_uri, imported_source) =
            if let Some((uri, state)) = workspace_document_for_path(documents, &import_path) {
                (uri.clone(), Cow::Borrowed(state.text.as_str()))
            } else {
                let Ok(source) = std::fs::read_to_string(&import_path) else {
                    visited.remove(&import_path);
                    continue;
                };
                (file_uri_from_path(&import_path), Cow::Owned(source))
            };
        if let Some(target) = importable_definition_target_in_source(
            &imported_uri,
            imported_source.as_ref(),
            label,
            family,
            preferred_line,
        ) {
            return Some(target);
        }
        let imported = parse_source(imported_source.as_ref());
        if let Some(import_base_dir) = import_path.parent() {
            if let Some(target) = imported_definition_target_for_family_from_program(
                &imported,
                import_base_dir,
                documents,
                label,
                family,
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

fn imported_definition_target(
    source_path: &Path,
    source: &str,
    documents: &Documents,
    label: &str,
    preferred_line: usize,
) -> Option<DefinitionTarget> {
    let base_dir = source_path.parent()?;
    let parsed = parse_source(source);
    let mut visited = HashSet::new();
    imported_definition_target_from_program(
        &parsed,
        base_dir,
        documents,
        label,
        preferred_line,
        &mut visited,
    )
}

fn imported_definition_target_from_program(
    parsed: &eng_compiler::ParsedProgram,
    base_dir: &Path,
    documents: &Documents,
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
        let (imported_uri, imported_source) =
            if let Some((uri, state)) = workspace_document_for_path(documents, &import_path) {
                (uri.clone(), Cow::Borrowed(state.text.as_str()))
            } else {
                let Ok(source) = std::fs::read_to_string(&import_path) else {
                    visited.remove(&import_path);
                    continue;
                };
                (file_uri_from_path(&import_path), Cow::Owned(source))
            };
        if let Some(target) = definition_target_in_source(
            &imported_uri,
            imported_source.as_ref(),
            label,
            preferred_line,
        ) {
            return Some(target);
        }
        let imported = parse_source(imported_source.as_ref());
        if let Some(import_base_dir) = import_path.parent() {
            if let Some(target) = imported_definition_target_from_program(
                &imported,
                import_base_dir,
                documents,
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
    format!("file://{}", percent_encode_file_uri_path(&path))
}

fn percent_encode_file_uri_path(path: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[(byte >> 4) as usize]));
            encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
    encoded
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

fn strict_utf16_character_to_byte(line_text: &str, character: usize) -> Option<usize> {
    let mut units = 0usize;
    for (byte_index, ch) in line_text.char_indices() {
        if units == character {
            return Some(byte_index);
        }
        if units > character {
            return None;
        }
        units += ch.len_utf16();
    }
    (units == character).then_some(line_text.len())
}

fn lsp_position_to_byte_offset(text: &str, line: usize, character: usize) -> Option<usize> {
    let mut line_start = 0usize;
    for _ in 0..line {
        let newline = text.get(line_start..)?.find('\n')?;
        line_start = line_start.checked_add(newline + 1)?;
    }

    let remaining = text.get(line_start..)?;
    let line_end = remaining
        .find('\n')
        .map_or(text.len(), |newline| line_start + newline);
    let content_end = if line_end > 0 && text.as_bytes().get(line_end - 1) == Some(&b'\r') {
        line_end - 1
    } else {
        line_end
    };
    let line_text = text.get(line_start..content_end)?;
    strict_utf16_character_to_byte(line_text, character)
        .and_then(|byte| line_start.checked_add(byte))
}

fn content_change_position(change: &Value, endpoint: &str) -> Option<(usize, usize)> {
    let position = change.get("range")?.get(endpoint)?;
    let line = position
        .get("line")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    let character = position
        .get("character")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())?;
    Some((line, character))
}

fn apply_document_content_changes(current: &str, changes: &[Value]) -> Option<String> {
    if changes.is_empty() {
        return None;
    }
    let mut text = current.to_owned();
    for change in changes {
        let replacement = change.get("text")?.as_str()?;
        if change.get("range").is_none() {
            text.clear();
            text.push_str(replacement);
            continue;
        }
        let (start_line, start_character) = content_change_position(change, "start")?;
        let (end_line, end_character) = content_change_position(change, "end")?;
        let start = lsp_position_to_byte_offset(&text, start_line, start_character)?;
        let end = lsp_position_to_byte_offset(&text, end_line, end_character)?;
        if start > end {
            return None;
        }
        text.replace_range(start..end, replacement);
    }
    Some(text)
}

fn document_state_from_notification(
    request: &Value,
    documents: &Documents,
) -> Option<(String, DocumentState)> {
    let uri = request_uri(request)?.to_owned();
    if stale_document_notification(request, documents.get(&uri)) {
        return None;
    }
    let version = document_version_from_request(request)
        .or_else(|| documents.get(&uri).and_then(|state| state.version));
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let text = match method {
        "textDocument/didOpen" => request
            .pointer("/params/textDocument/text")
            .and_then(Value::as_str)?
            .to_owned(),
        "textDocument/didChange" => {
            let current = documents.get(&uri)?;
            let changes = request
                .pointer("/params/contentChanges")
                .and_then(Value::as_array)?;
            apply_document_content_changes(&current.text, changes)?
        }
        "textDocument/didSave" => request
            .pointer("/params/text")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| documents.get(&uri).map(|state| state.text.clone()))
            .or_else(|| path_from_uri(&uri).and_then(|path| std::fs::read_to_string(path).ok()))?,
        _ => return None,
    };
    let state = DocumentState::updated(text, version, documents.get(&uri));
    Some((uri, state))
}

fn stale_document_notification(request: &Value, current: Option<&DocumentState>) -> bool {
    let Some(incoming_version) = document_version_from_request(request) else {
        return false;
    };
    let Some(current_version) = current.and_then(|state| state.version) else {
        return false;
    };
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    incoming_version < current_version
        || (incoming_version == current_version && method != "textDocument/didSave")
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
    let path = if cfg!(target_os = "windows") {
        if decoded.starts_with('/') && decoded.as_bytes().get(2) == Some(&b':') {
            decoded.trim_start_matches('/').replace('/', "\\")
        } else {
            decoded.replace('/', "\\")
        }
    } else {
        decoded
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

fn write_request_response<W: Write>(
    output: &mut W,
    response: Value,
    cancellation: Option<&RequestCancellation>,
) -> io::Result<()> {
    if cancellation.is_some_and(RequestCancellation::is_cancelled) {
        return write_request_cancelled(output, response.get("id").cloned());
    }
    write_response(output, response)
}

fn write_request_cancelled<W: Write>(output: &mut W, id: Option<Value>) -> io::Result<()> {
    write_response(
        output,
        json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": { "code": -32800, "message": "Request cancelled." }
        }),
    )
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
    fn pending_diagnostics_coalesce_reset_and_cancel_documents() {
        let start = Instant::now();
        let mut pending = PendingDiagnostics::new(Duration::from_millis(100));
        pending.schedule(["b.eng".to_owned(), "a.eng".to_owned()], start);
        pending.schedule(
            ["c.eng".to_owned(), "a.eng".to_owned()],
            start + Duration::from_millis(75),
        );

        assert!(pending
            .take_due(start + Duration::from_millis(174))
            .is_empty());
        assert_eq!(
            pending.take_due(start + Duration::from_millis(175)),
            vec!["a.eng".to_owned(), "b.eng".to_owned(), "c.eng".to_owned()]
        );
        assert!(pending.timeout(start).is_none());

        pending.schedule(
            ["a.eng".to_owned(), "b.eng".to_owned()],
            start + Duration::from_millis(200),
        );
        pending.cancel("a.eng");
        assert_eq!(
            pending.take_due(start + Duration::from_millis(300)),
            vec!["b.eng".to_owned()]
        );
    }

    #[test]
    fn persistent_diagnostics_debounce_reads_and_clamps_initialization_options() {
        assert_eq!(
            persistent_diagnostics_debounce(&json!({ "params": {} })),
            Duration::from_millis(DEFAULT_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS)
        );
        assert_eq!(
            persistent_diagnostics_debounce(&json!({
                "params": {
                    "initializationOptions": { "diagnosticsDebounceMs": 275 }
                }
            })),
            Duration::from_millis(275)
        );
        assert_eq!(
            persistent_diagnostics_debounce(&json!({
                "params": {
                    "initializationOptions": {
                        "englang": { "liveDiagnosticsDelayMs": 10_000 }
                    }
                }
            })),
            Duration::from_millis(MAX_PERSISTENT_DIAGNOSTICS_DEBOUNCE_MS)
        );
    }

    #[test]
    fn input_reader_cancels_matching_request_ids_and_cleans_completed_entries() {
        let registry = RequestCancellationRegistry::default();
        let request_event = lsp_input_event(
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "workspace/symbol",
                "params": { "query": "heat" }
            })
            .to_string(),
            &registry,
        );
        let request_cancellation = match &request_event {
            LspInputEvent::Message {
                cancellation: Some(cancellation),
                ..
            } => cancellation,
            _ => panic!("request should carry a cancellation registration"),
        };
        assert!(!request_cancellation.is_cancelled());

        let _wrong_id = lsp_input_event(
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": "7" }
            })
            .to_string(),
            &registry,
        );
        assert!(!request_cancellation.is_cancelled());

        let _cancel = lsp_input_event(
            json!({
                "jsonrpc": "2.0",
                "method": "$/cancelRequest",
                "params": { "id": 7 }
            })
            .to_string(),
            &registry,
        );
        assert!(request_cancellation.is_cancelled());

        drop(request_event);
        assert!(!registry.cancel(&json!(7)));
    }

    #[test]
    fn cancelled_workspace_walk_stops_before_reading_the_root() {
        let registry = RequestCancellationRegistry::default();
        let cancellation = registry.register(&json!(8)).unwrap();
        assert!(registry.cancel(&json!(8)));

        let mut files = Vec::new();
        let status = collect_workspace_eng_files_with_cancellation(
            Path::new("missing-cancelled-workspace"),
            &mut files,
            10,
            Some(&cancellation),
        );
        assert!(status.cancelled);
        assert!(!status.unreadable);
        assert!(files.is_empty());
    }

    #[test]
    fn semantic_token_delta_edits_reconstruct_current_data() {
        let previous = vec![0, 0, 4, 5, 1, 1, 2, 3, 6, 0, 0, 5, 2, 9, 4];
        let current = vec![0, 0, 4, 5, 1, 1, 3, 7, 6, 0, 0, 5, 2, 9, 4];
        let edits = semantic_token_delta_edits(&previous, &current);
        assert_eq!(edits.len(), 1);

        let edit = &edits[0];
        let start = edit["start"].as_u64().unwrap() as usize;
        let delete_count = edit["deleteCount"].as_u64().unwrap() as usize;
        let inserted = edit["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as usize);
        let mut reconstructed = previous.clone();
        reconstructed.splice(start..start + delete_count, inserted);
        assert_eq!(reconstructed, current);
        assert!(semantic_token_delta_edits(&current, &current).is_empty());

        let deletion = semantic_token_delta_edits(&[1, 2, 3], &[1, 3]);
        assert_eq!(deletion, vec![json!({ "start": 1, "deleteCount": 1 })]);
    }

    #[test]
    fn semantic_token_cache_bounds_history_and_falls_back_to_full_data() {
        let uri = "file:///C:/workspace/cache.eng";
        let mut cache = SemanticTokenCache::default();
        let first = cache.full_response(Some(uri), vec![1, 2, 3]);
        let first_id = first["resultId"].as_str().unwrap().to_owned();
        let delta = cache.delta_response(Some(uri), Some(&first_id), vec![1, 4, 3]);
        assert!(delta["edits"].is_array());
        assert!(delta.get("data").is_none());

        for value in 0..MAX_SEMANTIC_TOKEN_RESULTS_PER_DOCUMENT {
            cache.full_response(Some(uri), vec![value]);
        }
        let expired = cache.delta_response(Some(uri), Some(&first_id), vec![9]);
        assert_eq!(expired["data"], json!([9]));
        assert!(expired.get("edits").is_none());

        cache.remove_document(uri);
        let closed = cache.delta_response(Some(uri), Some(&first_id), vec![10]);
        assert_eq!(closed["data"], json!([10]));
    }

    #[test]
    fn symbol_hover_prefers_structured_metadata_over_role_fallbacks() {
        let hover = |name: &str, kind: &str, line: usize| eng_lsp::LspHover {
            name: name.to_owned(),
            kind: kind.to_owned(),
            line,
            source_id: eng_compiler::SourceSpan::ROOT_SOURCE_ID,
            source_path: None,
            detail: kind.to_owned(),
            quantity_kind: String::new(),
            display_unit: "-".to_owned(),
            status: None,
        };
        let hovers = vec![
            hover("Q_for_energy", "timeseries", 29),
            hover("where.Q_for_energy", "where_local", 31),
            hover("degC", "unit", 7),
        ];

        for line in [29, 31] {
            assert_eq!(
                hover_for_symbol_on_line(&hovers, "Q_for_energy", line)
                    .map(|hover| hover.kind.as_str()),
                Some("where_local")
            );
        }
        assert_eq!(
            hover_for_symbol_on_line(&hovers, "degC", 7).map(|hover| hover.kind.as_str()),
            Some("unit")
        );

        let mut imported = hover("ImportedOnly", "function", 29);
        imported.source_id = 1;
        assert!(root_hover_on_line(&[imported.clone()], 29).is_none());
        assert_eq!(
            hover_for_symbol_on_line(&[imported], "ImportedOnly", 29)
                .map(|hover| hover.kind.as_str()),
            Some("function")
        );
        assert_eq!(
            root_hover_on_line(&hovers, 29).map(|hover| hover.name.as_str()),
            Some("Q_for_energy")
        );

        let mut imported_shared = hover("shared", "class", 7);
        imported_shared.source_id = 1;
        let root_shared = hover("shared", "parameter", 7);
        let shared_hovers = [imported_shared, root_shared];
        let selected = hover_for_symbol_on_line(&shared_hovers, "shared", 7)
            .expect("root-owned same-name hover");
        assert!(selected.is_root_source());
        assert_eq!(selected.kind, "parameter");
        assert!(hover_for_symbol(&shared_hovers, "shared")
            .expect("root-owned symbol hover")
            .is_root_source());
    }

    #[test]
    fn semantic_token_hover_resolves_composite_unit_ranges() {
        let source = "irradiance: Irradiance [W/m2] = 300 W/m2\n";
        let snapshot = snapshot_for_source(Path::new("composite_unit_hover.eng"), source);
        let character = source.find("W/m2").unwrap() + 2;
        let hover = hover_for_semantic_token_position(&snapshot, source, 0, character)
            .expect("composite unit hover");

        assert_eq!(hover.name, "W/m2");
        assert_eq!(hover.kind, "unit");
        assert_eq!(hover.display_unit, "W/m2");
    }

    #[test]
    fn legacy_model_training_quick_fix_migrates_only_unambiguous_single_lines() {
        let uri = "file:///C:/workspace/legacy-model.eng";
        let diagnostic = json!({
            "range": {
                "start": { "line": 0, "character": 15 },
                "end": { "line": 0, "character": 31 }
            },
            "code": "W-ML-TRAIN-ALIAS",
            "message": "`regression_table(...)` is a compatibility-only model training alias."
        });
        let source = "legacy_model = regression_table(designs, y=annual_electricity, x=[cooling_cop], test_fraction=0.25, seed=7) # keep\r\n";
        let actions = code_actions_for_diagnostic(uri, source, &diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0]["title"],
            "Replace regression_table with train regression"
        );
        assert_eq!(
            actions[0]["edit"]["changes"][uri][0]["newText"],
            "legacy_model = train regression designs # keep\r\nwith {\r\n    target = annual_electricity\r\n    features = [cooling_cop]\r\n    test = 0.25\r\n    seed = 7\r\n}\r\n"
        );

        let train_alias_source = "model = train_regression(designs)\n";
        let train_alias_actions = code_actions_for_diagnostic(uri, train_alias_source, &diagnostic);
        assert_eq!(train_alias_actions.len(), 1);
        assert_eq!(
            train_alias_actions[0]["edit"]["changes"][uri][0]["newText"],
            "model = train regression designs\n"
        );

        let attached =
            "model = train_regression(designs)\n# existing options\nwith {\n    target = y\n}\n";
        assert!(code_actions_for_diagnostic(uri, attached, &diagnostic).is_empty());
        let duplicate = "model = regression_table(designs, target=y, y=other, features=[x])\n";
        assert!(code_actions_for_diagnostic(uri, duplicate, &diagnostic).is_empty());
    }

    #[test]
    fn ann_alias_quick_fix_replaces_only_the_diagnostic_range() {
        let uri = "file:///C:/workspace/ann-alias.eng";
        let source = "note = ann\r\nmodel = ann(split, hidden=[8], epochs=20)\r\n";
        let diagnostic = json!({
            "range": {
                "start": { "line": 1, "character": 8 },
                "end": { "line": 1, "character": 11 }
            },
            "code": "W-ML-ANN-ALIAS",
            "message": "`ann(...)` is a compatibility-only alias for `mlp(...)`."
        });

        let actions = code_actions_for_diagnostic(uri, source, &diagnostic);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["title"], "Replace ann with mlp");
        assert_eq!(
            actions[0]["edit"]["changes"][uri][0]["range"],
            diagnostic["range"]
        );
        assert_eq!(actions[0]["edit"]["changes"][uri][0]["newText"], "mlp");
    }

    #[test]
    fn sampling_alias_quick_fixes_replace_only_the_method_token() {
        let uri = "file:///C:/workspace/sampling-alias.eng";
        let source = concat!(
            "# uniform latin_hypercube latin-hypercube\r\n",
            "a = sample uniform\r\n",
            "b = sample latin_hypercube\r\n",
            "c = sample latin-hypercube\r\n",
        );
        for (line, code, alias, replacement, title) in [
            (
                1,
                "W-SAMPLING-UNIFORM-ALIAS",
                "uniform",
                "random",
                "Replace sampling method with random",
            ),
            (
                2,
                "W-SAMPLING-LATIN-HYPERCUBE-ALIAS",
                "latin_hypercube",
                "lhs",
                "Replace sampling method with lhs",
            ),
            (
                3,
                "W-SAMPLING-LATIN-HYPERCUBE-ALIAS",
                "latin-hypercube",
                "lhs",
                "Replace sampling method with lhs",
            ),
        ] {
            let source_line = source.lines().nth(line).expect("sampling alias line");
            let start = source_line.rfind(alias).expect("sampling alias token");
            let diagnostic = json!({
                "range": {
                    "start": { "line": line, "character": start },
                    "end": { "line": line, "character": start + alias.len() }
                },
                "code": code,
                "message": "sampling compatibility alias"
            });
            let actions = code_actions_for_diagnostic(uri, source, &diagnostic);
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0]["title"], title);
            assert_eq!(
                actions[0]["edit"]["changes"][uri][0]["range"],
                diagnostic["range"]
            );
            assert_eq!(
                actions[0]["edit"]["changes"][uri][0]["newText"],
                replacement
            );
        }
    }

    #[test]
    fn uncertainty_argument_alias_quick_fixes_replace_only_the_key() {
        let uri = "file:///C:/workspace/uncertainty-argument-alias.eng";
        let source = concat!(
            "# bias distribution error gain max min mu n sigma uncertainty\r\n",
            "a = propagate(source, bias=1, gain=2, n=3)\r\n",
            "b = distribution(distribution=normal, mu=4, sigma=5, n=6)\r\n",
            "c = uniform(min=7, max=8, n=9)\r\n",
            "d = measured(10, error=0.1, uncertainty=0.2)\r\n",
        );
        for (line, alias, canonical) in [
            (1, "bias", "offset"),
            (1, "gain", "scale"),
            (1, "n", "samples"),
            (2, "distribution", "kind"),
            (2, "mu", "mean"),
            (2, "sigma", "std"),
            (2, "n", "samples"),
            (3, "min", "lower"),
            (3, "max", "upper"),
            (3, "n", "samples"),
            (4, "error", "relative_error"),
            (4, "uncertainty", "std"),
        ] {
            let source_line = source.lines().nth(line).expect("uncertainty alias line");
            let start = if alias == "distribution" {
                source_line.rfind(alias)
            } else {
                source_line.find(&format!("{alias}="))
            }
            .expect("uncertainty alias key");
            let diagnostic = json!({
                "range": {
                    "start": { "line": line, "character": start },
                    "end": { "line": line, "character": start + alias.len() }
                },
                "code": "W-UNC-ARG-ALIAS",
                "message": format!(
                    "`{alias}` is a compatibility-only uncertainty argument name for `{canonical}`."
                )
            });
            let actions = code_actions_for_diagnostic(uri, source, &diagnostic);
            assert_eq!(actions.len(), 1, "missing quick fix for {alias}");
            assert_eq!(
                actions[0]["edit"]["changes"][uri][0]["range"],
                diagnostic["range"]
            );
            assert_eq!(actions[0]["edit"]["changes"][uri][0]["newText"], canonical);
        }

        let wrong_range = json!({
            "range": {
                "start": { "line": 0, "character": 2 },
                "end": { "line": 0, "character": 5 }
            },
            "code": "W-UNC-ARG-ALIAS",
            "message": "`bias` is a compatibility-only uncertainty argument name for `offset`."
        });
        assert!(code_actions_for_diagnostic(uri, source, &wrong_range).is_empty());
    }

    #[test]
    fn timeseries_fill_method_quick_fixes_require_an_explicit_policy_choice() {
        let uri = "file:///C:/workspace/fill.eng";
        let invalid_source = concat!(
            "filled = fill missing weather.wind_speed\n",
            "with {\n",
            "    method = spline\n",
            "}\n",
        );
        let invalid_diagnostic = json!({
            "range": {
                "start": { "line": 2, "character": 13 },
                "end": { "line": 2, "character": 19 }
            },
            "code": "E-TIMESERIES-FILL-METHOD",
            "message": "Unknown TimeSeries fill method `spline`."
        });
        let replacement_actions =
            code_actions_for_diagnostic(uri, invalid_source, &invalid_diagnostic);
        assert_eq!(replacement_actions.len(), 2);
        assert!(replacement_actions
            .iter()
            .all(|action| action.get("isPreferred").is_none()));
        let replacement_text = replacement_actions
            .iter()
            .flat_map(|action| action["edit"]["changes"][uri].as_array().unwrap())
            .filter_map(|edit| edit["newText"].as_str())
            .collect::<Vec<_>>();
        assert!(replacement_text.contains(&"interpolate"));
        assert!(replacement_text.contains(&"record_only"));

        let missing_source = "filled = fill missing weather.wind_speed\n";
        let missing_diagnostic = json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 48 }
            },
            "code": "W-TIMESERIES-FILL-METHOD-IMPLICIT",
            "message": "`fill missing` has no value-filling method."
        });
        let insertion_actions =
            code_actions_for_diagnostic(uri, missing_source, &missing_diagnostic);
        assert_eq!(insertion_actions.len(), 2);
        assert!(insertion_actions.iter().any(|action| {
            action["edit"]["changes"][uri]
                .as_array()
                .is_some_and(|edits| {
                    edits.iter().any(|edit| {
                        edit["newText"]
                            .as_str()
                            .is_some_and(|text| text.contains("method = interpolate"))
                    })
                })
        }));
        assert!(insertion_actions.iter().any(|action| {
            action["edit"]["changes"][uri]
                .as_array()
                .is_some_and(|edits| {
                    edits.iter().any(|edit| {
                        edit["newText"]
                            .as_str()
                            .is_some_and(|text| text.contains("method = record_only"))
                    })
                })
        }));
    }

    #[test]
    fn uncertainty_direct_compare_quick_fix_prefers_the_diagnostic_range() {
        let uri = "file:///C:/workspace/uncertainty.eng";
        let line = "note = \"\u{1f600} Q\"; validate Q < Q";
        let source = format!("{line}\n");
        let target_byte = line.rfind('Q').expect("right comparison operand");
        let target_character = utf16_len(&line[..target_byte]);
        let diagnostic = json!({
            "range": {
                "start": { "line": 0, "character": target_character },
                "end": { "line": 0, "character": target_character + 1 }
            },
            "code": "E-UNC-DIRECT-COMPARE",
            "message": "Uncertainty-valued expression `Q` cannot be compared directly."
        });

        let actions = code_actions_for_diagnostic(uri, &source, &diagnostic);
        assert_eq!(actions.len(), 1);
        let edits = actions[0]["edit"]["changes"][uri]
            .as_array()
            .expect("workspace edits");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["range"], diagnostic["range"]);
        assert_eq!(edits[0]["newText"], "mean(Q)");
    }

    #[test]
    fn simulation_and_solver_option_quick_fixes_use_precise_ranges_and_insert_missing_options() {
        let uri = "file:///C:/workspace/simulation.eng";
        let option_line = "    timestep = \"\u{1f600}\" // repeated \"\u{1f600}\"";
        let invalid_source =
            format!("sim = simulate Decay\nwith {{\n{option_line}\n    solver = fixed_step\n}}\n");
        let current_value = "\"\u{1f600}\"";
        let value_start_byte = option_line
            .find(current_value)
            .expect("timestep option value");
        let value_start = utf16_len(&option_line[..value_start_byte]);
        let invalid_diagnostic = json!({
            "range": {
                "start": { "line": 2, "character": value_start },
                "end": {
                    "line": 2,
                    "character": value_start + utf16_len(current_value)
                }
            },
            "code": "E-SIM-TIMESTEP-INVALID",
            "message": "`timestep` expects a positive duration."
        });

        let replacement_actions =
            code_actions_for_diagnostic(uri, &invalid_source, &invalid_diagnostic);
        assert_eq!(replacement_actions.len(), 1);
        let replacement_edit = &replacement_actions[0]["edit"]["changes"][uri][0];
        assert_eq!(replacement_edit["range"], invalid_diagnostic["range"]);
        assert_eq!(replacement_edit["newText"], "10 min");
        assert_eq!(replacement_actions[0]["isPreferred"], true);

        let attached_source = concat!(
            "sim = simulate Decay\r\n",
            "with {\r\n",
            "    solver = fixed_step\r\n",
            "}\r\n",
        );
        let missing_timestep = json!({
            "range": {
                "start": { "line": 0, "character": 6 },
                "end": { "line": 0, "character": 20 }
            },
            "code": "E-SIM-TIMESTEP-INVALID",
            "message": "`simulate` requires `with { timestep = <duration> }`."
        });
        let attached_actions = code_actions_for_diagnostic(uri, attached_source, &missing_timestep);
        assert_eq!(attached_actions.len(), 1);
        let attached_edit = &attached_actions[0]["edit"]["changes"][uri][0];
        assert_eq!(attached_edit["range"]["start"]["line"], 3);
        assert_eq!(attached_edit["range"]["start"]["character"], 0);
        assert_eq!(attached_edit["newText"], "    timestep = 10 min\r\n");

        let new_block_source = "result = solve component_graph\r\n";
        let missing_solver = json!({
            "range": {
                "start": { "line": 0, "character": 9 },
                "end": { "line": 0, "character": 30 }
            },
            "code": "E-SOLVE-SOLVER-UNSUPPORTED",
            "message": "`solve` requires a supported solver in the attached `with` block."
        });
        let new_block_actions = code_actions_for_diagnostic(uri, new_block_source, &missing_solver);
        assert_eq!(new_block_actions.len(), 1);
        let new_block_edit = &new_block_actions[0]["edit"]["changes"][uri][0];
        assert_eq!(new_block_edit["range"]["start"]["line"], 0);
        assert_eq!(new_block_edit["range"]["start"]["character"], 30);
        assert_eq!(
            new_block_edit["newText"],
            "\r\nwith {\r\n    solver = fixed_point\r\n}"
        );
        assert_eq!(new_block_actions[0]["isPreferred"], true);
    }

    #[test]
    fn file_uri_paths_percent_encode_reserved_and_utf8_bytes() {
        assert_eq!(
            percent_encode_file_uri_path("/tmp/a % #한.eng"),
            "/tmp/a%20%25%20%23%ED%95%9C.eng"
        );
    }

    #[test]
    fn file_uri_paths_round_trip_on_the_current_platform() {
        let path = if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\workspace\한 % #.eng")
        } else {
            PathBuf::from("/tmp/한 % #.eng")
        };
        assert_eq!(path_from_uri(&file_uri_from_path(&path)), Some(path));
    }

    #[test]
    fn stale_document_notifications_do_not_replace_latest_buffer() {
        let uri = "file:///C:/workspace/versioned.eng";
        let mut documents = Documents::new();
        documents.insert(
            uri.to_owned(),
            DocumentState::new("value = 2\n".to_owned(), Some(2)),
        );

        for method in ["textDocument/didOpen", "textDocument/didChange"] {
            let stale = json!({
                "method": method,
                "params": {
                    "textDocument": { "uri": uri, "version": 2, "text": "value := 1\n" },
                    "contentChanges": [{ "text": "value := 1\n" }]
                }
            });
            assert!(document_state_from_notification(&stale, &documents).is_none());
        }

        let save = json!({
            "method": "textDocument/didSave",
            "params": { "textDocument": { "uri": uri } }
        });
        let (_, saved) = document_state_from_notification(&save, &documents).unwrap();
        assert_eq!(saved.text, "value = 2\n");
        assert_eq!(saved.version, Some(2));

        let changed = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": "value = 3\n" }]
            }
        });
        let (_, changed) = document_state_from_notification(&changed, &documents).unwrap();
        assert_eq!(changed.text, "value = 3\n");
        assert_eq!(changed.version, Some(3));
    }

    #[test]
    fn incremental_document_changes_apply_in_order_with_utf16_ranges() {
        let uri = "file:///C:/workspace/incremental.eng";
        let source = "name = \"😀\"\r\nvalue = 2\r\n";
        let mut documents = Documents::new();
        documents.insert(
            uri.to_owned(),
            DocumentState::new(source.to_owned(), Some(1)),
        );

        let request = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [
                    {
                        "range": {
                            "start": { "line": 0, "character": 8 },
                            "end": { "line": 0, "character": 10 }
                        },
                        "text": "ok"
                    },
                    {
                        "range": {
                            "start": { "line": 1, "character": 0 },
                            "end": { "line": 1, "character": 5 }
                        },
                        "text": "result"
                    },
                    {
                        "range": {
                            "start": { "line": 1, "character": 10 },
                            "end": { "line": 1, "character": 10 }
                        },
                        "text": " kW"
                    }
                ]
            }
        });
        let (_, changed) = document_state_from_notification(&request, &documents).unwrap();
        assert_eq!(changed.text, "name = \"ok\"\r\nresult = 2 kW\r\n");
        assert_eq!(changed.version, Some(2));

        let full_then_incremental = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [
                    { "text": "x = 1\n" },
                    {
                        "range": {
                            "start": { "line": 0, "character": 4 },
                            "end": { "line": 0, "character": 5 }
                        },
                        "text": "2"
                    }
                ]
            }
        });
        let (_, changed) =
            document_state_from_notification(&full_then_incremental, &documents).unwrap();
        assert_eq!(changed.text, "x = 2\n");
    }

    #[test]
    fn invalid_incremental_ranges_do_not_replace_the_open_buffer() {
        let uri = "file:///C:/workspace/invalid-incremental.eng";
        let source = "name = \"😀\"\r\nvalue = 2\r\n";
        let mut documents = Documents::new();
        documents.insert(
            uri.to_owned(),
            DocumentState::new(source.to_owned(), Some(1)),
        );

        for range in [
            json!({
                "start": { "line": 0, "character": 9 },
                "end": { "line": 0, "character": 9 }
            }),
            json!({
                "start": { "line": 9, "character": 0 },
                "end": { "line": 9, "character": 0 }
            }),
            json!({
                "start": { "line": 1, "character": 1 },
                "end": { "line": 0, "character": 1 }
            }),
        ] {
            let request = json!({
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": 2 },
                    "contentChanges": [{ "range": range, "text": "invalid" }]
                }
            });
            assert!(document_state_from_notification(&request, &documents).is_none());
            assert_eq!(documents[uri].text, source);
            assert_eq!(documents[uri].version, Some(1));
        }
    }

    #[test]
    fn open_document_analysis_snapshot_is_reused_until_invalidated() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_document_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("analysis cache fixture should be created");
        let path = root.join("current.eng");
        let source = "value = 2 kW\n";
        std::fs::write(&path, source).expect("analysis cache source should be written");
        let path = path
            .canonicalize()
            .expect("analysis cache source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(uri.clone(), DocumentState::new(source.to_owned(), Some(1)));

        let completion_request = json!({
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 5 }
            }
        });
        let completions = completions_for_request(&completion_request, &documents);
        assert!(!completions.is_empty());
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 1, 0, true, false),
            "completion should cache the compiler report without eager snapshot projection"
        );
        let first = snapshot_for_open_documents(&path, source, &documents);
        let second = snapshot_for_open_documents(&path, source, &documents);
        assert_eq!(first, second);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (2, 1, 0, true, true),
            "snapshot requests should reuse the completion compiler report and projection"
        );

        let save = json!({
            "method": "textDocument/didSave",
            "params": { "textDocument": { "uri": uri } }
        });
        let (saved_uri, saved_state) =
            document_state_from_notification(&save, &documents).expect("saved document state");
        assert_eq!(saved_state.analysis_cache_stats(), (2, 1, 0, true, true));
        documents.insert(saved_uri, saved_state);
        let after_save = snapshot_for_open_documents(&path, source, &documents);
        assert_eq!(first, after_save);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (3, 1, 0, true, true)
        );

        invalidate_document_analyses(documents.values());
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (3, 1, 0, false, false)
        );
        let rebuilt = snapshot_for_open_documents(&path, source, &documents);
        assert_eq!(first, rebuilt);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (3, 2, 0, true, true)
        );
        std::fs::remove_dir_all(&root).expect("analysis cache fixture should be removed");
    }

    #[test]
    fn token_free_trivia_edits_reuse_analysis_with_exact_fallbacks() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_trivia_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("trivia cache fixture should be created");
        let path = root.join("current.eng");
        let initial_source = concat!(
            "value = 2 kW\n",
            "# alpha\n",
            "print \"value={value:kW}\"\n",
            "# short\n",
        );
        std::fs::write(&path, initial_source).expect("trivia cache source should be written");
        let path = path
            .canonicalize()
            .expect("trivia cache source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        let (_, initial_cached_snapshot) = documents[&uri]
            .analysis_cache_snapshot()
            .expect("initial snapshot should be cached");
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 1, 0, true, true)
        );

        let trivia_source = concat!(
            "value = 2 kW\n",
            "# bravo\n",
            "print \"value={value:kW}\"\n",
            "# a much longer trailing comment\n",
        );
        let trivia_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": trivia_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&trivia_change, &documents)
                .expect("trivia change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);

        let reused = snapshot_for_open_documents(&path, trivia_source, &documents);
        let fresh = snapshot_for_source(&path, trivia_source);
        assert_eq!(reused, fresh);
        let (cached_source, reused_cached_snapshot) = documents[&uri]
            .analysis_cache_snapshot()
            .expect("retargeted snapshot should be cached");
        assert_eq!(cached_source, trivia_source);
        assert!(!Arc::ptr_eq(
            &initial_cached_snapshot,
            &reused_cached_snapshot
        ));
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 1, true, true),
            "the report should be retargeted and the source-specific snapshot rebuilt lazily"
        );
        assert_eq!(
            snapshot_for_open_documents(&path, trivia_source, &documents),
            reused
        );
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 2, 1, true, true)
        );

        let shifted_source = concat!(
            "value = 2 kW\n",
            "# a longer middle comment\n",
            "print \"value={value:kW}\"\n",
            "# a much longer trailing comment\n",
        );
        let shifted_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": shifted_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&shifted_change, &documents)
                .expect("position-shifting change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, shifted_source, &documents),
            snapshot_for_source(&path, shifted_source)
        );
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 3, 1, true, true),
            "position shifts must fall back to a fresh compiler report"
        );

        let semantic_source = shifted_source.replacen("value = 2 kW", "value = 3 kW", 1);
        let semantic_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": semantic_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&semantic_change, &documents)
                .expect("semantic change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, &semantic_source, &documents),
            snapshot_for_source(&path, &semantic_source)
        );
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 4, 1, true, true),
            "semantic edits must not increment the trivia reuse counter"
        );

        std::fs::remove_dir_all(&root).expect("trivia cache fixture should be removed");
    }

    #[test]
    fn scalar_binding_dependency_edits_use_incremental_compiler_recheck() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_scalar_binding_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("incremental binding fixture should be created");
        let path = root.join("current.eng");
        let initial_source = "heat_rate = 2 kW\nratio = 1\n# aliases\nheat_rate_copy = heat_rate\n";
        std::fs::write(&path, initial_source)
            .expect("incremental binding source should be written");
        let path = path
            .canonicalize()
            .expect("incremental binding source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        let (_, initial_cached_snapshot) = documents[&uri]
            .analysis_cache_snapshot()
            .expect("initial snapshot should be cached");
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);

        let incremental_source =
            "heat_rate = 1800 W\nratio = 1\n# aliases\nheat_rate_copy = heat_rate\n";
        let incremental_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": incremental_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&incremental_change, &documents)
                .expect("dependency-bearing suffix change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);

        let incremental = snapshot_for_open_documents(&path, incremental_source, &documents);
        assert_eq!(incremental, snapshot_for_source(&path, incremental_source));
        let (_, incremental_cached_snapshot) = documents[&uri]
            .analysis_cache_snapshot()
            .expect("incremental snapshot should be cached");
        assert!(!Arc::ptr_eq(
            &initial_cached_snapshot,
            &incremental_cached_snapshot
        ));
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 0, true, true)
        );

        let suffix_source = "heat_rate = 1800 W\nratio = 1\n# aliases\nheat_rate_copy = ratio\n";
        let suffix_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": suffix_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&suffix_change, &documents)
                .expect("backward alias target change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, suffix_source, &documents),
            snapshot_for_source(&path, suffix_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 3, 0, true, true)
        );

        let renamed_source = "heat_rate = 1800 W\nfactor = 2\n# aliases\nscaled_ratio = factor\n";
        let renamed_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": renamed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&renamed_change, &documents)
                .expect("coordinated binding rename should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, renamed_source, &documents),
            snapshot_for_source(&path, renamed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 3);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 4, 0, true, true)
        );

        let shifted_trivia_source = concat!(
            "heat_rate = 1800 W\n",
            "factor = 2\n",
            "# aliases shifted by a longer comment\n",
            "scaled_ratio = factor\n",
        );
        let shifted_trivia_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 5 },
                "contentChanges": [{ "text": shifted_trivia_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&shifted_trivia_change, &documents)
                .expect("position-shifting trivia change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, shifted_trivia_source, &documents),
            snapshot_for_source(&path, shifted_trivia_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 4);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 5, 0, true, true),
            "a position-shifting trivia edit should recheck the following scalar suffix"
        );

        let inserted_trivia_source = concat!(
            "heat_rate = 1800 W\n",
            "factor = 2\n",
            "\n",
            "# aliases shifted by a longer comment\n",
            "scaled_ratio = factor\n",
        );
        let inserted_trivia_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 6 },
                "contentChanges": [{ "text": inserted_trivia_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&inserted_trivia_change, &documents)
                .expect("inserted trivia change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, inserted_trivia_source, &documents),
            snapshot_for_source(&path, inserted_trivia_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 5);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 6, 0, true, true),
            "inserted trivia should recheck the shifted scalar suffix"
        );

        let crlf_source = concat!(
            "heat_rate = 1800 W\r\n",
            "factor = 2\r\n",
            "\r\n",
            "# aliases shifted by a longer comment\r\n",
            "scaled_ratio = factor\r\n",
        );
        let crlf_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 7 },
                "contentChanges": [{ "text": crlf_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&crlf_change, &documents)
                .expect("line-ending change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, crlf_source, &documents),
            snapshot_for_source(&path, crlf_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 6);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 7, 0, true, true),
            "line-ending changes should recheck the affected scalar suffix"
        );

        let expanded_source = concat!(
            "heat_rate = 1800 W\r\n",
            "factor = 2\r\n",
            "\r\n",
            "# aliases shifted by a longer comment\r\n",
            "scaled_ratio = factor\r\n",
            "scaled_ratio_copy = scaled_ratio\r\n",
        );
        let expanded_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 8 },
                "contentChanges": [{ "text": expanded_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&expanded_change, &documents)
                .expect("appended scalar binding should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, expanded_source, &documents),
            snapshot_for_source(&path, expanded_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 7);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 8, 0, true, true),
            "an appended scalar binding should extend the cached semantic suffix"
        );

        let cleared_source = "# scalar bindings cleared\r\n";
        let cleared_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 9 },
                "contentChanges": [{ "text": cleared_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&cleared_change, &documents)
                .expect("cleared scalar document should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, cleared_source, &documents),
            snapshot_for_source(&path, cleared_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 8);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 9, 0, true, true),
            "removing every scalar binding should clear the cached semantic records"
        );

        let restarted_source = "# scalar bindings cleared\r\nvalue = 1\r\n";
        let restarted_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 10 },
                "contentChanges": [{ "text": restarted_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&restarted_change, &documents)
                .expect("restarted scalar document should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, restarted_source, &documents),
            snapshot_for_source(&path, restarted_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 9);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 10, 0, true, true),
            "adding a binding to a trivia-only report should rebuild the scalar suffix"
        );

        let arithmetic_source =
            concat!("# scalar bindings cleared\r\n", "ratio = (1 + 1) / 2\r\n",);
        let arithmetic_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 11 },
                "contentChanges": [{ "text": arithmetic_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&arithmetic_change, &documents)
                .expect("scalar arithmetic change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, arithmetic_source, &documents),
            snapshot_for_source(&path, arithmetic_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 10);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 11, 0, true, true),
            "pure scalar arithmetic should rebuild the cached semantic suffix"
        );

        let fallback_source = concat!("# scalar bindings cleared\r\n", "ratio = sqrt(4)\r\n",);
        let fallback_change = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 12 },
                "contentChanges": [{ "text": fallback_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&fallback_change, &documents)
                .expect("function expression change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, fallback_source, &documents),
            snapshot_for_source(&path, fallback_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 10);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 12, 0, true, true),
            "function calls must fall back to a fresh compiler report"
        );

        std::fs::remove_dir_all(&root).expect("incremental binding fixture should be removed");
    }

    #[test]
    fn scalar_suffix_edits_after_rich_prefix_use_incremental_compiler_recheck() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_rich_prefix_scalar_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("rich-prefix scalar fixture should be created");
        std::fs::write(
            root.join("system.eng"),
            r#"states ImportedEditorState {
    T_air: AbsoluteTemperature [degC]
}

inputs ImportedEditorInput {
    T_out: AbsoluteTemperature [degC]
}

system ImportedEditorStateSpace {
    state x: StateVector[ImportedEditorState] = [20 degC]
    input u: InputVector[ImportedEditorInput] = [8 degC]
    outputs y = [T_air]
    operator A: LinearOperator[ImportedEditorState -> Derivative[ImportedEditorState]] = [[-0.01 1/min]]
    operator B: LinearOperator[ImportedEditorInput -> Derivative[ImportedEditorState]] = [[0.01 1/min]]
    equation {
        der(x) eq A * x + B * u
    }
}

system ImportedRoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    parameter UA: Conductance = 150 W/K
    state T: AbsoluteTemperature = 24 degC
    input T_out: AbsoluteTemperature
    input Q_internal: HeatRate
    output T_report: AbsoluteTemperature = 24 degC
    equation {
        C * der(T) eq UA * (T_out - T) + Q_internal
    }
}
"#,
        )
        .expect("static system module should be written");
        std::fs::write(
            root.join("class.eng"),
            r#"use "system.eng"
class ImportedEnvelope {
    name: String
    conductance: Conductance [W/K] = 10 W/K
    validate {
        name != ""
    }
    method value() -> Conductance [W/K] = self.conductance
}

imported_envelope = ImportedEnvelope {
    name = "south_envelope"
    conductance = 12 W/K
}

imported_envelope_copy = imported_envelope with {
    conductance = 10 W/K
}
"#,
        )
        .expect("static class module should be written");
        std::fs::write(
            root.join("domain.eng"),
            r#"use "class.eng"
domain ImportedSignal[Axis DOF] package "example.signal" version "1.0.0" {
    across level: Ratio [1]
    through flow: Ratio [1]
    conservation sum(flow) = 0
}

component ImportedEditorController {
    port signal: ImportedSignal[Time]
    parameter gain: Ratio [1] = 0.5
    input setpoint: Ratio [1] = 1
    local_value = gain
    signal.level eq 0
}

system ImportedEditorComponentGraph {
    controller = ImportedEditorController(gain=0.75)
    connect controller.signal to controller.signal
}
"#,
        )
        .expect("static domain module should be written");
        std::fs::write(
            root.join("shared.eng"),
            r#"use "domain.eng"
schema ImportedPowerRow {
    time: DateTime index
    power: HeatRate [kW]
}
const imported_factor: Ratio [1] = 0.5
"#,
        )
        .expect("static rich module should be written");
        let path = root.join("current.eng");
        let initial_source = concat!(
            "use \"shared.eng\"\n",
            "use eng.stats\n",
            "input_file = file(\"input.csv\")\n",
            "series: TimeSeries[Time] of HeatRate [kW] = 5 kW\n",
            "process_result = run command \"cmd\"\n",
            "with {\n",
            "    cache = true\n",
            "    cache_key = [\"editor-prefix\"]\n",
            "}\n",
            "fn identity_power(value: HeatRate [kW]) -> HeatRate [kW] {\n",
            "    return value\n",
            "}\n",
            "\n",
            "print \"ready\"\n",
            "envelope_value = imported_envelope_copy.conductance\n",
            "base: HeatRate [kW] = 2 kW\n",
            "scaled = identity_power(base) * imported_factor\n",
        );
        std::fs::write(&path, initial_source).expect("rich-prefix scalar source should be written");
        let path = path
            .canonicalize()
            .expect("rich-prefix scalar source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        {
            let cache = documents[&uri]
                .analysis_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let report = &cache
                .analysis
                .as_ref()
                .expect("initial state-space import analysis")
                .report;
            assert_eq!(report.semantic_program.state_space_type_blocks.len(), 2);
            assert_eq!(report.semantic_program.state_space_vectors.len(), 3);
            assert_eq!(report.semantic_program.linear_operators.len(), 2);
            assert_eq!(report.semantic_program.class_objects.len(), 2);
            assert!(!report.semantic_program.class_objects[0]
                .span
                .is_root_source());
            assert_eq!(
                report.semantic_program.class_objects[1]
                    .source_object
                    .as_deref(),
                Some("imported_envelope")
            );
            let envelope_value = report
                .semantic_program
                .typed_bindings
                .iter()
                .find(|binding| binding.name == "envelope_value")
                .expect("root field access should resolve the imported class object");
            assert_eq!(envelope_value.semantic_type.quantity_kind, "Conductance");
            assert_eq!(envelope_value.semantic_type.display_unit, "W/K");
            assert_eq!(report.semantic_program.component_templates.len(), 1);
            assert_eq!(
                report.semantic_program.component_templates[0]
                    .local_expressions
                    .len(),
                2
            );
            assert_eq!(report.semantic_program.component_instances.len(), 1);
            assert!(!report.semantic_program.component_instances[0]
                .span
                .is_root_source());
            assert_eq!(report.semantic_program.connections.len(), 1);
            assert!(!report.semantic_program.connections[0]
                .left_span
                .is_root_source());
            assert_eq!(report.semantic_program.component_assemblies.len(), 1);
        }
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 1, 0, true, true)
        );

        let changed_source = concat!(
            "use \"shared.eng\"\n",
            "use eng.stats\n",
            "input_file = file(\"input.csv\")\n",
            "series: TimeSeries[Time] of HeatRate [kW] = 5 kW\n",
            "process_result = run command \"cmd\"\n",
            "with {\n",
            "    cache = true\n",
            "    cache_key = [\"editor-prefix\"]\n",
            "}\n",
            "fn identity_power(value: HeatRate [kW]) -> HeatRate [kW] {\n",
            "    return value\n",
            "}\n",
            "\n",
            "print \"ready\"\n",
            "envelope_value = imported_envelope_copy.value()\n",
            "base: HeatRate [W] = 1800 W\n",
            "scaled: HeatRate [W] = identity_power(base) * imported_factor + 0 W\n",
        );
        let changed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": changed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&changed_notification, &documents)
                .expect("rich-prefix scalar edit should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source(&path, changed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        {
            let cache = documents[&uri]
                .analysis_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let report = &cache
                .analysis
                .as_ref()
                .expect("reused component import analysis")
                .report;
            assert_eq!(
                report.semantic_program.component_templates[0].local_expressions[0].name,
                "local_value"
            );
            assert_eq!(
                report.semantic_program.component_assemblies[0].local_expression_count,
                2
            );
            assert_eq!(report.semantic_program.component_instances.len(), 1);
            assert_eq!(report.semantic_program.connections.len(), 1);
            assert_eq!(report.semantic_program.class_objects.len(), 2);
            assert_eq!(
                report.semantic_program.class_objects[1].fields[1].expression,
                "10 W/K"
            );
            let envelope_value = report
                .inferred_declarations
                .iter()
                .find(|declaration| declaration.name == "envelope_value")
                .expect("reused member call should retain inferred metadata");
            assert_eq!(envelope_value.expression, "imported_envelope_copy.value()");
            assert!(!report.semantic_program.connections[0]
                .right_span
                .is_root_source());
        }
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 0, true, true),
            "the unchanged recursive state-space/system/class/domain/component/schema import, module, file, axis, cache, helper, and print prefix should preserve a member-derived scalar suffix"
        );

        let fallback_source = changed_source.replace(
            "scaled: HeatRate [W] = identity_power(base) * imported_factor + 0 W\n",
            "scaled_input = input_file\n",
        );
        let fallback_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": fallback_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&fallback_notification, &documents)
                .expect("non-scalar alias edit should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, &fallback_source, &documents),
            snapshot_for_source(&path, &fallback_source)
        );
        assert_eq!(
            documents[&uri].scalar_binding_reuse_count(),
            1,
            "a non-scalar prefix alias must use a fresh compiler report"
        );
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 3, 0, true, true)
        );

        std::fs::remove_dir_all(&root).expect("rich-prefix scalar fixture should be removed");
    }

    #[test]
    fn explicit_scalar_declaration_edits_use_incremental_compiler_recheck() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_explicit_scalar_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("explicit scalar fixture should be created");
        let path = root.join("current.eng");
        let initial_source = concat!(
            "length: Length [m] = 2 m\n",
            "# explicit scalars\n",
            "scale: Ratio [1] = (1 + 1) / 2\n",
            "heat_rate: HeatRate [kW] = (2 kW + 500 W) * scale\n",
        );
        std::fs::write(&path, initial_source).expect("explicit scalar source should be written");
        let path = path
            .canonicalize()
            .expect("explicit scalar source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);

        let changed_source = concat!(
            "length: Length [m] = 2 m\n",
            "# explicit scalar inputs expanded\n",
            "scale: Ratio [1] = (3 - 1) / 2\n",
            "heat_rate: HeatRate [W] = (1800 W + 200 W) / scale\n",
        );
        let changed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": changed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&changed_notification, &documents)
                .expect("explicit scalar change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source(&path, changed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 0, true, true)
        );

        let appended_source = concat!(
            "length: Length [m] = 2 m\n",
            "# explicit scalar inputs expanded\n",
            "scale: Ratio [1] = (3 - 1) / 2\n",
            "heat_rate: HeatRate [W] = (1800 W + 200 W) / scale\n",
            "backup_heat_rate: HeatRate [W] = (heat_rate + 100 W) * scale\n",
        );
        let appended_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": appended_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&appended_notification, &documents)
                .expect("appended explicit scalar should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, appended_source, &documents),
            snapshot_for_source(&path, appended_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 3, 0, true, true)
        );

        let fallback_source = concat!(
            "length: Length [m] = 2 m\n",
            "scale: Ratio [1] = sqrt(4)\n",
            "heat_rate: HeatRate [W] = 2 kW\n",
        );
        let fallback_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": fallback_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&fallback_notification, &documents)
                .expect("explicit function expression should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, fallback_source, &documents),
            snapshot_for_source(&path, fallback_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 4, 0, true, true),
            "function calls should use a fresh compiler report"
        );

        std::fs::remove_dir_all(&root).expect("explicit scalar fixture should be removed");
    }

    #[test]
    fn mixed_scalar_declaration_edits_use_incremental_compiler_recheck() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_mixed_scalar_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mixed scalar fixture should be created");
        let path = root.join("current.eng");
        let initial_source = concat!(
            "distance = 1 m\n",
            "offset: Length [m] = distance + 2 m\n",
            "combined = offset + distance + 0 m\n",
        );
        std::fs::write(&path, initial_source).expect("mixed scalar source should be written");
        let path = path
            .canonicalize()
            .expect("mixed scalar source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);

        let changed_source = concat!(
            "distance: Length [cm] = 200 cm\n",
            "offset = distance + 3 m\n",
            "combined: Length [m] = offset + distance\n",
        );
        let changed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": changed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&changed_notification, &documents)
                .expect("mixed scalar style change should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source(&path, changed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 0, true, true)
        );

        let appended_source = concat!(
            "distance: Length [cm] = 200 cm\n",
            "offset = distance + 3 m\n",
            "combined: Length [m] = offset + distance\n",
            "combined_copy = combined\n",
        );
        let appended_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": appended_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&appended_notification, &documents)
                .expect("appended mixed scalar should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, appended_source, &documents),
            snapshot_for_source(&path, appended_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);

        let fallback_source = concat!(
            "distance: Length [cm] = 200 cm\n",
            "offset = sqrt(4)\n",
            "combined: Length [m] = distance + 1 m\n",
        );
        let fallback_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": fallback_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&fallback_notification, &documents)
                .expect("mixed scalar function expression should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, fallback_source, &documents),
            snapshot_for_source(&path, fallback_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 4, 0, true, true),
            "function calls must use a fresh compiler report"
        );

        std::fs::remove_dir_all(&root).expect("mixed scalar fixture should be removed");
    }

    #[test]
    fn scalar_const_edits_after_stdlib_import_use_incremental_compiler_recheck() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_scalar_const_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scalar const fixture should be created");
        let path = root.join("current.eng");
        let initial_source = concat!(
            "use eng.stats\n",
            "base = 2 m\n",
            "const factor: Ratio = 0.5\n",
            "adjusted: Length [m] = base * factor\n",
            "total = adjusted + 0 m\n",
        );
        std::fs::write(&path, initial_source).expect("scalar const source should be written");
        let path = path
            .canonicalize()
            .expect("scalar const source should exist");
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let _initial = snapshot_for_open_documents(&path, initial_source, &documents);
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);

        let changed_source = concat!(
            "use eng.stats\n",
            "base = 2 m\n",
            "const gain: Ratio [1] = 0.75\n",
            "adjusted: Length [cm] = base * gain\n",
            "total = adjusted + 0 m\n",
        );
        let changed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": changed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&changed_notification, &documents)
                .expect("scalar const rename should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source(&path, changed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 2, 0, true, true)
        );

        let appended_source = concat!(
            "use eng.stats\n",
            "base = 2 m\n",
            "const gain: Ratio [1] = 0.75\n",
            "adjusted: Length [cm] = base * gain\n",
            "total = adjusted + 0 m\n",
            "const reserve: Length [m] = total + 50 cm\n",
            "final = reserve + 0 m\n",
        );
        let appended_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": appended_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&appended_notification, &documents)
                .expect("appended scalar const should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, appended_source, &documents),
            snapshot_for_source(&path, appended_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);

        let fallback_source = concat!(
            "use eng.stats\n",
            "base = 2 m\n",
            "const gain: Ratio = sqrt(4)\n",
            "adjusted: Length [m] = base\n",
            "total = adjusted + 0 m\n",
        );
        let fallback_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 4 },
                "contentChanges": [{ "text": fallback_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&fallback_notification, &documents)
                .expect("scalar const function expression should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, fallback_source, &documents),
            snapshot_for_source(&path, fallback_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (0, 4, 0, true, true),
            "function calls in scalar constants must use a fresh compiler report"
        );

        std::fs::remove_dir_all(&root).expect("scalar const fixture should be removed");
    }

    #[test]
    fn scalar_edits_after_static_scalar_import_reuse_and_invalidate_with_import_changes() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_static_scalar_import_analysis_cache_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("static scalar import fixture should be created");
        let module_path = root.join("shared.eng");
        let initial_module_source = r#"const shared_length: Length [m] = 2 m
const shared_factor: Ratio [1] = 0.5
fn double_length(value: Length [m]) -> Length [m] {
    return value * 2
}
fn keep_ratio(value: Ratio [1]) -> Ratio [1] {
    return value
}
"#;
        std::fs::write(&module_path, initial_module_source)
            .expect("static scalar module should be written");
        let path = root.join("current.eng");
        let initial_source = r#"use "shared.eng"
base = double_length(double_length(shared_length)) + shared_length
const local_factor: Ratio [1] = keep_ratio(keep_ratio(0.5)) * shared_factor
adjusted: Length [m] = double_length(double_length(base * local_factor)) + shared_length
"#;
        std::fs::write(&path, initial_source).expect("importing source should be written");
        let module_path = module_path
            .canonicalize()
            .expect("static const module should exist");
        let path = path.canonicalize().expect("importing source should exist");
        let module_uri = file_uri_from_path(&module_path);
        let uri = file_uri_from_path(&path);
        let mut documents = Documents::new();
        documents.insert(
            module_uri.clone(),
            DocumentState::new(initial_module_source.to_owned(), Some(1)),
        );
        documents.insert(
            uri.clone(),
            DocumentState::new(initial_source.to_owned(), Some(1)),
        );

        let initial = snapshot_for_open_documents(&path, initial_source, &documents);
        assert_eq!(initial, snapshot_for_source(&path, initial_source));
        let base_hover = initial
            .hovers
            .iter()
            .find(|hover| hover.name == "base" && hover.is_root_source())
            .expect("an imported scalar function call should have editor hover metadata");
        assert_eq!(base_hover.quantity_kind, "Length");
        assert_eq!(base_hover.display_unit, "m");
        let hover_request = json!({
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 1 }
            }
        });
        let requested_hover = hover_for_request(&hover_request, &documents)
            .expect("the persistent editor session should serve function-call hover");
        assert_eq!(requested_hover.name, "base");
        assert_eq!(requested_hover.quantity_kind, "Length");
        assert_eq!(requested_hover.display_unit, "m");
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 0);

        let changed_source = r#"use "shared.eng"
base = double_length(double_length(shared_length)) + shared_length
const local_factor: Ratio [1] = keep_ratio(keep_ratio(0.75)) * shared_factor
adjusted: Length [cm] = double_length(double_length(base * local_factor)) + shared_length
total = double_length(double_length(adjusted)) + shared_length
"#;
        let changed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": changed_source }]
            }
        });
        let (changed_uri, changed_state) =
            document_state_from_notification(&changed_notification, &documents)
                .expect("root scalar edit should be accepted");
        documents.insert(changed_uri.clone(), changed_state);
        let affected = diagnostic_documents_after_change(&changed_uri, &documents);
        invalidate_dependent_document_analyses(&changed_uri, &affected);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source(&path, changed_source)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 2, 0, true, true)
        );

        let changed_module_source = r#"const shared_length: Length [m] = 3 m
const shared_factor: Ratio [1] = 0.6
fn double_length(value: Length [m]) -> Length [m] {
    return value * 3
}
fn keep_ratio(value: Ratio [1]) -> Ratio [1] {
    return value
}
"#;
        let module_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": module_uri, "version": 2 },
                "contentChanges": [{ "text": changed_module_source }]
            }
        });
        let (changed_module_uri, changed_module_state) =
            document_state_from_notification(&module_notification, &documents)
                .expect("imported scalar definition edit should be accepted");
        documents.insert(changed_module_uri.clone(), changed_module_state);
        let affected = diagnostic_documents_after_change(&changed_module_uri, &documents);
        assert!(affected
            .iter()
            .any(|(affected_uri, _)| affected_uri == &uri));
        invalidate_dependent_document_analyses(&changed_module_uri, &affected);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 2, 0, false, false),
            "an imported buffer edit must invalidate the dependent root report"
        );

        let overrides = import_source_overrides_from_documents(&documents);
        assert_eq!(
            snapshot_for_open_documents(&path, changed_source, &documents),
            snapshot_for_source_with_import_overrides(&path, changed_source, &overrides)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 1);
        assert_eq!(
            documents[&uri].analysis_cache_stats(),
            (1, 3, 0, true, true),
            "the dependent root must rebuild against the changed import environment"
        );

        let resumed_source = r#"use "shared.eng"
base = double_length(double_length(shared_length)) + shared_length
const local_factor: Ratio [1] = keep_ratio(keep_ratio(0.9)) * shared_factor
adjusted: Length [cm] = double_length(double_length(base * local_factor)) + shared_length
total = double_length(double_length(adjusted)) + shared_length
"#;
        let resumed_notification = json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "text": resumed_source }]
            }
        });
        let (resumed_uri, resumed_state) =
            document_state_from_notification(&resumed_notification, &documents)
                .expect("root scalar edit after import rebuild should be accepted");
        documents.insert(resumed_uri.clone(), resumed_state);
        let affected = diagnostic_documents_after_change(&resumed_uri, &documents);
        invalidate_dependent_document_analyses(&resumed_uri, &affected);
        let overrides = import_source_overrides_from_documents(&documents);
        assert_eq!(
            snapshot_for_open_documents(&path, resumed_source, &documents),
            snapshot_for_source_with_import_overrides(&path, resumed_source, &overrides)
        );
        assert_eq!(documents[&uri].scalar_binding_reuse_count(), 2);

        std::fs::remove_dir_all(&root).expect("static scalar import fixture should be removed");
    }

    #[test]
    fn changed_import_republishes_only_recursive_open_dependents() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_open_import_diagnostics_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("diagnostic fixture should be created");
        let module_path = root.join("module.eng");
        let bridge_path = root.join("bridge.eng");
        let current_path = root.join("current.eng");
        let unrelated_path = root.join("unrelated.eng");
        let sources = [
            (&module_path, "const SHARED_GAIN: Ratio = 0.9\n"),
            (&bridge_path, "use \"module.eng\"\n"),
            (&current_path, "use \"bridge.eng\"\nvalue = SHARED_GAIN\n"),
            (&unrelated_path, "value = 1\n"),
        ];
        let mut documents = Documents::new();
        for (path, source) in sources {
            std::fs::write(path, source).expect("diagnostic source should be written");
            let path = path.canonicalize().expect("diagnostic source should exist");
            documents.insert(
                file_uri_from_path(&path),
                DocumentState::new(source.to_owned(), Some(1)),
            );
        }
        let module_uri = file_uri_from_path(&module_path.canonicalize().unwrap());
        let bridge_uri = file_uri_from_path(&bridge_path.canonicalize().unwrap());
        let current_uri = file_uri_from_path(&current_path.canonicalize().unwrap());
        let unrelated_uri = file_uri_from_path(&unrelated_path.canonicalize().unwrap());

        for (uri, path) in [
            (&bridge_uri, &bridge_path),
            (&current_uri, &current_path),
            (&unrelated_uri, &unrelated_path),
        ] {
            let source = documents
                .get(uri)
                .expect("open analysis document")
                .text
                .clone();
            let first = snapshot_for_open_documents(path, &source, &documents);
            let second = snapshot_for_open_documents(path, &source, &documents);
            assert_eq!(first, second);
            assert_eq!(
                documents.get(uri).unwrap().analysis_cache_stats(),
                (1, 1, 0, true, true)
            );
        }

        let affected = diagnostic_documents_after_change(&module_uri, &documents);
        let affected_uris = affected
            .iter()
            .map(|(uri, _)| uri.as_str())
            .collect::<Vec<_>>();

        assert!(affected_uris.contains(&bridge_uri.as_str()));
        assert!(affected_uris.contains(&current_uri.as_str()));
        assert!(!affected_uris.contains(&unrelated_uri.as_str()));
        assert_eq!(affected_uris.last().copied(), Some(module_uri.as_str()));

        invalidate_dependent_document_analyses(&module_uri, &affected);
        assert_eq!(
            documents[&bridge_uri].analysis_cache_stats(),
            (1, 1, 0, false, false)
        );
        assert_eq!(
            documents[&current_uri].analysis_cache_stats(),
            (1, 1, 0, false, false)
        );
        assert_eq!(
            documents[&unrelated_uri].analysis_cache_stats(),
            (1, 1, 0, true, true)
        );

        let current_source = documents[&current_uri].text.clone();
        let unrelated_source = documents[&unrelated_uri].text.clone();
        let _ = snapshot_for_open_documents(&current_path, &current_source, &documents);
        let _ = snapshot_for_open_documents(&unrelated_path, &unrelated_source, &documents);
        assert_eq!(
            documents[&current_uri].analysis_cache_stats(),
            (1, 2, 0, true, true)
        );
        assert_eq!(
            documents[&unrelated_uri].analysis_cache_stats(),
            (2, 1, 0, true, true)
        );
        std::fs::remove_dir_all(&root).expect("diagnostic fixture should be removed");
    }

    #[test]
    fn closed_import_falls_back_to_disk_for_open_document_snapshots() {
        let root = std::env::temp_dir().join(format!(
            "eng_lsp_closed_import_snapshot_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("snapshot fixture should be created");
        let module_path = root.join("module.eng");
        let current_path = root.join("current.eng");
        let saved_module =
            "fn saved_gain(value: HeatRate [kW]) -> HeatRate [kW] {\n    return value\n}\n";
        let open_module =
            "fn open_gain(value: HeatRate [kW]) -> HeatRate [kW] {\n    return value\n}\n";
        let current_source = "use \"module.eng\"\nadjusted = open_gain(5 kW)\n";
        std::fs::write(&module_path, saved_module).expect("saved module should be written");
        std::fs::write(&current_path, current_source).expect("current source should be written");

        let module_path = module_path.canonicalize().expect("module should exist");
        let current_path = current_path
            .canonicalize()
            .expect("current source should exist");
        let module_uri = file_uri_from_path(&module_path);
        let current_uri = file_uri_from_path(&current_path);
        let mut documents = Documents::new();
        documents.insert(
            module_uri.clone(),
            DocumentState::new(open_module.to_owned(), Some(2)),
        );
        documents.insert(
            current_uri,
            DocumentState::new(current_source.to_owned(), Some(1)),
        );

        let open_snapshot = snapshot_for_open_documents(&current_path, current_source, &documents);
        assert!(!open_snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-FN-CALL-001"));

        let dependents = diagnostic_documents_after_change(&module_uri, &documents)
            .into_iter()
            .filter(|(uri, _)| uri != &module_uri)
            .collect::<Vec<_>>();
        invalidate_document_analyses(dependents.iter().map(|(_, state)| state));
        documents.remove(&module_uri);
        let disk_snapshot = snapshot_for_open_documents(&current_path, current_source, &documents);
        assert!(disk_snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-FN-CALL-001"));
        std::fs::remove_dir_all(&root).expect("snapshot fixture should be removed");
    }

    fn document_highlight_request(source: &str, line: usize, character: usize) -> Value {
        let uri = "file:///C:/workspace/highlights.eng".to_owned();
        let request = json!({
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        });
        let mut documents = Documents::new();
        documents.insert(uri, DocumentState::new(source.to_owned(), Some(1)));
        document_highlights_for_request(&request, &documents)
    }

    fn references_request(
        source: &str,
        line: usize,
        character: usize,
        include_declaration: bool,
    ) -> Value {
        let uri = "file:///C:/workspace/references.eng".to_owned();
        let request = json!({
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": include_declaration }
            }
        });
        let mut documents = Documents::new();
        documents.insert(uri, DocumentState::new(source.to_owned(), Some(1)));
        references_for_request(&request, &documents, &[])
    }

    fn prepare_rename_request(source: &str, line: usize, character: usize) -> Option<Value> {
        let uri = "file:///C:/workspace/rename.eng".to_owned();
        let request = json!({
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        });
        let mut documents = Documents::new();
        documents.insert(uri, DocumentState::new(source.to_owned(), Some(1)));
        prepare_rename_for_request(&request, &documents)
    }

    fn rename_request(
        source: &str,
        line: usize,
        character: usize,
        new_name: &str,
    ) -> Result<Value, String> {
        let uri = "file:///C:/workspace/rename.eng".to_owned();
        let request = json!({
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "newName": new_name
            }
        });
        let mut documents = Documents::new();
        documents.insert(uri, DocumentState::new(source.to_owned(), Some(1)));
        rename_for_request(&request, &documents, &[])
    }

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

    #[test]
    fn symbol_at_position_accepts_token_start_and_end_without_crossing_whitespace() {
        let source = "\u{1f600} Q_coil = integrate(Q_coil, over=Time)";
        assert_eq!(symbol_at_position(source, 0, 3).as_deref(), Some("Q_coil"));
        assert_eq!(symbol_at_position(source, 0, 9).as_deref(), Some("Q_coil"));
        assert_eq!(symbol_at_position(source, 0, 2), None);
        assert_eq!(symbol_at_position("alpha  beta", 0, 6), None);
    }

    #[test]
    fn document_highlights_follow_semantic_identifiers_not_strings_or_comments() {
        let source = "Q = 5 kW\nE = integrate Q over Time\nprint \"Q={Q}\"\n# Q is a comment\n";
        let highlights = document_highlight_request(source, 1, 14);
        let highlights = highlights
            .as_array()
            .expect("document highlights should be an array");
        assert_eq!(highlights.len(), 3);
        assert_eq!(highlights[0]["range"]["start"]["line"], 0);
        assert_eq!(highlights[0]["kind"], 3);
        assert_eq!(highlights[1]["range"]["start"]["line"], 1);
        assert_eq!(highlights[2]["range"]["start"]["line"], 2);
        assert!(highlights[1..]
            .iter()
            .all(|highlight| highlight["kind"] == 2));
    }

    #[test]
    fn references_return_current_file_locations_and_honor_declaration_context() {
        let source = "Q = 5 kW\nE = integrate Q over Time\nprint \"Q={Q}\"\n# Q is a comment\n";
        let character = source.lines().nth(1).unwrap().find('Q').unwrap();
        let with_declaration = references_request(source, 1, character, true);
        let with_declaration = with_declaration
            .as_array()
            .expect("references should be an array");
        assert_eq!(with_declaration.len(), 3);
        assert!(with_declaration
            .iter()
            .all(|location| { location["uri"] == "file:///C:/workspace/references.eng" }));
        assert_eq!(with_declaration[0]["range"]["start"]["line"], 0);

        let without_declaration = references_request(source, 1, character, false);
        let without_declaration = without_declaration
            .as_array()
            .expect("references without declarations should be an array");
        assert_eq!(without_declaration.len(), 2);
        assert!(without_declaration
            .iter()
            .all(|location| location["range"]["start"]["line"] != 0));
    }

    #[test]
    fn state_space_type_references_and_rename_follow_typed_vector_arguments() {
        let source = concat!(
            "states RoomState {\n",
            "    T_air: AbsoluteTemperature [degC]\n",
            "}\n",
            "system Fixture {\n",
            "    state x: StateVector[RoomState] = [22 degC]\n",
            "}\n",
        );
        let reference_line = source.lines().nth(4).expect("typed vector line");
        let character = reference_line.find("RoomState").expect("type argument");

        let with_declaration = references_request(source, 4, character, true);
        let with_declaration = with_declaration
            .as_array()
            .expect("state-space references should be an array");
        assert_eq!(with_declaration.len(), 2);
        assert_eq!(with_declaration[0]["range"]["start"]["line"], 0);
        assert_eq!(with_declaration[1]["range"]["start"]["line"], 4);

        let without_declaration = references_request(source, 4, character, false);
        let without_declaration = without_declaration
            .as_array()
            .expect("state-space references without declaration should be an array");
        assert_eq!(without_declaration.len(), 1);
        assert_eq!(without_declaration[0]["range"]["start"]["line"], 4);

        let prepared = prepare_rename_request(source, 4, character)
            .expect("state-space type rename preparation");
        assert_eq!(prepared["placeholder"], "RoomState");
        let edit = rename_request(source, 4, character, "ZoneState")
            .expect("state-space type rename edit");
        let edits = edit["changes"]["file:///C:/workspace/rename.eng"]
            .as_array()
            .expect("state-space rename edits");
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|edit| edit["newText"] == "ZoneState"));
    }

    #[test]
    fn references_keep_function_locals_in_their_semantic_scope() {
        let source = r#"fn first(x: Real) -> Real {
    local_value = x
    return local_value
}
fn second(x: Real) -> Real {
    local_value = x
    return local_value
}
"#;
        let references = references_request(source, 2, 12, true);
        let references = references
            .as_array()
            .expect("local references should be an array");
        assert_eq!(references.len(), 2);
        assert!(references
            .iter()
            .all(|location| location["range"]["start"]["line"].as_u64().unwrap() < 4));
    }

    #[test]
    fn document_highlights_keep_local_names_inside_their_function() {
        let source = r#"fn first(x: Real) -> Real {
    local_value = x
    return local_value
}
fn second(x: Real) -> Real {
    local_value = x
    return local_value
}
"#;
        let highlights = document_highlight_request(source, 2, 12);
        let highlights = highlights
            .as_array()
            .expect("document highlights should be an array");
        assert_eq!(highlights.len(), 2);
        assert!(highlights
            .iter()
            .all(|highlight| highlight["range"]["start"]["line"].as_u64().unwrap() < 4));
    }

    #[test]
    fn document_highlights_do_not_mix_global_and_local_bindings() {
        let source = r#"value = 5 kW
energy = integrate value over Time
fn normalize(x: Real) -> Real {
    value = x
    return value
}
"#;
        let highlights = document_highlight_request(source, 1, 19);
        let highlights = highlights
            .as_array()
            .expect("document highlights should be an array");
        assert_eq!(highlights.len(), 2);
        assert!(highlights
            .iter()
            .all(|highlight| highlight["range"]["start"]["line"].as_u64().unwrap() < 2));
    }

    #[test]
    fn document_highlights_ignore_literals_and_units() {
        let source = "Q = 5 kW\n";
        assert_eq!(document_highlight_request(source, 0, 4), json!([]));
        assert_eq!(document_highlight_request(source, 0, 7), json!([]));
    }

    #[test]
    fn semantic_rename_edits_only_current_file_symbol_occurrences() {
        let source = "Q = 5 kW\nE = integrate Q over Time\nprint \"Q={Q}\"\n# Q is a comment\n";
        let character = source.lines().nth(1).unwrap().find('Q').unwrap();
        let prepared = prepare_rename_request(source, 1, character).expect("rename preparation");
        assert_eq!(prepared["placeholder"], "Q");
        assert_eq!(prepared["range"]["start"]["line"], 1);

        let edit = rename_request(source, 1, character, "heat_rate").expect("rename edit");
        let edits = edit["changes"]["file:///C:/workspace/rename.eng"]
            .as_array()
            .expect("current-file edits");
        assert_eq!(edits.len(), 3);
        assert!(edits.iter().all(|edit| edit["newText"] == "heat_rate"));
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn semantic_rename_updates_user_function_declarations_and_calls() {
        let source = r#"fn scale(x: Real) -> Real {
    return x
}
value = scale(2)
"#;
        let character = source.lines().nth(3).unwrap().find("scale").unwrap();
        let edit = rename_request(source, 3, character, "rescale").expect("function rename edit");
        let edits = edit["changes"]["file:///C:/workspace/rename.eng"]
            .as_array()
            .expect("function edits");
        assert_eq!(edits.len(), 2);
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![0, 3]
        );
    }

    #[test]
    fn semantic_rename_keeps_local_edits_inside_their_function() {
        let source = r#"fn first(x: Real) -> Real {
    local_value = x
    return local_value
}
fn second(x: Real) -> Real {
    local_value = x
    return local_value
}
"#;
        let edit = rename_request(source, 2, 12, "first_value").expect("local rename edit");
        let edits = edit["changes"]["file:///C:/workspace/rename.eng"]
            .as_array()
            .expect("local edits");
        assert_eq!(edits.len(), 2);
        assert!(edits
            .iter()
            .all(|edit| edit["range"]["start"]["line"].as_u64().unwrap() < 4));
    }

    #[test]
    fn semantic_rename_rejects_invalid_reserved_and_conflicting_names() {
        let source = concat!(
            "left_power: HeatRate [kW] = 5 kW\n",
            "right_power: HeatRate [kW] = 3 kW\n",
            "total: HeatRate [kW] = left_power + right_power\n",
        );
        let character = source.lines().nth(2).unwrap().find("left_power").unwrap();
        let invalid = rename_request(source, 2, character, "1value").unwrap_err();
        assert!(invalid.contains("valid EngLang identifier"), "{invalid}");
        let reserved = rename_request(source, 2, character, "report").unwrap_err();
        assert!(reserved.contains("reserved"), "{reserved}");
        let conflict = rename_request(source, 2, character, "right_power").unwrap_err();
        assert!(conflict.contains("conflict"), "{conflict}");
    }

    #[test]
    fn semantic_rename_rejects_local_name_capture() {
        let source = r#"source_value = 5
fn combine(x: Real) -> Real {
    local_value = x
    return source_value + local_value
}
"#;
        let character = source
            .lines()
            .nth(3)
            .expect("return line should exist")
            .find("source_value")
            .expect("return line should use source_value");
        let conflict = rename_request(source, 3, character, "local_value").unwrap_err();
        assert!(conflict.contains("conflict"), "{conflict}");
    }

    #[test]
    fn semantic_rename_rejects_builtins_and_members() {
        let source = "Q = mean(sensor.m_dot, axis=Time)\n";
        let line = source.lines().next().unwrap();
        assert_eq!(
            prepare_rename_request(source, 0, line.find("mean").unwrap()),
            None
        );
        assert_eq!(
            prepare_rename_request(source, 0, line.find("m_dot").unwrap()),
            None
        );
    }

    #[test]
    fn semantic_rename_rejects_incomplete_semantic_occurrence_sets() {
        let source = "Q = 5 kW\nE = Q\nprint \"literal Q; value={Q}\"\n# Q\n";
        let declaration = eng_lsp::LspSemanticToken {
            line: 0,
            start: 0,
            length: 1,
            token_type: "variable".to_owned(),
            modifiers: vec!["declaration".to_owned()],
        };
        let symbol = SemanticSymbolOccurrences {
            selected: declaration.clone(),
            label: "Q".to_owned(),
            family: "variable".to_owned(),
            scope: None,
            occurrences: vec![declaration],
        };

        assert!(!semantic_symbol_is_renameable(source, &symbol));
    }

    #[test]
    fn semantic_rename_ranges_use_utf16_characters() {
        let source = "Q = 5 kW\nprint \"😀 Q={Q}\"\n";
        let line = source.lines().nth(1).unwrap();
        let byte = line.rfind('Q').unwrap();
        let character = utf16_len(&line[..byte]);
        let prepared =
            prepare_rename_request(source, 1, character).expect("UTF-16 rename preparation");
        assert_eq!(prepared["range"]["start"]["character"], character);
        assert_eq!(prepared["range"]["end"]["character"], character + 1);
    }

    #[test]
    fn workspace_walk_reports_truncation_and_unreadable_roots() {
        let root =
            std::env::temp_dir().join(format!("eng_lsp_workspace_walk_{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("prior workspace walk fixture should be removed");
        }
        std::fs::create_dir_all(&root).expect("workspace walk fixture should be created");
        std::fs::write(root.join("one.eng"), "one = 1\n")
            .expect("first workspace source should be written");
        std::fs::write(root.join("two.eng"), "two = 2\n")
            .expect("second workspace source should be written");

        let mut files = Vec::new();
        let status = collect_workspace_eng_files_with_cancellation(&root, &mut files, 1, None);
        assert_eq!(files.len(), 1);
        assert!(status.truncated);
        assert!(!status.unreadable);

        let missing = root.join("missing");
        let mut missing_files = Vec::new();
        let missing_status =
            collect_workspace_eng_files_with_cancellation(&missing, &mut missing_files, 1, None);
        assert!(missing_status.unreadable);
        std::fs::remove_dir_all(&root).expect("workspace walk fixture should be removed");
    }

    #[test]
    fn semantic_member_receiver_uses_the_caret_member_path_and_utf16_offsets() {
        let source = "\u{1f600}alpha.result + beta.result";
        let alpha = eng_lsp::LspSemanticToken {
            line: 0,
            start: 8,
            length: 6,
            token_type: "property".to_owned(),
            modifiers: Vec::new(),
        };
        let beta = eng_lsp::LspSemanticToken {
            line: 0,
            start: 22,
            length: 6,
            token_type: "property".to_owned(),
            modifiers: Vec::new(),
        };
        assert_eq!(
            semantic_member_receiver(source, &alpha).as_deref(),
            Some("alpha")
        );
        assert_eq!(
            semantic_member_receiver(source, &beta).as_deref(),
            Some("beta")
        );
    }
}
