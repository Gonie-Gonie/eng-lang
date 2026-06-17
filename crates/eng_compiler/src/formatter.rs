#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatResult {
    pub formatted: String,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatOptions {
    pub indent_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { indent_width: 4 }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BraceCounts {
    opens: usize,
    closes: usize,
}

pub fn format_source(source: &str) -> FormatResult {
    format_source_with_options(source, FormatOptions::default())
}

pub fn format_source_with_options(source: &str, options: FormatOptions) -> FormatResult {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut formatted = String::new();
    let mut indent_level = 0usize;
    let mut continuation_indent = false;

    if !normalized.is_empty() {
        for raw_line in normalized.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                formatted.push('\n');
                continue;
            }

            let leading_closes = leading_closing_braces(line);
            indent_level = indent_level.saturating_sub(leading_closes);
            let line_indent_level = indent_level + usize::from(continuation_indent);
            continuation_indent = false;

            formatted.push_str(&" ".repeat(line_indent_level * options.indent_width));
            formatted.push_str(line);
            formatted.push('\n');

            let counts = brace_counts(line);
            indent_level = indent_level.saturating_add(counts.opens);
            indent_level =
                indent_level.saturating_sub(counts.closes.saturating_sub(leading_closes));
            if counts.opens == 0 && counts.closes == 0 && structural_tail(line) == Some(':') {
                continuation_indent = true;
            }
        }
    }

    FormatResult {
        changed: formatted != source,
        formatted,
    }
}

fn leading_closing_braces(line: &str) -> usize {
    line.chars()
        .take_while(|character| *character == '}')
        .count()
}

fn brace_counts(line: &str) -> BraceCounts {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            break;
        }

        match character {
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }

    BraceCounts { opens, closes }
}

fn structural_tail(line: &str) -> Option<char> {
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    let mut tail = None;

    while let Some(character) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if !character.is_whitespace() {
            tail = Some(character);
        }
    }

    tail
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check_source, CheckOptions};

    #[test]
    fn formats_core_blocks_consistently() {
        let source = r#"args {
input: CsvFile = file("sensor.csv")
}

schema SensorData {
time: DateTime index
T_supply: AbsoluteTemperature [degC]
constraints {
time is monotonic
}
}

system Room {
parameter C: HeatCapacity [J/K]
state T: AbsoluteTemperature [K]
eq {
der(T) = (Q - U * (T - T_amb)) / C
}
}

report {
summarize Q by [mean]
plot Q over Time
where {
Q = 5 kW
}
with {
unit y = kW
}
}
"#;
        let formatted = format_source(source).formatted;

        assert!(formatted.contains("args {\n    input: CsvFile"));
        assert!(formatted.contains("schema SensorData {\n    time: DateTime index"));
        assert!(formatted.contains("    constraints {\n        time is monotonic\n    }"));
        assert!(formatted.contains("report {\n    summarize Q by [mean]"));
        assert!(formatted.contains("    where {\n        Q = 5 kW\n    }"));
        assert!(formatted.contains("    with {\n        unit y = kW\n    }"));
    }

    #[test]
    fn preserves_comments_and_ignores_structural_text_in_strings() {
        let source = r#"report {
plot Q over Time
with {
title = "literal { brace } and // text" // keep { comment }
}
}
"#;

        let formatted = format_source(source).formatted;

        assert!(formatted
            .contains("        title = \"literal { brace } and // text\" // keep { comment }"));
        assert!(formatted.ends_with("    }\n}\n"));
    }

    #[test]
    fn indents_colon_label_continuation_without_sticky_block_state() {
        let source = "system RoomThermal {\nequation energy_balance:\nC * der(T) eq Q\n}\n";
        let formatted = format_source(source).formatted;

        assert_eq!(
            formatted,
            "system RoomThermal {\n    equation energy_balance:\n        C * der(T) eq Q\n}\n"
        );
    }

    #[test]
    fn is_idempotent_after_first_pass() {
        let once = format_source("report {\nplot Q over Time\nwith {\ntitle = \"Q\"\n}\n}\n");
        let twice = format_source(&once.formatted);

        assert!(once.changed);
        assert!(!twice.changed);
        assert_eq!(once.formatted, twice.formatted);
    }

    #[test]
    fn keeps_valid_source_semantic_summary_stable() {
        let source = "args {\ninput: CsvFile = file(\"data/sensor.csv\")\n}\n\nQ = 5 kW\n\nreport {\nshow Q\n}\n";
        let formatted = format_source(source).formatted;
        let before = check_source("before.eng", source, &CheckOptions::default());
        let after = check_source("after.eng", &formatted, &CheckOptions::default());

        assert!(!before.has_errors());
        assert!(!after.has_errors());
        assert_eq!(before.syntax_summary, after.syntax_summary);
    }
}
