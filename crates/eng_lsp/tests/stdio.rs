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
    let q_coil_definition_char = source
        .lines()
        .nth(q_coil_line)
        .unwrap()
        .find("Q_coil")
        .unwrap();
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
        initialize["result"]["capabilities"]["documentSymbolProvider"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["foldingRangeProvider"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["documentFormattingProvider"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["documentRangeFormattingProvider"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["codeActionProvider"]["codeActionKinds"][0],
        "quickfix"
    );
    assert_eq!(
        initialize["result"]["capabilities"]["semanticTokensProvider"]["full"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["semanticTokensProvider"]["range"],
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
    assert_eq!(
        definition["result"]["range"]["start"]["character"],
        q_coil_definition_char
    );
    assert_eq!(
        definition["result"]["range"]["end"]["character"],
        q_coil_definition_char + "Q_coil".len()
    );

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
    let semantic_token_data = semantic_tokens["result"]["data"]
        .as_array()
        .expect("semantic tokens should be encoded as data");
    assert!(semantic_token_data.len() > 5);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "textDocument/semanticTokens/range",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": q_coil_line, "character": 0 },
                    "end": {
                        "line": q_coil_line,
                        "character": source.lines().nth(q_coil_line).unwrap().len()
                    }
                }
            }
        }),
    );
    let range_semantic_tokens = read_message(&mut stdout);
    assert_eq!(range_semantic_tokens["id"], 14);
    let range_semantic_token_data = range_semantic_tokens["result"]["data"]
        .as_array()
        .expect("range semantic tokens should be encoded as data");
    assert!(!range_semantic_token_data.is_empty());
    assert_eq!(range_semantic_token_data.len() % 5, 0);
    assert!(range_semantic_token_data.len() < semantic_token_data.len());

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let document_symbols = read_message(&mut stdout);
    assert_eq!(document_symbols["id"], 11);
    let symbols = document_symbols["result"]
        .as_array()
        .expect("document symbols should be an array");
    assert!(document_symbols_contain(symbols, "SensorData"));
    assert!(document_symbols_contain(symbols, "Q_coil"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "textDocument/foldingRange",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let folding_ranges = read_message(&mut stdout);
    assert_eq!(folding_ranges["id"], 12);
    assert!(!folding_ranges["result"]
        .as_array()
        .expect("folding ranges should be an array")
        .is_empty());

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

    let thermal_source_path = repo_root()
        .join("examples/official/07_functions_imports/thermal.eng")
        .canonicalize()
        .expect("thermal import example should exist");
    let thermal_source =
        std::fs::read_to_string(&thermal_source_path).expect("thermal import should be readable");
    let thermal_uri = file_uri(&thermal_source_path);
    let heat_loss_definition_line = thermal_source
        .lines()
        .position(|line| line.contains("fn heat_loss"))
        .expect("thermal import should define heat_loss");
    let heat_loss_definition_char = thermal_source
        .lines()
        .nth(heat_loss_definition_line)
        .unwrap()
        .find("heat_loss")
        .unwrap();

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": function_uri },
                "position": { "line": heat_loss_line, "character": heat_loss_char }
            }
        }),
    );
    let function_definition = read_message(&mut stdout);
    assert_eq!(function_definition["id"], 13);
    assert_eq!(function_definition["result"]["uri"], thermal_uri);
    assert_eq!(
        function_definition["result"]["range"]["start"]["line"],
        heat_loss_definition_line
    );
    assert_eq!(
        function_definition["result"]["range"]["start"]["character"],
        heat_loss_definition_char
    );
    assert_eq!(
        function_definition["result"]["range"]["end"]["character"],
        heat_loss_definition_char + "heat_loss".len()
    );

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
fn stdio_document_cache_tracks_versions_for_diagnostics() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/versioned_diagnostics.eng");
    let uri = file_uri(&source_path);
    let bad_source = "Q := 2 kW\n";
    let fixed_source = "Q = 2 kW\n";

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
    assert_eq!(
        initialize["result"]["capabilities"]["textDocumentSync"]["openClose"],
        true
    );
    assert_eq!(
        initialize["result"]["capabilities"]["textDocumentSync"]["change"],
        1
    );
    assert_eq!(
        initialize["result"]["capabilities"]["textDocumentSync"]["save"]["includeText"],
        true
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
                    "text": bad_source
                }
            }
        }),
    );
    let opened = read_message(&mut stdout);
    assert_eq!(opened["method"], "textDocument/publishDiagnostics");
    assert_eq!(opened["params"]["uri"], uri);
    assert_eq!(opened["params"]["version"], 1);
    assert!(diagnostics_contain_code(&opened, "E-SYNTAX-DECL-001"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [
                    { "text": fixed_source }
                ]
            }
        }),
    );
    let changed = read_message(&mut stdout);
    assert_eq!(changed["method"], "textDocument/publishDiagnostics");
    assert_eq!(changed["params"]["version"], 2);
    assert!(!diagnostics_contain_code(&changed, "E-SYNTAX-DECL-001"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let saved = read_message(&mut stdout);
    assert_eq!(saved["method"], "textDocument/publishDiagnostics");
    assert_eq!(saved["params"]["version"], 2);
    assert!(!diagnostics_contain_code(&saved, "E-SYNTAX-DECL-001"));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 2);
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
fn stdio_code_actions_offer_syntax_migrations() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/code_action_migrations.eng");
    let uri = file_uri(&source_path);
    let source = r#"struct Args {
    output: String = "out"
}

Q := 2 kW

script main {
    legacy = 1
}

system RoomThermal {
    parameter C: HeatCapacity = 500 kJ/K
    state T: AbsoluteTemperature = 24 degC
    input T_out: AbsoluteTemperature
    equation {
        C * der(T) == T_out
    }
}
"#;

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
    assert_eq!(
        initialize["result"]["capabilities"]["codeActionProvider"]["codeActionKinds"][0],
        "quickfix"
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
    let diagnostics = published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .clone();
    for code in [
        "E-STRUCT-ARGS-001",
        "E-SYNTAX-DECL-001",
        "E-SCRIPT-001",
        "E-EQ-BOOL-001",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic["code"] == code),
            "diagnostics should include {code}"
        );
    }

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": source.lines().count(), "character": 0 }
                },
                "context": {
                    "diagnostics": diagnostics
                }
            }
        }),
    );
    let code_actions = read_message(&mut stdout);
    assert_eq!(code_actions["id"], 2);
    let actions = code_actions["result"]
        .as_array()
        .expect("code action result should be an array");
    assert_replacement_action(actions, &uri, "Replace struct Args with args", "args");
    assert_replacement_action(actions, &uri, "Replace := with =", "=");
    assert_replacement_action(actions, &uri, "Replace == with eq", "eq");
    assert_script_wrapper_action(actions, &uri);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 3);
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
fn stdio_code_actions_offer_linter_quick_fixes() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/code_action_linter_fixes.eng");
    let uri = file_uri(&source_path);
    let fixture_path = source_path
        .parent()
        .expect("code action source should have a parent")
        .join("data/response.json");
    std::fs::create_dir_all(fixture_path.parent().expect("fixture should have a parent"))
        .expect("fixture directory should be writable");
    std::fs::write(&fixture_path, "{\"ok\":true}\n").expect("fixture should be writable");
    let source = r#"use eng.nte
