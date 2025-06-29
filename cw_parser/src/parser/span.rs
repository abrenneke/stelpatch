use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte offset from start of input (0-based)
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start position of the span
    pub start: Position,
    /// End position of the span (exclusive)
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_offsets(start_offset: usize, end_offset: usize, input: &str) -> Self {
        Self {
            start: Position::from_offset(start_offset, input),
            end: Position::from_offset(end_offset, input),
        }
    }

    /// Get the text covered by this span
    pub fn text<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start.offset..self.end.offset]
    }

    /// Check if this span contains another span
    pub fn contains(&self, other: &Span) -> bool {
        self.start.offset <= other.start.offset && other.end.offset <= self.end.offset
    }

    /// Check if this span overlaps with another span
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start.offset < other.end.offset && other.start.offset < self.end.offset
    }

    /// Get the union of two spans (smallest span containing both)
    pub fn union(&self, other: &Span) -> Self {
        Self {
            start: if self.start.offset <= other.start.offset {
                self.start
            } else {
                other.start
            },
            end: if self.end.offset >= other.end.offset {
                self.end
            } else {
                other.end
            },
        }
    }

    /// Check if a position is within this span
    pub fn contains_position(&self, pos: Position) -> bool {
        self.start.offset <= pos.offset && pos.offset < self.end.offset
    }
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub fn from_offset(offset: usize, input: &str) -> Self {
        let mut line = 1;
        let mut column = 1;

        for (i, ch) in input.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        Self {
            line,
            column,
            offset,
        }
    }

    pub fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

pub trait AstNode {
    fn span_range(&self) -> Range<usize>;

    fn span(&self, original_input: &str) -> Span {
        Span::from_offsets(
            self.span_range().start,
            self.span_range().end,
            original_input,
        )
    }

    /// Get the text content of this node from the original input
    fn text<'a>(&self, original_input: &'a str) -> &'a str {
        let range = self.span_range();
        &original_input[range]
    }
}
