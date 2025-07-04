use std::{ops::Range, str::FromStr};

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, token::literal};

use crate::{
    AstComment, AstNode, AstToken, StringError, opt_trailing_comment, opt_ws_and_comments,
};

/// An operator that can appear between a key and a value in an entity, like a > b. Usually this is = but it depends on the implementation.
/// For our purposes it doesn't really matter, we just have to remember what it is.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equals,
    NotEqual,
    MinusEquals,
    PlusEquals,
    MultiplyEquals,
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::Equals => "=",
            Self::NotEqual => "!=",
            Self::MinusEquals => "-=",
            Self::PlusEquals => "+=",
            Self::MultiplyEquals => "*=",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for Operator {
    type Err = StringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" => Ok(Operator::GreaterThan),
            ">=" => Ok(Operator::GreaterThanOrEqual),
            "<" => Ok(Operator::LessThan),
            "<=" => Ok(Operator::LessThanOrEqual),
            "=" => Ok(Operator::Equals),
            "!=" => Ok(Operator::NotEqual),
            "-=" => Ok(Operator::MinusEquals),
            "+=" => Ok(Operator::PlusEquals),
            "*=" => Ok(Operator::MultiplyEquals),
            _ => Err(StringError::new(format!("Unknown operator: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstOperator<'a> {
    pub operator: Operator,
    pub value: AstToken<'a>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstOperator<'a> {
    pub fn new(operator: &'a str, span: Range<usize>) -> Result<Self, StringError> {
        Ok(Self {
            operator: Operator::from_str(operator)?,
            value: AstToken::new(operator, span),
            leading_comments: vec![],
            trailing_comment: None,
        })
    }

    pub fn equals(span: Range<usize>) -> Self {
        Self::new("=", span).unwrap()
    }
}

impl<'a> AstNode<'a> for AstOperator<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

pub(crate) fn operator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstOperator<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (op, span) = alt((
        literal(">="),
        literal("<="),
        literal("!="),
        literal("+="),
        literal("-="),
        literal("*="),
        literal("="),
        literal(">"),
        literal("<"),
    ))
    .with_span()
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstOperator {
        operator: Operator::from_str(op).unwrap(),
        value: AstToken::new(op, span),
        leading_comments,
        trailing_comment,
    })
}