use eng.stats
use eng.system
power = 10 kW
Q_total = 10 + 2 kW
assert Q_total == 12 kW
test "golden path" {
    golden "summary.csv" matches "golden/summary.csv"
}
Q1 = 1 kW
Q2 = 2 kW
Q_series: TimeSeries[Time] of HeatRate [kW] = 1 kW
E_sum = sum(Q_series, over=Time)
E_total = integrate Q1 + Q2 over Time
E_where = integrate Q_before over Time
where {
    Q_before = Q_after
    Q_after = 2 kW
}
E_escape = integrate Q_escape over Time
where {
    Q_escape = 3 kW
}
print "escape={Q_escape: .2 kW}"
log trace "too noisy"
log "missing level"
bad_standard_text_value: HeatRate [kW] = 1 kW
write standard_text bad_standard_text_value
with {
    output = "outputs/bad_standard_text_value.txt"
}
write csv "outputs/bad.csv", bad_standard_text_value
legacy_station_id = select_first_row(stations, return_column="station_id", region=args.region)

schema SensorData {
    m_dot = 1 kg/s
}

move "a.txt" to "b.txt"
delete dir("old")

response = http get url("https://example.org/data.json")
with {
    fixture = file("data/response.json")
    expected_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
    retry = many
    timeout = never
    body_size_limit = unlimited
    cache = true
    cache_key = [now(), "demo"]
    cache_dir = dir("../outside")
    cache_ttl = forever
}
legacy_response_hash = response.hash

bad_url_response = http get url("ftp://example.org/data.json")

payload = read json file("data/response.json")
case_name = payload.ok

get_with_body = http get url("https://example.org/submit")
with {
    body = "submitted=true"
}

post_with_secret_body = http post url("https://example.org/submit")
with {
    body = secret env("API_KEY")
}

download url("https://example.org/file.csv") to file("build/raw/file.csv")
with {
    response_body_limit = 0 B
}

run command "unbound"
missing_command_result = run command
process_result = run command "cmd"
with {
    env = true
    cwd = true
    timeout = never
    retry = many
    allow_failure = sometimes
}
process_result = run command "other"

samples = sample lhs
with {
    count = 2
    seed = later
    x = uniform(0, 1)
}

bad_count_samples = sample lhs
with {
    count = 0
    seed = 42
    x = uniform(0, 1)
}

range_unit_samples = sample lhs
with {
    count = 2
    seed = 11
    load = uniform(1, 2 kW)
}

missing_seed_samples = sample random
with {
    count = 2
    x = uniform(0, 1)
}

standard_text_samples = sample lhs
with {
    count = 1
    seed = 9
    value = uniform(0, 1)
}
write standard_text standard_text_samples

Q_bad_normal = normal()
Q_bad_kind = distribution(kind=triangular, mean=5 kW, std=0.8 kW)
Q_bad_samples = normal(mean=5 kW, std=0.8 kW, samples=many)
Q_prop_source = normal(mean=5 kW, std=0.8 kW, samples=31)
validate Q_prop_source < 10 kW
Q_bad_propagation = propagate(Q_prop_source, method=monte_carlo, scale=1.0)
Q_plain_source = 5 kW
Q_bad_source = propagate(Q_plain_source, method=linear)
Q_unknown_source = propagate(Q_missing_unc, method=linear)
T_unknown_source = propagate(T_missing_unc, method=linear, scale=1 degC)
Q_missing_source = propagate(method=linear)

Q_bad_uncertainty_policy = Q_prop_source + 1 kW
with {
    uncertainty = quadratic
    samples = 0
    seed = abc
}

Q_missing_uncertainty_seed = Q_prop_source + 2 kW
with {
    uncertainty = monte_carlo
    samples = 64
}

system SimDecay {
    state T: AbsoluteTemperature = 24 degC
    equation {
        der(T) eq 0 K/s
    }
}

sim_bad = simulate SimDecay
with {
    timestep = never
    duration = forever
    solver = adaptive
    tolerance = zero
}

domain SolverScalar {
    across x: DimensionlessNumber [1]
    through balance: DimensionlessNumber [1]
    conservation sum(balance) = 0
}

component SolverLoopNode {
    port source: SolverScalar
    port target: SolverScalar
    source.x eq 0.5 * target.x
    source.balance eq 0
}

system FixedPointLoop {
    loop_node = SolverLoopNode()
    connect loop_node.source to loop_node.target
}

fixed_point_bad = solve component_graph
with {
    solver = fixed_point
    tolerance = -1
    max_iter = 0
    relaxation = 2
    initial = bad
}

unsupported_solve_solver = solve component_graph
with {
    solver = unsupported
}

model_designs = sample lhs
with {
    count = 4
    seed = 5
    cooling_cop = uniform(2.5, 5.0)
}

model_results = derive model_designs column annual_electricity = 10000 kWh - cooling_cop * 500 kWh
ml_eval_missing = evaluate(missing_model, split=model_split)
ml_direct_model = regression(Q_series, algorithm=linear)
model_bad = train regression model_results
with {
    target = annual_electricity
    features = [cooling_cop]
    test = 1.5
    seed = abc
    algorithm = tree
}

