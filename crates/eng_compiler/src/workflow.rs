#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workflow {
    pub kind: String,
    pub arg_name: Option<String>,
    pub arg_type: Option<String>,
    pub return_type: Option<String>,
    pub line: usize,
}

impl Workflow {
    pub fn top_level(line: usize) -> Self {
        Self {
            kind: "top_level".to_owned(),
            arg_name: Some("args".to_owned()),
            arg_type: Some("Args".to_owned()),
            return_type: Some("Report".to_owned()),
            line,
        }
    }

    pub fn signature(&self) -> String {
        let arg_name = self.arg_name.as_deref().unwrap_or("args");
        let arg_type = self.arg_type.as_deref().unwrap_or("Args");
        let return_type = self.return_type.as_deref().unwrap_or("Report");
        format!("top-level workflow({arg_name}: {arg_type}) -> {return_type}")
    }
}
