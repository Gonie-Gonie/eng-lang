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
            "id": 5,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": class_uri },
                "position": { "line": building_line, "character": building_member_char }
            }
        }),
    );
    let class_member_completion = read_message(&mut stdout);
    assert_eq!(class_member_completion["id"], 5);
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

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 6);
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