print "empty {} interpolation"
print "unknown {missing_value} interpolation"
Q_plot: HeatRate [kW] = 1 kW
bad_show = show Q_plot
bad_validate = validate Q_plot > 0 kW
bad_print_binding = print "bound"
bad_state_header = state T_bad: AbsoluteTemperature [K]
bad_return = return Q_plot
bad_unit_binding = unit y = kW
print "unterminated {Q_plot interpolation"
print "bad unit {Q_plot: .2 m} interpolation"
report {
    plot Q_plot over Time
    with {
        unit = kW
        x_unit = kW
        y_unit = kW
        confidence = sensor_std
    }
    plot Q_plot over Time
    with {
        unit y = m
    }
}
"#;

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
    assert_eq!(
        initialize["result"]["capabilities"]["codeActionProvider"]["codeActionKinds"][0],
        "quickfix"
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
    let mut diagnostics = published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .clone();
    for code in [
        "W-QTY-AMBIG-001",
        "E-DIM-ADD-002",
        "E-CMD-AMBIG-001",
        "E-ASSERT-001",
        "E-GOLDEN-002",
        "W-STATS-SUM-001",
        "W-STDLIB-MODULE-PLANNED",
        "W-STDLIB-MODULE-INTERNAL",
        "E-WHERE-FWD-001",
        "E-NAME-LOCAL-001",
        "E-PUBLIC-ANNOTATION-001",
        "E-FS-CONFIRM-001",
        "E-FS-DELETE-001",
        "E-NET-RETRY-POLICY",
        "E-NET-TIMEOUT",
        "E-NET-BODY-SIZE-LIMIT",
        "E-NET-INVALID-URL",
        "E-NET-BODY-METHOD",
        "E-NET-BODY-POLICY",
        "E-NET-HASH-MISMATCH",
        "W-NET-FIXTURE-ALIAS",
        "W-NET-RESPONSE-HASH-ALIAS",
        "E-IO-JSON-FIELD-ACCESS-001",
        "E-CACHE-KEY-NONDETERMINISTIC",
        "E-CACHE-DIR",
        "E-CACHE-TTL",
        "E-PROCESS-BINDING-001",
        "E-PROCESS-BINDING-002",
        "E-PROCESS-CMD-001",
        "E-PROCESS-ENV-001",
        "E-PROCESS-CWD-001",
        "E-PROCESS-TIMEOUT",
        "E-PROCESS-RETRY-POLICY",
        "E-PROCESS-ALLOW-FAILURE",
        "E-SAMPLING-COUNT-INVALID",
        "E-SAMPLING-SEED-INVALID",
        "E-SAMPLING-RANGE-UNIT",
        "E-WRITE-002",
        "E-WRITE-STANDARD-TEXT-001",
        "E-WRITE-STANDARD-TEXT-OUTPUT",
        "E-SIM-TIMESTEP-INVALID",
        "E-SIM-DURATION-INVALID",
        "E-SIM-TOLERANCE-INVALID",
        "E-SIM-SOLVER-UNSUPPORTED",
        "E-SOLVE-SOLVER-UNSUPPORTED",
        "E-SOLVE-TOLERANCE-INVALID",
        "E-SOLVE-MAX-ITER-INVALID",
        "E-SOLVE-RELAXATION-INVALID",
        "E-SOLVE-INITIAL-INVALID",
        "E-ML-SOURCE-001",
        "E-ML-SOURCE-002",
        "E-ML-ARGS-002",
        "E-ML-ARGS-003",
        "E-WITH-OPTION-001",
        "E-WITH-UNIT-001",
        "E-PRINT-FMT-001",
        "E-PRINT-FMT-002",
        "E-PRINT-FMT-003",
        "E-PRINT-FMT-004",
        "E-LOG-LEVEL-001",
        "E-REPORT-BINDING-001",
        "E-VALIDATE-BINDING-001",
        "E-SIDE-EFFECT-BINDING-001",
        "E-BLOCK-BINDING-001",
        "E-STATEMENT-BINDING-001",
        "E-OPTION-BINDING-001",
        "W-TABLE-LEGACY-SELECT-FIRST-ROW",
        "E-UNC-ARGS-001",
        "E-UNC-ARGS-002",
        "E-UNC-ARGS-003",
        "E-UNC-DIRECT-COMPARE",
        "E-UNC-SOURCE-001",
        "E-UNC-SOURCE-002",
        "E-WITH-UNCERTAINTY-POLICY-001",
        "E-WITH-UNCERTAINTY-SAMPLES-001",
        "E-WITH-UNCERTAINTY-SEED-001",
        "W-WITH-UNCERTAINTY-SEED-001",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic["code"] == code),
            "diagnostics should include {code}"
        );
    }
    let missing_seed_line = source
        .lines()
        .position(|line| line.starts_with("missing_seed_samples"))
        .expect("source should include missing seed sample");
    let missing_seed_line_len = source
        .lines()
        .nth(missing_seed_line)
        .expect("missing seed line should exist")
        .len();
    diagnostics.push(json!({
        "range": {
            "start": { "line": missing_seed_line, "character": 0 },
            "end": { "line": missing_seed_line, "character": missing_seed_line_len }
        },
        "severity": 1,
        "code": "E-SAMPLING-SEED-MISSING",
        "message": "repro profile requires sample `missing_seed_samples` to declare `seed`"
    }));
    let expected_sha_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("expected_sha256"))
        .expect("source should include expected_sha256");
    let expected_sha_line_len = source
        .lines()
        .nth(expected_sha_line)
        .expect("expected_sha256 line should exist")
        .len();
    diagnostics.push(json!({
        "range": {
            "start": { "line": expected_sha_line, "character": 4 },
            "end": { "line": expected_sha_line, "character": expected_sha_line_len }
        },
        "severity": 1,
        "code": "E-NET-HASH-MISMATCH",
        "message": "E-NET-HASH-MISMATCH: live HTTP `http://127.0.0.1/weather` expected SHA256 `0000000000000000000000000000000000000000000000000000000000000000` but observed `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`"
    }));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": source.lines().count(), "character": 0 }
                },
                "context": {
                    "diagnostics": diagnostics
                }
            }
        }),
    );
    let code_actions = read_message(&mut stdout);
    assert_eq!(code_actions["id"], 2);
    let actions = code_actions["result"]
        .as_array()
        .expect("code action result should be an array");
    assert_action_edit(actions, &uri, "Replace eng.nte with eng.net", "eng.net");
    let planned_stdlib_line = source
        .lines()
        .position(|line| line.trim_start() == "use eng.stats")
        .expect("source should include planned stdlib import");
    let internal_stdlib_line = source
        .lines()
        .position(|line| line.trim_start() == "use eng.system")
        .expect("source should include internal stdlib import");
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove planned stdlib module import",
        "",
        planned_stdlib_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove internal stdlib module import",
        "",
        internal_stdlib_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Annotate power as HeatRate [kW]",
        "power: HeatRate [kW] =",
    );
    assert_action_edit(actions, &uri, "Add unit kW to 10", " kW");
    assert_action_edit(actions, &uri, "Parenthesize command target", "(Q1 + Q2)");
    assert_action_edit(actions, &uri, "Replace sum with integrate", "integrate");
    assert_action_edit(actions, &uri, "Close interpolation with }", "}");
    assert_action_edit(actions, &uri, "Remove empty interpolation", "");
    assert_action_edit(actions, &uri, "Remove incompatible interpolation unit", "");
    assert_action_edit(
        actions,
        &uri,
        "Convert unresolved interpolation to literal text",
        "missing_value",
    );
    assert_action_edit(
        actions,
        &uri,
        "Move Q_after definition before first use",
        "    Q_after = 2 kW\n",
    );
    assert_action_edit(
        actions,
        &uri,
        "Promote Q_escape to top-level binding",
        "Q_escape = 3 kW\n",
    );
    assert_action_edit(
        actions,
        &uri,
        "Convert m_dot to schema column annotation",
        "    m_dot: MassFlowRate [kg/s]",
    );
    assert_action_edit(actions, &uri, "Change writer to text", "write text");
    assert_action_edit(actions, &uri, "Change write format to text", "text");
    assert_action_edit(actions, &uri, "Change write format to json", "json");
    assert_action_edit_contains(actions, &uri, "Add confirm = true", "confirm = true");
    assert_action_edit_contains(
        actions,
        &uri,
        "Add recursive = true and confirm = true",
        "recursive = true",
    );
    assert_action_edit(actions, &uri, "Disable retries: retry = 0", "0");
    assert_action_edit(actions, &uri, "Set timeout to 30 s: timeout = 30 s", "30 s");
    assert_action_edit(actions, &uri, "Set timeout to 10 s: timeout = 10 s", "10 s");
    assert_action_edit(
        actions,
        &uri,
        "Set response body limit to 10 MB: body_size_limit = 10 MB",
        "10 MB",
    );
    assert_action_edit(
        actions,
        &uri,
        "Update expected_sha256 to pinned response SHA-256",
        "\"e5f1eb4d806641698a35efe20e098efd20d7d57a9b90ee69079d5bb650920726\"",
    );
    assert_action_edit(
        actions,
        &uri,
        "Update expected_sha256 to pinned response SHA-256",
        "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
    );
    assert_action_edit(
        actions,
        &uri,
        "Rename fixture to offline_response",
        "offline_response",
    );
    assert_action_edit(
        actions,
        &uri,
        "Rename hash to response_hash",
        "response_hash",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Promote payload before field access",
        "schema PayloadSchema",
    );
    assert_action_edit(
        actions,
        &uri,
        "Promote payload before field access",
        "payload_typed.ok",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Promote payload before field access",
        "payload_typed = promote json payload as PayloadSchema",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set response body limit to 10 MB: response_body_limit = 10 MB",
        "10 MB",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set deterministic cache key: cache_key = [\"stable\", \"v1\"]",
        "[\"stable\", \"v1\"]",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set cache directory: cache_dir = dir(\"cache\")",
        "dir(\"cache\")",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set cache TTL to 1 h: cache_ttl = 1 h",
        "1 h",
    );
    assert_action_edit(
        actions,
        &uri,
        "Allow process failure: allow_failure = true",
        "true",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set process cwd: cwd = dir(\".\")",
        "dir(\".\")",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set process env: env = { NAME = \"value\" }",
        "{ NAME = \"value\" }",
    );
    assert_action_edit(actions, &uri, "Set sample count: count = 1", "1");
    assert_action_edit(actions, &uri, "Set sample seed: seed = 42", "42");
    assert_action_edit_contains(actions, &uri, "Add sample seed: seed = 42", "seed = 42");
    assert_action_edit(actions, &uri, "Add unit kW to sample lower endpoint", " kW");
    assert_action_edit_contains(
        actions,
        &uri,
        "Add standard_text output path",
        "output = join(args.output, \"standard_weather_file.txt\")",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set uncertainty policy: uncertainty = linear",
        "linear",
    );
    assert_action_edit(actions, &uri, "Set uncertainty samples: samples = 64", "64");
    assert_action_edit(actions, &uri, "Set uncertainty seed: seed = 7", "7");
    assert_action_edit_contains(actions, &uri, "Add uncertainty seed: seed = 7", "seed = 7");
    assert_action_edit(
        actions,
        &uri,
        "Set simulation timestep: timestep = 10 min",
        "10 min",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set simulation duration: duration = 30 min",
        "30 min",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set simulation tolerance: tolerance = 0.0001",
        "0.0001",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set simulation solver: solver = fixed_step",
        "fixed_step",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set solve solver: solver = fixed_point",
        "fixed_point",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set solver tolerance: tolerance = 0.0001",
        "0.0001",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set solver max iterations: max_iter = 50",
        "50",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set solver relaxation: relaxation = 0.5",
        "0.5",
    );
    assert_action_edit(actions, &uri, "Set solver initial value: initial = 1", "1");
    assert_action_edit(actions, &uri, "Set model test split: test = 0.25", "0.25");
    assert_action_edit(actions, &uri, "Set model seed: seed = 7", "7");
    assert_action_edit_contains(
        actions,
        &uri,
        "Define ML model source missing_model",
        "missing_model = regression(split, algorithm=linear)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Define ML split source model_split",
        "model_split = train_test_split(Q_ml_series, target=Q_ml_series, features=[feature_1], test=0.25, seed=7)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Create ML split from Q_series",
        "split = train_test_split(Q_series, target=Q_series, features=[feature_1], test=0.25, seed=7)",
    );
    assert_action_edit(actions, &uri, "Create ML split from Q_series", "split");
    assert_no_action_title_or_edit_text(actions, "test_fraction");
    assert_no_action_title_or_edit_text(actions, "layers =");
    assert_action_edit(
        actions,
        &uri,
        "Set regression algorithm: algorithm = linear",
        "linear",
    );
    assert_action_edit(
        actions,
        &uri,
        "Replace uncertainty call with normal(mean=5 kW, std=0.8 kW, samples=31)",
        "normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    assert_action_edit(actions, &uri, "Set distribution kind to normal", "normal");
    assert_action_edit(actions, &uri, "Set uncertainty samples to 31", "31");
    assert_action_edit(actions, &uri, "Set uncertainty method to linear", "linear");
    assert_action_edit(
        actions,
        &uri,
        "Compare mean(Q_prop_source) instead",
        "mean(Q_prop_source)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Replace select_first_row with filter + require_one",
        "legacy_station_id_rows = filter stations",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Define uncertainty source Q_missing_unc",
        "Q_missing_unc = normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    assert_action_edit(
        actions,
        &uri,
        "Convert Q_plain_source to measured uncertainty source",
        "measured(5 kW, std=0.8 kW)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Define uncertainty source T_missing_unc",
        "T_missing_unc = normal(mean=20 degC, std=0.8 K, samples=31)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Add uncertainty source Q_source_unc",
        "Q_source_unc = normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    let unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("unit ="))
        .expect("source should include unit option");
    let x_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("x_unit ="))
        .expect("source should include x_unit option");
    let y_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("y_unit ="))
        .expect("source should include y_unit option");
    let bad_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("unit y = m"))
        .expect("source should include incompatible unit option");
    let bad_log_level_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("log trace"))
        .expect("source should include unsupported log level");
    let bad_url_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("bad_url_response"))
        .expect("source should include invalid URL response");
    let get_with_body_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("get_with_body"))
        .expect("source should include GET with request body");
    let secret_body_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("body = secret env"))
        .expect("source should include secret request body option");
    let missing_log_level_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("log \"missing level\""))
        .expect("source should include missing log level");
    let unbound_process_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("run command \"unbound\""))
        .expect("source should include unbound process command");
    let duplicate_process_line = source
        .lines()
        .position(|line| {
            line.trim_start()
                .starts_with("process_result = run command \"other\"")
        })
        .expect("source should include duplicate process binding");
    let missing_command_line = source
        .lines()
        .position(|line| {
            line.trim_start()
                .starts_with("missing_command_result = run command")
        })
        .expect("source should include missing command process");
    let top_level_assert_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("assert Q_total"))
        .expect("source should include top-level assert");
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot y-axis option: unit y =",
        "unit y",
        unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot x-axis option: unit x =",
        "unit x",
        x_unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot y-axis option: unit y =",
        "unit y",
        y_unit_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Use confidence band option: confidence_band =",
        "confidence_band",
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove incompatible display unit option",
        "",
        bad_unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Set log level to info",
        "info",
        bad_log_level_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Set log level to info",
        "info ",
        missing_log_level_line,
    );
    assert_statement_unbind_actions(actions, &uri, source);
    assert_action_edit_at_line(
        actions,
        &uri,
        "Replace URL with https://example.org",
        "\"https://example.org\"",
        bad_url_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Change HTTP method to post",
        "post",
        get_with_body_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Replace request body with string literal: body = \"{}\"",
        "\"{}\"",
        secret_body_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Bind process result",
        "result = ",
        unbound_process_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Rename process result to process_result_2",
        "process_result_2",
        duplicate_process_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Add process command string",
        " \"tool\"",
        missing_command_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Wrap assertion in test block",
        "test \"assertion\" {\n    assert Q_total == 12 kW\n}\n",
        top_level_assert_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Wrap golden expected path with file(...)",
        "file(\"golden/summary.csv\")",
    );

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 3);
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
fn code_actions_stdin_returns_linter_quick_fixes_for_unsaved_source() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/code_actions_stdin.eng");
    let uri = file_uri(&source_path);
    let source = r#"use eng.stats
