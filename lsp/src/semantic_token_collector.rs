use cw_parser::{
    AstColor, AstConditionalBlock, AstExpression, AstMaths, AstModule, AstNode, AstNumber,
    AstOperator, AstString, AstVisitor,
};
use std::sync::Arc;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

/// Semantic token types supported by our LSP server
/// The order here determines the index used in semantic token data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[allow(dead_code)]
pub enum CwSemanticTokenType {
    Comment = 0,
    String = 1,
    Number = 2,
    Keyword = 3,
    Operator = 4,
    Property = 5,
    Variable = 6,
    Color = 7,       // For color values like rgb { 1.0 0.5 0.2 }
    Math = 8,        // For math expressions like @[x + 1]
    Conditional = 9, // For conditional blocks like [[PARAM_NAME] ...]
}

impl CwSemanticTokenType {
    /// Get all token types in order for capability registration
    pub fn all_types() -> Vec<SemanticTokenType> {
        vec![
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::new("color"), // Custom type for color values
            SemanticTokenType::new("math"),  // Custom type for math expressions
            SemanticTokenType::new("conditional"), // Custom type for conditional blocks
        ]
    }

    /// Convert to the integer index for semantic token data
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Convert to the LSP SemanticTokenType constant
    #[allow(dead_code)]
    pub fn as_lsp_type(self) -> SemanticTokenType {
        match self {
            Self::Comment => SemanticTokenType::COMMENT,
            Self::String => SemanticTokenType::STRING,
            Self::Number => SemanticTokenType::NUMBER,
            Self::Keyword => SemanticTokenType::KEYWORD,
            Self::Operator => SemanticTokenType::OPERATOR,
            Self::Property => SemanticTokenType::PROPERTY,
            Self::Variable => SemanticTokenType::VARIABLE,
            Self::Color => SemanticTokenType::new("color"),
            Self::Math => SemanticTokenType::new("math"),
            Self::Conditional => SemanticTokenType::new("conditional"),
        }
    }
}

/// Visitor that collects semantic tokens from the AST
pub struct SemanticTokenCollector {
    tokens: Vec<SemanticToken>,
    original_input: Arc<str>,
}

impl SemanticTokenCollector {
    pub fn new(input: Arc<str>) -> Self {
        Self {
            tokens: Vec::new(),
            original_input: input,
        }
    }

    fn add_token(&mut self, node: &dyn AstNode, token_type: u32) {
        let span = node.span(&self.original_input);

        // Convert to LSP semantic token format
        let semantic_token = SemanticToken {
            delta_line: span.start.line as u32 - 1, // Convert to 0-based
            delta_start: span.start.column as u32 - 1, // Convert to 0-based
            length: (span.end.offset - span.start.offset) as u32,
            token_type,
            token_modifiers_bitset: 0,
        };

        self.tokens.push(semantic_token);
    }

    pub fn build_tokens(mut self) -> Vec<SemanticToken> {
        // Sort tokens by position (line, then column)
        self.tokens.sort_by(|a, b| {
            a.delta_line
                .cmp(&b.delta_line)
                .then_with(|| a.delta_start.cmp(&b.delta_start))
        });

        // Convert to relative positions as required by LSP
        let mut result = Vec::new();
        let mut last_line = 0;
        let mut last_start = 0;

        for token in self.tokens {
            let delta_line = token.delta_line - last_line;
            let delta_start = if delta_line == 0 {
                token.delta_start - last_start
            } else {
                token.delta_start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            });

            last_line = token.delta_line;
            last_start = token.delta_start;
        }

        result
    }
}

