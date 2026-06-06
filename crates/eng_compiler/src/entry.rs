use crate::ast::ScriptDecl;
use crate::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryPoint {
    pub kind: String,
    pub name: String,
    pub arg_name: Option<String>,
    pub arg_type: Option<String>,
    pub return_type: Option<String>,
    pub line: usize,
}

impl EntryPoint {
    pub fn from_script(script: &ScriptDecl) -> Self {
        Self {
            kind: "script".to_owned(),
            name: script.name.clone(),
            arg_name: script.arg_name.clone(),
            arg_type: script.arg_type.clone(),
            return_type: script.return_type.clone(),
            line: script.span.line,
        }
    }

    pub fn signature(&self) -> String {
        let arg_name = self.arg_name.as_deref().unwrap_or("args");
        let arg_type = self.arg_type.as_deref().unwrap_or("Args");
        let return_type = self.return_type.as_deref().unwrap_or("Report");
        format!(
            "{} {}({arg_name}: {arg_type}) -> {return_type}",
            self.kind, self.name
        )
    }
}

pub fn select_entry(
    entries: &[EntryPoint],
    requested: Option<&str>,
) -> Result<EntryPoint, Diagnostic> {
    if let Some(requested) = requested {
        return entries
            .iter()
            .find(|entry| entry.name == requested)
            .cloned()
            .ok_or_else(|| {
                Diagnostic::error(
                    "E-ENTRY-NOT-FOUND-001",
                    1,
                    &format!("No entry point named `{requested}` was found."),
                    Some("Run `eng entries <file.eng>` to list available entry points."),
                )
            });
    }

    if entries.is_empty() {
        return Err(Diagnostic::error(
            "E-ENTRY-NOT-FOUND-001",
            1,
            "No entry point found.",
            Some("Add `script main(args: Args) -> Report { ... }` for file run/build."),
        ));
    }

    let main_entries = entries
        .iter()
        .filter(|entry| entry.kind == "script" && entry.name == "main")
        .collect::<Vec<_>>();
    if main_entries.len() == 1 {
        return Ok(main_entries[0].clone());
    }
    if main_entries.len() > 1 {
        return Err(Diagnostic::error(
            "E-ENTRY-MULTIPLE-001",
            main_entries[1].line,
            "Multiple `script main` entry points were found.",
            Some("Keep one default `script main` entry or run with `--entry <name>`."),
        ));
    }

    if entries.len() == 1 {
        return Ok(entries[0].clone());
    }

    Err(Diagnostic::error(
        "E-ENTRY-MULTIPLE-001",
        entries[1].line,
        "Multiple entry points were found and no `script main` default exists.",
        Some("Run with `--entry <name>` or add a default `script main(args: Args) -> Report`."),
    ))
}
