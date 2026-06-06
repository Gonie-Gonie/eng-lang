#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl SourceSpan {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLine {
    pub line: usize,
    pub start: usize,
    pub text: String,
}

pub fn source_lines(source: &str) -> Vec<SourceLine> {
    let mut lines = Vec::new();
    let mut start = 0usize;

    for (index, line) in source.lines().enumerate() {
        lines.push(SourceLine {
            line: index + 1,
            start,
            text: line.to_owned(),
        });
        start += line.len() + 1;
    }

    lines
}