impl<'a, 'ast> AstVisitor<'a, 'ast> for SemanticTokenCollector
where
    'a: 'ast,
{
    fn visit_string(&mut self, node: &AstString<'a>) -> () {
        self.add_token(node, CwSemanticTokenType::String.as_u32()); // STRING
    }

    fn visit_number(&mut self, node: &AstNumber<'a>) -> () {
        self.add_token(node, CwSemanticTokenType::Number.as_u32()); // NUMBER
    }

    fn visit_operator(&mut self, node: &AstOperator<'a>) -> () {
        self.add_token(node, CwSemanticTokenType::Operator.as_u32()); // OPERATOR
    }

    fn visit_expression(&mut self, node: &AstExpression<'a>) -> () {
        // Property keys are marked as PROPERTY
        self.add_token(&node.key, CwSemanticTokenType::Property.as_u32()); // PROPERTY
        self.visit_operator(&node.operator);
        self.visit_value(&node.value);
    }

    fn visit_color(&mut self, node: &AstColor<'a>) -> () {
        // Color type keyword (rgb/hsv) as custom COLOR type
        self.add_token(&node.color_type, CwSemanticTokenType::Color.as_u32());
        // Color components as numbers
        self.visit_number(&node.r);
        self.visit_number(&node.g);
        self.visit_number(&node.b);
        if let Some(a) = &node.a {
            self.visit_number(a);
        }
    }

    fn visit_maths(&mut self, node: &AstMaths<'a>) -> () {
        // Math expressions like @[x + 1] as custom MATH type
        self.add_token(&node.value, CwSemanticTokenType::Math.as_u32());
    }

    fn visit_conditional_block(&mut self, node: &AstConditionalBlock<'a>) -> () {
        // Conditional blocks like [[PARAM_NAME] ...] as custom CONDITIONAL type
        self.add_token(node, CwSemanticTokenType::Conditional.as_u32());
        // Also visit the key inside the conditional
        self.visit_string(&node.key);
        // Visit items inside the conditional block
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }
}

/// Generate semantic tokens for the given content
pub async fn generate_semantic_tokens(content: &str) -> Vec<SemanticToken> {
    // Parse the content using cw_parser
    let mut module = AstModule::new();

    // Parse the input first
    if let Err(error) = module.parse_input(content) {
        // Log the parsing error for debugging
        eprintln!("Parse error in semantic tokens: {}", error);

        // Try to provide basic tokens even if full parsing fails
        // This gives users some syntax highlighting even when there are errors
        return generate_basic_tokens(content).await;
    }

    // Create a visitor to collect semantic tokens
    let input_arc: Arc<str> = content.into();
    let mut collector = SemanticTokenCollector::new(input_arc);

    // Visit the parsed module
    collector.visit_module(&module);
    collector.build_tokens()
}

/// Generate basic semantic tokens when full parsing fails
/// This provides minimal syntax highlighting for common patterns
async fn generate_basic_tokens(content: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();

    // Simple regex-based tokenization for basic syntax highlighting
    // This is a fallback when the full parser fails
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        // Match quoted strings
        if let Some(string_tokens) = match_quoted_strings(line, line_num) {
            tokens.extend(string_tokens);
        }

        // Match numbers
        if let Some(number_tokens) = match_numbers(line, line_num) {
            tokens.extend(number_tokens);
        }

        // Match comments
        if let Some(comment_tokens) = match_comments(line, line_num) {
            tokens.extend(comment_tokens);
        }
    }

    tokens
}

/// Match quoted strings in a line
fn match_quoted_strings(line: &str, line_num: usize) -> Option<Vec<SemanticToken>> {
    let mut tokens = Vec::new();

    // Simple manual string matching
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '"' {
            // Find the end quote
            let mut end = start + 1;
            while let Some((pos, c)) = chars.next() {
                end = pos + c.len_utf8();
                if c == '"' {
                    break;
                }
            }

            tokens.push(SemanticToken {
                delta_line: line_num as u32,
                delta_start: start as u32,
                length: (end - start) as u32,
                token_type: CwSemanticTokenType::String.as_u32(),
                token_modifiers_bitset: 0,
            });
        }
    }

    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

/// Match numbers in a line
fn match_numbers(line: &str, line_num: usize) -> Option<Vec<SemanticToken>> {
    let mut tokens = Vec::new();

    // Simple manual number matching
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch.is_ascii_digit() {
            let mut end = start + ch.len_utf8();
            let mut has_dot = false;

            // Continue while we have digits or one decimal point
            while let Some((pos, c)) = chars.peek() {
                if c.is_ascii_digit() {
                    end = *pos + c.len_utf8();
                    chars.next();
                } else if *c == '.' && !has_dot {
                    has_dot = true;
                    end = *pos + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }

            tokens.push(SemanticToken {
                delta_line: line_num as u32,
                delta_start: start as u32,
                length: (end - start) as u32,
                token_type: CwSemanticTokenType::Number.as_u32(),
                token_modifiers_bitset: 0,
            });
        }
    }

    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

/// Match comments in a line
fn match_comments(line: &str, line_num: usize) -> Option<Vec<SemanticToken>> {
    // Simple comment detection
    if let Some(comment_start) = line.find('#') {
        Some(vec![SemanticToken {
            delta_line: line_num as u32,
            delta_start: comment_start as u32,
            length: (line.len() - comment_start) as u32,
            token_type: CwSemanticTokenType::Comment.as_u32(),
            token_modifiers_bitset: 0,
        }])
    } else {
        None
    }
}
