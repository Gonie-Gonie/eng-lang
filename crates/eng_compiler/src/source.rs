#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub source_id: usize,
}

impl SourceSpan {
    pub const ROOT_SOURCE_ID: usize = 0;

    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self::new_in_source(Self::ROOT_SOURCE_ID, start, end, line, column)
    }

    pub fn new_in_source(
        source_id: usize,
        start: usize,
        end: usize,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            start,
            end,
            line,
            column,
            source_id,
        }
    }

    pub fn is_root_source(self) -> bool {
        self.source_id == Self::ROOT_SOURCE_ID
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

    for (index, raw_line) in source.split_inclusive('\n').enumerate() {
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        lines.push(SourceLine {
            line: index + 1,
            start,
            text: line.to_owned(),
        });
        start += raw_line.len();
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_lines_preserve_crlf_byte_offsets() {
        let source = "first\r\n😀 = 1\r\n";
        let lines = source_lines(source);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[0].text, "first");
        assert_eq!(lines[1].start, "first\r\n".len());
        assert_eq!(lines[1].text, "😀 = 1");
        assert_eq!(&source[lines[1].start..lines[1].start + "😀".len()], "😀");
    }
}