use eng.system
power = 10 kW
Q_total = 10 + 2 kW
assert Q_total == 12 kW
test "golden path" {
    golden "summary.csv" matches "golden/summary.csv"
}
Q1 = 1 kW
Q2 = 2 kW
Q_series: TimeSeries[Time] of HeatRate [kW] = 1 kW
E_sum = sum(Q_series, over=Time)
E_total = integrate Q1 + Q2 over Time
E_where = integrate Q_before over Time
where {
    Q_before = Q_after
    Q_after = 2 kW
}
E_escape = integrate Q_escape over Time
where {
    Q_escape = 3 kW
}
print "escape={Q_escape: .2 kW}"
log trace "too noisy"
log "missing level"
bad_standard_text_value: HeatRate [kW] = 1 kW
write standard_text bad_standard_text_value
with {
    output = "outputs/bad_standard_text_value.txt"
}
write csv "outputs/bad.csv", bad_standard_text_value
legacy_station_id = select_first_row(stations, return_column="station_id", region=args.region)

schema SensorData {
    m_dot = 1 kg/s
}

bad_url_response = http get url("ftp://example.org/data.json")

get_with_body = http get url("https://example.org/submit")
with {
    body = "submitted=true"
}

post_with_secret_body = http post url("https://example.org/submit")
with {
    body = secret env("API_KEY")
}

