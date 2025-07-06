use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringError(String);

impl StringError {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StringError {}

/// A parsing error with position information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The error message
    pub message: String,
    /// The span in the input where the error occurred
    pub span: Range<usize>,
    /// The line number where the error occurred (0-based)
    pub line: usize,
    /// The column number where the error occurred (0-based)
    pub column: usize,
}

impl ParseError {
    /// Create a new parse error with position information
    pub fn new(message: impl Into<String>, span: Range<usize>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            span,
            line,
            column,
        }
    }

    /// Create a parse error from a winnow error with position information
    pub fn from_winnow_error<I>(
        error: winnow::error::ErrMode<winnow::error::ContextError>,
        input: I,
        original_input: &str,
    ) -> Self
    where
        I: winnow::stream::Stream + winnow::stream::Location,
    {
        // Extract the inner error
        let inner_error = match error {
            winnow::error::ErrMode::Backtrack(e) | winnow::error::ErrMode::Cut(e) => e,
            winnow::error::ErrMode::Incomplete(_) => {
                return Self {
                    message: "Incomplete input".to_string(),
                    span: 0..1,
                    line: 0,
                    column: 0,
                };
            }
        };

        // Get position information from the input
        let current_offset = input.current_token_start();

        // Convert offset to line/column
        let (line, column) = offset_to_line_col(original_input, current_offset);

        Self {
            message: format!("Parse error: {}", inner_error),
            span: current_offset..current_offset + 1,
            line,
            column,
        }
    }

    /// Create a parse error from a winnow error with a slice  
    pub fn from_winnow_error_with_slice<I>(
        error: winnow::error::ErrMode<winnow::error::ContextError>,
        input: I,
        original_input: &str,
    ) -> Self
    where
        I: winnow::stream::Stream + winnow::stream::Location,
    {
        Self::from_winnow_error(error, input, original_input)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at line {}, column {} (offset {})",
            self.message,
            self.line + 1,
            self.column + 1,
            self.span.start
        )
    }
}

impl std::error::Error for ParseError {}

/// A wrapper type that can hold either a parse error or any other error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwParseError {
    /// A parsing error with position information
    Parse(ParseError),
    /// Some other error
    Other(String),
}

impl CwParseError {
    /// Create a parse error
    pub fn parse(
        message: impl Into<String>,
        span: Range<usize>,
        line: usize,
        column: usize,
    ) -> Self {
        Self::Parse(ParseError::new(message, span, line, column))
    }

    /// Create an other error
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        match self {
            Self::Parse(err) => &err.message,
            Self::Other(err) => err,
        }
    }

    /// Get the position information if this is a parse error
    pub fn position(&self) -> Option<(Range<usize>, usize, usize)> {
        match self {
            Self::Parse(err) => Some((err.span.clone(), err.line, err.column)),
            Self::Other(_) => None,
        }
    }
}

impl std::fmt::Display for CwParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "{}", err),
            Self::Other(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CwParseError {}

impl From<ParseError> for CwParseError {
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<String> for CwParseError {
    fn from(err: String) -> Self {
        Self::Other(err)
    }
}

impl From<&str> for CwParseError {
    fn from(err: &str) -> Self {
        Self::Other(err.to_string())
    }
}

/// Convert byte offset to line and column (0-based)
fn offset_to_line_col(content: &str, offset: usize) -> (usize, usize) {
    if offset > content.len() {
        return (0, 0);
    }

    let mut line = 0;
    let mut col = 0;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            return (line, col);
        }

        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}
