#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentBehaviorCall {
    pub behavior_kind: String,
    pub arguments: Vec<String>,
}

pub const BEHAVIOR_STATUS_NOT_DECLARED: &str = "not_declared";
pub const BEHAVIOR_STATUS_DECLARED: &str = "declared_not_executed";
pub const BEHAVIOR_STATUS_EXECUTED: &str = "executed_in_behavior_graph";
pub const BEHAVIOR_GRAPH_EXECUTED: &str = "behavior_graph_executed";
pub const BEHAVIOR_GRAPH_NOT_EXECUTED: &str = "behavior_graph_not_executed";
pub const BEHAVIOR_SOLUTION_NOT_EXECUTED: &str = "not_solved_behavior_graph_not_executed";
pub const BEHAVIOR_VARIABLE_NOT_EVALUATED: &str = "behavior_variable_not_evaluated";
pub const BEHAVIOR_RELATIONSHIP_RESOLVED: &str = "relationship_resolved";
pub const BEHAVIOR_RELATIONSHIP_EXECUTED: &str = "relationship_evaluated_in_behavior_graph";
pub const BEHAVIOR_CONTRACT_RESOLVED: &str = "contract_resolved";
pub const BEHAVIOR_JACOBIAN_POLICY: &str = "finite_difference_on_execution";
pub const BEHAVIOR_PROFILE_POLICY: &str = "safe_repro_policy_on_execution";
pub const BEHAVIOR_RUNTIME_DIAGNOSTICS_UNAVAILABLE: &str = "runtime_diagnostics_not_available";
pub const BEHAVIOR_RUNTIME_DIAGNOSTICS_AVAILABLE: &str = "runtime_diagnostics_available";
pub const BEHAVIOR_IDENTITY_CONTRACT: &str = "typed_identity_contract";
pub const BEHAVIOR_IDENTITY_RUNTIME: &str = "typed_from_runtime_input";

pub fn component_behavior_calls(expression: &str) -> Vec<ComponentBehaviorCall> {
    let mut calls = Vec::new();
    push_behavior_call(&mut calls, expression, "delay", &["delay"]);
    push_behavior_call(
        &mut calls,
        expression,
        "predictor",
        &["predictor", "predict"],
    );
    push_behavior_call(&mut calls, expression, "external", &["external", "adapter"]);
    calls
}

fn push_behavior_call(
    calls: &mut Vec<ComponentBehaviorCall>,
    expression: &str,
    behavior_kind: &str,
    call_names: &[&str],
) {
    if let Some(arguments) = first_call_arguments(expression, call_names) {
        calls.push(ComponentBehaviorCall {
            behavior_kind: behavior_kind.to_owned(),
            arguments,
        });
    }
}

fn first_call_arguments(expression: &str, call_names: &[&str]) -> Option<Vec<String>> {
    let lowered = expression.to_ascii_lowercase();
    let open_index = call_names
        .iter()
        .filter_map(|call_name| exact_call_open_index(&lowered, call_name))
        .min()?;
    let mut depth = 0i32;
    let mut close_index = None;
    for (index, character) in expression[open_index..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_index = Some(open_index + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_index = close_index?;
    Some(
        split_arguments(&expression[open_index + 1..close_index])
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect(),
    )
}

fn exact_call_open_index(expression: &str, call_name: &str) -> Option<usize> {
    for (start, _) in expression.match_indices(call_name) {
        if start > 0 && is_identifier_byte(expression.as_bytes()[start - 1]) {
            continue;
        }
        let mut cursor = start + call_name.len();
        if expression
            .as_bytes()
            .get(cursor)
            .is_some_and(|byte| is_identifier_byte(*byte))
        {
            continue;
        }
        while expression
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        if expression.as_bytes().get(cursor) == Some(&b'(') {
            return Some(cursor);
        }
    }
    None
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn split_arguments(arguments: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(arguments[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(arguments[start..].trim().to_owned());
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_calls_require_exact_names_and_preserve_nested_arguments() {
        let calls = component_behavior_calls(
            "mydelay(source, 1 s) + delay (max(source, fallback), 5 s) \
             + predictor(delay(source, 1 s)) + adapter(source)",
        );

        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].behavior_kind, "delay");
        assert_eq!(calls[0].arguments, ["max(source, fallback)", "5 s"]);
        assert_eq!(calls[1].behavior_kind, "predictor");
        assert_eq!(calls[1].arguments, ["delay(source, 1 s)"]);
        assert_eq!(calls[2].behavior_kind, "external");
        assert_eq!(calls[2].arguments, ["source"]);
    }

    #[test]
    fn unrelated_call_name_suffixes_do_not_create_behavior_nodes() {
        assert!(component_behavior_calls(
            "mydelay(source, 1 s) + predictor_value(source) + externalized(source)"
        )
        .is_empty());
    }
}
