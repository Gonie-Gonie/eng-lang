use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

#[test]
fn stdio_server_round_trips_core_lsp_requests() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root()
        .join("examples/official/01_csv_plot/main.eng")
        .canonicalize()
        .expect("official example should exist");
    let source =
        std::fs::read_to_string(&source_path).expect("official example should be readable");
    let uri = file_uri(&source_path);
    let q_coil_line = source
        .lines()
        .position(|line| line.contains("Q_coil ="))
        .expect("official example should define Q_coil");
    let q_coil_hover_char = source
        .lines()
        .nth(q_coil_line)
        .unwrap()
        .find("Q_coil")
        .unwrap()
        + "Q_coil".len();
    let member_completion_char = source
        .lines()
        .nth(q_coil_line)
        .unwrap()
        .find("sensor.")
        .unwrap()
        + "sensor.".len();
    let q_coil_usage_line = source
        .lines()
        .position(|line| line.contains("integrate(Q_coil"))
        .expect("official example should use Q_coil");
    let q_coil_usage_char = source
        .lines()
        .nth(q_coil_usage_line)
        .unwrap()
        .find("Q_coil")
        .unwrap()
        + "Q_coil".len();

    let mut child = Command::new(server)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let mut stdout = child.stdout.take().expect("stdout should be piped");

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    let initialize = read_message(&mut stdout);
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["serverInfo"]["name"], "eng-lsp");
    assert_eq!(initialize["result"]["capabilities"]["hoverProvider"], true);
    assert_eq!(
        initialize["result"]["capabilities"]["definitionProvider"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["semanticTokensProvider"]["full"],
        true
    );
    assert!(
        initialize["result"]["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic token types should be advertised")
            .iter()
            .any(|token_type| token_type == "variable")
    );

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "englang",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    let published = read_message(&mut stdout);
    assert_eq!(published["method"], "textDocument/publishDiagnostics");
    assert_eq!(published["params"]["uri"], uri);
    assert!(published["params"]["diagnostics"].as_array().is_some());

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": q_coil_line, "character": q_coil_hover_char }
            }
        }),
    );
    let completion = read_message(&mut stdout);
    assert_eq!(completion["id"], 2);
    let completion_items = completion["result"]
        .as_array()
        .expect("completion result should be an array");
    assert!(completion_items
        .iter()
        .any(|item| item["label"] == "Q_coil"));
    assert!(completion_items
        .iter()
        .any(|item| item["label"] == "HeatRate"));
    assert!(completion_items.iter().any(|item| item["label"] == "kW"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": q_coil_line, "character": member_completion_char }
            }
        }),
    );
    let member_completion = read_message(&mut stdout);
    assert_eq!(member_completion["id"], 3);
    let member_items = member_completion["result"]
        .as_array()
        .expect("member completion result should be an array");
    assert!(member_items.iter().any(|item| item["label"] == "m_dot"));
    assert!(member_items.iter().any(|item| item["label"] == "T_return"));
    assert!(!member_items.iter().any(|item| item["label"] == "HeatRate"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": q_coil_line, "character": q_coil_hover_char }
            }
        }),
    );
    let hover = read_message(&mut stdout);
    assert_eq!(hover["id"], 4);
    let hover_text = hover["result"]["contents"]["value"]
        .as_str()
        .expect("hover should return markdown contents");
    assert!(hover_text.contains("Q_coil"));
    assert!(hover_text.contains("HeatRate"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": q_coil_usage_line, "character": q_coil_usage_char }
            }
        }),
    );
    let definition = read_message(&mut stdout);
    assert_eq!(definition["id"], 5);
    assert_eq!(definition["result"]["uri"], uri);
    assert_eq!(definition["result"]["range"]["start"]["line"], q_coil_line);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let semantic_tokens = read_message(&mut stdout);
    assert_eq!(semantic_tokens["id"], 10);
    assert!(
        semantic_tokens["result"]["data"]
            .as_array()
            .expect("semantic tokens should be encoded as data")
            .len()
            > 5
    );

    let class_source_path = repo_root()
        .join("examples/official/19_class_object/main.eng")
        .canonicalize()
        .expect("class object example should exist");
    let class_source = std::fs::read_to_string(&class_source_path)
        .expect("class object example should be readable");
    let class_uri = file_uri(&class_source_path);
    let building_line = class_source
        .lines()
        .position(|line| line.contains("building_name = building.name"))
        .expect("class object example should access building.name");
    let building_member_char = class_source
        .lines()
        .nth(building_line)
        .unwrap()
        .find("building.")
        .unwrap()
        + "building.".len();

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": class_uri,
                    "languageId": "englang",
                    "version": 1,
                    "text": class_source
                }
            }
        }),
    );
    let class_published = read_message(&mut stdout);
    assert_eq!(class_published["method"], "textDocument/publishDiagnostics");
    assert_eq!(class_published["params"]["uri"], class_uri);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": class_uri },
                "position": { "line": building_line, "character": building_member_char }
            }
        }),
    );
    let class_member_completion = read_message(&mut stdout);
    assert_eq!(class_member_completion["id"], 6);
    let class_member_items = class_member_completion["result"]
        .as_array()
        .expect("class member completion result should be an array");
    assert!(class_member_items.iter().any(|item| {
        item["label"] == "name"
            && item["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("required String [-] from Building"))
    }));
    assert!(class_member_items.iter().any(|item| {
        item["label"] == "summary()"
            && item["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("String [-] from Building"))
    }));

    let function_source_path = repo_root()
        .join("examples/official/07_functions_imports/main.eng")
        .canonicalize()
        .expect("functions example should exist");
    let function_source = std::fs::read_to_string(&function_source_path)
        .expect("functions example should be readable");
    let function_uri = file_uri(&function_source_path);
    let heat_loss_line = function_source
        .lines()
        .position(|line| line.contains("Q_wall = heat_loss"))
        .expect("functions example should call heat_loss");
    let heat_loss_char = function_source
        .lines()
        .nth(heat_loss_line)
        .unwrap()
        .find("heat_loss")
        .unwrap()
        + "heat_loss".len();

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": function_uri,
                    "languageId": "englang",
                    "version": 1,
                    "text": function_source
                }
            }
        }),
    );
    let function_published = read_message(&mut stdout);
    assert_eq!(
        function_published["method"],
        "textDocument/publishDiagnostics"
    );
    assert_eq!(function_published["params"]["uri"], function_uri);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": function_uri },
                "position": { "line": heat_loss_line, "character": heat_loss_char }
            }
        }),
    );
    let function_hover = read_message(&mut stdout);
    assert_eq!(function_hover["id"], 7);
    let function_hover_text = function_hover["result"]["contents"]["value"]
        .as_str()
        .expect("function hover should return markdown contents");
    assert!(function_hover_text.contains("heat_loss"));
    assert!(function_hover_text.contains("fn heat_loss"));
    assert!(function_hover_text.contains("-> HeatRate [W]"));

    let where_source_path = repo_root()
        .join("examples/official/09_command_where_with/main.eng")
        .canonicalize()
        .expect("where example should exist");
    let where_source =
        std::fs::read_to_string(&where_source_path).expect("where example should be readable");
    let where_uri = file_uri(&where_source_path);
    let where_line = where_source
        .lines()
        .position(|line| line.contains("Q_for_energy ="))
        .expect("where example should define Q_for_energy");
    let where_char = where_source
        .lines()
        .nth(where_line)
        .unwrap()
        .find("Q_for_energy")
        .unwrap()
        + "Q_for_energy".len();

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": where_uri,
                    "languageId": "englang",
                    "version": 1,
                    "text": where_source
                }
            }
        }),
    );
    let where_published = read_message(&mut stdout);
    assert_eq!(where_published["method"], "textDocument/publishDiagnostics");
    assert_eq!(where_published["params"]["uri"], where_uri);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": where_uri },
                "position": { "line": where_line, "character": where_char }
            }
        }),
    );
    let where_hover = read_message(&mut stdout);
    assert_eq!(where_hover["id"], 8);
    let where_hover_text = where_hover["result"]["contents"]["value"]
        .as_str()
        .expect("where hover should return markdown contents");
    assert!(where_hover_text.contains("where.Q_for_energy"));
    assert!(where_hover_text.contains("where local"));
    assert!(where_hover_text.contains("HeatRate"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 9);
    assert!(shutdown["result"].is_null());

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );
    drop(stdin);
    let status = child.wait().expect("eng-lsp should exit");
    assert!(status.success());
}