run command "unbound"
missing_command_result = run command
process_result = run command "cmd"
with {
    env = true
    cwd = true
    timeout = never
    retry = many
    allow_failure = sometimes
}
process_result = run command "other"

samples = sample lhs
with {
    count = 2
    seed = later
    x = uniform(0, 1)
}

bad_count_samples = sample lhs
with {
    count = 0
    seed = 42
    x = uniform(0, 1)
}

standard_text_samples = sample lhs
with {
    count = 1
    seed = 9
    value = uniform(0, 1)
}
write standard_text standard_text_samples

Q_bad_normal = normal()
Q_bad_kind = distribution(kind=triangular, mean=5 kW, std=0.8 kW)
Q_bad_samples = normal(mean=5 kW, std=0.8 kW, samples=many)
Q_prop_source = normal(mean=5 kW, std=0.8 kW, samples=31)
validate Q_prop_source < 10 kW
Q_bad_propagation = propagate(Q_prop_source, method=monte_carlo, scale=1.0)
Q_plain_source = 5 kW
Q_bad_source = propagate(Q_plain_source, method=linear)
Q_unknown_source = propagate(Q_missing_unc, method=linear)
T_unknown_source = propagate(T_missing_unc, method=linear, scale=1 degC)
Q_missing_source = propagate(method=linear)

Q_bad_uncertainty_policy = Q_prop_source + 1 kW
with {
    uncertainty = quadratic
    samples = 0
    seed = abc
}

Q_missing_uncertainty_seed = Q_prop_source + 2 kW
with {
    uncertainty = monte_carlo
    samples = 64
}

print "empty {} interpolation"
print "unknown {missing_value} interpolation"
Q_plot: HeatRate [kW] = 1 kW
bad_show = show Q_plot
bad_validate = validate Q_plot > 0 kW
bad_print_binding = print "bound"
bad_state_header = state T_bad: AbsoluteTemperature [K]
bad_return = return Q_plot
bad_unit_binding = unit y = kW
print "unterminated {Q_plot interpolation"
print "bad unit {Q_plot: .2 m} interpolation"
report {
    plot Q_plot over Time
    with {
        unit = kW
        x_unit = kW
        y_unit = kW
        confidence = sensor_std
    }
    plot Q_plot over Time
    with {
        unit y = m
    }
}
"#;
    let mut child = Command::new(server)
        .arg("--code-actions-stdin")
        .arg(&source_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp code-actions-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    let output = child
        .wait_with_output()
        .expect("code-actions-stdin should exit");

    assert!(
        output.status.success(),
        "code-actions-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("code-actions stdout should be JSON");
    assert_eq!(payload["format"], "eng-lsp-snapshot-v1");
    assert_eq!(payload["uri"], uri);
    let actions = payload["actions"]
        .as_array()
        .expect("actions should be an array");
    let planned_stdlib_line = source
        .lines()
        .position(|line| line.trim_start() == "use eng.stats")
        .expect("source should include planned stdlib import");
    let internal_stdlib_line = source
        .lines()
        .position(|line| line.trim_start() == "use eng.system")
        .expect("source should include internal stdlib import");
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove planned stdlib module import",
        "",
        planned_stdlib_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove internal stdlib module import",
        "",
        internal_stdlib_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Annotate power as HeatRate [kW]",
        "power: HeatRate [kW] =",
    );
    assert_action_edit(actions, &uri, "Add unit kW to 10", " kW");
    assert_action_edit(actions, &uri, "Parenthesize command target", "(Q1 + Q2)");
    assert_action_edit(actions, &uri, "Replace sum with integrate", "integrate");
    assert_action_edit(actions, &uri, "Close interpolation with }", "}");
    assert_action_edit(actions, &uri, "Remove empty interpolation", "");
    assert_action_edit(actions, &uri, "Remove incompatible interpolation unit", "");
    assert_action_edit(
        actions,
        &uri,
        "Convert unresolved interpolation to literal text",
        "missing_value",
    );
    assert_action_edit(
        actions,
        &uri,
        "Move Q_after definition before first use",
        "    Q_after = 2 kW\n",
    );
    assert_action_edit(
        actions,
        &uri,
        "Promote Q_escape to top-level binding",
        "Q_escape = 3 kW\n",
    );
    assert_action_edit(
        actions,
        &uri,
        "Convert m_dot to schema column annotation",
        "    m_dot: MassFlowRate [kg/s]",
    );
    assert_action_edit(actions, &uri, "Change writer to text", "write text");
    assert_action_edit(actions, &uri, "Change write format to text", "text");
    assert_action_edit(actions, &uri, "Change write format to json", "json");
    assert_action_edit(
        actions,
        &uri,
        "Replace URL with https://example.org",
        "\"https://example.org\"",
    );
    assert_action_edit(actions, &uri, "Change HTTP method to post", "post");
    assert_action_edit(
        actions,
        &uri,
        "Replace request body with string literal: body = \"{}\"",
        "\"{}\"",
    );
    assert_action_edit(actions, &uri, "Set timeout to 10 s: timeout = 10 s", "10 s");
    assert_action_edit(actions, &uri, "Disable retries: retry = 0", "0");
    assert_action_edit(
        actions,
        &uri,
        "Allow process failure: allow_failure = true",
        "true",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set process cwd: cwd = dir(\".\")",
        "dir(\".\")",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set process env: env = { NAME = \"value\" }",
        "{ NAME = \"value\" }",
    );
    assert_action_edit(actions, &uri, "Set sample count: count = 1", "1");
    assert_action_edit(actions, &uri, "Set sample seed: seed = 42", "42");
    assert_action_edit_contains(
        actions,
        &uri,
        "Add standard_text output path",
        "output = join(args.output, \"standard_weather_file.txt\")",
    );
    assert_action_edit(
        actions,
        &uri,
        "Set uncertainty policy: uncertainty = linear",
        "linear",
    );
    assert_action_edit(actions, &uri, "Set uncertainty samples: samples = 64", "64");
    assert_action_edit(actions, &uri, "Set uncertainty seed: seed = 7", "7");
    assert_action_edit_contains(actions, &uri, "Add uncertainty seed: seed = 7", "seed = 7");
    assert_action_edit(
        actions,
        &uri,
        "Replace uncertainty call with normal(mean=5 kW, std=0.8 kW, samples=31)",
        "normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    assert_action_edit(actions, &uri, "Set distribution kind to normal", "normal");
    assert_action_edit(actions, &uri, "Set uncertainty samples to 31", "31");
    assert_action_edit(actions, &uri, "Set uncertainty method to linear", "linear");
    assert_action_edit(
        actions,
        &uri,
        "Compare mean(Q_prop_source) instead",
        "mean(Q_prop_source)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Replace select_first_row with filter + require_one",
        "legacy_station_id_rows = filter stations",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Define uncertainty source Q_missing_unc",
        "Q_missing_unc = normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    assert_action_edit(
        actions,
        &uri,
        "Convert Q_plain_source to measured uncertainty source",
        "measured(5 kW, std=0.8 kW)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Define uncertainty source T_missing_unc",
        "T_missing_unc = normal(mean=20 degC, std=0.8 K, samples=31)",
    );
    assert_action_edit_contains(
        actions,
        &uri,
        "Add uncertainty source Q_source_unc",
        "Q_source_unc = normal(mean=5 kW, std=0.8 kW, samples=31)",
    );
    let unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("unit ="))
        .expect("source should include unit option");
    let x_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("x_unit ="))
        .expect("source should include x_unit option");
    let y_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("y_unit ="))
        .expect("source should include y_unit option");
    let bad_unit_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("unit y = m"))
        .expect("source should include incompatible unit option");
    let bad_log_level_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("log trace"))
        .expect("source should include unsupported log level");
    let bad_url_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("bad_url_response"))
        .expect("source should include invalid URL response");
    let missing_log_level_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("log \"missing level\""))
        .expect("source should include missing log level");
    let unbound_process_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("run command \"unbound\""))
        .expect("source should include unbound process command");
    let duplicate_process_line = source
        .lines()
        .position(|line| {
            line.trim_start()
                .starts_with("process_result = run command \"other\"")
        })
        .expect("source should include duplicate process binding");
    let missing_command_line = source
        .lines()
        .position(|line| {
            line.trim_start()
                .starts_with("missing_command_result = run command")
        })
        .expect("source should include missing command process");
    let top_level_assert_line = source
        .lines()
        .position(|line| line.trim_start().starts_with("assert Q_total"))
        .expect("source should include top-level assert");
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot y-axis option: unit y =",
        "unit y",
        unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot x-axis option: unit x =",
        "unit x",
        x_unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Use plot y-axis option: unit y =",
        "unit y",
        y_unit_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Use confidence band option: confidence_band =",
        "confidence_band",
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Remove incompatible display unit option",
        "",
        bad_unit_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Set log level to info",
        "info",
        bad_log_level_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Set log level to info",
        "info ",
        missing_log_level_line,
    );
    assert_statement_unbind_actions(actions, &uri, source);
    assert_action_edit_at_line(
        actions,
        &uri,
        "Replace URL with https://example.org",
        "\"https://example.org\"",
        bad_url_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Bind process result",
        "result = ",
        unbound_process_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Rename process result to process_result_2",
        "process_result_2",
        duplicate_process_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Add process command string",
        " \"tool\"",
        missing_command_line,
    );
    assert_action_edit_at_line(
        actions,
        &uri,
        "Wrap assertion in test block",
        "test \"assertion\" {\n    assert Q_total == 12 kW\n}\n",
        top_level_assert_line,
    );
    assert_action_edit(
        actions,
        &uri,
        "Wrap golden expected path with file(...)",
        "file(\"golden/summary.csv\")",
    );
}

