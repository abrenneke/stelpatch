use std::{ops::Range, str::FromStr};

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, token::literal};

use crate::{AstToken, StringError};

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
}

impl<'a> AstOperator<'a> {
    pub fn new(operator: &'a str, span: Range<usize>) -> Result<Self, StringError> {
        Ok(Self {
            operator: Operator::from_str(operator)?,
            value: AstToken {
                value: operator,
                span,
            },
        })
    }

    pub fn equals(span: Range<usize>) -> Self {
        Self::new("=", span).unwrap()
    }
}

pub(crate) fn operator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstOperator<'a>> {
    alt((
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
    .map(|(op, span)| AstOperator {
        operator: Operator::from_str(op).unwrap(),
        value: AstToken { value: op, span },
    })
    .parse_next(input)
}