#[test]
fn snapshot_stdin_reads_unsaved_source() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let mut child = Command::new(server)
        .arg("--snapshot-stdin")
        .arg("unsaved_buffer.eng")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp snapshot-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"Q = 2 kW - 1\n}\n")
        .expect("source should be written to stdin");
    let output = child
        .wait_with_output()
        .expect("snapshot-stdin should exit");

    assert!(
        output.status.success(),
        "snapshot-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot: Value =
        serde_json::from_slice(&output.stdout).expect("snapshot stdout should be JSON");
    assert_eq!(snapshot["format"], "eng-lsp-snapshot-v1");
    assert!(snapshot["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-DIM-ADD-002"));
    assert!(snapshot["semantic_tokens"]["tokens"]
        .as_array()
        .expect("semantic token snapshot should contain token objects")
        .iter()
        .any(|token| token["type"] == "type"));
}

#[test]
fn completion_stdin_returns_position_aware_items() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source = r#"schema SensorData {
    time: DateTime index
    T_supply: AbsoluteTemperature [degC]
    T_return: AbsoluteTemperature [degC]
    m_dot: MassFlowRate [kg/s]
}

sensor = promote csv "missing.csv" as SensorData
Q = sensor.T
"#;
    let line = source
        .lines()
        .position(|line| line.contains("sensor.T"))
        .expect("source should contain a member completion line");
    let character = source.lines().nth(line).unwrap().len();
    let mut child = Command::new(server)
        .arg("--completion-stdin")
        .arg("completion.eng")
        .arg(line.to_string())
        .arg(character.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp completion-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    let output = child
        .wait_with_output()
        .expect("completion-stdin should exit");

    assert!(
        output.status.success(),
        "completion-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("completion stdout should be JSON");
    assert_eq!(payload["format"], "eng-lsp-snapshot-v1");
    let completions = payload["completions"]
        .as_array()
        .expect("completions should be an array");
    assert!(completions
        .iter()
        .any(|completion| completion["label"] == "T_supply"));
    assert!(completions
        .iter()
        .any(|completion| completion["label"] == "T_return"));
    assert!(!completions
        .iter()
        .any(|completion| completion["label"] == "HeatRate"));
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("eng_lsp crate should live under crates/")
        .to_path_buf()
}

fn file_uri(path: &Path) -> String {
    let mut path = path.to_string_lossy().replace('\\', "/");
    if path.as_bytes().get(1) == Some(&b':') {
        path = format!("/{path}");
    }
    format!("file://{}", path.replace(' ', "%20"))
}

fn write_message<W: Write>(writer: &mut W, value: Value) {
    let body = value.to_string();
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)
        .expect("LSP message should be writable");
    writer.flush().expect("LSP message should flush");
}

fn read_message<R: Read>(reader: &mut R) -> Value {
    let mut headers = Vec::new();
    let mut byte = [0u8; 1];
    while reader
        .read(&mut byte)
        .expect("LSP header should be readable")
        == 1
    {
        headers.push(byte[0]);
        if headers.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    assert!(!headers.is_empty(), "LSP server closed stdout");
    let headers = String::from_utf8_lossy(&headers);
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .expect("LSP message should include Content-Length");
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .expect("LSP body should be readable");
    serde_json::from_slice(&body).expect("LSP body should be JSON")
}