#[test]
fn stdio_formatting_formats_unsaved_document() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/formatting.eng");
    let uri = file_uri(&source_path);
    let source = "report {\nplot Q over Time\nwith {\ntitle = \"Q\"\n}\n}\n";
    let expected = "report {\n    plot Q over Time\n    with {\n        title = \"Q\"\n    }\n}\n";

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
    assert_eq!(
        initialize["result"]["capabilities"]["documentFormattingProvider"],
        true
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

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/formatting",
            "params": {
                "textDocument": { "uri": uri },
                "options": { "tabSize": 4, "insertSpaces": true }
            }
        }),
    );
    let formatting = read_message(&mut stdout);
    assert_eq!(formatting["id"], 2);
    let edits = formatting["result"]
        .as_array()
        .expect("formatting result should be an array");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["newText"], expected);
    assert_eq!(edits[0]["range"]["start"]["line"], 0);
    assert_eq!(edits[0]["range"]["start"]["character"], 0);
    assert_eq!(
        edits[0]["range"]["end"]["line"].as_u64(),
        Some((source.split('\n').count() - 1) as u64)
    );

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/rangeFormatting",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 4, "character": 1 }
                },
                "options": { "tabSize": 4, "insertSpaces": true }
            }
        }),
    );
    let range_formatting = read_message(&mut stdout);
    assert_eq!(range_formatting["id"], 3);
    let range_edits = range_formatting["result"]
        .as_array()
        .expect("range formatting result should be an array");
    assert_eq!(range_edits.len(), 1);
    assert_eq!(
        range_edits[0]["newText"],
        "    plot Q over Time\n    with {\n        title = \"Q\"\n    }"
    );
    assert_eq!(range_edits[0]["range"]["start"]["line"], 1);
    assert_eq!(range_edits[0]["range"]["start"]["character"], 0);
    assert_eq!(range_edits[0]["range"]["end"]["line"], 4);
    assert_eq!(range_edits[0]["range"]["end"]["character"], 1);

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 4);
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
    assert!(snapshot["document_symbols"]
        .as_array()
        .expect("snapshot should contain document symbols")
        .iter()
        .any(|symbol| symbol["name"] == "Q"));
    assert!(snapshot["folding_ranges"]
        .as_array()
        .expect("snapshot should contain folding ranges")
        .is_empty());
}

#[test]
fn snapshot_stdin_marks_sqlite_readback_tokens_as_db_boundary() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source = r#"schema PersistedRun {
    case_id: String
    status: String
}

db = open sqlite file("outputs/results.sqlite")
persisted_runs = read sqlite db.table("runs") as PersistedRun
"#;
    let mut child = Command::new(server)
        .arg("--snapshot-stdin")
        .arg("sqlite_readback_tokens.eng")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp snapshot-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
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
    let tokens = snapshot["semantic_tokens"]["tokens"]
        .as_array()
        .expect("semantic token snapshot should contain token objects");
    let read_line = source
        .lines()
        .position(|line| line.contains("read sqlite"))
        .expect("source should contain a sqlite readback line");
    let line = source.lines().nth(read_line).unwrap();
    let read_start = line.find("read").unwrap();
    let sqlite_start = line.find("sqlite").unwrap();
    let table_start = line.find("table").unwrap();
    let as_start = line.find("as PersistedRun").unwrap();
    let schema_start = as_start + "as ".len();

    assert!(
        semantic_token_has_modifiers(
            tokens,
            read_line,
            read_start,
            4,
            "keyword",
            &["db", "external"]
        ),
        "read keyword should carry db/external semantic modifiers"
    );
    assert!(
        semantic_token_has_modifiers(
            tokens,
            read_line,
            sqlite_start,
            6,
            "keyword",
            &["db", "external"]
        ),
        "sqlite keyword should carry db/external semantic modifiers"
    );
    assert!(
        semantic_token_has_modifiers(
            tokens,
            read_line,
            table_start,
            5,
            "method",
            &["db", "external"]
        ),
        "table call should carry db/external semantic modifiers"
    );
    assert!(
        semantic_token_has_modifiers(
            tokens,
            read_line,
            as_start,
            2,
            "keyword",
            &["db", "external"]
        ),
        "as keyword should carry db/external semantic modifiers in sqlite reads"
    );
    assert!(
        semantic_token_has_modifiers(
            tokens,
            read_line,
            schema_start,
            "PersistedRun".len(),
            "class",
            &[]
        ),
        "sqlite readback schema should be a class token"
    );
}

#[test]
fn snapshot_stdin_reports_write_text_interpolation_diagnostics() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source = "Q = 2 kW\nwrite text \"summary.txt\", \"Q={Q: .2 m} missing={missing_value}\"\n";
    let mut child = Command::new(server)
        .arg("--snapshot-stdin")
        .arg("unsaved_write_text.eng")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp snapshot-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
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
    let diagnostics = snapshot["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array");
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-WRITE-FMT-003"));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-WRITE-FMT-004"));
}

#[test]
fn format_stdin_formats_unsaved_source() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source = "report {\nplot Q over Time\nwith {\ntitle = \"Q\"\n}\n}\n";
    let expected = "report {\n    plot Q over Time\n    with {\n        title = \"Q\"\n    }\n}\n";
    let mut child = Command::new(server)
        .arg("--format-stdin")
        .arg("unsaved_buffer.eng")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp format-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    let output = child.wait_with_output().expect("format-stdin should exit");

    assert!(
        output.status.success(),
        "format-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("format stdout should be JSON");
    assert_eq!(payload["format"], "eng-lsp-snapshot-v1");
    assert_eq!(payload["changed"], true);
    assert_eq!(payload["formatted"], expected);
}

#[test]
fn definition_stdin_follows_static_imports() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root()
        .join("examples/official/07_functions_imports/main.eng")
        .canonicalize()
        .expect("function example source should exist");
    let source = std::fs::read_to_string(&source_path).expect("source should be readable");
    let heat_loss_line = source
        .lines()
        .position(|line| line.contains("Q_wall = heat_loss"))
        .expect("source should call heat_loss");
    let heat_loss_char = source
        .lines()
        .nth(heat_loss_line)
        .expect("source line should exist")
        .find("heat_loss")
        .expect("source line should contain heat_loss")
        + "heat_loss".len();

    let mut child = Command::new(server)
        .arg("--definition-stdin")
        .arg(&source_path)
        .arg(heat_loss_line.to_string())
        .arg(heat_loss_char.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp definition-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    let output = child
        .wait_with_output()
        .expect("definition-stdin should exit");

    assert!(
        output.status.success(),
        "definition-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let definition: Value =
        serde_json::from_slice(&output.stdout).expect("definition stdout should be JSON");
    let thermal_path = repo_root()
        .join("examples/official/07_functions_imports/thermal.eng")
        .canonicalize()
        .expect("imported source should exist");
    let thermal_uri = file_uri(&thermal_path);
    let thermal_source =
        std::fs::read_to_string(&thermal_path).expect("imported source should be readable");
    let heat_loss_definition_line = thermal_source
        .lines()
        .position(|line| line.contains("fn heat_loss"))
        .expect("imported source should define heat_loss");
    let heat_loss_definition_char = thermal_source
        .lines()
        .nth(heat_loss_definition_line)
        .expect("definition line should exist")
        .find("heat_loss")
        .expect("definition line should contain heat_loss");

    assert_eq!(definition["uri"], thermal_uri);
    let definition_uri = definition["uri"]
        .as_str()
        .expect("definition URI should be a string");
    assert!(!definition_uri.contains("/?/"));
    assert_eq!(
        definition["range"]["start"]["line"],
        heat_loss_definition_line
    );
    assert_eq!(
        definition["range"]["start"]["character"],
        heat_loss_definition_char
    );
    assert_eq!(
        definition["range"]["end"]["character"],
        heat_loss_definition_char + "heat_loss".len()
    );
}

#[test]
fn definition_stdin_follows_stdlib_modules() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let source_path = repo_root().join("build/editor-tests/stdlib_module_definition.eng");
    let source = "use eng.net\n";
    let module_char = source
        .lines()
        .next()
        .expect("source line should exist")
        .find("net")
        .expect("source should import eng.net")
        + 1;

    let mut child = Command::new(server)
        .arg("--definition-stdin")
        .arg(&source_path)
        .arg("0")
        .arg(module_char.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("eng-lsp definition-stdin should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(source.as_bytes())
        .expect("source should be written to stdin");
    let output = child
        .wait_with_output()
        .expect("definition-stdin should exit");

    assert!(
        output.status.success(),
        "definition-stdin failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let definition: Value =
        serde_json::from_slice(&output.stdout).expect("definition stdout should be JSON");
    let module_path = repo_root()
        .join("stdlib/eng/net.eng")
        .canonicalize()
        .expect("stdlib module source should exist");
    let module_uri = file_uri(&module_path);
    let module_source =
        std::fs::read_to_string(&module_path).expect("stdlib module should be readable");
    let module_line = module_source
        .lines()
        .position(|line| line.contains("module: eng.net"))
        .expect("stdlib module should declare its module name");
    let module_char = module_source
        .lines()
        .nth(module_line)
        .expect("module line should exist")
        .find("eng.net")
        .expect("module line should contain eng.net");

    assert_eq!(definition["uri"], module_uri);
    assert_eq!(definition["range"]["start"]["line"], module_line);
    assert_eq!(definition["range"]["start"]["character"], module_char);
    assert_eq!(
        definition["range"]["end"]["character"],
        module_char + "eng.net".len()
    );
}

#[test]
fn stdio_workspace_symbol_searches_workspace_roots() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let workspace_root = repo_root().join("build/editor-tests/workspace_symbols");
    std::fs::create_dir_all(&workspace_root).expect("workspace root should be writable");
    let source_path = workspace_root.join("symbols.eng");
    std::fs::write(
        &source_path,
        "schema WorkspaceThing {\n    value: Float\n}\n\nworkspace_value = 1\n",
    )
    .expect("workspace symbol source should be writable");
    let root_uri = file_uri(
        &workspace_root
            .canonicalize()
            .expect("workspace root should exist"),
    );
    let source_uri = file_uri(
        &source_path
            .canonicalize()
            .expect("workspace source should exist"),
    );

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
            "params": {
                "rootUri": root_uri
            }
        }),
    );
    let initialize = read_message(&mut stdout);
    assert_eq!(initialize["id"], 1);
    assert_eq!(
        initialize["result"]["capabilities"]["workspaceSymbolProvider"],
        true
    );

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workspace/symbol",
            "params": {
                "query": "WorkspaceThing"
            }
        }),
    );
    let workspace_symbols = read_message(&mut stdout);
    assert_eq!(workspace_symbols["id"], 2);
    let symbols = workspace_symbols["result"]
        .as_array()
        .expect("workspace symbols should be an array");
    assert!(symbols.iter().any(|symbol| {
        symbol["name"] == "WorkspaceThing" && symbol["location"]["uri"] == source_uri
    }));

    write_message(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown"
        }),
    );
    let shutdown = read_message(&mut stdout);
    assert_eq!(shutdown["id"], 3);
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
fn workspace_symbols_cli_searches_workspace_root() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let workspace_root = repo_root().join("build/editor-tests/workspace_symbols_cli");
    std::fs::create_dir_all(&workspace_root).expect("workspace root should be writable");
    let source_path = workspace_root.join("bridge.eng");
    std::fs::write(
        &source_path,
        "schema WorkspaceBridgeThing {\n    value: Float\n}\n\nbridge_value = 1\n",
    )
    .expect("workspace symbol source should be writable");
    let source_uri = file_uri(
        &source_path
            .canonicalize()
            .expect("workspace source should exist"),
    );

    let output = Command::new(server)
        .arg("--workspace-symbols")
        .arg(&workspace_root)
        .arg("WorkspaceBridgeThing")
        .output()
        .expect("eng-lsp workspace symbol CLI should run");

    assert!(
        output.status.success(),
        "workspace-symbols failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value =
        serde_json::from_slice(&output.stdout).expect("workspace-symbols stdout should be JSON");
    assert_eq!(payload["format"], "eng-lsp-snapshot-v1");
    let symbols = payload["symbols"]
        .as_array()
        .expect("workspace symbols should be an array");
    assert!(symbols.iter().any(|symbol| {
        symbol["name"] == "WorkspaceBridgeThing" && symbol["location"]["uri"] == source_uri
    }));
}

#[test]
fn editor_metadata_cli_exports_editor_contract() {
    let server = env!("CARGO_BIN_EXE_eng-lsp");
    let output = Command::new(server)
        .arg("--editor-metadata")
        .output()
        .expect("eng-lsp editor metadata should run");

    assert!(
        output.status.success(),
        "editor metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("editor metadata stdout should be JSON");
    assert_eq!(metadata["format"], "eng-lsp-editor-metadata-v2");
    assert!(metadata["semantic_token_legend"]["token_types"]
        .as_array()
        .expect("token types should be an array")
        .iter()
        .any(|token_type| token_type == "keyword"));
    assert!(metadata["semantic_token_legend"]["token_modifiers"]
        .as_array()
        .expect("token modifiers should be an array")
        .iter()
        .any(|modifier| modifier == "workflowStep"));
    let workflow_options = metadata["syntax_catalog"]["workflow_options"]
        .as_array()
        .expect("workflow options should be an array");
    assert!(workflow_options
        .iter()
        .any(|option| option["label"] == "offline_response"));
    assert!(workflow_options
        .iter()
        .any(|option| option["label"] == "unit y"));
    assert!(metadata["syntax_catalog"]["units"]
        .as_array()
        .expect("units should be an array")
        .iter()
        .any(|unit| unit["label"] == "kW"));
    let completions = metadata["completion_items"]
        .as_array()
        .expect("completion items should be an array");
    assert_eq!(
        metadata["completion_items_count"].as_u64(),
        Some(completions.len() as u64)
    );
    assert!(metadata.get("completion_seed").is_none());
    assert!(metadata.get("completion_seed_count").is_none());
    for label in ["records", "promote json records", "read json", "eng.table"] {
        assert!(
            completions
                .iter()
                .any(|completion| completion["label"] == label),
            "editor metadata should include {label}"
        );
    }
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

fn diagnostics_contain_code(message: &Value, code: &str) -> bool {
    message["params"]["diagnostics"]
        .as_array()
        .is_some_and(|diagnostics| {
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic["code"] == code)
        })
}
fn document_symbols_contain(symbols: &[Value], name: &str) -> bool {
    symbols.iter().any(|symbol| {
        symbol["name"] == name
            || symbol["children"]
                .as_array()
                .is_some_and(|children| document_symbols_contain(children, name))
    })
}

fn semantic_token_has_modifiers(
    tokens: &[Value],
    line: usize,
    start: usize,
    length: usize,
    token_type: &str,
    modifiers: &[&str],
) -> bool {
    tokens.iter().any(|token| {
        token["line"].as_u64() == Some(line as u64)
            && token["start"].as_u64() == Some(start as u64)
            && token["length"].as_u64() == Some(length as u64)
            && token["type"].as_str() == Some(token_type)
            && token["modifiers"].as_array().is_some_and(|actual| {
                modifiers
                    .iter()
                    .all(|modifier| actual.iter().any(|value| value.as_str() == Some(*modifier)))
            })
    })
}

fn assert_replacement_action(actions: &[Value], uri: &str, title: &str, new_text: &str) {
    let action = actions
        .iter()
        .find(|action| action["title"] == title)
        .unwrap_or_else(|| panic!("code actions should include {title}"));
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["isPreferred"], true);
    let edits = action["edit"]["changes"][uri]
        .as_array()
        .unwrap_or_else(|| panic!("code action {title} should edit {uri}"));
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["newText"], new_text);
    assert!(
        edits[0]["range"]["start"]["line"].is_number(),
        "code action {title} should include a start line"
    );
    assert!(
        edits[0]["range"]["end"]["character"].is_number(),
        "code action {title} should include an end character"
    );
}

fn assert_action_edit(actions: &[Value], uri: &str, title: &str, new_text: &str) {
    let action = actions
        .iter()
        .find(|action| {
            action["title"] == title
                && action["edit"]["changes"][uri]
                    .as_array()
                    .is_some_and(|edits| edits.iter().any(|edit| edit["newText"] == new_text))
        })
        .unwrap_or_else(|| panic!("code actions should include {title} editing to {new_text}"));
    assert_eq!(action["kind"], "quickfix");
}

fn assert_statement_unbind_actions(actions: &[Value], uri: &str, source: &str) {
    for prefix in [
        "bad_show =",
        "bad_validate =",
        "bad_print_binding =",
        "bad_state_header =",
        "bad_return =",
        "bad_unit_binding =",
    ] {
        let line = source
            .lines()
            .position(|line| line.trim_start().starts_with(prefix))
            .unwrap_or_else(|| panic!("source should include {prefix}"));
        assert_action_edit_at_line(actions, uri, "Remove invalid binding prefix", "", line);
    }
}

fn assert_action_edit_at_line(
    actions: &[Value],
    uri: &str,
    title: &str,
    new_text: &str,
    line: usize,
) {
    let action = actions
        .iter()
        .find(|action| {
            action["title"] == title
                && action["edit"]["changes"][uri]
                    .as_array()
                    .is_some_and(|edits| {
                        edits.iter().any(|edit| {
                            edit["newText"] == new_text
                                && edit["range"]["start"]["line"]
                                    .as_u64()
                                    .is_some_and(|actual| actual == line as u64)
                        })
                    })
        })
        .unwrap_or_else(|| {
            panic!("code actions should include {title} editing to {new_text} on line {line}")
        });
    assert_eq!(action["kind"], "quickfix");
}

fn assert_no_action_title_or_edit_text(actions: &[Value], text: &str) {
    for action in actions {
        assert!(
            !action["title"]
                .as_str()
                .is_some_and(|title| title.contains(text)),
            "code action title should not expose `{text}`: {action:?}"
        );
        let exposes_edit_text = action["edit"]["changes"]
            .as_object()
            .into_iter()
            .flat_map(|changes| changes.values())
            .filter_map(|edits| edits.as_array())
            .flat_map(|edits| edits.iter())
            .filter_map(|edit| edit["newText"].as_str())
            .any(|new_text| new_text.contains(text));
        assert!(
            !exposes_edit_text,
            "code action edit should not expose `{text}`: {action:?}"
        );
    }
}

fn assert_action_edit_contains(actions: &[Value], uri: &str, title: &str, text: &str) {
    let action = actions
        .iter()
        .find(|action| {
            action["title"] == title
                && action["edit"]["changes"][uri]
                    .as_array()
                    .is_some_and(|edits| {
                        edits.iter().any(|edit| {
                            edit["newText"]
                                .as_str()
                                .is_some_and(|new_text| new_text.contains(text))
                        })
                    })
        })
        .unwrap_or_else(|| panic!("code actions should include {title} containing {text}"));
    assert_eq!(action["kind"], "quickfix");
}

fn assert_script_wrapper_action(actions: &[Value], uri: &str) {
    let title = "Promote script body to top-level workflow";
    let action = actions
        .iter()
        .find(|action| action["title"] == title)
        .unwrap_or_else(|| panic!("code actions should include {title}"));
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["isPreferred"], true);
    let edits = action["edit"]["changes"][uri]
        .as_array()
        .unwrap_or_else(|| panic!("code action {title} should edit {uri}"));
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|edit| edit["newText"] == ""));
    assert_eq!(edits[0]["range"]["start"]["line"], 8);
    assert_eq!(edits[1]["range"]["start"]["line"], 6);
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
